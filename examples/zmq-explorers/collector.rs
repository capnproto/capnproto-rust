extern mod capnp;
extern mod zmq;

pub mod explorers_capnp;


fn frames_to_segments<'a>(frames : &'a [zmq::Message] ) -> ~ [&'a [capnp::common::Word]] {
    let mut result : ~ [&'a [capnp::common::Word]] = box [];

    for frame in frames.iter() {
        unsafe {
            let slice = frame.with_bytes(|v|
                    std::unstable::raw::Slice { data : v.unsafe_ref(0),
                                                len : v.len() } );
            result.push(std::cast::transmute(slice));
        }
    }

    return result;
}

pub fn main() {

    let mut context = zmq::Context::new();
    let mut subscriber = context.socket(zmq::SUB).unwrap();

    assert!(subscriber.bind("tcp://*:5555").is_ok());
    assert!(subscriber.set_subscribe([]).is_ok());

    capnp::message::MessageBuilder::new_default::<()>(|message| {
            let grid = message.init_root::<explorers_capnp::Grid::Builder>();
            let cells = grid.init_cells(100);
            for ii in range::<uint>(0, cells.size()) {
                cells.init(ii, 100);
            }

            loop {
                let mut frames = ~[];
                loop {
                    match subscriber.recv_msg(0) {
                        Ok(m) => frames.push(m),
                        Err(_) => fail!()
                    }

                    match subscriber.get_rcvmore() {
                        Ok(true) => (),
                        Ok(false) => break,
                        Err(_) => fail!()
                    }
                }

                println!("got {} frames", frames.len());

                let segments = frames_to_segments(frames);

                let reader = capnp::message::MessageReader::new(segments, capnp::message::DEFAULT_READER_OPTIONS);

                let obs = reader.get_root::<explorers_capnp::Observation::Reader>();

                println!("x, y: {}, {}", obs.get_x(), obs.get_y());
            }

        });
}
