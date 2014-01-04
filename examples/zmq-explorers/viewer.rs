use capnp;
use zmq;
use capnp_zmq;
use std;
use extra;
use explorers_capnp::Grid;

enum OutputMode {
    Colors,
    Confidence
}

fn write_ppm(path : &std::path::Path, grid : Grid::Reader, mode : OutputMode) {
    match std::io::File::open_mode(path, std::io::Truncate, std::io::Write) {
        None => fail!("could not open"),
        Some(writer) => {
            let mut buffered = std::io::buffered::BufferedWriter::new(writer);
            writeln!(&mut buffered, "P6");

            let cells = grid.get_cells();
            let width = cells.size();
            assert!(width > 0);
            let height = cells[0].size();

            writeln!(&mut buffered, "{} {}", width, height);
            writeln!(&mut buffered, "255");

            for x in range(0, width) {
                assert!(cells[x].size() == height);
            }

            for y in range(0, height) {
                for x in range(0, width) {
                    let cell = cells[x][y];

                    match mode {
                        Colors => {
                            buffered.write_u8((cell.get_mean_red()).floor() as u8);
                            buffered.write_u8((cell.get_mean_green()).floor() as u8);
                            buffered.write_u8((cell.get_mean_blue()).floor() as u8);
                        }
                        Confidence => {
                            let mut age = extra::time::now().to_timespec().sec - cell.get_latest_timestamp();
                            if age < 0 { age = 0 };
                            age *= 25;
                            if age > 255 { age = 255 };
                            age = 255 - age;

                            let mut n = cell.get_number_of_updates();
                            n *= 10;
                            if n > 255 { n = 255 };

                            buffered.write_u8(0 as u8);

                            buffered.write_u8(n as u8);

                            buffered.write_u8(age as u8);
                        }
                    }
                }
            }

            buffered.flush()
        }
    }
}

pub fn main() {

    let mut context = zmq::Context::new();
    let mut requester = context.socket(zmq::REQ).unwrap();

    assert!(requester.connect("tcp://localhost:5556").is_ok());

    let mut c : uint = 0;

    loop {
        requester.send([], 0);

        let frames = capnp_zmq::recv(&mut requester).unwrap();
        let segments = capnp_zmq::frames_to_segments(frames);
        let reader = capnp::message::MessageReader::new(segments,
                                                        capnp::message::DEFAULT_READER_OPTIONS);
        let grid = reader.get_root::<Grid::Reader>();

        println!("{:05d}", grid.get_latest_timestamp());

        let filename = std::path::Path::new(format!("colors{}.ppm", c));
        write_ppm(&filename, grid, Colors);

        let filename = std::path::Path::new(format!("conf{}.ppm", c));
        write_ppm(&filename, grid, Confidence);

        c += 1;
        std::io::timer::sleep(5000);
    }
}
