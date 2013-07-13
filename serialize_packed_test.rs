use std;

use serialize::*;
use serialize_packed::*;

pub fn expectPacksTo(unpacked : &[u8],
                     packed : &[u8]) {

    let bytes = do std::io::with_bytes_writer |writer| {
        let packedOutputStream =
            @PackedOutputStream {inner : @writer  as @OutputStream};

        packedOutputStream.write(unpacked);
    };

    println!("%?", bytes);
    assert!(bytes.slice(0, bytes.len()).equals(&packed));

}


#[test]
pub fn simplePacking() {
    expectPacksTo([], []);
    expectPacksTo([0,0,0,0,0,0,0,0], [0,0])
}