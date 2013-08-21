/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std::rand::*;
//use common::*;
use carsales_capnp::*;

pub fn carValue (car : Car::Reader) -> u64 {
    let mut result : u64 = 0;
    result += car.getSeats() as u64 * 200;
    result += car.getDoors() as u64 * 350;

    // TODO Lists should have iterators.
    for i in range(0, car.getWheels().size()) {
        let wheel = car.getWheels().get(i);
        result += wheel.getDiameter() as u64 * wheel.getDiameter() as u64;
        result += if (wheel.getSnowTires()) { 100 } else { 0 };
    }

    result += car.getLength() as u64 * car.getWidth() as u64 * car.getHeight() as u64 / 50;

    let engine = car.getEngine();
    result += engine.getHorsepower() as u64 * 40;
    if (engine.getUsesElectric()) {
        if (engine.getUsesGas()) {
            //# hybrid
            result += 5000;
        } else {
            result += 3000;
        }
    }

    result += if (car.getHasPowerWindows()) { 100 } else { 0 };
    result += if (car.getHasPowerSteering()) { 200 } else { 0 };
    result += if (car.getHasCruiseControl()) { 400 } else { 0 };
    result += if (car.getHasNavSystem()) { 2000 } else { 0 };

    result += car.getCupHolders() as u64 * 25;

    return result;
}

static MAKES : [&'static str, .. 5] = ["Toyota", "GM", "Ford", "Honda", "Tesla"];
static MODELS : [&'static str, .. 6] = ["Camry", "Prius", "Volt", "Accord", "Leaf", "Model S"];

pub fn randomCar<R : Rng>(rng : &mut R, car : Car::Builder) {
    car.setMake(MAKES[rng.gen_uint_range(0, MAKES.len())]);
    car.setModel(MODELS[rng.gen_uint_range(0, MODELS.len())]);
}

