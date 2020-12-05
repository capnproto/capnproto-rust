pub trait Primitive {
    type Raw;
    type RawAligned;

    /// Reads the value, swapping bytes on big-endian processors.
    fn get(raw: &Self::Raw) -> Self;

    /// Reads the value, swapping bytes on big-endian processors.
    fn get_aligned(raw: &Self::RawAligned) -> Self;

    /// Writes the value, swapping bytes on big-endian processors.
    fn set(raw: &mut Self::Raw, value: Self);

    /// Writes the value, swapping bytes on big-endian processors.
    fn set(raw: &mut Self::RawAligned, value: Self);
}

macro_rules! primitive_impl(
    ($typ:ty, $n:expr) => (
        impl Primitive for $typ {
            type Raw = [u8; $n];
            type RawAligned = $typ;

            #[inline]
            fn get(raw: &Self::Raw) -> Self {
                <$typ>::from_le_bytes(*raw)
            }

            #[inline]
            fn get_aligned(raw: &Self::RawAligned) -> Self {
                raw.to_le()
            }

            #[inline]
            fn set(raw: &mut Self::Raw, value: Self) {
                *raw = value.to_le_bytes();
            }

            #[inline]
            fn set_aligned(raw: &mut Self::RawAligned, value: Self) {
                *raw = value.to_le()
            }
        }
        );
    );

primitive_impl!(u8, 1);
primitive_impl!(i8, 1);
primitive_impl!(u16, 2);
primitive_impl!(i16, 2);
primitive_impl!(u32, 4);
primitive_impl!(i32, 4);
primitive_impl!(u64, 8);
primitive_impl!(i64, 8);

#[cfg(feature = "unaligned")]
primitive_impl!(f32, 4);

#[cfg(feature = "unaligned")]
primitive_impl!(f64, 8);

impl Primitive for f32 {
    type Raw = [u8; 4];
    type RawAligned = f32;

    fn get(raw: &Self::Raw) -> Self {
        f32::from_le_bytes(*raw)
    }

    fn get_aligned(raw: &Self::RawAligned) -> Self {
        f32::from_bits(raw.to_bits().to_le())
    }

    fn set(raw: &mut Self::Raw, value: Self) {
        *raw = value.to_le_bytes();
    }

    fn set_aligned(raw: &mut Self::RawAligned, value: Self) {
        *raw = f32::from_bits(value.to_bits().to_le())
    }
}

impl Primitive for f64 {
    type Raw = [u8; 8];
    type RawAligned = f64;

    fn get(raw: &Self::Raw) -> Self {
        f64::from_le_bytes(*raw)
    }

    fn get_aligned(raw: &Self::RawAligned) -> Self {
        f64::from_bits(raw.to_bits().to_le())
    }

    fn set(raw: &mut Self::Raw, value: Self) {
        *raw = value.to_le_bytes();
    }

    fn set_aligned(raw: &mut Self::RawAligned, value: Self) {
        *raw = f64::from_bits(value.to_bits().to_le())
    }
}

/// A value casted directly from a little-endian byte buffer. On big-endian
/// processors, the bytes of the value need to be swapped upon reading and writing.
#[repr(C)]
pub struct WireValue<T> where T: Primitive {
    value: <T as Primitive>::Raw,
}

impl<T> WireValue<T> where T: Primitive {
    /// Reads the value, swapping bytes on big-endian processors.
    #[inline]
    pub fn get(&self) -> T { <T as Primitive>::get(&self.value) }

    /// Writes the value, swapping bytes on big-endian processors.
    #[inline]
    pub fn set(&mut self, value: T) { <T as Primitive>::set(&mut self.value, value) }
}
