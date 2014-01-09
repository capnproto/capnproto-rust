/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use serialize_packed::{PackedOutputStream, PackedInputStream};
use io;

pub fn expect_packs_to(unpacked : &[u8],
                       packed : &[u8]) {

    use std::io::{Reader, Writer};

    // --------
    // write

    let bytes = std::io::mem::with_mem_writer(|writer| {
            let mut bufferedOutputStream = io::BufferedOutputStream::new(writer);
            let mut packedOutputStream = PackedOutputStream {inner : &mut bufferedOutputStream};
            packedOutputStream.write(unpacked);
            packedOutputStream.flush();
        });

    assert!(bytes.slice(0, bytes.len()).equals(&packed),
            "expected: {:?}, got: {:?}", packed, bytes);

    // --------
    // read

    let mut reader = std::io::mem::BufReader::new(packed);
    let mut bufferedReader = io::BufferedInputStream::new(&mut reader);
    let mut packedInputStream = PackedInputStream {inner : &mut bufferedReader};

    let bytes = packedInputStream.read_bytes(unpacked.len());

//    assert!(packedInputStream.eof());
    assert!(bytes.slice(0, bytes.len()).equals(&unpacked),
            "expected: {:?}, got: {:?}", unpacked, bytes);

}

static zeroes : &'static[u8] = &[0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0];

#[test]
pub fn simple_packing() {
    expect_packs_to([], []);
    expect_packs_to(zeroes.slice(0, 8), [0,0]);
    expect_packs_to([0,0,12,0,0,34,0,0], [0x24,12,34]);
    expect_packs_to([1,3,2,4,5,7,6,8], [0xff,1,3,2,4,5,7,6,8,0]);
    expect_packs_to([0,0,0,0,0,0,0,0,1,3,2,4,5,7,6,8], [0,0,0xff,1,3,2,4,5,7,6,8,0]);
    expect_packs_to([0,0,12,0,0,34,0,0,1,3,2,4,5,7,6,8], [0x24,12,34,0xff,1,3,2,4,5,7,6,8,0]);
    expect_packs_to([1,3,2,4,5,7,6,8,8,6,7,4,5,2,3,1], [0xff,1,3,2,4,5,7,6,8,1,8,6,7,4,5,2,3,1]);

    expect_packs_to(
        [1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 0,2,4,0,9,0,5,1],
        [0xff,1,2,3,4,5,6,7,8, 3, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8,
         0xd6,2,4,9,5,1]);
    expect_packs_to(
        [1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 6,2,4,3,9,0,5,1, 1,2,3,4,5,6,7,8, 0,2,4,0,9,0,5,1],
        [0xff,1,2,3,4,5,6,7,8, 3, 1,2,3,4,5,6,7,8, 6,2,4,3,9,0,5,1, 1,2,3,4,5,6,7,8,
         0xd6,2,4,9,5,1]);

    expect_packs_to(
        [8,0,100,6,0,1,1,2, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,1,0,2,0,3,1],
        [0xed,8,100,6,1,1,2, 0,2, 0xd4,1,2,3,1]);

    expect_packs_to(zeroes.slice(0,16), [0,1]);
    expect_packs_to([0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0], [0,2]);

}
