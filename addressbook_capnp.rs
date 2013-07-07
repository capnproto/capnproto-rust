pub mod Person {
    use layout::*;
//    use addressbook_capnp::*;

    pub struct Reader<'self> {
        _reader : StructReader<'self>
    }

    impl <'self> Reader<'self> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ _reader : reader }
        }

        pub fn totalSizeInWords(&self) -> uint {
            self._reader.totalSize() as uint
        }
    }

    pub struct Builder {
        _builder : StructBuilder
    }

    list_submodule!(addressbook_capnp, Person)
}

pub mod AddressBook {
    use layout::*;
    use addressbook_capnp::*;

    pub struct Reader<'self> {
        _reader : StructReader<'self>
    }

    impl <'self> Reader<'self> {

        pub fn new<'a>(reader : StructReader<'a>) -> Reader<'a> {
            Reader{ _reader : reader }
        }

        pub fn totalSizeInWords(&self) -> uint {
            self._reader.totalSize() as uint
        }

        pub fn getPeople(&self) -> Person::List::Reader<'self> {
            Person::List::Reader::new(self._reader.getListField(0, INLINE_COMPOSITE, 0))
        }
    }

    pub struct Builder {
        _builder : StructBuilder
    }

    impl Builder {
        pub fn new(builder : StructBuilder) -> Builder {
            Builder { _builder : builder }
        }

/*
        pub fn initRoot() -> Builder {
        }
*/

        pub fn initPeople(&self, size : uint) -> uint {
            fail!()
        }

    }

    pub static STRUCT_SIZE : StructSize = StructSize {data : 0, pointers : 1,
                                                      preferredListEncoding : POINTER};

    list_submodule!(addressbook_capnp, AddressBook)
}