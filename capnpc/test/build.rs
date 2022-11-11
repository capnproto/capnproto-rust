fn main() {
    capnpc::CompilerCommand::new()
        .file("test.capnp")
        .file("in-submodule.capnp")
        .file("in-other-submodule.capnp")
        .file("schema/test-in-dir.capnp")
        .file("schema-with-src-prefix/test-in-src-prefix-dir.capnp")
        .src_prefix("schema-with-src-prefix")
        .raw_code_generator_request_path(
            std::env::var("OUT_DIR").expect("OUT_DIR env var is not set")
                + "/raw_code_gen_request.bin",
        )
        .run()
        .expect("compiling schema");

    capnpc::CompilerCommand::new()
        .file("test-default-parent-module.capnp")
        .file("test-default-parent-module-override.capnp")
        .default_parent_module(vec![
            "test_default_parent_module".into(),
            "test_default_parent_module_inner".into(),
        ])
        .run()
        .expect("compiling schema");
}
