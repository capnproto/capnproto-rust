/*
 * Copyright (c) 2013-2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[cfg(target_endian = "big")]
use std::intrinsics::{bswap16, bswap32, bswap64};
#[cfg(target_endian = "big")]
use std::cast::transmute;

pub struct WireValue<T> {
    value : T
}

impl<T:Endian> WireValue<T> {
    #[inline]
    pub fn get(&self) -> T { self.value.get() }

    #[inline]
    pub fn set(&mut self, value : T) { self.value.set(value) }
}

pub trait Endian {
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

