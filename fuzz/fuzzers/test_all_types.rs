#![no_main]
extern crate libfuzzer_sys;
extern crate capnp;

use capnp::{serialize, message};
use test_capnp::test_all_types;

pub mod test_capnp;

fn traverse(v: test_all_types::Reader) -> ::capnp::Result<()> {
    v.get_int64_field();
    try!(v.get_text_field());
    try!(v.get_data_field());
    try!(v.get_struct_field());

    let structs = try!(v.get_struct_list());
    for s in structs.iter() {
        try!(s.get_text_field());
    }

    try!(v.get_bool_list());
    Ok(())
}

fn try_go(mut data: &[u8]) -> ::capnp::Result<()> {
    let orig_data = data;
    let message_reader = try!(serialize::read_message(
        &mut data,
        *message::ReaderOptions::new().traversal_limit_in_words(4 * 1024)));
    assert!(orig_data.len() > data.len());

    let root: test_all_types::Reader = try!(message_reader.get_root());
    try!(root.total_size());
    try!(traverse(root));

    let mut message = message::Builder::new_default();
    try!(message.set_root(root));
    {
        let root_builder = try!(message.get_root::<test_all_types::Builder>());
        try!(root_builder.total_size());
        try!(traverse(root_builder.as_reader()));
    }

    // init_root() will zero the previous value
    let mut new_root = message.init_root::<test_all_types::Builder>();

    try!(new_root.set_struct_field(root));
    Ok(())
}

#[export_name="rust_fuzzer_test_input"]
pub extern fn go(data: &[u8]) {
    let _ = try_go(data);
}
