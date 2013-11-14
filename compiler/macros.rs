/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[macro_escape];

// The struct list reader needs to be able to instantiate element readers
// of the appropriate type.

macro_rules! list_submodule(
    ( $capnp:ident::$($m:ident)::+ ) => (
        pub mod List {
            use capnprust;
            use $capnp;

            pub struct Reader<'a> {
                reader : capnprust::layout::ListReader<'a>
            }

            impl <'a> Reader<'a> {
                pub fn new<'b>(reader : capnprust::layout::ListReader<'b>) -> Reader<'b> {
                    Reader { reader : reader }
                }
                pub fn size(&self) -> uint { self.reader.size() }
            }

            impl <'a> Index<uint, $capnp::$($m)::+::Reader<'a>> for Reader<'a> {
                fn index(&self, index : &uint) -> $capnp::$($m)::+::Reader<'a> {
                    $capnp::$($m)::+::Reader::new(self.reader.getStructElement(*index))
                }
            }

            pub struct Builder {
                builder : capnprust::layout::ListBuilder
            }

            impl Builder {
                pub fn new(builder : capnprust::layout::ListBuilder) -> Builder {
                    Builder {builder : builder}
                }
                pub fn size(&self) -> uint { self.builder.size() }
            }

            impl Index<uint, $capnp::$($m)::+::Builder> for Builder {
                fn index(&self, index : &uint) -> $capnp::$($m)::+::Builder {
                    $capnp::$($m)::+::Builder::new(self.builder.getStructElement(*index))
                }
            }
        }
    );
)
