#![no_main]

capnp::generated_code!(pub mod test_capnp);

use capnp::{message, serialize};
use libfuzzer_sys::fuzz_target;
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
    for idx in 0..bools.len() {
        bools.get(idx);
    }

    let enums = v.get_enum_list()?;
    for idx in 0..enums.len() {
        enums.get(idx)?;
    }

    let int8s = v.get_int8_list()?;
    for idx in 0..int8s.len() {
        int8s.get(idx);
    }

    Ok(())
}

fn assert_equal(v1: test_all_types::Reader, v2: test_all_types::Reader, depth: usize) {
    assert!(depth < 100);
    assert_eq!(v1.get_int8_field(), v2.get_int8_field());
    assert_eq!(v1.get_int16_field(), v2.get_int16_field());
    assert_eq!(v1.get_int32_field(), v2.get_int32_field());
    assert_eq!(v1.get_int64_field(), v2.get_int64_field());
    assert_eq!(
        v1.get_float32_field().to_bits(),
        v2.get_float32_field().to_bits()
    );
    assert_eq!(
        v1.get_float64_field().to_bits(),
        v2.get_float64_field().to_bits()
    );

    if let Ok(t) = v1.get_text_field() {
        assert_eq!(t, v2.get_text_field().unwrap());
    }
    if let Ok(d) = v1.get_data_field() {
        assert_eq!(d, v2.get_data_field().unwrap());
    }

    if v1.has_struct_field() {
        if let Ok(s) = v1.get_struct_field() {
            assert_equal(s, v2.get_struct_field().unwrap(), depth + 1);
        }
    }

    if let Ok(l1) = v1.get_void_list() {
        let l2 = v2.get_void_list().unwrap();
        assert_eq!(l1.len(), l2.len());
    }

    if let Ok(l1) = v1.get_bool_list() {
        let l2 = v2.get_bool_list().unwrap();
        assert_eq!(l1.len(), l2.len());
        for ii in 0..l1.len() {
            assert_eq!(l1.get(ii), l2.get(ii));
        }
    }

    if let Ok(l1) = v1.get_enum_list() {
        let l2 = v2.get_enum_list().unwrap();
        assert_eq!(l1.len(), l2.len());
        for ii in 0..l1.len() {
            assert_eq!(l1.get(ii), l2.get(ii));
        }
    }

    if let Ok(l1) = v1.get_int16_list() {
        let l2 = v2.get_int16_list().unwrap();
        assert_eq!(l1.len(), l2.len());
        for ii in 0..l1.len() {
            assert_eq!(l1.get(ii), l2.get(ii));
        }
    }

    if let Ok(l1) = v1.get_text_list() {
        let l2 = v2.get_text_list().unwrap();
        assert_eq!(l1.len(), l2.len());
        for ii in 0..l1.len() {
            if let Ok(s1) = l1.get(ii) {
                assert_eq!(s1, l2.get(ii).unwrap());
            }
        }
    }

    if let Ok(l1) = v1.get_struct_list() {
        let l2 = v2.get_struct_list().unwrap();
        assert_eq!(l1.len(), l2.len());
        for ii in 0..l1.len() {
            assert_equal(l1.get(ii), l2.get(ii), depth + 1);
        }
    }
}

fn try_go(mut data: &[u8]) -> ::capnp::Result<()> {
    let orig_data = data;
    let message_reader = serialize::read_message(
        &mut data,
        *message::ReaderOptions::new().traversal_limit_in_words(Some(4 * 1024)),
    )?;
    assert!(orig_data.len() > data.len());

    let root: test_all_types::Reader = message_reader.get_root()?;
    root.total_size()?;
    traverse(root)?;

    let mut message = message::Builder::new_default();
    message.set_root(root)?;

    assert_equal(
        root,
        message
            .get_root_as_reader::<test_all_types::Reader>()
            .unwrap(),
        0,
    );

    {
        let mut root_builder = message.get_root::<test_all_types::Builder>()?;
        root_builder.total_size()?;

        let mut sl = root_builder.reborrow().get_struct_list()?;
        for idx in 0..sl.len() {
            let mut s = sl.reborrow().get(idx);
            s.reborrow().get_text_list()?;
            s.set_text_list(root.get_text_list()?)?;

            s.reborrow().get_bool_list()?;
            s.reborrow().set_bool_list(root.get_bool_list()?)?;

            s.reborrow().get_enum_list()?;
            s.reborrow().set_enum_list(root.get_enum_list()?)?;

            s.reborrow().get_int32_list()?;
            s.reborrow().set_int32_list(root.get_int32_list()?)?;
        }

        root_builder.set_struct_field(root)?;
        {
            let mut list = root_builder.reborrow().init_struct_list(2);
            list.set_with_caveats(0, root)?;
            list.set_with_caveats(1, root)?;
        }

        traverse(root_builder.into_reader())?;
    }

    {
        // init_root() will zero the previous value
        let mut new_root = message.init_root::<test_all_types::Builder>();
        new_root.set_struct_field(root)?;
    }

    Ok(())
}

fuzz_target!(|data: &[u8]| {
    let _ = try_go(data);
});
