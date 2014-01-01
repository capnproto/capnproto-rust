extern mod capnp;
extern mod zmq;

pub mod common;
pub mod explorers_capnp;


fn write_ppm(path : &std::path::Path, grid : explorers_capnp::Grid::Reader) {
    match std::io::File::open_mode(path, std::io::Truncate, std::io::Write) {
        None => fail!("could not open"),
        Some(ref mut writer) => {
            writeln!(writer, "P6");

            let cells = grid.get_cells();
            let width = cells.size();
            assert!(width > 0);
            let height = cells[0].size();

            writeln!(writer, "{} {}", width, height);
            writeln!(writer, "255");

            for x in range(0, width) {
                assert!(cells[x].size() == height);
            }

            for y in range(0, height) {
                for x in range(0, width) {
                    let cell = cells[x][y];
                    writer.write_u8((cell.get_mean_red()).floor() as u8);
                    writer.write_u8((cell.get_mean_green()).floor() as u8);
                    writer.write_u8((cell.get_mean_blue()).floor() as u8);
                }
            }
        }
    }
}

pub fn main() {
    use explorers_capnp::Grid;

    let mut context = zmq::Context::new();
    let mut requester = context.socket(zmq::REQ).unwrap();

    assert!(requester.connect("tcp://localhost:5556").is_ok());

    let mut c : uint = 0;

    loop {
        requester.send([], 0);

        let mut frames = ~[];
        loop {
            match requester.recv_msg(0) {
                Ok(m) => frames.push(m),
                Err(_) => fail!()
            }
            match requester.get_rcvmore() {
                Ok(true) => (),
                Ok(false) => break,
                Err(_) => fail!()
            }
        }

        let segments = common::frames_to_segments(frames);
        let reader = capnp::message::MessageReader::new(segments,
                                                        capnp::message::DEFAULT_READER_OPTIONS);

        let grid = reader.get_root::<Grid::Reader>();

        println!("{}", grid.get_latest_timestamp());

        let filename = std::path::Path::new(format!("out{}.ppm", c));
        write_ppm(&filename, grid);

        c += 1;
        std::io::timer::sleep(1000);
    }
}
