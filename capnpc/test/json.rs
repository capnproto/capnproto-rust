use crate::test_capnp::{self, test_all_types};
use capnp::{message, Result};
use serde_json::json;

#[test]
fn all_types() -> Result<()> {
    let mut message = message::Builder::new_default();
    let mut root = message.init_root::<test_all_types::Builder<'_>>();
    root.set_bool_field(true);
    root.set_text_field("foo bar");

    // let actual = capnp::json::serialize(root.reborrow_as_reader().into(), Default::default())?;

    // let expected = json!({});

    // assert_eq!(expected, actual);
    // Ok(())
    todo!()
}
