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

use std;
use common::*;
use eval_capnp::*;

pub type RequestBuilder<'a> = expression::Builder<'a>;
pub type ResponseBuilder<'a> = evaluation_result::Builder<'a>;
pub type Expectation = i32;
pub type RequestReader<'a> = expression::Reader<'a>;
pub type ResponseReader<'a> = evaluation_result::Reader<'a>;

fn make_expression(rng : &mut FastRand, exp : expression::Builder, depth : u32) -> i32 {
    exp.set_op(unsafe {
            std::mem::transmute(rng.next_less_than( operation::Modulus as u32 + 1) as u16)});

    let left : i32 =
    if rng.next_less_than(8) < depth {
        let tmp = (rng.next_less_than(128) + 1) as i32;
        exp.get_left().set_value(tmp);
        tmp
    } else {
        make_expression(rng, exp.get_left().init_expression(), depth + 1)
    };

    let right : i32 =
    if rng.next_less_than(8) < depth {
        let tmp = (rng.next_less_than(128) + 1) as i32;
        exp.get_right().set_value(tmp);
        tmp
    } else {
        make_expression(rng, exp.get_right().init_expression(), depth + 1)
    };

    match exp.get_op() {
        Some(operation::Add) => { return left + right }
        Some(operation::Subtract) => { return left - right }
        Some(operation::Multiply) => { return left * right }
        Some(operation::Divide) => { return div(left, right) }
        Some(operation::Modulus) => { return modulus(left, right) }
        None => { panic!("impossible") }
    }
}

fn evaluate_expression(exp : expression::Reader) -> i32 {
    let left = match exp.get_left().which() {
        Some(expression::left::Value(v)) => v,
        Some(expression::left::Expression(e)) => evaluate_expression(e),
        None => panic!("impossible")
    };
    let right = match exp.get_right().which() {
        Some(expression::right::Value(v)) => v,
        Some(expression::right::Expression(e)) => evaluate_expression(e),
        None => panic!("impossible")
    };

    match exp.get_op() {
        Some(operation::Add) => return left + right,
        Some(operation::Subtract) => return left - right,
        Some(operation::Multiply) => return left * right,
        Some(operation::Divide) => return div(left, right),
        Some(operation::Modulus) => return modulus(left, right),
        None => panic!("impossible")
    }
}

#[inline]
pub fn setup_request(rng : &mut FastRand, request : expression::Builder) -> i32 {
    make_expression(rng, request, 0)
}

#[inline]
pub fn handle_request(request : expression::Reader, response : evaluation_result::Builder) {
    response.set_value(evaluate_expression(request));
}

#[inline]
pub fn check_response(response : evaluation_result::Reader, expected : i32) -> bool {
    response.get_value() == expected
}
