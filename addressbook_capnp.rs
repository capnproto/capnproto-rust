mod macros;

pub mod Person {
    use capnprust::layout::*;
//    use addressbook_capnp::*;

    pub static STRUCT_SIZE : StructSize = StructSize {data : 1, pointers : 4,
                                                      preferredListEncoding : INLINE_COMPOSITE};

    list_submodule!(addressbook_capnp, Person)

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

        pub fn getId(&self) -> u32 {
            self._reader.getDataField::<u32>(0)
        }

        pub fn getName(&self) -> &'self str {
            self._reader.getTextField(0, "")
        }

        pub fn getEmail(&self) -> &'self str {
            self._reader.getTextField(1, "")
        }

        pub fn getPhones(&self) -> PhoneNumber::List::Reader<'self> {
            PhoneNumber::List::Reader::new(
                self._reader.getListField(2, PhoneNumber::STRUCT_SIZE.preferredListEncoding,
                                          None))
        }

        pub fn getEmployment(&self) -> Employment::Reader<'self> {
            match self._reader.getDataField::<u16>(2) {
                0 => {
                    return Employment::UNEMPLOYED(())
                }
                1 => {
                    return Employment::EMPLOYER(
                        self._reader.getTextField(3, ""));
                }
                2 => {
                    return Employment::SCHOOL(
                        self._reader.getTextField(3, ""));
                }
                3 => { return Employment::SELF_EMPLOYED(()) }
                _ => fail!("impossible")
            }
        }
    }

    pub struct Builder {
        _builder : StructBuilder
    }

    impl Builder {
        pub fn new(builder : StructBuilder) -> Builder {
            Builder { _builder : builder }
        }

        pub fn setId(&self, value : u32) {
            self._builder.setDataField::<u32>(0, value);
        }

        pub fn setName(&self, value : &str) {
            self._builder.setTextField(0, value);
        }

        pub fn setEmail(&self, value : &str) {
            self._builder.setTextField(1, value);
        }

        pub fn initPhones(&self, size : uint) -> PhoneNumber::List::Builder {
            PhoneNumber::List::Builder::new(
                self._builder.initStructListField(2, size, PhoneNumber::STRUCT_SIZE))
        }

        pub fn getEmployment(&self) -> Employment::Builder {
            Employment::Builder::new(self._builder)
        }
    }

    pub mod Employment {
        use capnprust::layout::*;

        pub enum Reader<'self> {
            UNEMPLOYED(()),
            EMPLOYER(&'self str),
            SCHOOL(&'self str),
            SELF_EMPLOYED(())
        }

        pub struct Builder {
            _builder : StructBuilder
        }

        impl Builder {
            pub fn new(builder : StructBuilder) -> Builder {
                Builder { _builder : builder }
            }

            pub fn setUnemployed(&self, _value : ()) {
                self._builder.setDataField::<u16>(2, 0);
            }

            pub fn setEmployer(&self, value : &str) {
                self._builder.setDataField::<u16>(2, 1);
                self._builder.setTextField(3, value);
            }

            pub fn setSchool(&self, value : &str) {
                self._builder.setDataField::<u16>(2, 2);
                self._builder.setTextField(3, value);
            }

            pub fn setSelfEmployed(&self, _value : ()) {
                self._builder.setDataField::<u16>(2, 3);
            }
        }
    }

    pub mod PhoneNumber {
        use std;
        use capnprust::layout::*;
//        use addressbook_capnp::*;

        pub static STRUCT_SIZE : StructSize =
            StructSize {data : 1, pointers : 1,
                        preferredListEncoding : INLINE_COMPOSITE};

        list_submodule!(addressbook_capnp, Person::PhoneNumber)

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

            pub fn getNumber(&self) -> &'self str {
                self._reader.getTextField(0, "")
            }

            pub fn getType(&self) -> Type::Type {
                unsafe {
                    std::cast::transmute(self._reader.getDataField::<u16>(0) as uint)
                }
            }
        }

        pub struct Builder {
            _builder : StructBuilder
        }

        impl Builder {
            pub fn new(builder : StructBuilder) -> Builder {
                Builder { _builder : builder }
            }

            pub fn setNumber(&self, value : &str) {
                self._builder.setTextField(0, value)
            }

            pub fn setType(&self, value : Type::Type) {
                self._builder.setDataField::<u16>(0, value as u16)
            }

        }

        pub mod Type {
            pub enum Type {
                MOBILE = 0,
                HOME = 1,
                WORK = 2
            }
        }

    }
}

pub mod AddressBook {
    use capnprust::layout::*;
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
            Person::List::Reader::new(self._reader.getListField(0, INLINE_COMPOSITE, None))
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

        pub fn initPeople(&self, size : uint) -> Person::List::Builder {
            Person::List::Builder::new(
                self._builder.initStructListField(0, size, Person::STRUCT_SIZE))
        }

    }

    pub static STRUCT_SIZE : StructSize = StructSize {data : 0, pointers : 1,
                                                      preferredListEncoding : POINTER};

    list_submodule!(addressbook_capnp, AddressBook)
}