use capnp;
use capnp::message::MessageBuilder;
use zmq;
use std;
use rand;
use rand::Rng;
use capnp_zmq;
use explorers_capnp::Observation;
use time;

struct Pixel {
    red : u8,
    green : u8,
    blue : u8
}

fn fudge(x : u8) -> u8 {
    let error = rand::task_rng().gen_range::<i16>(-60, 60);
    let y = x as i16 + error;
    if y < 0 { return 0; }
    if y > 255 { return 255; }
    return y as u8;
}

struct Image {
    width : u32,
    height : u32,
    pixels : ~[Pixel]
}

impl Image {

    // quick and dirty parsing of a PPM image
    fn load(file : &std::path::Path) -> std::io::IoResult<Image> {
        use std::io::{Open, Read};
        match std::io::File::open_mode(file, Open, Read) {
            Err(_e) => fail!("could not open"),
            Ok(reader) => {
                let mut buffered = std::io::BufferedReader::new(reader);
                match buffered.read_line() {
                    Ok(s) => {
                        assert!(s.trim() == "P6");
                    }
                    Err(_e) => fail!("premature end of file")
                }
                let (width, height) = match buffered.read_line() {
                    Ok(s) => {
                        let dims : ~[&str] = s.split(' ').collect();
                        if dims.len() != 2 { fail!("could not read dimensions") }
                        (from_str::<u32>(dims[0].trim()).unwrap(), from_str::<u32>(dims[1].trim()).unwrap())
                    }
                    Err(_e) => { fail!("premature end of file") }
                };
                match buffered.read_line() {
                    Ok(s) => { assert!(s.trim() == "255") }
                    Err(_e) => fail!("premature end of file")
                }
                let mut result = Image { width : width, height : height, pixels : ~[] };
                for _ in range(0, width * height) {
                    result.pixels.push(
                        Pixel {
                            red : try!(buffered.read_u8()),
                            green : try!(buffered.read_u8()),
                            blue : try!(buffered.read_u8())
                        });
                }
                return Ok(result);
            }
        }
    }

    fn get_pixel(&self, x : u32, y : u32) -> Pixel {
        assert!(x < self.width);
        assert!(y < self.height);
        self.pixels[((y * self.width) + x)]
    }

    fn take_measurement(&self, x : f32, y : f32, obs : Observation::Builder) {

        assert!(x >= 0.0); assert!(y >= 0.0); assert!(x < 1.0); assert!(y < 1.0);

        obs.set_timestamp(time::now().to_timespec().sec);
        obs.set_x(x);
        obs.set_y(y);

        let pixel = self.get_pixel((x * self.width as f32).floor() as u32,
                                   (y * self.height as f32).floor() as u32);

        obs.set_red(fudge(pixel.red));
        obs.set_green(fudge(pixel.green));
        obs.set_blue(fudge(pixel.blue));

        add_diagnostic(obs);
    }
}

static WORDS : [&'static str, .. 20] = [
   "syntax", "personality", "rhymist", "shopwalker", "gooseskin", "overtask",
    "churme", "heathen", "economiser", "radium", "attainable", "nonius", "knaggy",
    "inframedian", "tamperer", "disentitle", "horary", "morsure", "bonnaz", "alien",
];

// With small probability, add a gibberish warning to the observation.
fn add_diagnostic<'a>(obs : Observation::Builder<'a>) {
    let mut rng = rand::task_rng();
    if rng.gen_range::<u16>(0, 3000) < 2 {
        let mut warning = ~"";
        warning.push_str(rng.choose(WORDS));
        warning.push_str(" ");
        warning.push_str(rng.gen_ascii_str(8));
        warning.push_str(" ");
        warning.push_str(rng.choose(WORDS));
        obs.init_diagnostic().set_warning(warning);
    }
}

pub fn main () {

    let args = std::os::args();
    if args.len() != 3 {
        println!("usage: {} explorer [filename]", args[0]);
        return;
    }

    let image = Image::load(&std::path::Path::new(args[2])).unwrap();

    let mut context = zmq::Context::new();
    let mut publisher = context.socket(zmq::PUB).unwrap();
    assert!(publisher.connect("tcp://localhost:5555").is_ok());

    let mut rng = rand::task_rng();
    let mut x = rng.gen_range::<f32>(0.0, 1.0);
    let mut y = rng.gen_range::<f32>(0.0, 1.0);

    loop {
        x += rng.gen_range::<f32>(-0.01, 0.01);
        y += rng.gen_range::<f32>(-0.01, 0.01);

        if x >= 1.0 { x -= 1.0 }
        if y >= 1.0 { y -= 1.0 }
        if x < 0.0 { x += 1.0 }
        if y < 0.0 { y += 1.0 }

        let mut message = capnp::message::MallocMessageBuilder::new_default();
        let obs = message.init_root::<Observation::Builder>();
        image.take_measurement(x, y, obs);
        capnp_zmq::send(&mut publisher, &mut message).unwrap();


        std::io::timer::sleep(5);
    }

}
