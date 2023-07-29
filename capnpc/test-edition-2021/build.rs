capnp_import::capnp_extract_bin!();

fn main() {
    let output_dir = commandhandle().unwrap();
    let cmdpath = output_dir.path().join("capnp");

    capnpc::CompilerCommand::new()
        .capnp_executable(cmdpath)
        .crate_provides("external_crate", [0xe6f94f52f7be8fe2])
        .file("../test/test.capnp")
        .src_prefix("../test/")
        .run()
        .expect("compiling schema");
}
