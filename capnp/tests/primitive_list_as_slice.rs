#![cfg(all(target_endian = "little", feature = "alloc"))]

use capnp::{message, primitive_list};

#[test]
pub fn primitive_list_as_slice() {
    let mut msg = message::Builder::new_default();

    {
        let mut void_list = msg.initn_root::<primitive_list::Builder<()>>(0);
        assert_eq!(void_list.as_slice().unwrap().len(), 0);
        assert_eq!(void_list.into_reader().as_slice().unwrap().len(), 0);
    }

    {
        let mut void_list = msg.initn_root::<primitive_list::Builder<()>>(5);
        assert_eq!(void_list.as_slice().unwrap(), &[(), (), (), (), ()]);
        assert_eq!(
            void_list.into_reader().as_slice().unwrap(),
            &[(), (), (), (), ()]
        );
    }

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

    {
        // Test the case when the list elements are InlineComposite.
        use capnp::{schema_capnp, struct_list};
        let nodelist = msg.initn_root::<struct_list::Builder<schema_capnp::node::Owned>>(2);
        nodelist.get(0).set_id(0xabcd);
        let mut u64list = msg.get_root::<primitive_list::Builder<u64>>().unwrap();
        assert!(u64list.as_slice().is_none());
        assert_eq!(u64list.get(0), 0xabcd);

        let u64list = u64list.into_reader();
        assert!(u64list.as_slice().is_none());
        assert_eq!(u64list.get(0), 0xabcd);
    }
}
