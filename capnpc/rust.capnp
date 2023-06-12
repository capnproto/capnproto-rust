# This file contains annotations that are recognized by the capnpc-rust code generator.
#
# To use this file, you will need to make sure that it is included in the directories
# searched by `capnp compile`. An easy way to do that is to copy it into your project
# alongside your own schema files.

@0x83b3c14c3c8dd083;

annotation name @0xc2fe4c6d100166d0 (field, struct, enum, enumerant, union, group) :Text;
# Rename something in the generated code. The value that you specify in this
# annotation should follow capnp capitalization conventions. So, for example,
# a struct should use CamelCase capitalization like `StructFoo`, even though
# that will get translated to a `struct_foo` module in the generated Rust code.
#
# TODO: support annotating more kinds of things with this.

annotation parentModule @0xabee386cd1450364 (file) :Text;
# A Rust module path indicating where the generated code will be included.
# For example, if this is set to "foo::bar" and the schema file is named
# "baz.capnp", then you could include the generated code like this:
#
#  pub mod foo {
#    pub mod bar {
#      pub mod baz_capnp {
#        include!(concat!(env!("OUT_DIR"), "/baz_capnp.rs"));
#      }
#    }
#  }

annotation option @0xabfef22c4ee1964e (field) :Void;
# Make the generated getters return Option<T> instead of T. Supported on
# pointer types (e.g. structs, lists, and blobs).
#
# Capnp pointer types are nullable. Normally get_field will return the default
# value if the field isn't set. With this annotation you get Some(...) when
# the field is set and None when it isn't.
#
# Given
#
#     struct Test {
#         field @0 :Text $Rust.option;
#     }
#
# you get getters like so
#
#     assert_eq!(struct_with.get_field(), Some("foo"));
#     assert_eq!(struct_without.get_field(), None));
#
# The setters are unchanged to match the Rust convention.
#
# Note: Support for this annotation on interfaces isn't implemented yet.
