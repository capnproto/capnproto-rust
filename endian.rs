use std;
use common::*;

pub struct WireValue<T> {
    value : T
}

impl<T : Copy> WireValue<T> {

    #[inline(always)]
    pub fn get(&self) -> T { copy self.value }

    #[inline(always)]
    pub fn set(&mut self, value : T) { self.value = value }

    #[inline(always)]
    pub fn getFromBuf<'a>(buf : &'a [u8], index : ByteCount) -> &'a WireValue<T> {
        unsafe {
            let p : *WireValue<T> =
                std::cast::transmute(buf.unsafe_ref(index));
            &*p
        }
    }

    #[inline(always)]
    pub fn getFromBufMut<'a>(buf : &'a mut [u8], index : ByteCount) -> &'a mut WireValue<T> {
        unsafe {
            let p : * mut WireValue<T> =
                std::cast::transmute(buf.unsafe_ref(index));
            &mut *p
        }
    }

}


// TODO handle big endian systems.