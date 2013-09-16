/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std::rand::*;
use common::*;
use catrank_capnp::*;

static URL_PREFIX : &'static str = "http://example.com";

pub fn setupRequest(rng : &mut FastRand, request : SearchResultList::Builder) -> int {
    let count = rng.nextLessThan(1000) as uint;
    let mut goodCount : int = 0;

    let list = request.initResults(count);

    for i in range(0, count) {
        let result = list.get(i);
        result.setScore(1000.0 - i as f64);
        let _urlSize = rng.nextLessThan(100) as uint;

//        let url = result.initUrl(100);


        let isCat = rng.nextLessThan(8) == 0;
        let isDog = rng.nextLessThan(8) == 0;
        if (isCat && !isDog) {
            goodCount += 1;
        }
    }

    goodCount
}
