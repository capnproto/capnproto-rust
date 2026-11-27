#![cfg(feature = "alloc")]

use std::cell::Cell;
use std::rc::Rc;

use capnp::message::{Allocator, Builder, HeapAllocator};

/// A HeapAllocator that counts the segments it has allocated and not yet deallocated.
struct CountingAllocator {
    inner: HeapAllocator,
    live: Rc<Cell<usize>>,
}

unsafe impl Allocator for CountingAllocator {
    fn allocate_segment(&mut self, minimum_size: u32) -> (core::ptr::NonNull<u8>, u32) {
        self.live.set(self.live.get() + 1);
        self.inner.allocate_segment(minimum_size)
    }

    unsafe fn deallocate_segment(
        &mut self,
        ptr: core::ptr::NonNull<u8>,
        word_size: u32,
        words_used: u32,
    ) {
        self.live.set(self.live.get() - 1);
        unsafe { self.inner.deallocate_segment(ptr, word_size, words_used) }
    }
}

/// Builds a message whose root is `text_bytes` bytes of text, starting with a first segment of
/// `first_segment_words`, and returns it with its allocator's count of live segments.
fn build(
    first_segment_words: u32,
    text_bytes: usize,
) -> (Builder<CountingAllocator>, Rc<Cell<usize>>) {
    let live = Rc::new(Cell::new(0));
    let allocator = CountingAllocator {
        inner: HeapAllocator::new().first_segment_words(first_segment_words),
        live: Rc::clone(&live),
    };
    let mut message = Builder::new(allocator);
    message.set_root("x".repeat(text_bytes).as_str()).unwrap();
    (message, live)
}

#[test]
fn dropping_a_builder_deallocates_every_segment() {
    // One segment, then several: a first segment of one word holds only the root pointer.
    for (first_segment_words, text_bytes, multiple) in [(1024, 8, false), (1, 64, true)] {
        let (message, live) = build(first_segment_words, text_bytes);
        let segments = message.get_segments_for_output().len();
        assert_eq!(segments > 1, multiple);
        assert_eq!(live.get(), segments);
        drop(message);
        assert_eq!(live.get(), 0);
    }
}

#[test]
fn into_allocator_deallocates_every_segment() {
    for (first_segment_words, text_bytes) in [(1024, 8), (1, 64)] {
        let (message, live) = build(first_segment_words, text_bytes);
        let _allocator = message.into_allocator();
        assert_eq!(live.get(), 0);
    }
}
