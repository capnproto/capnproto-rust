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
// of the appropriate type. It is implemented as a macro.