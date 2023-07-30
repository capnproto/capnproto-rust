//! Dynamically typed values.

use crate::introspect::{self, TypeVariant};
use crate::schema_capnp::value;
use crate::Result;
use crate::{dynamic_list, dynamic_struct};

/// A dynamically-typed read-only value.
#[derive(Clone, Copy)]
pub enum Reader<'a> {
    Void,
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    Enum(Enum),
    Text(crate::text::Reader<'a>),
    Data(crate::data::Reader<'a>),
    Struct(dynamic_struct::Reader<'a>),
    List(dynamic_list::Reader<'a>),
    AnyPointer(crate::any_pointer::Reader<'a>),
    Capability(Capability),
}

impl<'a> Reader<'a> {
    pub fn new(value: value::Reader<'a>, ty: introspect::Type) -> Result<Self> {
        match (value.which()?, ty.which()) {
            (value::Void(()), _) => Ok(Reader::Void),
            (value::Bool(b), _) => Ok(Reader::Bool(b)),
            (value::Int8(x), _) => Ok(Reader::Int8(x)),
            (value::Int16(x), _) => Ok(Reader::Int16(x)),
            (value::Int32(x), _) => Ok(Reader::Int32(x)),
            (value::Int64(x), _) => Ok(Reader::Int64(x)),
            (value::Uint8(x), _) => Ok(Reader::UInt8(x)),
            (value::Uint16(x), _) => Ok(Reader::UInt16(x)),
            (value::Uint32(x), _) => Ok(Reader::UInt32(x)),
            (value::Uint64(x), _) => Ok(Reader::UInt64(x)),
            (value::Float32(x), _) => Ok(Reader::Float32(x)),
            (value::Float64(x), _) => Ok(Reader::Float64(x)),
            (value::Enum(d), TypeVariant::Enum(e)) => Ok(Reader::Enum(Enum::new(d, e.into()))),
            (value::Text(t), _) => Ok(Reader::Text(t?)),
            (value::Data(d), _) => Ok(Reader::Data(d?)),
            (value::Struct(d), TypeVariant::Struct(schema)) => Ok(Reader::Struct(
                dynamic_struct::Reader::new(d.reader.get_struct(None)?, schema.into()),
            )),
            (value::List(l), TypeVariant::List(element_type)) => {
                Ok(Reader::List(dynamic_list::Reader::new(
                    l.reader
                        .get_list(element_type.expected_element_size(), None)?,
                    element_type,
                )))
            }
            (value::Interface(()), TypeVariant::Capability) => Ok(Capability.into()),
            (value::AnyPointer(a), TypeVariant::AnyPointer) => Ok(a.into()),
            _ => Err(crate::Error::from_kind(crate::ErrorKind::TypeMismatch)),
        }
    }

    /// Downcasts the `Reader` into a more specific type. Panics if the
    /// expected type does not match the value.
    pub fn downcast<T: DowncastReader<'a>>(self) -> T {
        T::downcast_reader(self)
    }
}

impl<'a> From<()> for Reader<'a> {
    fn from((): ()) -> Reader<'a> {
        Reader::Void
    }
}

macro_rules! primitive_dynamic_value(
    ($t:ty, $v:ident) => (
        impl <'a> From<$t> for Reader<'a> {
            fn from(x: $t) -> Reader<'a> { Reader::$v(x) }
        }
    )
);

primitive_dynamic_value!(bool, Bool);
primitive_dynamic_value!(i8, Int8);
primitive_dynamic_value!(i16, Int16);
primitive_dynamic_value!(i32, Int32);
primitive_dynamic_value!(i64, Int64);
primitive_dynamic_value!(u8, UInt8);
primitive_dynamic_value!(u16, UInt16);
primitive_dynamic_value!(u32, UInt32);
primitive_dynamic_value!(u64, UInt64);
primitive_dynamic_value!(f32, Float32);
primitive_dynamic_value!(f64, Float64);

/// Helper trait for the `dynamic_value::Reader::downcast()` method.
pub trait DowncastReader<'a> {
    fn downcast_reader(v: Reader<'a>) -> Self;
}

impl<'a> DowncastReader<'a> for () {
    fn downcast_reader(value: Reader<'a>) {
        let Reader::Void = value else {
            panic!("error downcasting to void")
        };
    }
}

macro_rules! downcast_reader_impl(
    ($t:ty, $v:ident, $s:expr) => (
        impl <'a> DowncastReader<'a> for $t {
            fn downcast_reader(value: Reader<'a>) -> Self {
                let Reader::$v(x) = value else { panic!("error downcasting to {}", $s) };
                x
            }
        }
    )
);

downcast_reader_impl!(bool, Bool, "bool");
downcast_reader_impl!(i8, Int8, "i8");
downcast_reader_impl!(i16, Int16, "i16");
downcast_reader_impl!(i32, Int32, "i32");
downcast_reader_impl!(i64, Int64, "i64");
downcast_reader_impl!(u8, UInt8, "u8");
downcast_reader_impl!(u16, UInt16, "u16");
downcast_reader_impl!(u32, UInt32, "u32");
downcast_reader_impl!(u64, UInt64, "u64");
downcast_reader_impl!(f32, Float32, "f32");
downcast_reader_impl!(f64, Float64, "f64");
downcast_reader_impl!(Enum, Enum, "enum");
downcast_reader_impl!(crate::text::Reader<'a>, Text, "text");
downcast_reader_impl!(crate::data::Reader<'a>, Data, "data");
downcast_reader_impl!(dynamic_list::Reader<'a>, List, "list");
downcast_reader_impl!(dynamic_struct::Reader<'a>, Struct, "struct");
downcast_reader_impl!(crate::any_pointer::Reader<'a>, AnyPointer, "anypointer");

/// A dynamically-typed value with mutable interior.
pub enum Builder<'a> {
    Void,
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    Enum(Enum),
    Text(crate::text::Builder<'a>),
    Data(crate::data::Builder<'a>),
    Struct(dynamic_struct::Builder<'a>),
    List(dynamic_list::Builder<'a>),
    AnyPointer(crate::any_pointer::Builder<'a>),
    Capability(Capability),
}

impl<'a> Builder<'a> {
    pub fn reborrow(&mut self) -> Builder<'_> {
        match self {
            Builder::Void => Builder::Void,
            Builder::Bool(b) => Builder::Bool(*b),
            Builder::Int8(x) => Builder::Int8(*x),
            Builder::Int16(x) => Builder::Int16(*x),
            Builder::Int32(x) => Builder::Int32(*x),
            Builder::Int64(x) => Builder::Int64(*x),
            Builder::UInt8(x) => Builder::UInt8(*x),
            Builder::UInt16(x) => Builder::UInt16(*x),
            Builder::UInt32(x) => Builder::UInt32(*x),
            Builder::UInt64(x) => Builder::UInt64(*x),
            Builder::Float32(x) => Builder::Float32(*x),
            Builder::Float64(x) => Builder::Float64(*x),
            Builder::Enum(e) => Builder::Enum(*e),
            Builder::Text(t) => Builder::Text(t.reborrow()),
            Builder::Data(d) => Builder::Data(d),
            Builder::Struct(ref mut s) => Builder::Struct(s.reborrow()),
            Builder::List(ref mut l) => Builder::List(l.reborrow()),
            Builder::AnyPointer(ref mut a) => Builder::AnyPointer(a.reborrow()),
            Builder::Capability(c) => Builder::Capability(*c),
        }
    }

    pub fn into_reader(self) -> Reader<'a> {
        match self {
            Builder::Void => Reader::Void,
            Builder::Bool(b) => Reader::Bool(b),
            Builder::Int8(x) => Reader::Int8(x),
            Builder::Int16(x) => Reader::Int16(x),
            Builder::Int32(x) => Reader::Int32(x),
            Builder::Int64(x) => Reader::Int64(x),
            Builder::UInt8(x) => Reader::UInt8(x),
            Builder::UInt16(x) => Reader::UInt16(x),
            Builder::UInt32(x) => Reader::UInt32(x),
            Builder::UInt64(x) => Reader::UInt64(x),
            Builder::Float32(x) => Reader::Float32(x),
            Builder::Float64(x) => Reader::Float64(x),
            Builder::Enum(e) => Reader::Enum(e),
            Builder::Text(t) => Reader::Text(t.into_reader()),
            Builder::Data(d) => Reader::Data(d),
            Builder::Struct(s) => Reader::Struct(s.into_reader()),
            Builder::List(l) => Reader::List(l.into_reader()),
            Builder::AnyPointer(a) => Reader::AnyPointer(a.into_reader()),
            Builder::Capability(c) => Reader::Capability(c),
        }
    }

    /// Downcasts the `Reader` into a more specific type. Panics if the
    /// expected type does not match the value.
    pub fn downcast<T: DowncastBuilder<'a>>(self) -> T {
        T::downcast_builder(self)
    }
}

/// Helper trait for the `dynamic_value::Builder::downcast()` method.
pub trait DowncastBuilder<'a> {
    fn downcast_builder(v: Builder<'a>) -> Self;
}

impl<'a> DowncastBuilder<'a> for () {
    fn downcast_builder(value: Builder<'a>) {
        let Builder::Void = value else {
            panic!("error downcasting to void")
        };
    }
}

macro_rules! downcast_builder_impl(
    ($t:ty, $v:ident, $s:expr) => (
        impl <'a> DowncastBuilder<'a> for $t {
            fn downcast_builder(value: Builder<'a>) -> Self {
                let Builder::$v(x) = value else { panic!("error downcasting to {}", $s) };
                x
            }
        }
    )
);

downcast_builder_impl!(bool, Bool, "bool");
downcast_builder_impl!(i8, Int8, "i8");
downcast_builder_impl!(i16, Int16, "i16");
downcast_builder_impl!(i32, Int32, "i32");
downcast_builder_impl!(i64, Int64, "i64");
downcast_builder_impl!(u8, UInt8, "u8");
downcast_builder_impl!(u16, UInt16, "u16");
downcast_builder_impl!(u32, UInt32, "u32");
downcast_builder_impl!(u64, UInt64, "u64");
downcast_builder_impl!(f32, Float32, "f32");
downcast_builder_impl!(f64, Float64, "f64");
downcast_builder_impl!(Enum, Enum, "enum");
downcast_builder_impl!(crate::text::Builder<'a>, Text, "text");
downcast_builder_impl!(crate::data::Builder<'a>, Data, "data");
downcast_builder_impl!(dynamic_list::Builder<'a>, List, "list");
downcast_builder_impl!(dynamic_struct::Builder<'a>, Struct, "struct");
downcast_builder_impl!(crate::any_pointer::Builder<'a>, AnyPointer, "anypointer");

/// A dynamically-typed enum value.
#[derive(Clone, Copy)]
pub struct Enum {
    value: u16,
    schema: crate::schema::EnumSchema,
}

impl Enum {
    pub fn new(value: u16, schema: crate::schema::EnumSchema) -> Self {
        Self { value, schema }
    }

    /// Gets the u16 representation of this value.
    pub fn get_value(&self) -> u16 {
        self.value
    }

    /// Gets the schema of this enumerant.
    pub fn get_enumerant(self) -> crate::Result<Option<crate::schema::Enumerant>> {
        let enumerants = self.schema.get_enumerants()?;
        if (self.value) < enumerants.len() {
            Ok(Some(enumerants.get(self.value)))
        } else {
            Ok(None)
        }
    }
}

impl<'a> From<Enum> for Reader<'a> {
    fn from(e: Enum) -> Reader<'a> {
        Reader::Enum(e)
    }
}

impl<'a> From<Enum> for Builder<'a> {
    fn from(e: Enum) -> Builder<'a> {
        Builder::Enum(e)
    }
}

/// A dynamic capability. Currently, this is just a stub and does not support calling
/// of methods.
#[derive(Clone, Copy)]
pub struct Capability;

impl<'a> From<Capability> for Reader<'a> {
    fn from(c: Capability) -> Reader<'a> {
        Reader::Capability(c)
    }
}

impl<'a> From<Capability> for Builder<'a> {
    fn from(c: Capability) -> Builder<'a> {
        Builder::Capability(c)
    }
}
