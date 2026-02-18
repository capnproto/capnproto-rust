// Regression test for the async PackedWrite run-length overflow bug.
//
// When a message contains more than 255 consecutive non-zero (or zero) words,
// the run count byte must be capped at 255 to avoid silent truncation via
// `as u8`. This test verifies that the async packed writer produces output
// that can be correctly read back by both the async and synchronous packed
// readers, even for large messages that exceed the 255-word run threshold.

capnp::generated_code!(pub mod addressbook_capnp);

#[cfg(test)]
mod tests {
    use crate::addressbook_capnp::address_book;
    use capnp::message;
    use capnp::message::HeapAllocator;
    use capnp::serialize::OwnedSegments;

    fn populate_large_address_book(address_book: address_book::Builder) {
        let mut people = address_book.init_people(1);
        let mut entry = people.reborrow().get(0);

        // A long name ensures a big contiguous non-zero region.
        let long_name: String = "A".repeat(100_000);
        entry.set_name(&long_name);
    }

    fn verify_large_address_book(reader: address_book::Reader) {
        let people = reader.get_people().unwrap();
        assert_eq!(people.len(), 1);
        let entry = people.get(0);

        let name = entry.get_name().unwrap().to_str().unwrap();
        assert_eq!(name.len(), 100_000);
        assert!(name.chars().all(|c| c == 'A'));
    }

    fn write_sync(builder: &message::Builder<HeapAllocator>) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        capnp::serialize_packed::write_message(&mut buf, builder).unwrap();
        buf
    }

    fn write_async(builder: &message::Builder<HeapAllocator>) -> Vec<u8> {
        futures::executor::block_on(async {
            let mut buf: Vec<u8> = Vec::new();
            capnp_futures::serialize_packed::write_message(&mut buf, builder)
                .await
                .unwrap();
            buf
        })
    }

    fn read_sync(buf: &[u8]) -> message::Reader<OwnedSegments> {
        capnp::serialize_packed::read_message(buf, message::DEFAULT_READER_OPTIONS).unwrap()
    }

    fn read_async(buf: &[u8]) -> message::Reader<OwnedSegments> {
        futures::executor::block_on(async {
            capnp_futures::serialize_packed::read_message(buf, message::DEFAULT_READER_OPTIONS)
                .await
                .unwrap()
        })
    }

    #[test]
    fn test_write_sync_write_async_equivalence() {
        let mut builder = message::Builder::new(HeapAllocator::new());
        populate_large_address_book(builder.init_root());

        let sync_buf = write_sync(&builder);
        let async_buf = write_async(&builder);

        assert_eq!(sync_buf, async_buf);

        verify_large_address_book(read_sync(&sync_buf).get_root().unwrap());
        verify_large_address_book(read_sync(&async_buf).get_root().unwrap());

        verify_large_address_book(read_async(&sync_buf).get_root().unwrap());
        verify_large_address_book(read_async(&async_buf).get_root().unwrap());
    }
}
