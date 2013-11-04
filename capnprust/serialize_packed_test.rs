/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use serialize_packed::{PackedOutputStream, PackedInputStream};

pub fn expectPacksTo(unpacked : &[u8],
                     packed : &[u8]) {

    use std::rt::io::{Reader, Writer};

    // --------
    // write

    let bytes = do std::rt::io::mem::with_mem_writer |writer| {
        let mut packedOutputStream = PackedOutputStream {inner : writer};
        packedOutputStream.write(unpacked);
    };

    assert!(bytes.slice(0, bytes.len()).equals(&packed));

    // --------
    // read

    let mut reader = std::rt::io::mem::BufReader::new(packed);
    let mut packedInputStream = PackedInputStream {inner : &mut reader};

    let bytes = packedInputStream.read_bytes(unpacked.len());

    assert!(packedInputStream.eof());
    assert!(bytes.slice(0, bytes.len()).equals(&unpacked));

}

static zeroes : &'static[u8] = &[0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0];

#[test]
pub fn simplePacking() {
    expectPacksTo([], []);
    expectPacksTo(zeroes.slice(0, 8), [0,0]);
    expectPacksTo([0,0,12,0,0,34,0,0], [0x24,12,34]);
    expectPacksTo([1,3,2,4,5,7,6,8], [0xff,1,3,2,4,5,7,6,8,0]);
    expectPacksTo([0,0,0,0,0,0,0,0,1,3,2,4,5,7,6,8], [0,0,0xff,1,3,2,4,5,7,6,8,0]);
    expectPacksTo([0,0,12,0,0,34,0,0,1,3,2,4,5,7,6,8], [0x24,12,34,0xff,1,3,2,4,5,7,6,8,0]);
    expectPacksTo([1,3,2,4,5,7,6,8,8,6,7,4,5,2,3,1], [0xff,1,3,2,4,5,7,6,8,1,8,6,7,4,5,2,3,1]);

    expectPacksTo(
        [1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 0,2,4,0,9,0,5,1],
        [0xff,1,2,3,4,5,6,7,8, 3, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8,
         0xd6,2,4,9,5,1]);
    expectPacksTo(
        [1,2,3,4,5,6,7,8, 1,2,3,4,5,6,7,8, 6,2,4,3,9,0,5,1, 1,2,3,4,5,6,7,8, 0,2,4,0,9,0,5,1],
        [0xff,1,2,3,4,5,6,7,8, 3, 1,2,3,4,5,6,7,8, 6,2,4,3,9,0,5,1, 1,2,3,4,5,6,7,8,
         0xd6,2,4,9,5,1]);

    expectPacksTo(
        [8,0,100,6,0,1,1,2, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,1,0,2,0,3,1],
        [0xed,8,100,6,1,1,2, 0,2, 0xd4,1,2,3,1]);

    expectPacksTo(zeroes.slice(0,16), [0,1]);
    expectPacksTo([0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0, 0,0,0,0,0,0,0,0], [0,2]);

}
