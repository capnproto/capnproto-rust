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

#[cfg(target_endian = "big")]
use std::intrinsics::{bswap16, bswap32, bswap64};
#[cfg(target_endian = "big")]
use std::cast::transmute;

#[repr(C)]
pub struct WireValue<T> {
    value : T
}

impl<T:Endian> WireValue<T> {
    #[inline]
    pub fn get(&self) -> T { self.value.get() }

    #[inline]
    pub fn set(&mut self, value : T) { self.value.set(value) }
}

pub trait Endian : Sized {
    fn get(&self) -> Self;
    fn set(&mut self, value : Self);
}

macro_rules! endian_impl(
    ($typ:ty) => (
        impl Endian for $typ {
            #[inline]
            fn get(&self) -> $typ { *self }
            #[inline]
            fn set(&mut self, value : $typ) {*self = value;}
        }
        );
    ($typ:ty, $typ2:ty, $swapper:ident) => (
        impl Endian for $typ {
            #[inline]
            fn get(&self) -> $typ { unsafe { transmute($swapper(transmute::<$typ, $typ2>(*self)))} }
            #[inline]
            fn set(&mut self, value : $typ) {
                *self = unsafe { transmute($swapper(transmute::<$typ,$typ2>(value))) };
            }
        }
        );
    );

endian_impl!(());
endian_impl!(bool);
endian_impl!(u8);
endian_impl!(i8);

#[cfg(target_endian = "little")]
endian_impl!(u16);
#[cfg(target_endian = "little")]
endian_impl!(i16);

#[cfg(target_endian = "little")]
endian_impl!(u32);
#[cfg(target_endian = "little")]
endian_impl!(i32);

#[cfg(target_endian = "little")]
endian_impl!(u64);
#[cfg(target_endian = "little")]
endian_impl!(i64);
#[cfg(target_endian = "little")]
endian_impl!(f32);
#[cfg(target_endian = "little")]
endian_impl!(f64);

#[cfg(target_endian = "big")]
endian_impl!(u16, i16, bswap16);
#[cfg(target_endian = "big")]
endian_impl!(i16, i16, bswap16);

#[cfg(target_endian = "big")]
endian_impl!(u32, i32, bswap32);
#[cfg(target_endian = "big")]
endian_impl!(i32, i32, bswap32);

#[cfg(target_endian = "big")]
endian_impl!(u64, i64, bswap64);
#[cfg(target_endian = "big")]
endian_impl!(i64, i64, bswap64);
#[cfg(target_endian = "big")]
endian_impl!(f32, i32, bswap32);
#[cfg(target_endian = "big")]
endian_impl!(f64, i64, bswap64);

