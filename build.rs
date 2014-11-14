extern crate capnpc;

fn main() {

    // CAPNP_INCLUDE_DIR=$(shell dirname $(shell which capnp))/../include

    let prefix = {
        let output = ::std::io::Command::new("which").arg("capnp")
            .output().unwrap().output;
        let path = Path::new(output.as_slice());
        let mut path1 = Path::new(path.dirname());
        path1.push("../include/capnp");
        path1
    };

    ::capnpc::compile(prefix.clone(),
                      vec!(prefix.join(Path::new("rpc.capnp")),
                           prefix.join(Path::new("rpc-twoparty.capnp"))).as_slice());
}
