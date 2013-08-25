/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std::rand::*;
use common::*;
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

pub fn randomCar(rng : &mut FastRand, car : Car::Builder) {
    use std::cast::*;

    car.setMake(MAKES[rng.nextLessThan(MAKES.len() as u32)]);
    car.setModel(MODELS[rng.nextLessThan(MODELS.len() as u32)]);

    car.setColor(unsafe {transmute(rng.gen_uint_range(0, Color::silver as uint + 1)) });
    car.setSeats(2 + rng.nextLessThan(6) as u8);
    car.setDoors(2 + rng.nextLessThan(3) as u8);

    let wheels = car.initWheels(4);
    for i in range(0, wheels.size()) {
        let wheel = wheels.get(i);
        wheel.setDiameter(25 + rng.nextLessThan(15) as u16);
        wheel.setAirPressure(30.0 + rng.nextDouble(20.0) as f32);
        wheel.setSnowTires(rng.nextLessThan(16) == 0);
    }

    car.setLength(170 + rng.nextLessThan(150) as u16);
    car.setWidth(48 + rng.nextLessThan(36) as u16);
    car.setHeight(54 + rng.nextLessThan(48) as u16);
    car.setWeight(car.getLength() as u32 * car.getWidth() as u32 * car.getHeight() as u32 / 200);

    let engine = car.initEngine();
    engine.setHorsepower(100 * rng.nextLessThan(400) as u16);
    engine.setCylinders(4 + 2 * rng.nextLessThan(3) as u8);
    engine.setCc(800 + rng.nextLessThan(10000));
    engine.setUsesGas(true);
    engine.setUsesElectric(rng.gen());

    car.setFuelCapacity(10.0 + rng.nextDouble(30.0) as f32);
    car.setFuelLevel(rng.nextDouble(car.getFuelCapacity() as f64) as f32);
    car.setHasPowerWindows(rng.gen());
    car.setHasPowerSteering(rng.gen());
    car.setHasCruiseControl(rng.gen());
    car.setCupHolders(rng.nextLessThan(12) as u8);
    car.setHasNavSystem(rng.gen());
}

pub fn setupRequest(rng : &mut FastRand, request : ParkingLot::Builder) -> u64 {
    let mut result = 0;
    let cars = request.initCars(rng.nextLessThan(200) as uint);
    for i in range(0, cars.size()) {
        let car = cars.get(i);
        randomCar(rng, car);
        result += do car.asReader |carReader| {carValue(carReader)};
    }
//    printfln!("number of cars: %?", cars.size());

    result
}

pub fn handleRequest(request : ParkingLot::Reader, response : TotalValue::Builder) {
    let mut result = 0;
    let cars = request.getCars();
    for i in range(0, cars.size()) {
        result += carValue(cars.get(i));
    }
    response.setAmount(result);
}

#[inline]
pub fn checkResponse(response : TotalValue::Reader, expected : u64) -> bool {
    response.getAmount() == expected
}
