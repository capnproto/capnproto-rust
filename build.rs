extern crate capnpc;

fn main() {

    // CAPNP_INCLUDE_DIR=$(shell dirname $(shell which capnp))/../include

    let prefix = {
        let output = ::std::process::Command::new("which").arg("capnp")
            .output().unwrap().stdout;
        let path = ::std::path::Path::new(::std::str::from_utf8(&output).unwrap());
        let mut path1 = path.parent().unwrap().parent().unwrap().to_path_buf();
        path1.push("include/capnp");
        path1
    };

    ::capnpc::compile(&prefix,
                      &[&prefix.clone().join("rpc.capnp"),
                        &prefix.join("rpc-twoparty.capnp")]).unwrap();
}
