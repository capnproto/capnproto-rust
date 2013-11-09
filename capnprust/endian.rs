/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */


use std;
use common::*;

pub struct WireValue<T> {
    value : T
}

impl<T : Clone> WireValue<T> {

    #[inline]
    pub fn get(&self) -> T { self.value.clone() }

    #[inline]
    pub fn set(&mut self, value : T) { self.value = value }

    #[inline]
    pub fn getFromBufMut<'a>(buf : &'a mut [u8], index : ByteCount) -> &'a mut WireValue<T> {
        unsafe {
            let p : * mut WireValue<T> =
                std::cast::transmute(buf.unsafe_ref(index));
            &mut *p
        }
    }

}


// TODO handle big endian systems.
