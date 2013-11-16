/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

pub struct WireValue<T> {
    priv value : T
}

impl<T : Clone> WireValue<T> {

    #[inline]
    pub fn get(&self) -> T { self.value.clone() }

    #[inline]
    pub fn set(&mut self, value : T) { self.value = value }
}

// TODO handle big endian systems.
