#![no_std]

#![feature(core_intrinsics, alloc_error_handler)]

extern crate alloc;

use alloc::alloc::{GlobalAlloc, Layout};
use core::panic::PanicInfo;

// Allocator that fails on every allocation. This is to show that we can read capnproto
// messages without doing any allocations. Note, however, that capnp::Error does allocate,
// so for a real application we would want an actual allocator.
struct NullAllocator;

unsafe impl GlobalAlloc for NullAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 { core::ptr::null_mut() }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static A: NullAllocator = NullAllocator;

#[alloc_error_handler]
fn alloc_error(_: Layout) -> ! {
    core::intrinsics::abort()
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    core::intrinsics::abort()
}

pub mod wasm_hello_world_capnp {
  include!(concat!(env!("OUT_DIR"), "/wasm_hello_world_capnp.rs"));
}

#[no_mangle]
pub extern "C" fn add_numbers(ptr: i32, len: i32) -> i32 {
    let buf: &[u8] = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as usize) };
    let segments = &[buf];
    let message = capnp::message::Reader::new(capnp::message::SegmentArray::new(segments),
                                              core::default::Default::default());

    let foo = message.get_root::<wasm_hello_world_capnp::foo::Reader>().unwrap();
    let numbers = foo.get_numbers().unwrap();

    let mut total: i32 = 0;
    for ii in 0 .. numbers.len() {
        total += numbers.get(ii) as i32;
    }
    total
}
