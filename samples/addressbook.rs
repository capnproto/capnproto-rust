/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[feature(macro_rules)];

#[link(name = "capnproto-rust-test", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnprust;

pub mod addressbook_capnp;

fn writeAddressBook() {
    use capnprust::message::MessageBuilder;
    use addressbook_capnp::{AddressBook, Person};

    let mut message = MessageBuilder::new_default();

    let addressbook = message.initRoot::<AddressBook::Builder>();


    let people = addressbook.initPeople(4);

    let person = people.get(0);
    person.setId(1);
    person.setName("Alice");
    person.setEmail("alice@widgco.biz");

    let phones = person.initPhones(2);
    phones.get(0).setNumber("(555) 555-5555");
    phones.get(0).setType(Person::PhoneNumber::Type::Work);
    phones.get(1).setNumber("(777) 123-4567");
    phones.get(1).setType(Person::PhoneNumber::Type::Home);
    person.getEmployment().setEmployer("widgco");

    let person = people.get(1);
    person.setId(2);
    person.setName("Bob");
    person.setEmail("bob@bobnet.org");
    person.getEmployment().setSelfEmployed(());

    let person = people.get(2);
    person.setId(3);
    person.setName("Charlie");
    person.setEmail("chuckie@cccc.ch");
    person.getEmployment().setUnemployed(());

    let person = people.get(3);
    person.setId(255);
    person.setEmail("di@di.com");
    person.setName("Diane");
    person.getEmployment().setSchool("Caltech");

//    capnprust::serialize::writeMessage(&mut std::rt::io::stdout(), message)
    capnprust::serialize_packed::writePackedMessage(&mut std::rt::io::stdout(), message)
}

fn printAddressBook() {
    use capnprust;
    use addressbook_capnp::{AddressBook, Person};

    let mut inp = capnprust::serialize_packed::PackedInputStream {
        inner : &mut std::rt::io::stdin() };
//    let inp = std::io::stdin();

    do capnprust::serialize::InputStreamMessageReader::new(
        &mut inp, capnprust::message::DEFAULT_READER_OPTIONS) |messageReader| {
        let addressBook =
            AddressBook::Reader::new(messageReader.getRoot());
        let people = addressBook.getPeople();

        for i in range(0, people.size()) {
            let person = people.get(i);
            println!("{}: {}", person.getName(), person.getEmail());
            let phones = person.getPhones();
            for j in range(0, phones.size()) {
                let phone = phones.get(j);
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
