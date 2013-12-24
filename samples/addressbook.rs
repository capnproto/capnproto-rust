/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[crate_type = "bin"];

extern mod capnp;

pub mod addressbook_capnp;

fn write_address_book() {
    use capnp::message::MessageBuilder;
    use capnp::serialize_packed::{WritePackedWrapper, WritePacked};
    use addressbook_capnp::{AddressBook, Person};

    MessageBuilder::new_default(|message| {

        let address_book = message.init_root::<AddressBook::Builder>();

        let people = address_book.init_people(2);

        let alice = people[0];
        alice.set_id(123);
        alice.set_name("Alice");
        alice.set_email("alice@example.com");

        let alice_phones = alice.init_phones(1);
        alice_phones[0].set_number("555-1212");
        alice_phones[0].set_type(Person::PhoneNumber::Type::Mobile);
        alice.get_employment().set_school("MIT");

        let bob = people[1];
        bob.set_id(456);
        bob.set_name("Bob");
        bob.set_email("bob@example.com");
        let bob_phones = bob.init_phones(2);
        bob_phones[0].set_number("555-4567");
        bob_phones[0].set_type(Person::PhoneNumber::Type::Home);
        bob_phones[1].set_number("555-7654");
        bob_phones[1].set_type(Person::PhoneNumber::Type::Work);
        bob.get_employment().set_unemployed(());

        WritePackedWrapper{writer:&mut std::io::stdout()}.write_packed_message(message);
    });
}

fn print_address_book() {
    use capnp;
    use addressbook_capnp::{AddressBook, Person};

    let mut inp1 = std::io::stdin();
    let mut inp = capnp::serialize_packed::PackedInputStream {
        inner : &mut capnp::io::BufferedInputStream::new(&mut inp1)
    };

    capnp::serialize::InputStreamMessageReader::new(
        &mut inp, capnp::message::DEFAULT_READER_OPTIONS,
        |message_reader| {
        let address_book = message_reader.get_root::<AddressBook::Reader>();
        let people = address_book.get_people();

        for i in range(0, people.size()) {
            let person = people[i];
            println!("{}: {}", person.get_name(), person.get_email());
            let phones = person.get_phones();
            for j in range(0, phones.size()) {
                let phone = phones[j];
                let type_name = match phone.get_type() {
                    Some(Person::PhoneNumber::Type::Mobile) => {"mobile"}
                    Some(Person::PhoneNumber::Type::Home) => {"home"}
                    Some(Person::PhoneNumber::Type::Work) => {"work"}
                    None => {"UNKNOWN"}
                };
                println!("  {} phone: {}", type_name, phone.get_number());

            }
            match person.get_employment().which() {
                Some(Person::Employment::Unemployed(())) => {
                    println("  unemployed");
                }
                Some(Person::Employment::Employer(employer)) => {
                    println!("  employer: {}", employer);
                }
                Some(Person::Employment::School(school)) => {
                    println!("  student at: {}", school);
                }
                Some(Person::Employment::SelfEmployed(())) => {
                    println("  self-employed");
                }
                None => { }
            }
        }
    });
}

fn main() {

    let args = std::os::args();
    if (args.len() < 2) {
        println!("usage: $ {} [write | read]", args[0]);
    } else {
        match args[1] {
            ~"write" => write_address_book(),
            ~"read" => print_address_book(),
            _ => {println("unrecognized argument") }
        }
    }

}
