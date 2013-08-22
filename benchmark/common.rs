/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std::rand::*;

pub struct FastRand {
    state : u32
}

impl Rng for FastRand {
    #[inline]
    fn next(&mut self) -> u32 {
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

    pub fn nextLessThan(&mut self, range : u32) -> u32 {
        self.next() % range
    }

    pub fn nextDouble(&mut self, range : f64) -> f64 {
        use std::u32;
        self.next() as f64 * range / (u32::max_value as f64)
    }
}


