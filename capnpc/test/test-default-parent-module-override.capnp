@0xd603e26b6cf7ff81;

using Rust = import "rust.capnp";

# This should override the CompilerCommand::default_parent_module() option.
$Rust.parentModule("test_default_parent_module");

struct Baz {
    n @0 :UInt32;
}
