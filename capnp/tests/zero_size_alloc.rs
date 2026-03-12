#![cfg(feature = "alloc")]

use capnp::message::{AllocationStrategy, HeapAllocator};

#[test]
fn zero_size_alloc() {
    // Configure next_size = 0 via the public builder-style API.
    let mut alloc = HeapAllocator::new()
        .first_segment_words(0)
        .allocation_strategy(AllocationStrategy::FixedSize);

    // Trigger <HeapAllocator as Allocator>::allocate_segment with minimum_size = 0.
    // This makes size = max(0, 0) = 0, so alloc_zeroed(Layout{size:0, align:8}) is called.
    let (_ptr, size_words) = capnp::message::Allocator::allocate_segment(&mut alloc, 0);

    // Use the returned size so the call isn't optimized away under release/Miri.
    assert_eq!(size_words, 0);
}
