#![cfg(feature = "alloc")]

use capnp::message;
use capnp::serialize;
use capnp::text;

#[test]
fn message_size_in_words() {
    let mut testdata = b"Hello, World!".to_vec();
    let testdata = text::Builder::new(&mut testdata);

    let mut message = message::Builder::new_default();
    message.set_root(testdata.into_reader()).unwrap();
    assert_eq!(message.size_in_words(), 3);

    let buffer = serialize::write_message_to_words(&message);
    assert_eq!(buffer.len(), 32);

    let message =
        serialize::read_message_from_flat_slice(&mut &*buffer, Default::default()).unwrap();
    assert_eq!(message.size_in_words(), 3);
}
