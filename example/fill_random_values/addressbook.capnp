@0x9eb32e19f86ee174;

using Fill = import "fill.capnp";
using Corpora = import "corpora.capnp";

struct Person {
  id @0 :UInt32;
  name @1 :Text $Fill.SelectFrom(List(Text)).choices(Corpora.scientists);
  email @2 :Text $Fill.SelectFrom(List(Text)).choices(Corpora.emails);
  phones @3 :List(PhoneNumber) $Fill.lengthRange((min = 0, max = 3));

  struct PhoneNumber {
    number @0 :Text $Fill.phoneNumber;
    type @1 :Type;

    enum Type {
      mobile @0;
      home @1;
      work @2;
    }
  }

  employment :union {
    unemployed @4 :Void;

    employer @5 :Text $Fill.SelectFrom(List(Text)).choices(Corpora.corporations);

    school @6 :Text $Fill.SelectFrom(List(Text)).choices(Corpora.schools);
    selfEmployed @7 :Void;
  }
}

struct AddressBook {
  people @0 :List(Person) $Fill.lengthRange((max = 5));
}

