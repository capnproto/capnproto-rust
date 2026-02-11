#![cfg(feature = "alloc")]

use capnp::{any_pointer, message, text};

#[test]
#[should_panic(expected = "text size too large")]
pub fn init_text_overflow() {
    let mut msg1 = message::Builder::new_default();
    let root: any_pointer::Builder = msg1.get_root().unwrap();

    let _: text::Builder = root.initn_as(u32::MAX);
}
