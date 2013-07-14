use std;

use serialize::*;
use serialize_packed::*;

pub fn expectPacksTo(unpacked : &[u8],
                     packed : &[u8]) {

    // --------
    // write

    let bytes = do std::io::with_bytes_writer |writer| {
        let packedOutputStream =
            @PackedOutputStream {inner : @writer  as @OutputStream};

        packedOutputStream.write(unpacked);
    };

    assert!(bytes.slice(0, bytes.len()).equals(&packed));

    // --------
    // read


    do std::io::with_bytes_reader(packed) |reader| {
        let packedInputStream =
            @PackedInputStream {inner : reader} as @std::io::Reader;

        let bytes = packedInputStream.read_whole_stream();

        println!("%?", bytes);

        assert!(bytes.slice(0, bytes.len()).equals(&unpacked));
    }
}


#[test]
pub fn simplePacking() {
    expectPacksTo([], []);
    expectPacksTo([0,0,0,0,0,0,0,0], [0,0]);
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
}