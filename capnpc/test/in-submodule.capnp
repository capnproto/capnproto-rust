@0xb9523c11cf10d3bd;

using Rust = import "rust.capnp";
using Other = import "in-other-submodule.capnp";

$Rust.parentModule("foo::bar");

struct Foo {
   recursive @0 :Foo;
   other @1: Other.Baz;
}


