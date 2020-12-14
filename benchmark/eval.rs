// Copyright (c) 2013-2014 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use capnp::private::arena::{BuilderArena, ReaderArena};

use crate::common::*;
use crate::eval_capnp::{expression, evaluation_result, Operation};

fn make_expression(rng: &mut FastRand, mut exp: expression::Builder<impl BuilderArena>, depth : u32) -> i32 {
    exp.set_op(::capnp::traits::FromU16::from_u16(rng.next_less_than( Operation::Modulus as u32 + 1) as u16).unwrap());

    let left : i32 =
    if rng.next_less_than(8) < depth {
        let tmp = (rng.next_less_than(128) + 1) as i32;
        exp.reborrow().get_left().set_value(tmp);
        tmp
    } else {
        make_expression(rng, exp.reborrow().get_left().init_expression(), depth + 1)
    };

    let right : i32 =
    if rng.next_less_than(8) < depth {
        let tmp = (rng.next_less_than(128) + 1) as i32;
        exp.reborrow().get_right().set_value(tmp);
        tmp
    } else {
        make_expression(rng, exp.reborrow().get_right().init_expression(), depth + 1)
    };

    match exp.get_op().unwrap() {
        Operation::Add => { return left + right }
        Operation::Subtract => { return left - right }
        Operation::Multiply => { return left * right }
        Operation::Divide => { return div(left, right) }
        Operation::Modulus => { return modulus(left, right) }
    }
}

fn evaluate_expression(exp: expression::Reader<impl ReaderArena>) -> ::capnp::Result<i32> {
    let left = match exp.get_left().which()? {
        expression::left::Value(v) => v,
        expression::left::Expression(e) => evaluate_expression(e?)?,
    };
    let right = match exp.get_right().which()? {
        expression::right::Value(v) => v,
        expression::right::Expression(e) => evaluate_expression(e?)?,
    };

    match exp.get_op()? {
        Operation::Add => Ok(left + right),
        Operation::Subtract => Ok(left - right),
        Operation::Multiply => Ok(left * right),
        Operation::Divide => Ok(div(left, right)),
        Operation::Modulus => Ok(modulus(left, right)),
    }
}

pub struct Eval;

impl crate::TestCase for Eval {
    type Request = expression::Owned;
    type Response = evaluation_result::Owned;
    type Expectation = i32;

    fn setup_request<A>(&self, rng: &mut FastRand, request: expression::Builder<A>) -> i32 where A: BuilderArena {
        make_expression(rng, request, 0)
    }

    fn handle_request<A,B>(&self, request: expression::Reader<A>, mut response: evaluation_result::Builder<B>)
                           -> ::capnp::Result<()>
        where A: ReaderArena, B: BuilderArena
    {
        response.set_value(evaluate_expression(request)?);
        Ok(())
    }

    fn check_response<A>(&self, response: evaluation_result::Reader<A>, expected : i32) -> ::capnp::Result<()>
        where A: ReaderArena
    {
        if response.get_value() == expected {
            Ok(())
        } else {
            Err(::capnp::Error::failed(
                format!("check_response() expected {} but got {}", expected, response.get_value())))
        }
    }
}
