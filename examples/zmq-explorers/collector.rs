extern mod capnp;
extern mod zmq;

pub mod capnp_zmq;
pub mod explorers_capnp;

static GRID_WIDTH : uint = 100;
static GRID_HEIGHT : uint = 100;

pub fn main() {
    use explorers_capnp::Observation;

    let mut context = zmq::Context::new();
    let mut subscriber = context.socket(zmq::SUB).unwrap();
    let mut responder = context.socket(zmq::REP).unwrap();

    assert!(subscriber.bind("tcp://*:5555").is_ok());
    assert!(subscriber.set_subscribe([]).is_ok());
    assert!(responder.bind("tcp://*:5556").is_ok());

    let mut poll_items = [responder.as_poll_item(zmq::POLLIN),
                          subscriber.as_poll_item(zmq::POLLIN)];

    capnp::message::MessageBuilder::new_default::<()>(|message| {
            let grid = message.init_root::<explorers_capnp::Grid::Builder>();
            let cells = grid.init_cells(GRID_WIDTH);
            for ii in range::<uint>(0, cells.size()) {
                cells.init(ii, GRID_HEIGHT);
            }

            loop {

                match zmq::poll(poll_items, -1) { Ok(()) => {}, Err(_) => fail!("poll failure") };

                if (poll_items[0].revents & zmq::POLLIN) != 0 {

                    assert!(responder.recv_msg(0).is_ok())
                    capnp_zmq::send(&mut responder, message);

                } else if (poll_items[1].revents & zmq::POLLIN) != 0 {
                    // there's an observation waiting for us

                    let frames = capnp_zmq::recv(&mut subscriber).unwrap();
                    let segments = capnp_zmq::frames_to_segments(frames);
                    let reader = capnp::message::MessageReader::new(segments,
                                                                    capnp::message::DEFAULT_READER_OPTIONS);
                    let obs = reader.get_root::<Observation::Reader>();

                    if obs.get_x() >= 1.0 || obs.get_x() < 0.0 ||
                        obs.get_y() >= 1.0 || obs.get_y() < 0.0 {
                        error!("out of range");
                    }

                    match obs.get_diagnostic().which() {
                        Some(Observation::Diagnostic::Ok(())) => {}
                        Some(Observation::Diagnostic::Warning(s)) => {
                            println!("received diagnostic: {}", s);
                        }
                        None => {}
                    }

                    let x = (obs.get_x() * GRID_WIDTH as f32).floor() as uint;
                    let y = (obs.get_y() * GRID_HEIGHT as f32).floor() as uint;

                    grid.set_latest_timestamp(obs.get_timestamp());
                    grid.set_number_of_updates(grid.get_number_of_updates() + 1);

                    let cell = cells[x][y];
                    cell.set_latest_timestamp(obs.get_timestamp());

                    let n = cell.get_number_of_updates();
                    cell.set_mean_red((n as f32 * cell.get_mean_red() + obs.get_red() as f32) / (n + 1) as f32);
                    cell.set_mean_green(
                        (n as f32 * cell.get_mean_green() + obs.get_green() as f32) / (n + 1) as f32);
                    cell.set_mean_blue(
                        (n as f32 * cell.get_mean_blue() + obs.get_blue() as f32) / (n + 1) as f32);
                    cell.set_number_of_updates(n + 1);
                }

            }

        });
}
