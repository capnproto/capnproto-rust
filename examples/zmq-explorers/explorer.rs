extern mod capnp;
extern mod extra;

use std::rand::Rng;

pub mod explorers_capnp;

struct Pixel {
    red : u8,
    green : u8,
    blue : u8
}

fn fudge(x : u8) -> u8 {
    let error = std::rand::task_rng().gen_range::<i16>(-20, 20);
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
    fn load(file : &std::path::Path) -> Image {
        use std::io::{Open, Read};
        match std::io::File::open_mode(file, Open, Read) {
            None => fail!("could not open"),
            Some(reader) => {
                let mut buffered = std::io::buffered::BufferedReader::new(reader);
                match buffered.read_line() {
                    Some(s) => {
                        assert!(s.trim() == "P6");
                    }
                    None => fail!("premature end of file")
                }
                let (width, height) = match buffered.read_line() {
                    Some(s) => {
                        let dims : ~[&str] = s.split(' ').collect();
                        if dims.len() != 2 { fail!("could not read dimensions") }
                        (from_str::<u32>(dims[0].trim()).unwrap(), from_str::<u32>(dims[1].trim()).unwrap())
                    }
                    None => { fail!("premature end of file") }
                };
                match buffered.read_line() {
                    Some(s) => { assert!(s.trim() == "255") }
                    None => fail!("premature end of file")
                }
                let mut result = Image { width : width, height : height, pixels : ~[] };
                for _ in range(0, width * height) {
                    result.pixels.push(
                        Pixel {
                            red : buffered.read_u8(),
                            green : buffered.read_u8(),
                            blue : buffered.read_u8()
                        });
                }
                return result;
            }
        }
    }

    fn get_pixel(&self, x : u32, y : u32) -> Pixel {
        assert!(x < self.width);
        assert!(y < self.height);
        self.pixels[((y * self.width) + x)]
    }

    fn take_measurement(&self, x : f32, y : f32) -> Pixel {

        assert!(x >= 0.0); assert!(y >= 0.0); assert!(x < 1.0); assert!(y < 1.0);

        let mut result = self.get_pixel((x * self.width as f32).floor() as u32,
                                        (y * self.height as f32).floor() as u32);

        result.red = fudge(result.red);
        result.green = fudge(result.green);
        result.blue = fudge(result.blue);

        result
    }
}

static WORDS : [&'static str, .. 20] = [
   "syntax", "personality", "rhymist", "shopwalker", "gooseskin", "overtask",
    "churme", "heathen", "economiser", "radium", "attainable", "nonius", "knaggy",
    "inframedian", "tamperer", "disentitle", "horary", "morsure", "bonnaz", "alien",
];


fn add_diagnostic<'a>(obs : explorers_capnp::Observation::Builder<'a>) {
    let mut rng = std::rand::task_rng();
    if rng.gen_range::<u16>(0, 1000) < 200 {
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
    if args.len() != 2 {
        error!("please supply a file name");
        return;
    }

    let image = Image::load(&std::path::Path::new(args[1]));

    let mut x : f32 = 0.5;
    let mut y : f32 = 0.5;

    loop {
        x += std::rand::task_rng().gen_range::<f32>(-0.01, 0.01);
        y += std::rand::task_rng().gen_range::<f32>(-0.01, 0.01);

        if x >= 1.0 { x -= 1.0 }
        if y >= 1.0 { y -= 1.0 }
        if x < 0.0 { x += 1.0 }
        if y < 0.0 { y += 1.0 }

        let pixel = image.take_measurement(x,y);

        capnp::message::MessageBuilder::new_default(
            |message| {
                let obs = message.init_root::<explorers_capnp::Observation::Builder>();

                obs.set_timestamp(extra::time::now().to_timespec().sec);
                obs.set_x(x);
                obs.set_y(y);
                obs.set_red(pixel.red);
                obs.set_green(pixel.green);
                obs.set_blue(pixel.blue);
                add_diagnostic(obs);

                capnp::serialize::write_message(&mut std::io::stdout(), message);
            });

    }

}
