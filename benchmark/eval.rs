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
