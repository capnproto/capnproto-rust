//! Dynamically-typed lists.

use crate::dynamic_value;
use crate::introspect::{Type, TypeVariant};
use crate::private::layout::{self, PrimitiveElement};
use crate::traits::{IndexMove, ListIter};
use crate::{Error, ErrorKind, Result};

/// A read-only dynamically-typed list.
#[derive(Copy, Clone)]
pub struct Reader<'a> {
    pub(crate) reader: layout::ListReader<'a>,
    pub(crate) element_type: Type,
}

impl<'a> From<Reader<'a>> for dynamic_value::Reader<'a> {
    fn from(x: Reader<'a>) -> dynamic_value::Reader<'a> {
        dynamic_value::Reader::List(x)
    }
}

impl<'a> Reader<'a> {
    pub(crate) fn new(reader: layout::ListReader<'a>, element_type: Type) -> Self {
        Self {
            reader,
            element_type,
        }
    }

    pub fn len(&self) -> u32 {
        self.reader.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn element_type(&self) -> Type {
        self.element_type
    }

    pub fn get(self, index: u32) -> Result<crate::dynamic_value::Reader<'a>> {
        assert!(index < self.reader.len());
        match self.element_type.which() {
            TypeVariant::Void => Ok(dynamic_value::Reader::Void),
            TypeVariant::Bool => Ok(dynamic_value::Reader::Bool(PrimitiveElement::get(
                &self.reader,
                index,
            ))),
            TypeVariant::Int8 => Ok(dynamic_value::Reader::Int8(PrimitiveElement::get(
                &self.reader,
                index,
            ))),
            TypeVariant::Int16 => Ok(dynamic_value::Reader::Int16(PrimitiveElement::get(
                &self.reader,
                index,
            ))),
            TypeVariant::Int32 => Ok(dynamic_value::Reader::Int32(PrimitiveElement::get(
                &self.reader,
                index,
            ))),
            TypeVariant::Int64 => Ok(dynamic_value::Reader::Int64(PrimitiveElement::get(
                &self.reader,
                index,
            ))),
            TypeVariant::UInt8 => Ok(dynamic_value::Reader::UInt8(PrimitiveElement::get(
                &self.reader,
                index,
            ))),
            TypeVariant::UInt16 => Ok(dynamic_value::Reader::UInt16(PrimitiveElement::get(
                &self.reader,
                index,
            ))),
            TypeVariant::UInt32 => Ok(dynamic_value::Reader::UInt32(PrimitiveElement::get(
                &self.reader,
                index,
            ))),
            TypeVariant::UInt64 => Ok(dynamic_value::Reader::UInt64(PrimitiveElement::get(
                &self.reader,
                index,
            ))),
            TypeVariant::Float32 => Ok(dynamic_value::Reader::Float32(PrimitiveElement::get(
                &self.reader,
                index,
            ))),
            TypeVariant::Float64 => Ok(dynamic_value::Reader::Float64(PrimitiveElement::get(
                &self.reader,
                index,
            ))),
            TypeVariant::Enum(e) => Ok(dynamic_value::Enum::new(
                PrimitiveElement::get(&self.reader, index),
                e.into(),
            )
            .into()),
            TypeVariant::Text => Ok(dynamic_value::Reader::Text(
                self.reader.get_pointer_element(index).get_text(None)?,
            )),
            TypeVariant::Data => Ok(dynamic_value::Reader::Data(
                self.reader.get_pointer_element(index).get_data(None)?,
            )),
            TypeVariant::List(element_type) => Ok(Reader {
                reader: self
                    .reader
                    .get_pointer_element(index)
                    .get_list(element_type.expected_element_size(), None)?,
                element_type,
            }
            .into()),
            TypeVariant::Struct(ss) => {
                let r = self.reader.get_struct_element(index);
                Ok(dynamic_value::Reader::Struct(
                    crate::dynamic_struct::Reader::new(r, ss.into()),
                ))
            }
            TypeVariant::AnyPointer => {
                Ok(crate::any_pointer::Reader::new(self.reader.get_pointer_element(index)).into())
            }
            TypeVariant::Capability => {
                Ok(dynamic_value::Reader::Capability(dynamic_value::Capability))
            }
        }
    }

    pub fn iter(self) -> ListIter<Reader<'a>, Result<dynamic_value::Reader<'a>>> {
        ListIter::new(self, self.len())
    }
}

impl<'a> IndexMove<u32, Result<dynamic_value::Reader<'a>>> for Reader<'a> {
    fn index_move(&self, index: u32) -> Result<dynamic_value::Reader<'a>> {
        self.get(index)
    }
}

impl<'a> ::core::iter::IntoIterator for Reader<'a> {
    type Item = Result<dynamic_value::Reader<'a>>;
    type IntoIter = ListIter<Reader<'a>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A mutable dynamically-typed list.
pub struct Builder<'a> {
    pub(crate) builder: layout::ListBuilder<'a>,
    pub(crate) element_type: Type,
}

