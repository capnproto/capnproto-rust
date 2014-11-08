use std::os;
use std::io::Command;

fn main() {

    // CAPNP_INCLUDE_DIR=$(shell dirname $(shell which capnp))/../include

    let include_dir = {
        let output = Command::new("which").arg("capnp")
            .output().unwrap().output;
        let path = Path::new(output.as_slice());
        let mut path1 = Path::new(path.dirname());
        path1.push("../include");
        format!("{}", path1.display())
    };

    let out_dir = os::getenv("OUT_DIR").unwrap();

    let _output = Command::new("capnp")
        .arg("compile")
        .arg(format!("-orust:{}", out_dir))
        .arg(format!("--src-prefix={}/capnp", include_dir))
        .arg(format!("{}/capnp/rpc.capnp", include_dir))
        .arg(format!("{}/capnp/rpc-twoparty.capnp", include_dir))
        .output()
        .unwrap();

}
