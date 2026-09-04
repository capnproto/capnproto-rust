#![cfg(feature = "alloc")]

use capnp::message;

#[test]
pub fn scratch_space_heap_allocator() {
    let mut buffer = capnp::Word::allocate_zeroed_vec(200);
    {
        let allocator = message::ScratchSpaceHeapAllocator::new(capnp::Word::words_to_bytes_mut(
            &mut buffer[..],
        ));
        let mut msg = message::Builder::new(allocator);
        msg.set_root("hello world!").unwrap();

        let s: capnp::text::Reader = msg.get_root_as_reader().unwrap();
        assert_eq!("hello world!", s);
    }

    for w in buffer {
        assert_eq!(w, capnp::word(0, 0, 0, 0, 0, 0, 0, 0));
    }
}

#[test]
pub fn scratch_space_exact_fit() {
    // The builder's first request is one word, for the root pointer. A one-word
    // scratch space is exactly big enough, so that word must come from the scratch
    // space rather than from the heap.
    let mut buffer = capnp::Word::allocate_zeroed_vec(1);
    let scratch_ptr = buffer.as_ptr() as *const u8;
    {
        let allocator = message::ScratchSpaceHeapAllocator::new(capnp::Word::words_to_bytes_mut(
            &mut buffer[..],
        ));
        let mut msg = message::Builder::new(allocator);
        msg.init_root::<capnp::any_pointer::Builder>();

        let segments = msg.get_segments_for_output();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].as_ptr(), scratch_ptr);
    }
}
