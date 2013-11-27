/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use std::rand::*;
use common::*;
use carsales_capnp::*;
use capnp;

pub type RequestBuilder = ParkingLot::Builder;
pub type RequestReader<'self> = ParkingLot::Reader<'self>;
pub type ResponseBuilder = TotalValue::Builder;
pub type ResponseReader<'self> = TotalValue::Reader<'self>;
pub type Expectation = u64;

pub fn newRequestReader<'a>(sr : capnp::layout::StructReader<'a>) -> ParkingLot::Reader<'a> {
    ParkingLot::Reader::new(sr)
}

pub fn newResponseReader<'a>(sr : capnp::layout::StructReader<'a>) -> TotalValue::Reader<'a> {
    TotalValue::Reader::new(sr)
}

pub fn carValue (car : Car::Reader) -> u64 {
    let mut result : u64 = 0;
    result += car.get_seats() as u64 * 200;
    result += car.get_doors() as u64 * 350;

    // TODO Lists should have iterators.
    for i in range(0, car.get_wheels().size()) {
        let wheel = car.get_wheels()[i];
        result += wheel.get_diameter() as u64 * wheel.get_diameter() as u64;
        result += if (wheel.get_snow_tires()) { 100 } else { 0 };
    }

    result += car.get_length() as u64 * car.get_width() as u64 * car.get_height() as u64 / 50;

    let engine = car.get_engine();
    result += engine.get_horsepower() as u64 * 40;
    if (engine.get_uses_electric()) {
        if (engine.get_uses_gas()) {
            //# hybrid
            result += 5000;
        } else {
            result += 3000;
        }
    }

    result += if (car.get_has_power_windows()) { 100 } else { 0 };
    result += if (car.get_has_power_steering()) { 200 } else { 0 };
    result += if (car.get_has_cruise_control()) { 400 } else { 0 };
    result += if (car.get_has_nav_system()) { 2000 } else { 0 };

    result += car.get_cup_holders() as u64 * 25;

    return result;
}

static MAKES : [&'static str, .. 5] = ["Toyota", "GM", "Ford", "Honda", "Tesla"];
static MODELS : [&'static str, .. 6] = ["Camry", "Prius", "Volt", "Accord", "Leaf", "Model S"];

pub fn randomCar(rng : &mut FastRand, car : Car::Builder) {
    use std::cast::*;

    car.set_make(MAKES[rng.nextLessThan(MAKES.len() as u32)]);
    car.set_model(MODELS[rng.nextLessThan(MODELS.len() as u32)]);

    car.set_color(unsafe {transmute(rng.nextLessThan(Color::Silver as u32 + 1) as u16) });
    car.set_seats(2 + rng.nextLessThan(6) as u8);
    car.set_doors(2 + rng.nextLessThan(3) as u8);

    let wheels = car.init_wheels(4);
    for i in range(0, wheels.size()) {
        let wheel = wheels[i];
        wheel.set_diameter(25 + rng.nextLessThan(15) as u16);
        wheel.set_air_pressure((30.0 + rng.nextDouble(20.0)) as f32);
        wheel.set_snow_tires(rng.nextLessThan(16) == 0);
    }

    car.set_length(170 + rng.nextLessThan(150) as u16);
    car.set_width(48 + rng.nextLessThan(36) as u16);
    car.set_height(54 + rng.nextLessThan(48) as u16);
    car.set_weight(car.get_length() as u32 * car.get_width() as u32 *
                   car.get_height() as u32 / 200);

    let engine = car.init_engine();
    engine.set_horsepower(100 * rng.nextLessThan(400) as u16);
    engine.set_cylinders(4 + 2 * rng.nextLessThan(3) as u8);
    engine.set_cc(800 + rng.nextLessThan(10000));
    engine.set_uses_gas(true);
    engine.set_uses_electric(rng.gen());

    car.set_fuel_capacity((10.0 + rng.nextDouble(30.0)) as f32);
    car.set_fuel_level(rng.nextDouble(car.get_fuel_capacity() as f64) as f32);
    car.set_has_power_windows(rng.gen());
    car.set_has_power_steering(rng.gen());
    car.set_has_cruise_control(rng.gen());
    car.set_cup_holders(rng.nextLessThan(12) as u8);
    car.set_has_nav_system(rng.gen());
}

pub fn setupRequest(rng : &mut FastRand, request : ParkingLot::Builder) -> u64 {
    let mut result = 0;
    let cars = request.init_cars(rng.nextLessThan(200) as uint);
    for i in range(0, cars.size()) {
        let car = cars[i];
        randomCar(rng, car);
        result += car.asReader(|carReader| {carValue(carReader)});
    }
//    printfln!("number of cars: %?", cars.size());

    result
}

pub fn handleRequest(request : ParkingLot::Reader, response : TotalValue::Builder) {
    let mut result = 0;
    let cars = request.get_cars();
    for i in range(0, cars.size()) {
        result += carValue(cars[i]);
    }
    response.set_amount(result);
}

#[inline]
pub fn checkResponse(response : TotalValue::Reader, expected : u64) -> bool {
    response.get_amount() == expected
}
