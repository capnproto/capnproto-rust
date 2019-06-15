extern crate capnpc;

fn main() {
    capnpc::CompilerCommand::new()
        .src_prefix("schema")
        .file("schema/rpc.capnp")
        .file("schema/rpc-twoparty.capnp")
        .edition(capnpc::RustEdition::Rust2018)
        .run().expect("capnp compile");
}
