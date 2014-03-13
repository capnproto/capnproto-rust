/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use rand::*;
use common::*;
use eval_capnp::*;

pub type RequestBuilder<'a> = Expression::Builder<'a>;
pub type ResponseBuilder<'a> = EvaluationResult::Builder<'a>;
pub type Expectation = i32;
pub type RequestReader<'a> = Expression::Reader<'a>;
pub type ResponseReader<'a> = EvaluationResult::Reader<'a>;

fn make_expression(rng : &mut FastRand, exp : Expression::Builder, depth : u32) -> i32 {
    exp.set_op(unsafe {
            std::cast::transmute(rng.nextLessThan( Operation::Modulus as u32 + 1) as u16)});

    let left : i32 =
    if rng.nextLessThan(8) < depth {
        let tmp = (rng.nextLessThan(128) + 1) as i32;
        exp.get_left().set_value(tmp);
        tmp
    } else {
        make_expression(rng, exp.get_left().init_expression(), depth + 1)
    };

    let right : i32 =
    if rng.nextLessThan(8) < depth {
        let tmp = (rng.nextLessThan(128) + 1) as i32;
        exp.get_right().set_value(tmp);
        tmp
    } else {
        make_expression(rng, exp.get_right().init_expression(), depth + 1)
    };

    match exp.get_op() {
        Some(Operation::Add) => { return left + right }
        Some(Operation::Subtract) => { return left - right }
        Some(Operation::Multiply) => { return left * right }
        Some(Operation::Divide) => { return div(left, right) }
        Some(Operation::Modulus) => { return modulus(left, right) }
        None => { fail!("impossible") }
    }
}

fn evaluate_expression(exp : Expression::Reader) -> i32 {
    let left = match exp.get_left().which() {
        Some(Expression::Left::Value(v)) => v,
        Some(Expression::Left::Expression(e)) => evaluate_expression(e),
        None => fail!("impossible")
    };
    let right = match exp.get_right().which() {
        Some(Expression::Right::Value(v)) => v,
        Some(Expression::Right::Expression(e)) => evaluate_expression(e),
        None => fail!("impossible")
    };

    match exp.get_op() {
        Some(Operation::Add) => return left + right,
        Some(Operation::Subtract) => return left - right,
        Some(Operation::Multiply) => return left * right,
        Some(Operation::Divide) => return div(left, right),
        Some(Operation::Modulus) => return modulus(left, right),
        None => fail!("impossible")
    }
}

#[inline]
pub fn setup_request(rng : &mut FastRand, request : Expression::Builder) -> i32 {
    make_expression(rng, request, 0)
}

#[inline]
pub fn handle_request(request : Expression::Reader, response : EvaluationResult::Builder) {
    response.set_value(evaluate_expression(request));
}

#[inline]
pub fn check_response(response : EvaluationResult::Reader, expected : i32) -> bool {
    response.get_value() == expected
}
