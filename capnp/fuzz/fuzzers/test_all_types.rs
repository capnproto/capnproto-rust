#![no_main]
extern crate libfuzzer_sys;
extern crate capnp;

pub mod test_capnp {
  include!(concat!(env!("OUT_DIR"), "/test_capnp.rs"));
}

use capnp::{serialize, message};
use test_capnp::test_all_types;


fn traverse(v: test_all_types::Reader) -> ::capnp::Result<()> {
    v.get_int64_field();
    v.get_float32_field();
    v.get_float64_field();
    v.get_text_field()?;
    v.get_data_field()?;
    v.get_struct_field()?;
    v.get_enum_field()?;

    let structs = v.get_struct_list()?;
    for s in structs.iter() {
        s.get_text_field()?;
        s.get_data_field()?;
    }

    let bools = v.get_bool_list()?;
    for idx in 0 .. bools.len() {
        bools.get(idx);
    }

    let enums = v.get_enum_list()?;
    for idx in 0 .. enums.len() {
        enums.get(idx)?;
    }

    let int8s = v.get_int8_list()?;
    for idx in 0 .. int8s.len() {
        int8s.get(idx);
    }

    Ok(())
}

fn try_go(mut data: &[u8]) -> ::capnp::Result<()> {
    let orig_data = data;
    let message_reader = serialize::read_message(
        &mut data,
        *message::ReaderOptions::new().traversal_limit_in_words(4 * 1024))?;
    assert!(orig_data.len() > data.len());

    let root: test_all_types::Reader = message_reader.get_root()?;
    root.total_size()?;
    traverse(root)?;

    let mut message = message::Builder::new_default();
    message.set_root(root)?;
    {
        let mut root_builder = message.get_root::<test_all_types::Builder>()?;
        root_builder.total_size()?;

        root_builder.set_struct_field(root)?;
        {
            let list = root_builder.reborrow().init_struct_list(2);
            list.set_with_caveats(0,  root)?;
            list.set_with_caveats(1,  root)?;
        }

        traverse(root_builder.into_reader())?;
    }

    // init_root() will zero the previous value
    let mut new_root = message.init_root::<test_all_types::Builder>();
    new_root.set_struct_field(root)?;

    Ok(())
}

#[export_name="rust_fuzzer_test_input"]
pub extern fn go(data: &[u8]) {
    let _ = try_go(data);
}
