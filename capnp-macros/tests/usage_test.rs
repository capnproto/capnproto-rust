capnp_import::capnp_import!("tests/example.capnp");

use capnp::capability::Promise;
use capnp::IntoResult;
use capnp_macros::{capnp_build, capnp_let};
use example_capnp::date_list;
use example_capnp::person as person_capnp;
use example_capnp::text_list;

fn get_person_message() -> capnp::message::TypedBuilder<person_capnp::Owned> {
    let mut message = capnp::message::TypedBuilder::<person_capnp::Owned>::new_default();
    let mut person = message.init_root();
    person.set_name("Tom".into());
    person.set_email("tom@gmail.com".into());
    let mut birthdate = person.reborrow().init_birthdate();
    birthdate.set_day(1);
    birthdate.set_month(2);
    birthdate.set_year_as_text("1990".into());
    message
    //capnp::serialize::write_message_to_words(&message)
}

fn capnp_let_test_impl(person: person_capnp::Reader) -> Promise<(), capnp::Error> {
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

fn capnp_build_struct_test_impl(
    mut person_builder: person_capnp::Builder,
) -> Promise<(), capnp::Error> {
    let name = "Tom";
    let custom_email = "tom@gmail.com";

    //capnp_build!(person_builder, {name, birthdate: {day = 1, month = 1 + 1, year_as_text => year_builder}, email = custom_email});
    capnp_build!(person_builder, {name, birthdate: {day = 1, month = 1 + 1, year_as_text = "1990"}, email = custom_email});
    // Following line wouldn't work, because length isn't long enough
    //year_builder.push_str("1990");

    let person = person_builder.into_reader();
    let message = get_person_message();
    let other_person = capnp_rpc::pry!(message.get_root_as_reader());
    assert_eq!(person.get_name().unwrap(), other_person.get_name().unwrap());
    assert_eq!(
        person.get_email().unwrap(),
        other_person.get_email().unwrap()
    );

    let birthdate = capnp_rpc::pry!(person.get_birthdate());
    let other_birthdate = capnp_rpc::pry!(other_person.get_birthdate());
    assert_eq!(birthdate.get_day(), other_birthdate.get_day());
    assert_eq!(birthdate.get_month(), other_birthdate.get_month());
    assert_eq!(
        birthdate.get_year_as_text().unwrap(),
        other_birthdate.get_year_as_text().unwrap()
    );
    Promise::ok(())
}

fn try_getting_lambdas_to_work(
    mut person_builder: person_capnp::Builder,
) -> Promise<(), capnp::Error> {
    let name = "Tom";
    let custom_email = "tom@gmail.com";
    //capnp_build!(person_builder, {name, birthdate: {day = 1, month = 1 + 1, year_as_text => year_builder}, email = custom_email});
    // Proposed new syntax:
    // capnp_build!(person_builder,
    //              {name,
    //              birthdate:
    //                  {day = 1,
    //                  month = 1 + 1,
    //                  year_as_text => |year_builder| {year_builder.push_str("1990")}},
    //              email = custom_email});
    capnp_build!(person_builder, {
        birthdate => |mut birthdate_builder: example_capnp::date::Builder | {
                let day = 1;
                birthdate_builder.set_day(day);
                let month = 2 * day;
                birthdate_builder.set_month(month);
        },

        birthdate: {year_as_text = "1990"}
    });
    let person_reader = person_builder.into_reader();
    capnp_let!({birthdate: {month, year_as_text}} = person_reader);
    assert_eq!(month, 2);
    assert_eq!(year_as_text, "1990");
    // Expansion start:

    // person_builder.set_name(name.into());
    // let mut birthdate_builder =
    //     capnp_rpc::pry!(person_builder.reborrow().get_birthdate().into_result());
    // birthdate_builder.set_day(1);
    // birthdate_builder.set_month(1 + 1);
    // /*let mut year_builder = capnp_rpc::pry!(birthdate_builder
    // .reborrow()
    // .get_year_as_text()
    // .into_result());*/
    // capnp_rpc::pry!(birthdate_builder
    //     .reborrow()
    //     .get_year_as_text()
    //     .into_result())
    // .push_str("1990");
    // person_builder.set_email(custom_email.into());
    // Expansion end
    Promise::ok(())
}

// fn macro_usage_two(mut person_builder: person_capnp::Builder) -> Promise<(), capnp::Error> {
//     //let birthdate_builder = capnp_rpc::pry!(person.get_birthdate());
//     //birthdate_builder.set_day(1);
//     //birthdate_builder.set_month(2);
//     //let message_reader = capnp_rpc::pry!(capnp::serialize::read_message(
//     //    get_person().as_slice(),
//     //    capnp::message::ReaderOptions::new(),
//     //));
//     let message = get_person_message();
//     let person = capnp_rpc::pry!(message.get_root_as_reader());
//     //let person = capnp_rpc::pry!(message_reader.get_root::<person_capnp::Reader>());
//     capnp_let!(
//         {name, birthdate, email} = person
//     );
//     capnp_build!(person_builder, {name, birthdate, email});
//     let another_person = person_builder.reborrow_as_reader();
//     capnp_let!({name: name2, birthdate: birthdate2, email: email2} = another_person);
//     assert_eq!(name, name2);
//     assert_eq!(email, email2);

//     capnp_build!(person_builder, { name = "s".to_uppercase().as_str() });
//     let another_person = person_builder.reborrow_as_reader();
//     assert_eq!(another_person.get_name().unwrap(), "S");

//     capnp_build!(person_builder, { name => name_setter });
//     name_setter.clear();
//     name_setter.push_str("a");
//     let another_person = person_builder.reborrow_as_reader();
//     assert_eq!(another_person.get_name().unwrap(), "a");
//     //let mut birthdate_builder = capnp_rpc::pry!(person_builder.get_birthdate().into_result());
//     //birthdate_builder.set_day(1u8);
//     //birthdate_builder.set_month(2u8);
//     //birthdate_builder.set_year_as_text("1990");
//     Promise::ok(())
// }

// fn macro_usage_three(mut date_list: date_list::Builder) -> Promise<(), capnp::Error> {
//     //let x = date_list.get_dates().unwrap();
//     capnp_build!(date_list, {dates => dates});
//     //capnp_build!(dates, [{day = 1, month = 2, year_as_text = "1990"},
//     //                     {day = 2, month = 3, year_as_text = "1234"},
//     //                     {day = 7, month = 8, year_as_text = "2023"}]);
//     assert_eq!(dates.get(0).get_day(), 1);
//     let v = vec![(1, 2, 3), (4, 5, 6), (7, 8, 9)].into_iter();
//     let mut x = date_list.init_dates(v.len() as u32);
//     // let f =
//     //     |(x, y, z), w: example_capnp::date::Builder| {capnp_build!({day: x, month: y, year_as_text: z} = w);}
//     // let temp = x.reborrow().get(0);
//     // x.set_with_caveats(0, todo!());
//     // x.set_with_caveats(1, todo!());
//     // x.set_with_caveats(2, todo!());
//     Promise::ok(())
// }

fn macro_usage_four(
    mut date_list: capnp::struct_list::Builder<example_capnp::date::Owned>,
) -> Promise<(), capnp::Error> {
    capnp_build!(date_list, [{day = 1, month = 2, year_as_text = "1990"},
                         {day = 2, month = 3, year_as_text = "1234"},
                         {day = 7, month = 8, year_as_text = "2023"}]);

    assert_eq!(date_list.reborrow().get(0).get_day(), 1);
    assert_eq!(date_list.reborrow().get(1).get_month(), 3);
    Promise::ok(())
}

#[tokio::test]
async fn capnp_let_test() -> capnp::Result<()> {
    //let message_reader = capnp::serialize::read_message(
    //    get_person().as_slice(),
    //    capnp::message::ReaderOptions::new(),
    //)?;
    let person_message = get_person_message();
    let person: person_capnp::Reader = person_message.get_root_as_reader()?;
    //let person = message_reader.get_root::<person_capnp::Reader>()?;

    capnp_let_test_impl(person).await
}

#[tokio::test]
async fn capnp_build_test() -> capnp::Result<()> {
    //let message_reader = capnp::serialize::read_message(
    //    get_person().as_slice(),
    //   capnp::message::ReaderOptions::new(),
    //)?;
    //let mut person_message = get_person_message();
    //let person_builder = person_message.get_root()?;
    let mut message = capnp::message::Builder::new_default();
    let person_builder = message.init_root::<person_capnp::Builder>();
    //capnp_build_struct_test_impl(person_builder).await //?;
    try_getting_lambdas_to_work(person_builder).await

    //let person: person_capnp::Builder = message.init_root::<person_capnp::Builder>();
    //macro_usage_two(person).await?;

    //let mut message = capnp::message::Builder::new_default();
    //let date_list = message.init_root::<date_list::Builder>();
    //macro_usage_three(date_list).await
}

#[tokio::test]
async fn capnp_temp_test() -> capnp::Result<()> {
    let mut message = capnp::message::Builder::new_default();
    let mut date_list: capnp::struct_list::Builder<example_capnp::date::Owned> =
        message.initn_root(3);
    macro_usage_four(date_list).await
}
