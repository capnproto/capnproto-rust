use std::str;
use std::path::Path;
use std::process::{self, Command};

extern crate capnpc;

fn main() {

    // CAPNP_INCLUDE_DIR=$(shell dirname $(shell which capnp))/../include

    let prefix = {
        let output = Command::new("which")
                             .arg("capnp")
                             .output()
                             .expect("Failed to run `which capnp`");

        if !output.status.success() {
            println!("Failed to find `capnp` executable");
            process::exit(1);
        }

        let path = Path::new(str::from_utf8(&output.stdout).unwrap());
        path.join("../../include/capnp")
    };

    capnpc::compile(&prefix,
                    &[prefix.join("rpc.capnp"),
                      prefix.join("rpc-twoparty.capnp")]).unwrap();
}
