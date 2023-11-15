#![no_main]

use capnp::{message, serialize_packed};
use libfuzzer_sys::fuzz_target;

fn try_go(mut data: &[u8]) -> ::capnp::Result<()> {
    let mut read_buffer = [capnp::word(0, 0, 0, 0, 0, 0, 0, 0); 512];
    let _message = serialize_packed::read_message_no_alloc(
        &mut data,
        capnp::Word::words_to_bytes_mut(&mut read_buffer),
        *message::ReaderOptions::new().traversal_limit_in_words(Some(256)),
    )?;
    Ok(())
}

fuzz_target!(|data: &[u8]| {
    let _ = try_go(data);
});
