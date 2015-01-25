// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

pub mod text {

    pub type Reader<'a> = &'a str;

    static EMPTY : &'static str = "";

    // len does not include the required null terminator at the end
    pub fn new_reader<'a>(p : *const u8, len : u32) -> Result<Reader<'a>, ::std::str::Utf8Error> {
        // XXX The empty case is special and I don't know why.
        if len == 0 { return Ok(EMPTY); }
        let v : &'a [u8] =
            unsafe { ::std::mem::transmute(::std::raw::Slice { data: p, len: len as usize}) };
        ::std::str::from_utf8(v)
    }

    impl <'a> ::traits::FromPointerReader<'a> for Reader<'a> {
        fn get_from_pointer(reader : &::layout::PointerReader<'a>) -> Reader<'a> {
            reader.get_text(::std::ptr::null(), 0)
        }
    }

    pub struct Builder<'a> {
        ptr : *mut u8,
        len : usize,
    }

    impl <'a> Builder <'a> {

        pub fn new<'b>(p : *mut u8, len : u32) -> Builder<'b> {
            Builder { ptr : p, len : len as usize}
        }

        pub fn as_mut_bytes(self) -> &'a mut [u8] {
             unsafe { ::std::mem::transmute(::std::raw::Slice { data:self.ptr as *const u8, len: self.len }) }
        }

        pub unsafe fn as_ptr(&self) -> *mut u8 {
            self.ptr
        }

        pub fn borrow<'b>(&'b mut self) -> Builder<'b> {
            Builder { ptr : self.ptr, len : self.len }
        }
    }

    impl <'a> ::std::str::Str for Builder<'a> {
        fn as_slice<'b>(&'b self) -> &'b str {
            let v : &'b [u8] =
                unsafe { ::std::mem::transmute(::std::raw::Slice { data: self.ptr as *const u8,
                                                                   len: self.len as usize}) };
            ::std::str::from_utf8(v).unwrap()
        }
    }

    impl <'a> ::traits::FromPointerBuilder<'a> for Builder<'a> {
        fn init_pointer(builder : ::layout::PointerBuilder<'a>, size : u32) -> Builder<'a> {
            builder.init_text(size)
        }
        fn get_from_pointer(builder : ::layout::PointerBuilder<'a>) -> Builder<'a> {
            builder.get_text(::std::ptr::null(), 0)
        }
    }

    impl <'a> ::traits::SetPointerBuilder<Builder<'a>> for Reader<'a> {
        fn set_pointer_builder<'b>(pointer : ::layout::PointerBuilder<'b>, value : Reader<'a>) {
            pointer.set_text(value);
        }
    }
}

pub mod data {

    pub type Reader<'a> = &'a [u8];

    pub fn new_reader<'a>(p : *const u8, len : u32) -> Reader<'a> {
        unsafe {
            let v = ::std::raw::Slice { data: p, len: len as usize};
            ::std::mem::transmute(v)
        }
    }

    impl <'a> ::traits::FromPointerReader<'a> for Reader<'a> {
        fn get_from_pointer(reader : &::layout::PointerReader<'a>) -> Reader<'a> {
            reader.get_data(::std::ptr::null(), 0)
        }
    }

    pub type Builder<'a> = &'a mut [u8];

    pub fn new_builder<'a>(p : *mut u8, len : u32) -> Builder<'a> {
        unsafe {
            let v = ::std::raw::Slice { data: p as *const u8, len: len as usize};
            ::std::mem::transmute(v)
        }
    }

    impl <'a> ::traits::FromPointerBuilder<'a> for Builder<'a> {
        fn init_pointer(builder : ::layout::PointerBuilder<'a>, size : u32) -> Builder<'a> {
            builder.init_data(size)
        }
        fn get_from_pointer(builder : ::layout::PointerBuilder<'a>) -> Builder<'a> {
            builder.get_data(::std::ptr::null(), 0)
        }
    }

    impl <'a> ::traits::SetPointerBuilder<Builder<'a>> for Reader<'a> {
        fn set_pointer_builder<'b>(pointer : ::layout::PointerBuilder<'b>, value : Reader<'a>) {
            pointer.set_data(value);
        }
    }
}
