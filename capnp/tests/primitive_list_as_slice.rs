#![cfg(all(target_endian = "little", feature = "alloc"))]

use capnp::{message, primitive_list};

#[test]
pub fn scratch_space_heap_allocator() {
    let mut msg = message::Builder::new_default();
    {
        let mut u8list = msg.initn_root::<primitive_list::Builder<u8>>(0);
        assert_eq!(u8list.as_slice().unwrap().len(), 0);
        assert_eq!(u8list.into_reader().as_slice().unwrap().len(), 0);
    }

    {
        let mut u8list = msg.initn_root::<primitive_list::Builder<u8>>(3);
        u8list.set(0, 0);
        u8list.set(1, 1);
        u8list.set(2, 2);
        assert_eq!(u8list.as_slice().unwrap(), &[0, 1, 2]);
    }

    {
        let mut u16list = msg.initn_root::<primitive_list::Builder<u16>>(4);
        u16list.set(0, 0xab);
        u16list.set(1, 0xcd);
        u16list.set(2, 0xde);
        u16list.set(3, 0xff);
        assert_eq!(u16list.as_slice().unwrap(), &[0xab, 0xcd, 0xde, 0xff]);
        assert_eq!(
            u16list.into_reader().as_slice().unwrap(),
            &[0xab, 0xcd, 0xde, 0xff]
        );
    }
}
