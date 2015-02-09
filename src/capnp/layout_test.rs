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


#[test]
fn simple_raw_data_struct() {
    let data : ::private::AlignedData<[u8; 16]> = ::private::AlignedData {
        _dummy: 0,
        data : [0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
                0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]
    };

    let reader = unsafe { ::layout::PointerReader::get_root_unchecked(
        ::std::mem::transmute(data.data.as_ptr())).get_struct(::std::ptr::null()) };

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
