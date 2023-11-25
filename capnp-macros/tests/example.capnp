@0xdbb9ad1f14bf0b36;  # unique file ID, generated by `capnp id`

struct Person {
  name @0 :Text;
  birthdate @2 :Date;

  email @1 :Text;

}

struct Date {
  yearAsText @0 :Text;
  month @1 :UInt8;
  day @2 :UInt8;
}

struct TextList {
  items @0 :List(Text);
}

struct DateList {
  dates @0 :List(Date);
}
