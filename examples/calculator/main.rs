/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


#[crate_id="calculator"];
#[crate_type="bin"];

extern mod capnp;
extern mod extra;
extern mod capnp_rpc = "capnp-rpc";

pub mod calculator_capnp;


pub fn main() {
    use capnp_rpc::rpc::{InitParams, WaitForContent};
    use calculator_capnp::Calculator;

    let args = std::os::args();

    if args.len() != 2 {
        println!("usage: {} <server address>", args[0]);
        return;
    }

    let mut rpc_client = capnp_rpc::ez_rpc::EzRpcClient::new(args[1]);

    let calculator_client : Calculator::Client  = rpc_client.import_cap("calculator");

    let mut req = calculator_client.evaluate_request();
    {
        let params = req.init_params();
        let exp = params.init_expression();
        exp.set_literal(123.45);
    }
    let mut res = req.send();
    let value = {
        let results = res.wait();
        results.get_value()
    };

    let mut result = value.read_request().send();
    println!("the value is: {}", result.wait().get_value());


    rpc_client.netcat.wait();
}
