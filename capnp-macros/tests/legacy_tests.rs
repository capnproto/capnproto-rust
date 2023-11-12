// Original file with tests for capnp_build! and capnp_let!.
// They should be restructured or maybe deleted, but some of these might be more readable than
// the ones in the other files.
capnp_import::capnp_import!("tests/example.capnp");

use capnp::capability::Promise;
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
}

#[tokio::test]
async fn capnp_let_test() -> capnp::Result<()> {
    let person_message = get_person_message();
    let person: person_capnp::Reader = person_message.get_root_as_reader()?;

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

    capnp_let_test_impl(person).await
}

#[tokio::test]
async fn capnp_build_test() -> capnp::Result<()> {
    fn legacy_struct_test_closure(
        mut person_builder: person_capnp::Builder,
    ) -> Promise<(), capnp::Error> {
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
        Promise::ok(())
    }

    let mut message = capnp::message::Builder::new_default();
    let person_builder = message.init_root::<person_capnp::Builder>();
    legacy_struct_test_closure(person_builder).await?;

    fn structlist_tests_impl(mut date_list: date_list::Builder) -> Promise<(), capnp::Error> {
        let v = vec![(1, 2, "3"), (4, 5, "6"), (7, 8, "9")].into_iter();
        capnp_build!(date_list, {
            dates: [
                for (date_builder, (d, m, y)) in v {
                    capnp_build!(date_builder, {day = d, month = m, year_as_text = y});
        }]});

        let dates = capnp_rpc::pry!(date_list.reborrow_as_reader().get_dates());
        assert_eq!(dates.get(0).get_day(), 1);
        assert_eq!(dates.get(1).get_month(), 5);
        assert_eq!(dates.get(2).get_year_as_text().unwrap().to_str(), Ok("9"));
        Promise::ok(())
    }

    let date_list = message.init_root::<date_list::Builder>();
    structlist_tests_impl(date_list).await
}

#[tokio::test]
async fn capnp_build_list_test() -> capnp::Result<()> {
    fn structlist_pattern_test_impl(
        mut date_list: capnp::struct_list::Builder<example_capnp::date::Owned>,
    ) -> Promise<(), capnp::Error> {
        capnp_build!(date_list, [{day = 1, month = 2, year_as_text = "1990"},
                             {day = 2, month = 3, year_as_text = "1234"},
                             {day = 7, month = 8, year_as_text = "2023"}]);
        let date_reader = date_list.into_reader();
        assert_eq!(date_reader.get(0).get_day(), 1);
        assert_eq!(date_reader.get(1).get_month(), 3);
        assert_eq!(
            date_reader.get(2).get_year_as_text().unwrap().to_str(),
            Ok("2023")
        );
        Promise::ok(())
    }

    let mut message = capnp::message::Builder::new_default();
    let date_list: capnp::struct_list::Builder<example_capnp::date::Owned> = message.initn_root(3);
    structlist_pattern_test_impl(date_list).await?;

    fn textlist_assign_from_expression_test_impl(
        mut text_list: text_list::Builder,
    ) -> Promise<(), capnp::Error> {
        capnp_build!(text_list, {items: [="a", ="b", ="long_text"]});
        let items = capnp_rpc::pry!(text_list.reborrow_as_reader().get_items());
        assert_eq!(items.get(0).unwrap().to_str(), Ok("a"));
        assert_eq!(items.get(1).unwrap().to_str(), Ok("b"));
        assert_eq!(items.get(2).unwrap().to_str(), Ok("long_text"));
        Promise::ok(())
    }

    let mut message = capnp::message::Builder::new_default();
    let text_list = message.init_root::<text_list::Builder>();
    textlist_assign_from_expression_test_impl(text_list).await
}
