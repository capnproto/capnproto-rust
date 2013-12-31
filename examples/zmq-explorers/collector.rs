extern mod capnp;
extern mod zmq;

pub mod explorers_capnp;

pub fn main() {

    let mut context = zmq::Context::new();
    let mut subscriber = context.socket(zmq::SUB).unwrap();

    assert!(subscriber.bind("tcp://*:5555").is_ok());

    capnp::message::MessageBuilder::new_default(|message| {
            let grid = message.init_root::<explorers_capnp::Grid::Builder>();
            let cells = grid.init_cells(100);
            for ii in range::<uint>(0, cells.size()) {
                cells.init(ii, 100);
            }
        });
}
