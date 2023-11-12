capnp_import::capnp_import!("tests/test_schema.capnp");

use capnp::capability::Promise;
use capnp_macros::capnp_let;
use test_schema_capnp::test_struct;

mod capnp_let {
    use super::*;

    #[tokio::test]
    async fn extract_to_same_symbol_test() -> capnp::Result<()> {
        fn test_impl(test_reader: test_struct::Reader) -> Promise<(), capnp::Error> {
            capnp_let!({ float_field } = test_reader);
            assert_eq!(float_field, 3.4);
            Promise::ok(())
        }
        let mut message = capnp::message::Builder::new_default();
        let mut test_builder = message.init_root::<test_struct::Builder>();
        test_builder.set_float_field(3.4);
        let test_reader = test_builder.into_reader();
        test_impl(test_reader).await
    }

    #[tokio::test]
    async fn extract_to_different_symbol_test() -> capnp::Result<()> {
        fn test_impl(test_reader: test_struct::Reader) -> Promise<(), capnp::Error> {
            capnp_let!({ float_field: value } = test_reader);
            assert_eq!(value, 7.9);
            Promise::ok(())
        }
        let mut message = capnp::message::Builder::new_default();
        let mut test_builder = message.init_root::<test_struct::Builder>();
        test_builder.set_float_field(7.9);
        let test_reader = test_builder.into_reader();
        test_impl(test_reader).await
    }

    #[tokio::test]
    async fn extract_to_struct_pattern() -> capnp::Result<()> {
        fn test_impl(test_reader: test_struct::Reader) -> Promise<(), capnp::Error> {
            capnp_let!({ struct_field: {float_field, struct_field: {float_field: inner_float_field}} } = test_reader);
            assert_eq!(float_field, 1.5);
            assert_eq!(inner_float_field, 17.7);
            Promise::ok(())
        }
        let mut message = capnp::message::Builder::new_default();
        let mut test_builder = message.init_root::<test_struct::Builder>();
        test_builder
            .reborrow()
            .get_struct_field()?
            .set_float_field(1.5);
        test_builder
            .reborrow()
            .get_struct_field()?
            .get_struct_field()?
            .set_float_field(17.7);
        let test_reader = test_builder.into_reader();
        test_impl(test_reader).await
    }
}
