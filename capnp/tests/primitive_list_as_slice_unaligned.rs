#![cfg(feature = "alloc")]

use capnp::{message, primitive_list};

#[test]
pub fn primitive_list_as_slice_small_values() {
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
}
