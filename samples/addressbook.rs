/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[feature(macro_rules)];

#[link(name = "addressbook", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnprust;

pub mod addressbook_capnp;

fn writeAddressBook() {
    use capnprust::message::MessageBuilder;
    use capnprust::serialize_packed::{WritePackedWrapper, WritePacked};
    use addressbook_capnp::{AddressBook, Person};

    let mut message = MessageBuilder::new_default();

    let addressbook = message.initRoot::<AddressBook::Builder>();

    let people = addressbook.initPeople(4);

    people[0].setId(1);
    people[0].setName("Alice");
    people[0].setEmail("alice@widgco.biz");

    let phones = people[0].initPhones(2);
    phones[0].setNumber("(555) 555-5555");
    phones[0].setType(Person::PhoneNumber::Type::Work);
    phones[1].setNumber("(777) 123-4567");
    phones[1].setType(Person::PhoneNumber::Type::Home);
    people[0].getEmployment().setEmployer("widgco");

    people[1].setId(2);
    people[1].setName("Bob");
    people[1].setEmail("bob@bobnet.org");
    people[1].getEmployment().setSelfEmployed(());

    people[2].setId(3);
    people[2].setName("Charlie");
    people[2].setEmail("chuckie@cccc.ch");
    people[2].getEmployment().setUnemployed(());

    people[3].setId(255);
    people[3].setEmail("di@di.com");
    people[3].setName("Diane");
    people[3].getEmployment().setSchool("Caltech");

//    capnprust::serialize::writeMessage(&mut std::io::stdout(), message)
    WritePackedWrapper{writer:&mut std::io::stdout()}.writePackedMessage(message);
}

fn printAddressBook() {
    use capnprust;
    use addressbook_capnp::{AddressBook, Person};

    let mut inp1 = std::io::stdin();
    let mut inp = capnprust::serialize_packed::PackedInputStream {
        inner : &mut capnprust::io::BufferedInputStream::new(&mut inp1)
    };
//    let mut inp = std::io::stdin();

    do capnprust::serialize::InputStreamMessageReader::new(
        &mut inp, capnprust::message::DEFAULT_READER_OPTIONS) |messageReader| {
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
