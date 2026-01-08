#![cfg(all(
    target_endian = "little",
    feature = "alloc",
    not(feature = "unaligned")
))]

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
        let mut bool_list = msg.initn_root::<primitive_list::Builder<bool>>(8);

        bool_list.set(0, true);
        bool_list.set(1, true);

        // Rust's slices cannot represent bit-packed bools.
        assert!(bool_list.as_slice().is_none());
        assert!(bool_list.into_reader().as_slice().is_none());
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
        let mut u16list = msg.initn_root::<primitive_list::Builder<u16>>(0);
        assert_eq!(u16list.as_slice().unwrap().len(), 0);
        assert_eq!(u16list.into_reader().as_slice().unwrap().len(), 0);
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
        let mut u32list = msg.initn_root::<primitive_list::Builder<u32>>(0);
        assert_eq!(u32list.as_slice().unwrap().len(), 0);
        assert_eq!(u32list.into_reader().as_slice().unwrap().len(), 0);
    }

    {
        let mut u32list = msg.initn_root::<primitive_list::Builder<u32>>(4);
        u32list.set(0, 0xab);
        u32list.set(1, 0xcd);
        u32list.set(2, 0xde);
        u32list.set(3, 0xff);
        assert_eq!(u32list.as_slice().unwrap(), &[0xab, 0xcd, 0xde, 0xff]);
        assert_eq!(
            u32list.into_reader().as_slice().unwrap(),
            &[0xab, 0xcd, 0xde, 0xff]
        );
    }

    {
        let mut u64list = msg.initn_root::<primitive_list::Builder<u64>>(0);
        assert_eq!(u64list.as_slice().unwrap().len(), 0);
        assert_eq!(u64list.into_reader().as_slice().unwrap().len(), 0);
    }

    {
        let mut u64list = msg.initn_root::<primitive_list::Builder<u64>>(4);
        u64list.set(0, 0xab);
        u64list.set(1, 0xcd);
        u64list.set(2, 0xde);
        u64list.set(3, 0xff);
        assert_eq!(u64list.as_slice().unwrap(), &[0xab, 0xcd, 0xde, 0xff]);
        assert_eq!(
            u64list.into_reader().as_slice().unwrap(),
            &[0xab, 0xcd, 0xde, 0xff]
        );
    }

    {
        let mut f32list = msg.initn_root::<primitive_list::Builder<f32>>(0);
        assert_eq!(f32list.as_slice().unwrap().len(), 0);
        assert_eq!(f32list.into_reader().as_slice().unwrap().len(), 0);
    }

    {
        let mut f32list = msg.initn_root::<primitive_list::Builder<f32>>(5);
        f32list.set(0, 0.3);
        f32list.set(1, 0.0);
        f32list.set(2, f32::NEG_INFINITY);
        f32list.set(3, -0.0);
        f32list.set(4, f32::MAX);
        assert_eq!(
            f32list.as_slice().unwrap(),
            &[0.3, 0.0, f32::NEG_INFINITY, -0.0, f32::MAX]
        );
        assert_eq!(
            f32list.into_reader().as_slice().unwrap(),
            &[0.3, 0.0, f32::NEG_INFINITY, -0.0, f32::MAX]
        );
    }

    {
        let mut f64list = msg.initn_root::<primitive_list::Builder<f64>>(0);
        assert_eq!(f64list.as_slice().unwrap().len(), 0);
        assert_eq!(f64list.into_reader().as_slice().unwrap().len(), 0);
    }

    {
        let mut f64list = msg.initn_root::<primitive_list::Builder<f64>>(5);
        f64list.set(0, 0.3);
        f64list.set(1, 0.0);
        f64list.set(2, f64::NEG_INFINITY);
        f64list.set(3, -0.0);
        f64list.set(4, f64::MAX);
        assert_eq!(
            f64list.as_slice().unwrap(),
            &[0.3, 0.0, f64::NEG_INFINITY, -0.0, f64::MAX]
        );
        assert_eq!(
            f64list.into_reader().as_slice().unwrap(),
            &[0.3, 0.0, f64::NEG_INFINITY, -0.0, f64::MAX]
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
