@0xb52bd743a2d5af47;

using Rust = import "rust.capnp";

$Rust.parentModule("baz");

struct Baz {
    recursive @0 :Baz;
}
