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

pub trait Mask {
    type T;
    fn mask(value : Self, mask : Self::T) -> Self;
}

macro_rules! int_mask(
    ($s:ident, $t:ident) => (
        impl Mask for $s {
            type T = $t;
            #[inline]
            fn mask(value : $s, mask : $t) -> $s {
                value ^ (mask as $s)
            }
        }
    )
);

int_mask!(i8, i8);
int_mask!(i16, i16);
int_mask!(i32, i32);
int_mask!(i64, i64);
int_mask!(u8, u8);
int_mask!(u16, u16);
int_mask!(u32, u32);
int_mask!(u64, u64);

impl Mask for f32 {
    type T = u32;
    #[inline]
    fn mask(value : f32, mask : u32) -> f32 {
        unsafe {
            let v : u32 = ::std::mem::transmute(value);
            ::std::mem::transmute(v ^ mask)
        }
    }
}

impl Mask for f64 {
    type T = u64;
    #[inline]
    fn mask(value : f64, mask : u64) -> f64 {
        unsafe {
            let v : u64 = ::std::mem::transmute(value);
            ::std::mem::transmute(v ^ mask)
        }
    }
}
