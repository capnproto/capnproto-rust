//! Schema compiler plugin specialized for the sole purpose of bootstrapping schema.capnp.
//! Because the generated code lives in the capnp crate, we need to make sure that
//! it uses `crate::` rather than `::capnp::` to refer to things in that crate.

pub fn main() {
    ::capnpc::codegen::CodeGenerationCommand::new()
        .output_directory(::std::path::Path::new("."))
        .capnp_root("crate")
        .run(::std::io::stdin())
        .expect("failed to generate code");
}
