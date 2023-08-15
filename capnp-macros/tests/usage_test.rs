capnp_import::capnp_import!("tests/example.capnp");

use capnp::capability::Promise;
use capnp::IntoResult;
use capnp_macros::capnp_let;
use example_capnp::person as person_capnp;

fn get_person() -> Vec<u8> {
    let mut message = capnp::message::Builder::new_default();
    let mut person = message.init_root::<person_capnp::Builder>();
    person.set_name("Tom");
    person.set_email("tom@gmail.com");
    let mut birthdate = person.reborrow().init_birthdate();
    birthdate.set_day(1);
    birthdate.set_month(2);
    birthdate.set_year_as_text("1990");

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

fn macro_usage_two(person: person_capnp::Builder) -> Promise<(), capnp::Error> {
    //let birthdate_builder = capnp_rpc::pry!(person.get_birthdate());
    //birthdate_builder.set_day(1);
    //birthdate_builder.set_month(2);

    let mut birthdate_builder = capnp_rpc::pry!(person.get_birthdate().into_result());
    birthdate_builder.set_day(1u8);
    birthdate_builder.set_month(2u8);
    birthdate_builder.set_year_as_text("1990");
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
