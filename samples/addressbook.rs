/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[link(name = "capnproto-rust-test", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnprust;

use capnprust::*;

pub mod addressbook_capnp;

fn writeAddressBook() {
    use capnprust::message::*;
    use addressbook_capnp::*;

    let message = MessageBuilder::new_default();

    let addressbook = AddressBook::Builder::new(message.initRoot(AddressBook::STRUCT_SIZE));

    let people = addressbook.initPeople(4);

    let person = people.get(0);
    person.setId(1);
    person.setName("Alice");
    person.setEmail("alice@widgco.biz");

    let phones = person.initPhones(2);
    phones.get(0).setNumber("(555) 555-5555");
    phones.get(0).setType(Person::PhoneNumber::Type::WORK);
    phones.get(1).setNumber("(777) 123-4567");
    phones.get(1).setType(Person::PhoneNumber::Type::HOME);
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

    let outStream = @std::io::stdout() as @serialize::OutputStream;

//    serialize::writeMessage(outStream, message)
    serialize_packed::writePackedMessage(outStream, message)
}

fn printAddressBook() {
    use capnprust::serialize::*;
    use capnprust::serialize_packed::*;
    use addressbook_capnp::*;

    let inp = @PackedInputStream { inner : std::io::stdin()} as @std::io::Reader;
//    let inp = std::io::stdin();

    do InputStreamMessageReader::new(inp, message::DEFAULT_READER_OPTIONS) | messageReader | {
        let addressBook =
            AddressBook::Reader::new(messageReader.getRoot());
        let people = addressBook.getPeople();

        for std::uint::iterate(0, people.size()) |i| {
            let person = people.get(i);
            printfln!("%s: %s", person.getName(), person.getEmail());
            let phones = person.getPhones();
            for std::uint::iterate(0, phones.size()) |j| {
                let phone = phones.get(j);
                let typeName = match phone.getType() {
                    Person::PhoneNumber::Type::MOBILE => {"mobile"}
                    Person::PhoneNumber::Type::HOME => {"home"}
                    Person::PhoneNumber::Type::WORK => {"work"}
                };
                printfln!("  %s phone: %s", typeName, phone.getNumber());

            }
            match person.getEmployment() {
                Person::Employment::unemployed(()) => {
                    println("  unemployed");
                }
                Person::Employment::employer(employer) => {
                    printfln!("  employer: %s", employer);
                }
                Person::Employment::school(school) => {
                    printfln!("  student at: %s", school);
                }
                Person::Employment::selfEmployed(()) => {
                    println("  self-employed");
                }
            }

        }

    }
}

fn main() {

    let args = std::os::args();
    if (args.len() < 2) {
        std::io::println(fmt!("usage: $ %s [write | read]", args[0]));
    } else {
        match args[1] {
            ~"write" => writeAddressBook(),
            ~"read" => printAddressBook(),
            _ => {std::io::println("unrecognized argument") }
        }
    }

}
