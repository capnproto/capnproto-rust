/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


use carsales_capnp::*;

pub fn carValue (car : Car::Reader) -> u64 {
    let mut result : u64 = 0;
    result += car.getSeats() as u64 * 200;
    result += car.getDoors() as u64 * 350;

    // TODO Lists should have iterators.

    return result;
}
