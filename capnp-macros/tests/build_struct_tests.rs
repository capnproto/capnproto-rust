capnp_import::capnp_import!("tests/test_schema.capnp");
use capnp::capability::Promise;
use capnp::IntoResult; // Required for the macro to work
use capnp_macros::capnp_build;

// for better test names
mod capnp_build {
    use super::*;

    // {field}
    #[tokio::test]
    async fn struct_test_assign_from_name() -> capnp::Result<()> {
        fn test_impl(
            mut struct_builder: test_schema_capnp::test_struct::Builder,
        ) -> Promise<(), capnp::Error> {
            let text_field = "Example text";
            capnp_build!(struct_builder, { text_field });
            // Original variable doesn't get overwritten
            assert_eq!(text_field, "Example text");
            // Builder's struct gets updated
            assert_eq!(
                capnp_rpc::pry!(struct_builder.get_text_field()).to_str(),
                Ok("Example text")
            );
            Promise::ok(())
        }

        let mut message = capnp::message::Builder::new_default();
        let struct_builder = message.init_root::<test_schema_capnp::test_struct::Builder>();
        test_impl(struct_builder).await
    }

    // {field = expr}
    #[tokio::test]
    async fn struct_test_assign_from_expr() -> capnp::Result<()> {
        fn test_impl(
            mut struct_builder: test_schema_capnp::test_struct::Builder,
        ) -> Promise<(), capnp::Error> {
            capnp_build!(struct_builder, { text_field = "Example text" });
            assert_eq!(
                capnp_rpc::pry!(struct_builder.get_text_field()).to_str(),
                Ok("Example text")
            );
            Promise::ok(())
        }
        let mut message = capnp::message::Builder::new_default();
        let struct_builder = message.init_root::<test_schema_capnp::test_struct::Builder>();
        test_impl(struct_builder).await
    }

    // {field: struct_pat}
    #[tokio::test]
    async fn struct_test_struct_pattern() -> capnp::Result<()> {
        fn test_impl(
            mut struct_builder: test_schema_capnp::test_struct::Builder,
        ) -> Promise<(), capnp::Error> {
            capnp_build!(struct_builder, {struct_field: {uint_field = 8}});
            assert_eq!(
                capnp_rpc::pry!(struct_builder.get_struct_field()).get_uint_field(),
                8
            );
            Promise::ok(())
        }
        let mut message = capnp::message::Builder::new_default();
        let struct_builder = message.init_root::<test_schema_capnp::test_struct::Builder>();
        test_impl(struct_builder).await
    }

    // {field: list_pat}
    #[tokio::test]
    async fn struct_test_list_pattern() -> capnp::Result<()> {
        fn test_impl(
            mut struct_builder: test_schema_capnp::test_struct::Builder,
        ) -> Promise<(), capnp::Error> {
            capnp_build!(struct_builder, {intlist_field: [=30, =15, =7]});
            let intlist = capnp_rpc::pry!(struct_builder.get_intlist_field()).into_reader();
            assert_eq!(intlist.get(0), 30);
            assert_eq!(intlist.get(1), 15);
            assert_eq!(intlist.get(2), 7);
            Promise::ok(())
        }
        let mut message = capnp::message::Builder::new_default();
        let struct_builder = message.init_root::<test_schema_capnp::test_struct::Builder>();
        test_impl(struct_builder).await
    }

    // {field => closure}
    #[tokio::test]
    async fn struct_test_closure() -> capnp::Result<()> {
        fn test_impl(
            mut struct_builder: test_schema_capnp::test_struct::Builder,
        ) -> Promise<(), capnp::Error> {
            capnp_build!(struct_builder, {struct_field: {uint_field = 1}}); // deleting this line fails the test
            capnp_build!(struct_builder, {struct_field => |mut inner_struct_builder: test_schema_capnp::test_struct::Builder| {
                if inner_struct_builder.reborrow_as_reader().get_uint_field() != 0 {
                    inner_struct_builder.set_uint_field(13);
                } else {
                    inner_struct_builder.set_uint_field(14);
                }
            }});
            assert_eq!(
                capnp_rpc::pry!(struct_builder.get_struct_field()).get_uint_field(),
                13
            );
            Promise::ok(())
        }
        let mut message = capnp::message::Builder::new_default();
        let struct_builder = message.init_root::<test_schema_capnp::test_struct::Builder>();
        test_impl(struct_builder).await
    }
}
