extern mod capnp;
extern mod zmq;

pub mod common;
pub mod explorers_capnp;

pub fn main() {
    use explorers_capnp::Grid;

    let mut context = zmq::Context::new();
    let mut requester = context.socket(zmq::REQ).unwrap();

    assert!(requester.connect("tcp://localhost:5556").is_ok());

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

        std::io::timer::sleep(1000);
    }
}
