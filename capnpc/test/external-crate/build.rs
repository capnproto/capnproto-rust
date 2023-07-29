capnp_import::capnp_extract_bin!();

fn main() {
    let output_dir = commandhandle().unwrap();
    let cmdpath = output_dir.path().join("capnp");

    capnpc::CompilerCommand::new()
        .capnp_executable(cmdpath)
        .file("external.capnp")
        .run()
        .expect("compiling schema");
}
