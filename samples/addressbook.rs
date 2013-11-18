/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[feature(macro_rules)];

#[link(name = "addressbook", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnp;

pub mod addressbook_capnp;

fn writeAddressBook() {
    use capnp::message::MessageBuilder;
    use capnp::serialize_packed::{WritePackedWrapper, WritePacked};
    use addressbook_capnp::{AddressBook, Person};

    let mut message = MessageBuilder::new_default();

    let addressbook = message.initRoot::<AddressBook::Builder>();

    let people = addressbook.initPeople(2);

    let alice = people[0];
    alice.setId(123);
    alice.setName("Alice");
    alice.setEmail("alice@example.com");

    let alicePhones = alice.initPhones(1);
    alicePhones[0].setNumber("555-1212");
    alicePhones[0].setType(Person::PhoneNumber::Type::Mobile);
    alice.getEmployment().setSchool("MIT");

    let bob = people[1];
    bob.setId(456);
    bob.setName("Bob");
    bob.setEmail("bob@example.com");
    let bobPhones = bob.initPhones(2);
    bobPhones[0].setNumber("555-4567");
    bobPhones[0].setType(Person::PhoneNumber::Type::Home);
    bobPhones[1].setNumber("555-7654");
    bobPhones[1].setType(Person::PhoneNumber::Type::Work);
    bob.getEmployment().setUnemployed(());

    WritePackedWrapper{writer:&mut std::io::stdout()}.writePackedMessage(message);
}

fn printAddressBook() {
    use capnp;
    use addressbook_capnp::{AddressBook, Person};

    let mut inp1 = std::io::stdin();
    let mut inp = capnp::serialize_packed::PackedInputStream {
        inner : &mut capnp::io::BufferedInputStream::new(&mut inp1)
    };

    do capnp::serialize::InputStreamMessageReader::new(
        &mut inp, capnp::message::DEFAULT_READER_OPTIONS) |messageReader| {
        let addressBook =
            AddressBook::Reader::new(messageReader.getRoot());
        let people = addressBook.getPeople();

        for i in range(0, people.size()) {
            let person = people[i];
            println!("{}: {}", person.getName(), person.getEmail());
            let phones = person.getPhones();
            for j in range(0, phones.size()) {
                let phone = phones[j];
                let typeName = match phone.getType() {
                    Some(Person::PhoneNumber::Type::Mobile) => {"mobile"}
                    Some(Person::PhoneNumber::Type::Home) => {"home"}
                    Some(Person::PhoneNumber::Type::Work) => {"work"}
                    None => {"UNKNOWN"}
                };
                println!("  {} phone: {}", typeName, phone.getNumber());

            }
            match person.getEmployment().which() {
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
    }
}

fn main() {

    let args = std::os::args();
    if (args.len() < 2) {
        println!("usage: $ {} [write | read]", args[0]);
    } else {
        match args[1] {
            ~"write" => writeAddressBook(),
            ~"read" => printAddressBook(),
            _ => {println("unrecognized argument") }
        }
    }

}
