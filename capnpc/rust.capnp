# This file contains annotations that are recognized by the capnpc-rust code generator.

@0x83b3c14c3c8dd083;

annotation name @0xc2fe4c6d100166d0 (field, struct, enum, enumerant, union, group) :Text;
# Rename something in the generated code. The value that you specify in this
# annotation should follow capnp capitalization conventions. So, for example,
# a struct should use CamelCase capitalization like `StructFoo`, even though
# that will get translated to a `struct_foo` module in the generated Rust code.
#
# TODO: support annoting more kinds of things with this.
