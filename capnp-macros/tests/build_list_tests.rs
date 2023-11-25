capnp_import::capnp_import!("tests/test_schema.capnp");
use capnp::capability::Promise;
use capnp_macros::capnp_build;

// for better test names
mod capnp_build {
    use super::*;

    #[tokio::test]
    async fn list_test_assign_from_expr() -> capnp::Result<()> {
        fn test_impl(
            mut list_builder: capnp::primitive_list::Builder<i32>,
        ) -> Promise<(), capnp::Error> {
            capnp_build!(list_builder, [= 1, = 2, = 3]);
            let list_reader = list_builder.into_reader();
            assert_eq!(list_reader.get(0), 1);
            assert_eq!(list_reader.get(1), 2);
            assert_eq!(list_reader.get(2), 3);
            Promise::ok(())
        }
        let mut message = capnp::message::Builder::new_default();
        let intlist = message.initn_root(3);
        test_impl(intlist).await
    }

    #[tokio::test]
    async fn list_test_listpat() -> capnp::Result<()> {
        fn test_impl(
            mut list_builder: capnp::list_list::Builder<capnp::text_list::Owned>,
        ) -> Promise<(), capnp::Error> {
            capnp_build!(list_builder, [[="a", ="b", ="longer_text"], [="d"], [="e", ="f"]]);
            let list_reader = list_builder.into_reader();
            let first_list = capnp_rpc::pry!(list_reader.get(0));
            assert_eq!(first_list.get(0).unwrap().to_str(), Ok("a"));
            assert_eq!(first_list.get(1).unwrap().to_str(), Ok("b"));
            assert_eq!(first_list.get(2).unwrap().to_str(), Ok("longer_text"));

            let second_list = capnp_rpc::pry!(list_reader.get(1));
            assert_eq!(second_list.get(0).unwrap().to_str(), Ok("d"));

            let third_list = capnp_rpc::pry!(list_reader.get(2));
            assert_eq!(third_list.get(0).unwrap().to_str(), Ok("e"));
            assert_eq!(third_list.get(1).unwrap().to_str(), Ok("f"));

            Promise::ok(())
        }
        let mut message = capnp::message::Builder::new_default();
        let listlist = message.initn_root(3);
        test_impl(listlist).await
    }

    #[tokio::test]
    async fn list_test_struct() -> capnp::Result<()> {
        fn test_impl(
            mut list_builder: capnp::struct_list::Builder<test_schema_capnp::test_struct::Owned>,
        ) -> Promise<(), capnp::Error> {
            capnp_build!(
                list_builder,
                [{ uint_field = 6 }, { text_field = "text" }, {
                    bool_field = false
                }]
            );
            let list_reader = list_builder.into_reader();
            assert_eq!(list_reader.get(0).get_uint_field(), 6);
            assert_eq!(
                list_reader.get(1).get_text_field().unwrap().to_str(),
                Ok("text")
            );
            assert_eq!(list_reader.get(2).get_bool_field(), false);
            Promise::ok(())
        }
        let mut message = capnp::message::Builder::new_default();
        let structlist = message.initn_root(3);
        test_impl(structlist).await
    }

    #[tokio::test]
    async fn list_test_from_iter() -> capnp::Result<()> {
        fn test_impl(
            mut list_builder: capnp::struct_list::Builder<test_schema_capnp::test_struct::Owned>,
        ) -> Promise<(), capnp::Error> {
            let numbers = 1..=10; // it's an ExactSizeIterator
            capnp_build!(
                list_builder,
                [for (test_struct, c) in numbers {
                    test_struct.set_uint_field(c);
                    test_struct.set_text_field(c.to_string().as_str().into())
                }]
            );
            let list_reader = list_builder.into_reader();
            for (index, item) in list_reader.into_iter().enumerate() {
                assert_eq!(item.get_uint_field(), (index + 1) as u8);
                assert_eq!(
                    item.get_text_field().unwrap().to_string(),
                    Ok((index + 1).to_string())
                );
            }
            Promise::ok(())
        }
        let mut message = capnp::message::Builder::new_default();
        let structlist = message.initn_root(10);
        test_impl(structlist).await
    }
}
