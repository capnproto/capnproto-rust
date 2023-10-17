// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

#![allow(clippy::bool_assert_comparison)]

use crate::private::layout::PointerReader;

fn test_at_alignments(words: &[crate::Word], verify: &dyn Fn(PointerReader)) {
    verify(unsafe { PointerReader::get_root_unchecked(words.as_ptr() as *const u8) });

    #[cfg(all(feature = "unaligned", feature = "alloc"))]
    {
        let mut unaligned_data = crate::Vec::with_capacity((words.len() + 1) * 8);
        for offset in 0..8 {
            unaligned_data.clear();
            unaligned_data.resize(offset, 0);
            unaligned_data.extend(crate::Word::words_to_bytes(words));
            verify(unsafe {
                PointerReader::get_root_unchecked((unaligned_data[offset..]).as_ptr())
            });
        }
    }
}

#[test]
fn simple_raw_data_struct() {
    let data: &[crate::Word] = &[
        crate::word(0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00),
        crate::word(0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef),
    ];

    test_at_alignments(data, &verify);
    fn verify(reader: PointerReader) {
        let reader = reader.get_struct(None).unwrap();

        assert_eq!(0xefcdab8967452301u64, reader.get_data_field::<u64>(0));
        assert_eq!(0, reader.get_data_field::<u64>(1)); // past end of struct --> default value

        assert_eq!(0x67452301u32, reader.get_data_field::<u32>(0));
        assert_eq!(0xefcdab89u32, reader.get_data_field::<u32>(1));
        assert_eq!(0, reader.get_data_field::<u32>(2)); // past end of struct --> default value

        assert_eq!(0x2301u16, reader.get_data_field::<u16>(0));
        assert_eq!(0x6745u16, reader.get_data_field::<u16>(1));
        assert_eq!(0xab89u16, reader.get_data_field::<u16>(2));
        assert_eq!(0xefcdu16, reader.get_data_field::<u16>(3));
        assert_eq!(0u16, reader.get_data_field::<u16>(4)); // past end of struct --> default value
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

        assert_eq!(reader.get_bool_field(8), true);
        assert_eq!(reader.get_bool_field(9), true);
        assert_eq!(reader.get_bool_field(10), false);
        assert_eq!(reader.get_bool_field(11), false);
        assert_eq!(reader.get_bool_field(12), false);
        assert_eq!(reader.get_bool_field(13), true);
        assert_eq!(reader.get_bool_field(14), false);
        assert_eq!(reader.get_bool_field(15), false);

        assert_eq!(reader.get_bool_field(63), true);
        assert_eq!(reader.get_bool_field(64), false); // past end of struct --> default value
    }
}

#[test]
fn bool_list() {
    // [true, false, true, false,
    //  true, true, true, false,
    //  false, true]

    let data: &[crate::Word] = &[
        crate::word(0x01, 0x00, 0x00, 0x00, 0x51, 0x00, 0x00, 0x00),
        crate::word(0x75, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),
    ];

    test_at_alignments(data, &verify);
    fn verify(pointer_reader: PointerReader) {
        use crate::private::layout::PrimitiveElement;
        use crate::traits::FromPointerReader;

        let reader = pointer_reader
            .get_list(crate::private::layout::ElementSize::Bit, None)
            .unwrap();

        assert_eq!(reader.len(), 10);
        assert_eq!(bool::get(&reader, 0), true);
        assert_eq!(bool::get(&reader, 1), false);
        assert_eq!(bool::get(&reader, 2), true);
        assert_eq!(bool::get(&reader, 3), false);
        assert_eq!(bool::get(&reader, 4), true);
        assert_eq!(bool::get(&reader, 5), true);
        assert_eq!(bool::get(&reader, 6), true);
        assert_eq!(bool::get(&reader, 7), false);
        assert_eq!(bool::get(&reader, 8), false);
        assert_eq!(bool::get(&reader, 9), true);

        let reader =
            crate::primitive_list::Reader::<bool>::get_from_pointer(&pointer_reader, None).unwrap();

        assert_eq!(reader.len(), 10);
        assert_eq!(reader.get(0), true);
        assert_eq!(reader.get(1), false);
        assert_eq!(reader.get(2), true);
        assert_eq!(reader.get(3), false);
        assert_eq!(reader.get(4), true);
        assert_eq!(reader.get(5), true);
        assert_eq!(reader.get(6), true);
        assert_eq!(reader.get(7), false);
        assert_eq!(reader.get(8), false);
        assert_eq!(reader.get(9), true);
    }
}

#[test]
fn struct_size() {
    let data: &[crate::Word] = &[
        crate::word(0x00, 0x00, 0x00, 0x00, 0x2, 0x00, 0x01, 0x00),
        crate::word(0x0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),
        crate::word(0x0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),
        crate::word(0x0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),
    ];

    test_at_alignments(data, &verify);
    fn verify(pointer_reader: PointerReader) {
        assert_eq!(pointer_reader.total_size().unwrap().word_count, 3);
    }
}

#[test]
fn struct_list_size() {
    let data: &[crate::Word] = &[
        crate::word(0x01, 0, 0, 0, 0x1f, 0, 0, 0), // inline-composite list. 4 words long.
        crate::word(0x4, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00), // 1 element long
        crate::word(0x0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),
        crate::word(0x0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),
        crate::word(0x0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),
    ];

    // The list pointer claims that the list consumes four words, but the struct
    // tag says there is only one element and it has a size of one word.
    // So there is an inconsistency! total_size() should report the value computed from
    // the struct tag, because that's what is relevant when the data is copied.

    test_at_alignments(data, &verify);
    fn verify(pointer_reader: PointerReader) {
        assert_eq!(pointer_reader.total_size().unwrap().word_count, 2);
    }
}

#[test]
fn empty_struct_list_size() {
    let data: &[crate::Word] = &[
        // Struct, one pointer
        crate::word(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00),
        // Inline-composite list, zero words long
        crate::word(0x01, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00),
        // Tag
        crate::word(0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00),
    ];

    test_at_alignments(data, &verify);
    fn verify(pointer_reader: PointerReader) {
        assert_eq!(2, pointer_reader.total_size().unwrap().word_count);
    }
}
