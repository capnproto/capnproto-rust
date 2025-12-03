fn main() {
    capnpc::CompilerCommand::new()
        .crate_provides("external_crate", [0xe6f94f52f7be8fe2])
        .crate_provides("capnp_json", [0x8ef99297a43a5e34]) // json.capnp
        .file("test.capnp")
        .file("json-test.capnp")
        .run()
        .expect("compiling schema");
}
