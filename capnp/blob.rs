/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

pub mod Text {
    use std;

    pub type Reader<'a> = &'a str;

    // len does not include the required null terminator at the end
    pub fn new_reader<'a>(p : *u8, len : uint) -> Reader<'a> {
        unsafe {
            let v = std::unstable::raw::Slice { data: p, len: len };
            assert!(std::str::is_utf8(std::cast::transmute(v)));
            std::cast::transmute(v)
        }
    }

    pub struct Builder<'a> {
        priv ptr : *mut u8,
        priv length : uint,
        priv pos : uint
    }

    impl <'a> Builder <'a> {
/*        fn putc(&mut self, ) {
            unsafe {
                *self.ptr
        } */
    }

    impl <'a> std::io::Writer for Builder<'a> {
        fn write(&mut self, buf: &[u8]) {
            assert!(self.pos + buf.len() <= self.length);
            unsafe {
                std::ptr::copy_nonoverlapping_memory(self.ptr, buf.unsafe_ref(0), buf.len());
            }
            self.pos += buf.len();
        }
    }

}

pub mod Data {
    use std;

    pub type Reader<'a> = &'a [u8];

    pub fn new_reader<'a>(p : *u8, len : uint) -> Reader<'a> {
        unsafe {
            let v = std::unstable::raw::Slice { data: p, len: len };
            std::cast::transmute(v)
        }
    }

    pub struct Builder<'a> {
        priv ptr : *mut u8,
        priv length : uint,
        priv pos : uint
    }
}
