/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use rand::*;
use common::*;
use carsales_capnp::*;

pub type RequestBuilder<'a> = parking_lot::Builder<'a>;
pub type RequestReader<'a> = parking_lot::Reader<'a>;
pub type ResponseBuilder<'a> = total_value::Builder<'a>;
pub type ResponseReader<'a> = total_value::Reader<'a>;
pub type Expectation = u64;

trait CarValue {
    fn car_value(&self) -> u64;
}

macro_rules! car_value_impl(
    ($typ:ident) => (
            impl <'a> CarValue for car::$typ<'a> {
                fn car_value (&self) -> u64 {
                    let mut result : u64 = 0;
                    result += self.get_seats() as u64 * 200;
                    result += self.get_doors() as u64 * 350;

                    // Using an iterator here slows things down considerably.
                    // TODO: investigate why.
                    let wheels = self.get_wheels();
                    for ii in range(0, wheels.size()) {
                        let wheel = wheels.get(ii);
                        result += wheel.get_diameter() as u64 * wheel.get_diameter() as u64;
                        result += if wheel.get_snow_tires() { 100 } else { 0 };
                    }

                    result += self.get_length() as u64 * self.get_width() as u64 * self.get_height() as u64 / 50;

                    let engine = self.get_engine();
                    result += engine.get_horsepower() as u64 * 40;
                    if engine.get_uses_electric() {
                        if engine.get_uses_gas() {
                            //# hybrid
                            result += 5000;
                        } else {
                            result += 3000;
                        }
                    }

                    result += if self.get_has_power_windows() { 100 } else { 0 };
                    result += if self.get_has_power_steering() { 200 } else { 0 };
                    result += if self.get_has_cruise_control() { 400 } else { 0 };
                    result += if self.get_has_nav_system() { 2000 } else { 0 };

                    result += self.get_cup_holders() as u64 * 25;

                    return result;
                }

            }
        )
   )

car_value_impl!(Reader)
car_value_impl!(Builder)

const MAKES : [&'static str, .. 5] = ["Toyota", "GM", "Ford", "Honda", "Tesla"];
const MODELS : [&'static str, .. 6] = ["Camry", "Prius", "Volt", "Accord", "Leaf", "Model S"];

pub fn random_car(rng : &mut FastRand, mut car : car::Builder) {
    use std::mem::transmute;

    car.set_make(MAKES[rng.next_less_than(MAKES.len() as u32) as uint]);
    car.set_model(MODELS[rng.next_less_than(MODELS.len() as u32) as uint]);

    car.set_color(unsafe {transmute(rng.next_less_than(color::Silver as u32 + 1) as u16) });
    car.set_seats(2 + rng.next_less_than(6) as u8);
    car.set_doors(2 + rng.next_less_than(3) as u8);

    for mut wheel in car.init_wheels(4).iter() {
        wheel.set_diameter(25 + rng.next_less_than(15) as u16);
        wheel.set_air_pressure((30.0 + rng.next_double(20.0)) as f32);
        wheel.set_snow_tires(rng.next_less_than(16) == 0);
    }

    car.set_length(170 + rng.next_less_than(150) as u16);
    car.set_width(48 + rng.next_less_than(36) as u16);
    car.set_height(54 + rng.next_less_than(48) as u16);
    let weight = car.get_length() as u32 * car.get_width() as u32 *
                 car.get_height() as u32 / 200;
    car.set_weight(weight);

    let mut engine = car.init_engine();
    engine.set_horsepower(100 * rng.next_less_than(400) as u16);
    engine.set_cylinders(4 + 2 * rng.next_less_than(3) as u8);
    engine.set_cc(800 + rng.next_less_than(10000));
    engine.set_uses_gas(true);
    engine.set_uses_electric(rng.gen());

    car.set_fuel_capacity((10.0 + rng.next_double(30.0)) as f32);
    let fuel_level = rng.next_double(car.get_fuel_capacity() as f64) as f32;
    car.set_fuel_level(fuel_level);
    car.set_has_power_windows(rng.gen());
    car.set_has_power_steering(rng.gen());
    car.set_has_cruise_control(rng.gen());
    car.set_cup_holders(rng.next_less_than(12) as u8);
    car.set_has_nav_system(rng.gen());
}

pub fn setup_request(rng : &mut FastRand, mut request : parking_lot::Builder) -> u64 {
    let mut result = 0;
    for car in request.init_cars(rng.next_less_than(200)).iter() {
        random_car(rng, car);
        result += car.car_value();
    }

    result
}

pub fn handle_request(request : parking_lot::Reader, mut response : total_value::Builder) {
    let mut result = 0;
    for car in request.get_cars().iter() {
        result += car.car_value();
    }
    response.set_amount(result);
}

#[inline]
pub fn check_response(response : total_value::Reader, expected : u64) -> bool {
    response.get_amount() == expected
}
