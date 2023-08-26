capnp_import::capnp_import!("tests/example.capnp");

use capnp::capability::Promise;
use capnp::IntoResult;
use capnp_macros::{capnp_build, capnp_let};
use example_capnp::date_list;
use example_capnp::person as person_capnp;
use example_capnp::text_list;

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

fn macro_usage_two(mut person_builder: person_capnp::Builder) -> Promise<(), capnp::Error> {
    //let birthdate_builder = capnp_rpc::pry!(person.get_birthdate());
    //birthdate_builder.set_day(1);
    //birthdate_builder.set_month(2);
    let message_reader = capnp_rpc::pry!(capnp::serialize::read_message(
        get_person().as_slice(),
        capnp::message::ReaderOptions::new(),
    ));
    let person = capnp_rpc::pry!(message_reader.get_root::<person_capnp::Reader>());
    capnp_let!(
        {name, birthdate, email} = person
    );
    capnp_build!(person_builder, {name, birthdate, email});
    let another_person = person_builder.reborrow_as_reader();
    capnp_let!({name: name2, birthdate: birthdate2, email: email2} = another_person);
    assert_eq!(name, name2);
    assert_eq!(email, email2);

    capnp_build!(person_builder, { name = "s".to_uppercase().as_str() });
    let another_person = person_builder.reborrow_as_reader();
    assert_eq!(another_person.get_name().unwrap(), "S");

    capnp_build!(person_builder, { name => name_setter });
    name_setter.clear();
    name_setter.push_str("a");
    let another_person = person_builder.reborrow_as_reader();
    assert_eq!(another_person.get_name().unwrap(), "a");
    //let mut birthdate_builder = capnp_rpc::pry!(person_builder.get_birthdate().into_result());
    //birthdate_builder.set_day(1u8);
    //birthdate_builder.set_month(2u8);
    //birthdate_builder.set_year_as_text("1990");
    Promise::ok(())
}

fn macro_usage_three(mut date_list: date_list::Builder) -> Promise<(), capnp::Error> {
    let v = vec![(1, 2, 3), (4, 5, 6), (7, 8, 9)].into_iter();
    let mut x = date_list.init_dates(v.len() as u32);
    // TODO capnp_build version of taking expressions must have unique syntax to renaming variables
    // let f =
    //     |(x, y, z), w: example_capnp::date::Builder| {capnp_build!({day: x, month: y, year_as_text: z} = w);}
    // let temp = x.reborrow().get(0);
    // x.set_with_caveats(0, todo!());
    // x.set_with_caveats(1, todo!());
    // x.set_with_caveats(2, todo!());
    Promise::ok(())
}

#[tokio::test]
async fn capnp_let_test() -> capnp::Result<()> {
    let message_reader = capnp::serialize::read_message(
        get_person().as_slice(),
        capnp::message::ReaderOptions::new(),
    )?;
    let person = message_reader.get_root::<person_capnp::Reader>()?;

    macro_usage(person).await
}

#[tokio::test]
async fn capnp_build_test() -> capnp::Result<()> {
    let message_reader = capnp::serialize::read_message(
        get_person().as_slice(),
        capnp::message::ReaderOptions::new(),
    )?;
    let mut message = capnp::message::Builder::new_default();

    let person: person_capnp::Builder = message.init_root::<person_capnp::Builder>();
    macro_usage_two(person).await?;

    let date_list = message.init_root::<date_list::Builder>();
    macro_usage_three(date_list).await
}
