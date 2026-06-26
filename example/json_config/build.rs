fn main() {
    capnpc::CompilerCommand::new()
        .file("app_config.capnp")
        .run()
        .expect("schema compilation");
}
