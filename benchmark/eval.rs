/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std;
use std::rand::*;
use capnprust;
use common::*;
use eval_capnp::*;

pub type RequestBuilder = Expression::Builder;
pub type ResponseBuilder = EvaluationResult::Builder;
pub type Expectation = i32;

pub fn newRequestReader<'a>(sr : capnprust::layout::StructReader<'a>) -> Expression::Reader<'a> {
    Expression::Reader::new(sr)
}

pub fn newResponseReader<'a>(sr : capnprust::layout::StructReader<'a>) -> EvaluationResult::Reader<'a> {
    EvaluationResult::Reader::new(sr)
}

fn makeExpression(rng : &mut FastRand, exp : Expression::Builder, depth : u32) -> i32 {
    exp.setOp(unsafe {
            std::cast::transmute(rng.gen_range::<u16>(0, Operation::Modulus as u16 + 1))});

    let left : i32 =
    if (rng.nextLessThan(8) < depth) {
        let tmp = (rng.nextLessThan(128) + 1) as i32;
        exp.getLeft().setValue(tmp);
        tmp
    } else {
        makeExpression(rng, exp.getLeft().initExpression(), depth + 1)
    };

    let right : i32 =
    if (rng.nextLessThan(8) < depth) {
        let tmp = (rng.nextLessThan(128) + 1) as i32;
        exp.getRight().setValue(tmp);
        tmp
    } else {
        makeExpression(rng, exp.getRight().initExpression(), depth + 1)
    };

    match exp.getOp() {
        Some(Operation::Add) => { return left + right }
        Some(Operation::Subtract) => { return left - right }
        Some(Operation::Multiply) => { return left * right }
        Some(Operation::Divide) => { return div(left, right) }
        Some(Operation::Modulus) => { return modulus(left, right) }
        None => { fail!("impossible") }
    }
}

fn evaluateExpression(exp : Expression::Reader) -> i32 {
    let left = match exp.getLeft().which() {
        Some(Expression::Left::Value(v)) => v,
        Some(Expression::Left::Expression(e)) => evaluateExpression(e),
        None => fail!("impossible")
    };
    let right = match exp.getRight().which() {
        Some(Expression::Right::Value(v)) => v,
        Some(Expression::Right::Expression(e)) => evaluateExpression(e),
        None => fail!("impossible")
    };

    match exp.getOp() {
        Some(Operation::Add) => return left + right,
        Some(Operation::Subtract) => return left - right,
        Some(Operation::Multiply) => return left * right,
        Some(Operation::Divide) => return div(left, right),
        Some(Operation::Modulus) => return modulus(left, right),
        None => fail!("impossible")
    }
}

#[inline]
pub fn setupRequest(rng : &mut FastRand, request : Expression::Builder) -> i32 {
    makeExpression(rng, request, 0)
}

#[inline]
pub fn handleRequest(request : Expression::Reader, response : EvaluationResult::Builder) {
    response.setValue(evaluateExpression(request));
}

#[inline]
pub fn checkResponse(response : EvaluationResult::Reader, expected : i32) -> bool {
    response.getValue() == expected
}
