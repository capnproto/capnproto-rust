capnp_import::capnp_import!("tests/example.capnp");

use capnp::capability::Promise;
use capnp::IntoResult;
use capnp_macros::capnp_let;
use example_capnp::person as person_capnp;

fn get_person() -> Vec<u8> {
    let mut message = capnp::message::Builder::new_default();
    let mut person = message.init_root::<person_capnp::Builder>();
    person.set_name("Tom".into());
    person.set_email("tom@gmail.com".into());
    let mut birthdate = person.reborrow().init_birthdate();
    birthdate.set_day(1);
    birthdate.set_month(2);
    birthdate.set_year_as_text("1990".into());

    capnp::serialize::write_message_to_words(&message)
}

fn macro_usage(person: person_capnp::Reader) -> Promise<(), capnp::Error> {
    capnp_let!(
        {name, birthdate: {year_as_text: year, month}, email: contact_email} = person
    );
    assert_eq!(name, "Tom");
    assert_eq!(year, "1990");
    assert_eq!(month, 2);
    assert_eq!(contact_email, "tom@gmail.com");
    // `birthdate` as a Reader is also in scope
    assert_eq!(birthdate.get_day(), 1);
    Promise::ok(())
}

#[tokio::test]
async fn usage_test() -> capnp::Result<()> {
    let message_reader = capnp::serialize::read_message(
        get_person().as_slice(),
        capnp::message::ReaderOptions::new(),
    )?;
    let person = message_reader.get_root::<person_capnp::Reader>()?;

    macro_usage(person).await
}
