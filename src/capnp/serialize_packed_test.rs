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

use std;
use serialize_packed::{PackedOutputStream, PackedInputStream};
use io;

pub fn expect_packs_to(unpacked : &[u8],
                       packed : &[u8]) {

    use std::old_io::{Reader, Writer};

    // --------
    // write

    let mut bytes : std::vec::Vec<u8> = ::std::iter::repeat(0u8).take(packed.len()).collect();
    {
        let mut writer = io::ArrayOutputStream::new(bytes.as_mut_slice());
        let mut packed_output_stream = PackedOutputStream {inner : &mut writer};
        packed_output_stream.write_all(unpacked).unwrap();
        packed_output_stream.flush().unwrap();
    }

    assert!(bytes.as_slice().eq(packed),
            "expected: {:?}, got: {:?}", packed, bytes);

    // --------
    // read

    let mut reader = io::ArrayInputStream::new(packed);
    let mut packed_input_stream = PackedInputStream {inner : &mut reader};

    let bytes = packed_input_stream.read_exact(unpacked.len()).unwrap();

//    assert!(packed_input_stream.eof());
    assert!(bytes[].eq(unpacked),
            "expected: {:?}, got: {:?}", unpacked, bytes);

}

static ZEROES : &'static[u8] = &[0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0];

#[test]
pub fn simple_packing() {
    expect_packs_to(&[], &[]);
    expect_packs_to(&ZEROES[0 .. 8], &[0,0]);
    expect_packs_to(&[0,0,12,0,0,34,0,0], &[0x24,12,34]);
    expect_packs_to(&[1,3,2,4,5,7,6,8], &[0xff,1,3,2,4,5,7,6,8,0]);
    expect_packs_to(&[0,0,0,0,0,0,0,0,1,3,2,4,5,7,6,8], &[0,0,0xff,1,3,2,4,5,7,6,8,0]);
    expect_packs_to(&[0,0,12,0,0,34,0,0,1,3,2,4,5,7,6,8], &[0x24,12,34,0xff,1,3,2,4,5,7,6,8,0]);
    expect_packs_to(&[1,3,2,4,5,7,6,8,8,6,7,4,5,2,3,1], &[0xff,1,3,2,4,5,7,6,8,1,8,6,7,4,5,2,3,1]);

    expect_packs_to(
        &[1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 0,2,4,0,9,0,5,1],
        &[0xff,1,2,3,4,5,6,7,8, 3, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8,
          0xd6,2,4,9,5,1]);
    expect_packs_to(
        &[1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 6,2,4,3,9,0,5,1, 1,2,3,4,5,6,7,8, 0,2,4,0,9,0,5,1],
        &[0xff,1,2,3,4,5,6,7,8, 3, 1,2,3,4,5,6,7,8, 6,2,4,3,9,0,5,1, 1,2,3,4,5,6,7,8,
          0xd6,2,4,9,5,1]);

    expect_packs_to(
        &[8,0,100,6,0,1,1,2, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,1,0,2,0,3,1],
        &[0xed,8,100,6,1,1,2, 0,2, 0xd4,1,2,3,1]);

    expect_packs_to(&ZEROES[0 .. 16], &[0,1]);
    expect_packs_to(&[0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0], &[0,2]);

}
