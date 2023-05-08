use capnp::dynamic_value;
use fill_random_values::Filler;

pub mod shapes_capnp {
    include!(concat!(env!("OUT_DIR"), "/shapes_capnp.rs"));
}

pub mod fill_capnp {
    include!(concat!(env!("OUT_DIR"), "/fill_capnp.rs"));
}

#[derive(Clone, Copy, Debug)]
struct Viewport {
    // center
    x: f64,
    y: f64,

    width: f64,
    height: f64,
}

impl Viewport {
    fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn upper_left(&self) -> (f64, f64) {
        (self.x - (self.width / 2.0), self.y - (self.height / 2.0))
    }

    fn denormalize(&self, x: f64, y: f64) -> (f64, f64) {
        let (ulx, uly) = self.upper_left();
        (ulx + (x * self.width), uly + (y * self.height))
    }

    fn subview(&self, x: f64, y: f64, w: f64, h: f64) -> Self {
        let (ulx, uly) = self.upper_left();
        Self {
            x: ulx + (x * self.width),
            y: uly + (y * self.width),
            width: w * self.width,
            height: h * self.height,
        }
    }

    fn diag_len(&self) -> f64 {
        (self.width * self.width + self.height * self.height).sqrt()
    }
}

fn color_to_svg(color: shapes_capnp::color::Reader) -> String {
    format!(
        "rgb({}, {}, {})",
        color.get_red(),
        color.get_green(),
        color.get_blue()
    )
}

struct SvgBuilder {
    counter: u32,
}

impl SvgBuilder {
    fn new() -> Self {
        Self { counter: 0 }
    }

    fn canvas_to_svg(
        &mut self,
        view: Viewport,
        canvas: shapes_capnp::canvas::Reader,
    ) -> ::capnp::Result<String> {
        if !canvas.has_background_color() {
            // probably recursion depth was exceeded
            return Ok("".into());
        }
        let bc = color_to_svg(canvas.get_background_color()?);
        let (vx, vy) = view.upper_left();
        let mut result = format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{bc}\" />\n",
            vx, vy, view.width, view.height
        );
        let clipr = format!("clip-{}", self.counter);
        self.counter += 1;
        result += &format!("<defs><clipPath id=\"{clipr}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath></defs>\n", vx, vy, view.width, view.height);
        result += &format!("<g clip-path=\"url(#{clipr})\">\n");
        for line in canvas.get_lines()? {
            if line.has_start() {
                let start = line.get_start()?;
                let (x1, y1) = view.denormalize(start.get_x(), start.get_y());
                let end = line.get_end()?;
                let (x2, y2) = view.denormalize(end.get_x(), end.get_y());
                let c = color_to_svg(line.get_color()?);
                let sw = line.get_thickness() * view.diag_len() / 100.0;
                result +=
                    &format!("<line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" stroke=\"{c}\" stroke-width=\"{sw}\" stroke-linecap=\"round\" clip-path=\"url(#{clipr})\"/>\n");
            }
        }

        for circ in canvas.get_circles()? {
            if circ.has_center() {
                let center = circ.get_center()?;
                let (cx, cy) = view.denormalize(center.get_x(), center.get_y());
                let r = circ.get_radius() * view.diag_len();
                let c = color_to_svg(circ.get_fill_color()?);
                result += &format!("<circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" fill=\"{c}\" clip-path=\"url(#{clipr})\"/>\n");
            }
        }

        for sc in canvas.get_subcanvases()? {
            let center = sc.get_center()?;
            let v1 = view.subview(
                center.get_x(),
                center.get_y(),
                sc.get_width(),
                sc.get_height(),
            );
            if sc.has_canvas() {
                result += &self.canvas_to_svg(v1, sc.get_canvas()?)?;
            }
        }
        result += "</g>\n";
        Ok(result)
    }

    fn base_to_svg(&mut self, canvas: shapes_capnp::canvas::Reader) -> ::capnp::Result<String> {
        let view = Viewport::new(256.0, 256.0, 512.0, 512.0);
        let c = self.canvas_to_svg(view, canvas)?;
        Ok(format!(
            r#"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512" width="512" height="512">
        {c}
       </svg>"#
        ))
    }
}

pub fn main() {
    let mut message = ::capnp::message::Builder::new_default();
    let mut canvas = message.init_root::<shapes_capnp::canvas::Builder>();

    let mut filler = Filler::new(::rand::thread_rng(), 10);
    let dynamic: dynamic_value::Builder = canvas.reborrow().into();
    filler.fill(dynamic.downcast()).unwrap();

    let reader = canvas.into_reader();
    let mut svg_builder = SvgBuilder::new();
    println!("{}", svg_builder.base_to_svg(reader).unwrap());
}
