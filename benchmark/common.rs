/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std::rand::*;
use std::i32;

pub struct FastRand {
    state : u32
}

impl Rng for FastRand {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        let a = 1664525;
        let c = 1013904223;
        self.state = a * self.state + c;
        self.state
    }
}

impl FastRand {
    pub fn new() -> FastRand {
        FastRand {state : 1013904223}
    }

    #[inline]
    pub fn nextLessThan(&mut self, range : u32) -> u32 {
        self.next_u32() % range
    }

    #[inline]
    pub fn nextDouble(&mut self, range : f64) -> f64 {
        use std::u32;
        self.next_u32() as f64 * range / (u32::MAX as f64)
    }
}

#[inline]
pub fn div(a : i32, b: i32) -> i32 {
    if b == 0 { return i32::MAX }
    if a == i32::MIN && b == -1 {
        return i32::MAX;
    }
    return a / b;
}

#[inline]
pub fn modulus(a : i32, b: i32) -> i32 {
    if b == 0 { return i32::MAX }
    if a == i32::MIN && b == -1 {
        return i32::MAX;
    }
    return a % b;
}

pub static WORDS : [&'static str, .. 13] = [
    "foo ", "bar ", "baz ", "qux ", "quux ", "corge ", "grault ", "garply ", "waldo ", "fred ",
    "plugh ", "xyzzy ", "thud "];


