#[macro_escape];

pub mod PrimitiveList {
    use layout::*;

    pub struct Reader<'self> {
        reader : ListReader<'self>
    }

    impl <'self> Reader<'self> {
        pub fn new<'a>(reader : ListReader<'a>) -> Reader<'a> {
            Reader { reader : reader }
        }

        pub fn size(&self) -> uint { self.reader.size() }

        pub fn get<T : Copy>(&self, index : uint) -> T {
            self.reader.getDataElement(index)
        }
    }
}

// The struct list reader needs to be able to instantiate element readers
// of the appropriate type.

macro_rules! list_submodule(
    ( $capnp:ident, $($m:ident)::+ ) => (
        pub mod List {
            use layout;
            use $capnp;

            pub struct Reader<'self> {
                reader : layout::ListReader<'self>
            }

            impl <'self> Reader<'self> {
                pub fn new<'a>(reader : layout::ListReader<'a>) -> Reader<'a> {
                    Reader { reader : reader }
                }
                pub fn size(&self) -> uint { self.reader.size() }
                pub fn get(&self, index : uint) -> $capnp::$($m)::+::Reader<'self> {
                    $capnp::$($m)::+::Reader::new(self.reader.getStructElement(index))
                }
            }
        }
    );
)
