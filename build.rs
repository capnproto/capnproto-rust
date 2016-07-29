extern crate capnpc;

fn main() {
    capnpc::compile("schema",
                    &["schema/rpc.capnp",
                      "schema/rpc-twoparty.capnp"]).expect("capnp compile");
}
