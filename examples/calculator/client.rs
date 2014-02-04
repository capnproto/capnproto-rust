/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use capnp_rpc::ez_rpc::EzRpcClient;
use capnp_rpc::rpc::{InitParams, WaitForContent};
use calculator_capnp::Calculator;

pub fn main() {
    let args = std::os::args();
    if args.len() != 3 {
        println!("usage: {} client <server address>", args[0]);
        return;
    }

    let mut rpc_client = EzRpcClient::new(args[2]);

    let calculator_client : Calculator::Client  = rpc_client.import_cap("calculator");

    {
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
    }


    {
        let mut req = calculator_client.evaluate_request();
        {
            let params = req.init_params();
            let exp = params.init_expression();
            exp.set_literal(55.5);
        }
        let res = req.send();
        let mut result = res.pipeline.get_value().read_request().send();
        let answer = result.wait().get_value();
        println!("the value is: {}", answer);
    }




    rpc_client.netcat.wait();
}
