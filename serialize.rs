pub mod InputStreamMessageReader {

    use std;
    use endian::*;
    use message::*;

    pub fn new<T>(inputStream : @ std::io::Reader,
                  options : ReaderOptions,
                  cont : &fn(v : &mut MessageReader) -> T) -> T {

        let firstWord = inputStream.read_bytes(8);

        let segmentCount : u32 =
            unsafe {let p : *WireValue<u32> = std::cast::transmute(firstWord.unsafe_ref(0));
                    (*p).get() + 1
                   };


        let segment0Size =
            if (segmentCount == 0) { 0 } else {
            unsafe {let p : *WireValue<u32> = std::cast::transmute(firstWord.unsafe_ref(4));
                    (*p).get()
                   }
            };

        let mut totalWords = segment0Size;

        if (segmentCount >= 512) {
            fail!("too many segments");
        }

        let mut moreSizes : ~[u32] = std::vec::from_elem((segmentCount & !1) as uint, 0);

        if (segmentCount > 1) {
            let moreSizesRaw = inputStream.read_bytes((8 * (segmentCount & !1)) as uint);
            for std::u32::range(0, segmentCount - 1) |ii| {
                moreSizes[ii] = unsafe {
                    let p : *WireValue<u32> =
                        std::cast::transmute(moreSizesRaw.unsafe_ref(ii as uint * 4));
                    (*p).get()
                };
                totalWords += moreSizes[ii];
            }
        }

        // Don't accept a message which the receiver couldn't possibly
        // traverse without hitting the traversal limit. Without this
        // check, a malicious client could transmit a very large
        // segment size to make the receiver allocate excessive space
        // and possibly crash.
        assert!(totalWords as u64 <= options.traversalLimitInWords);

        let mut ownedSpace : ~[u8] = std::vec::from_elem(8 * totalWords as uint, 0);


        // Do this first in order to appease the borrow checker
        inputStream.read(ownedSpace, totalWords as uint * 8);
        // TODO lazy reading like in capnp-c++. Is that possible
        // within the std::io::Reader interface?

        let segment0 : &[u8] = ownedSpace.slice(0, 8 * segment0Size as uint);

        let mut segments : ~[&[u8]] = ~[segment0];

        if (segmentCount > 1) {
            let mut offset = segment0Size;

            for std::u32::range(0, segmentCount - 1) |ii| {
                segments.push(ownedSpace.slice(offset as uint * 8,
                                               (offset + moreSizes[ii]) as uint * 8));
                offset += moreSizes[ii];
            }
        }

        let mut result = ~MessageReader::<'a> {
            segments : segments,
            options : options
        };

        cont(result)

    }
}