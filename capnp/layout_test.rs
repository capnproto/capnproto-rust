/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use layout;

#[test]
fn simple_raw_data_struct() {
    let data : layout::AlignedData<[u8, .. 16]> = layout::AlignedData {
        _dummy: 0,
        words : [0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
                 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]
    };

    let reader = unsafe { layout::PointerReader::get_root_unchecked(
        std::mem::transmute(data.words.unsafe_ref(0))).get_struct(std::ptr::null()) };

    assert_eq!(0xefcdab8967452301u64, reader.get_data_field::<u64>(0));
    assert_eq!(0, reader.get_data_field::<u64>(1));
    assert_eq!(0x67452301u32, reader.get_data_field::<u32>(0));
    assert_eq!(0xefcdab89u32, reader.get_data_field::<u32>(1));
    assert_eq!(0, reader.get_data_field::<u32>(2));
    assert_eq!(0x2301u16, reader.get_data_field::<u16>(0));
    // TODO the rest of uints.

    // Bits.
    assert_eq!(reader.get_bool_field(0), true);
    assert_eq!(reader.get_bool_field(1), false);
    assert_eq!(reader.get_bool_field(2), false);
    assert_eq!(reader.get_bool_field(3), false);
    assert_eq!(reader.get_bool_field(4), false);
    assert_eq!(reader.get_bool_field(5), false);
    assert_eq!(reader.get_bool_field(6), false);
    assert_eq!(reader.get_bool_field(7), false);

    assert_eq!(reader.get_bool_field(8),  true);
    assert_eq!(reader.get_bool_field(9),  true);
    assert_eq!(reader.get_bool_field(10), false);
    assert_eq!(reader.get_bool_field(11), false);
    assert_eq!(reader.get_bool_field(12), false);
    assert_eq!(reader.get_bool_field(13), true);
    assert_eq!(reader.get_bool_field(14), false);
    assert_eq!(reader.get_bool_field(15), false);

    assert_eq!(reader.get_bool_field(63), true);
    assert_eq!(reader.get_bool_field(64), false);
}
