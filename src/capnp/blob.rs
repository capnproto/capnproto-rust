/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

pub mod text {

    pub type Reader<'a> = &'a str;

    static EMPTY : &'static str = "";

    // len does not include the required null terminator at the end
    pub fn new_reader<'a>(p : *const u8, len : u32) -> Option<Reader<'a>> {
        // XXX The empty case is special and I don't know why.
        if len == 0 { return Some(EMPTY); }
        let v : &'a [u8] =
            unsafe { ::std::mem::transmute(::std::raw::Slice { data: p, len: len as uint}) };
        ::std::str::from_utf8(v)
    }

    impl <'a> ::traits::FromPointerReader<'a> for Reader<'a> {
        fn get_from_pointer(reader : &::layout::PointerReader<'a>) -> Reader<'a> {
            reader.get_text(::std::ptr::null(), 0)
        }
    }

    #[deriving(Copy)]
    pub struct Builder<'a> {
        ptr : *mut u8,
        len : uint,
    }

    impl <'a> Builder <'a> {

        pub fn new<'b>(p : *mut u8, len : u32) -> Builder<'b> {
            Builder { ptr : p, len : len as uint}
        }

        pub fn as_mut_bytes(&self) -> &'a mut [u8] {
             unsafe { ::std::mem::transmute(::std::raw::Slice { data:self.ptr as *const u8, len: self.len }) }
        }

        pub fn as_ptr(&self) -> *mut u8 {
            self.ptr
        }
    }

    impl <'a> ::std::str::Str for Builder<'a> {
        fn as_slice<'b>(&'b self) -> &'b str {
            let v : &'b [u8] =
                unsafe { ::std::mem::transmute(::std::raw::Slice { data: self.ptr as *const u8,
                                                                   len: self.len as uint}) };
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
            let v = ::std::raw::Slice { data: p, len: len as uint};
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
            let v = ::std::raw::Slice { data: p as *const u8, len: len as uint};
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
