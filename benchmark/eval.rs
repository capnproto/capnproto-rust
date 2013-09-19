/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use std::rand::*;
use common::*;
use eval_capnp::*;

pub type RequestBuilder = Expression::Builder;
pub type ResponseBuilder = EvaluationResult::Builder;
pub type Expectation = i32;

fn makeExpression(rng : &mut FastRand, exp : Expression::Builder, depth : uint) -> i32 {
    exp.setOp(unsafe {
            std::cast::transmute(rng.gen_uint_range(0, Operation::modulus as uint + 1))});

    let mut left : u32 = 0;
    let mut right : u32 = 0;

    if (rng.nextLessThan(8) < (depth as u32)) {
        left = rng.nextLessThan(128) + 1;
        exp.getLeft().setValue(left as i32);
    } else {
        // how are we going to do this?
    }

    0
}