impl<'a> From<Builder<'a>> for dynamic_value::Builder<'a> {
    fn from(x: Builder<'a>) -> dynamic_value::Builder<'a> {
        dynamic_value::Builder::List(x)
    }
}

impl<'a> Builder<'a> {
    pub(crate) fn new(builder: layout::ListBuilder<'a>, element_type: Type) -> Self {
        Self {
            builder,
            element_type,
        }
    }

    pub fn reborrow(&mut self) -> Builder<'_> {
        Builder {
            builder: self.builder.reborrow(),
            element_type: self.element_type,
        }
    }

    pub fn into_reader(self) -> Reader<'a> {
        Reader {
            reader: self.builder.into_reader(),
            element_type: self.element_type,
        }
    }

    pub fn len(&self) -> u32 {
        self.builder.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn element_type(&self) -> Type {
        self.element_type
    }

    pub fn get(self, index: u32) -> Result<dynamic_value::Builder<'a>> {
        assert!(index < self.builder.len());
        match self.element_type.which() {
            TypeVariant::Void => Ok(dynamic_value::Builder::Void),
            TypeVariant::Bool => Ok(dynamic_value::Builder::Bool(
                PrimitiveElement::get_from_builder(&self.builder, index),
            )),
            TypeVariant::Int8 => Ok(dynamic_value::Builder::Int8(
                PrimitiveElement::get_from_builder(&self.builder, index),
            )),
            TypeVariant::Int16 => Ok(dynamic_value::Builder::Int16(
                PrimitiveElement::get_from_builder(&self.builder, index),
            )),
            TypeVariant::Int32 => Ok(dynamic_value::Builder::Int32(
                PrimitiveElement::get_from_builder(&self.builder, index),
            )),
            TypeVariant::Int64 => Ok(dynamic_value::Builder::Int64(
                PrimitiveElement::get_from_builder(&self.builder, index),
            )),
            TypeVariant::UInt8 => Ok(dynamic_value::Builder::UInt8(
                PrimitiveElement::get_from_builder(&self.builder, index),
            )),
            TypeVariant::UInt16 => Ok(dynamic_value::Builder::UInt16(
                PrimitiveElement::get_from_builder(&self.builder, index),
            )),
            TypeVariant::UInt32 => Ok(dynamic_value::Builder::UInt32(
                PrimitiveElement::get_from_builder(&self.builder, index),
            )),
            TypeVariant::UInt64 => Ok(dynamic_value::Builder::UInt64(
                PrimitiveElement::get_from_builder(&self.builder, index),
            )),
            TypeVariant::Float32 => Ok(dynamic_value::Builder::Float32(
                PrimitiveElement::get_from_builder(&self.builder, index),
            )),
            TypeVariant::Float64 => Ok(dynamic_value::Builder::Float64(
                PrimitiveElement::get_from_builder(&self.builder, index),
            )),
            TypeVariant::Enum(e) => Ok(dynamic_value::Enum::new(
                PrimitiveElement::get_from_builder(&self.builder, index),
                e.into(),
            )
            .into()),
            TypeVariant::Text => Ok(dynamic_value::Builder::Text(
                self.builder.get_pointer_element(index).get_text(None)?,
            )),
            TypeVariant::Data => Ok(dynamic_value::Builder::Data(
                self.builder.get_pointer_element(index).get_data(None)?,
            )),
            TypeVariant::List(element_type) => Ok(Builder {
                builder: self
                    .builder
                    .get_pointer_element(index)
                    .get_list(element_type.expected_element_size(), None)?,
                element_type,
            }
            .into()),
            TypeVariant::Struct(ss) => {
                let r = self.builder.get_struct_element(index);
                Ok(dynamic_value::Builder::Struct(
                    crate::dynamic_struct::Builder::new(r, ss.into()),
                ))
            }
            TypeVariant::AnyPointer => Ok(crate::any_pointer::Builder::new(
                self.builder.get_pointer_element(index),
            )
            .into()),
            TypeVariant::Capability => Ok(dynamic_value::Builder::Capability(
                dynamic_value::Capability,
            )),
        }
    }

    pub fn set(&mut self, index: u32, value: dynamic_value::Reader<'_>) -> Result<()> {
        assert!(index < self.builder.len());
        match (self.element_type.which(), value) {
            (TypeVariant::Void, _) => Ok(()),
            (TypeVariant::Bool, dynamic_value::Reader::Bool(b)) => {
                PrimitiveElement::set(&self.builder, index, b);
                Ok(())
            }
            (TypeVariant::Int8, dynamic_value::Reader::Int8(x)) => {
                PrimitiveElement::set(&self.builder, index, x);
                Ok(())
            }
            (TypeVariant::Int16, dynamic_value::Reader::Int16(x)) => {
                PrimitiveElement::set(&self.builder, index, x);
                Ok(())
            }
            (TypeVariant::Int32, dynamic_value::Reader::Int32(x)) => {
                PrimitiveElement::set(&self.builder, index, x);
                Ok(())
            }
            (TypeVariant::Int64, dynamic_value::Reader::Int64(x)) => {
                PrimitiveElement::set(&self.builder, index, x);
                Ok(())
            }
            (TypeVariant::UInt8, dynamic_value::Reader::UInt8(x)) => {
                PrimitiveElement::set(&self.builder, index, x);
                Ok(())
            }
            (TypeVariant::UInt16, dynamic_value::Reader::UInt16(x)) => {
                PrimitiveElement::set(&self.builder, index, x);
                Ok(())
            }
            (TypeVariant::UInt32, dynamic_value::Reader::UInt32(x)) => {
                PrimitiveElement::set(&self.builder, index, x);
                Ok(())
            }
            (TypeVariant::UInt64, dynamic_value::Reader::UInt64(x)) => {
                PrimitiveElement::set(&self.builder, index, x);
                Ok(())
            }
            (TypeVariant::Float32, dynamic_value::Reader::Float32(x)) => {
                PrimitiveElement::set(&self.builder, index, x);
                Ok(())
            }
            (TypeVariant::Float64, dynamic_value::Reader::Float64(x)) => {
                PrimitiveElement::set(&self.builder, index, x);
                Ok(())
            }
            (TypeVariant::Enum(_es), dynamic_value::Reader::Enum(e)) => {
                PrimitiveElement::set(&self.builder, index, e.get_value());
                Ok(())
            }
            (TypeVariant::Text, dynamic_value::Reader::Text(t)) => {
                self.builder
                    .reborrow()
                    .get_pointer_element(index)
                    .set_text(t);
                Ok(())
            }
            (TypeVariant::Data, dynamic_value::Reader::Data(d)) => {
                self.builder
                    .reborrow()
                    .get_pointer_element(index)
                    .set_data(d);
                Ok(())
            }
            (TypeVariant::Struct(ss), dynamic_value::Reader::Struct(s)) => {
                assert_eq!(ss, s.get_schema().raw);
                self.builder
                    .reborrow()
                    .get_struct_element(index)
                    .copy_content_from(&s.reader)
            }
            (TypeVariant::List(_element_type), dynamic_value::Reader::List(list)) => self
                .builder
                .reborrow()
                .get_pointer_element(index)
                .set_list(&list.reader, false),
            (TypeVariant::AnyPointer, _) => {
                Err(Error::from_kind(ErrorKind::ListAnyPointerNotSupported))
            }
            (TypeVariant::Capability, dynamic_value::Reader::Capability(_)) => {
                Err(Error::from_kind(ErrorKind::ListCapabilityNotSupported))
            }
            (_, _) => Err(Error::from_kind(ErrorKind::TypeMismatch)),
        }
    }

    pub fn init(self, index: u32, size: u32) -> Result<dynamic_value::Builder<'a>> {
        assert!(index < self.builder.len());
        match self.element_type.which() {
            TypeVariant::Void
            | TypeVariant::Bool
            | TypeVariant::Int8
            | TypeVariant::Int16
            | TypeVariant::Int32
            | TypeVariant::Int64
            | TypeVariant::UInt8
            | TypeVariant::UInt16
            | TypeVariant::UInt32
            | TypeVariant::UInt64
            | TypeVariant::Float32
            | TypeVariant::Float64
            | TypeVariant::Enum(_)
            | TypeVariant::Struct(_)
            | TypeVariant::Capability => Err(Error::from_kind(ErrorKind::ExpectedAListOrBlob)),
            TypeVariant::Text => Ok(self
                .builder
                .get_pointer_element(index)
                .init_text(size)
                .into()),
            TypeVariant::Data => Ok(self
                .builder
                .get_pointer_element(index)
                .init_data(size)
                .into()),
            TypeVariant::List(inner_element_type) => match inner_element_type.which() {
                TypeVariant::Struct(rbs) => Ok(Builder::new(
                    self.builder.get_pointer_element(index).init_struct_list(
                        size,
                        crate::dynamic_struct::struct_size_from_schema(rbs.into())?,
                    ),
                    inner_element_type,
                )
                .into()),
                _ => Ok(Builder::new(
                    self.builder
                        .get_pointer_element(index)
                        .init_list(inner_element_type.expected_element_size(), size),
                    inner_element_type,
                )
                .into()),
            },
            TypeVariant::AnyPointer => Err(Error::from_kind(ErrorKind::ListAnyPointerNotSupported)),
        }
    }
}

impl<'a> crate::traits::SetPointerBuilder for Reader<'a> {
    fn set_pointer_builder<'b>(
        mut pointer: crate::private::layout::PointerBuilder<'b>,
        value: Reader<'a>,
        canonicalize: bool,
    ) -> Result<()> {
        pointer.set_list(&value.reader, canonicalize)
    }
}
