//! Dynamically-typed structs.

use crate::introspect::TypeVariant;
use crate::private::layout;
use crate::schema::{Field, StructSchema};
use crate::schema_capnp::{field, node, value};
use crate::{dynamic_list, dynamic_value};
use crate::{Error, ErrorKind, Result};

fn has_discriminant_value(reader: field::Reader) -> bool {
    reader.get_discriminant_value() != field::NO_DISCRIMINANT
}

pub(crate) fn struct_size_from_schema(schema: StructSchema) -> Result<layout::StructSize> {
    if let node::Struct(s) = schema.proto.which()? {
        Ok(layout::StructSize {
            data: s.get_data_word_count(),
            pointers: s.get_pointer_count(),
        })
    } else {
        Err(Error::from_kind(ErrorKind::NotAStruct))
    }
}

/// A read-only dynamically-typed struct.
#[derive(Clone, Copy)]
pub struct Reader<'a> {
    pub(crate) reader: layout::StructReader<'a>,
    schema: StructSchema,
}

impl<'a> From<Reader<'a>> for dynamic_value::Reader<'a> {
    fn from(x: Reader<'a>) -> dynamic_value::Reader<'a> {
        dynamic_value::Reader::Struct(x)
    }
}

impl<'a> Reader<'a> {
    pub fn new(reader: layout::StructReader<'a>, schema: StructSchema) -> Self {
        Self { reader, schema }
    }

    pub fn total_size(&self) -> crate::Result<crate::MessageSize> {
        self.reader.total_size()
    }

    pub fn get_schema(&self) -> StructSchema {
        self.schema
    }

    pub fn get(self, field: Field) -> Result<dynamic_value::Reader<'a>> {
        assert_eq!(self.schema.raw, field.parent.raw);
        let ty = field.get_type();
        match field.get_proto().which()? {
            field::Slot(slot) => {
                let offset = slot.get_offset();
                let default_value = slot.get_default_value()?;

                match (ty.which(), default_value.which()?) {
                    (TypeVariant::Void, _) => Ok(dynamic_value::Reader::Void),
                    (TypeVariant::Bool, value::Bool(b)) => Ok(dynamic_value::Reader::Bool(
                        self.reader.get_bool_field_mask(offset as usize, b),
                    )),
                    (TypeVariant::Int8, value::Int8(x)) => Ok(dynamic_value::Reader::Int8(
                        self.reader.get_data_field_mask::<i8>(offset as usize, x),
                    )),
                    (TypeVariant::Int16, value::Int16(x)) => Ok(dynamic_value::Reader::Int16(
                        self.reader.get_data_field_mask::<i16>(offset as usize, x),
                    )),
                    (TypeVariant::Int32, value::Int32(x)) => Ok(dynamic_value::Reader::Int32(
                        self.reader.get_data_field_mask::<i32>(offset as usize, x),
                    )),
                    (TypeVariant::Int64, value::Int64(x)) => Ok(dynamic_value::Reader::Int64(
                        self.reader.get_data_field_mask::<i64>(offset as usize, x),
                    )),
                    (TypeVariant::UInt8, value::Uint8(x)) => Ok(dynamic_value::Reader::UInt8(
                        self.reader.get_data_field_mask::<u8>(offset as usize, x),
                    )),
                    (TypeVariant::UInt16, value::Uint16(x)) => Ok(dynamic_value::Reader::UInt16(
                        self.reader.get_data_field_mask::<u16>(offset as usize, x),
                    )),
                    (TypeVariant::UInt32, value::Uint32(x)) => Ok(dynamic_value::Reader::UInt32(
                        self.reader.get_data_field_mask::<u32>(offset as usize, x),
                    )),
                    (TypeVariant::UInt64, value::Uint64(x)) => Ok(dynamic_value::Reader::UInt64(
                        self.reader.get_data_field_mask::<u64>(offset as usize, x),
                    )),
                    (TypeVariant::Float32, value::Float32(x)) => {
                        Ok(dynamic_value::Reader::Float32(
                            self.reader
                                .get_data_field_mask::<f32>(offset as usize, x.to_bits()),
                        ))
                    }
                    (TypeVariant::Float64, value::Float64(x)) => {
                        Ok(dynamic_value::Reader::Float64(
                            self.reader
                                .get_data_field_mask::<f64>(offset as usize, x.to_bits()),
                        ))
                    }
                    (TypeVariant::Enum(schema), value::Enum(d)) => Ok(dynamic_value::Enum::new(
                        self.reader.get_data_field_mask::<u16>(offset as usize, d),
                        schema.into(),
                    )
                    .into()),
                    (TypeVariant::Text, dval) => {
                        let p = self.reader.get_pointer_field(offset as usize);
                        // If the type is a generic, then the default value
                        // is always an empty AnyPointer. Ignore that case.
                        let t1 = if let (true, value::Text(t)) = (p.is_null(), dval) {
                            t?
                        } else {
                            p.get_text(None)?
                        };
                        Ok(dynamic_value::Reader::Text(t1))
                    }
                    (TypeVariant::Data, dval) => {
                        let p = self.reader.get_pointer_field(offset as usize);
                        // If the type is a generic, then the default value
                        // is always an empty AnyPointer. Ignore that case.
                        let d1 = if let (true, value::Data(d)) = (p.is_null(), dval) {
                            d?
                        } else {
                            p.get_data(None)?
                        };
                        Ok(dynamic_value::Reader::Data(d1))
                    }
                    (TypeVariant::Struct(schema), dval) => {
                        let p = self.reader.get_pointer_field(offset as usize);
                        // If the type is a generic, then the default value
                        // is always an empty AnyPointer. Ignore that case.
                        let p1 = if let (true, value::Struct(s)) = (p.is_null(), dval) {
                            s.reader
                        } else {
                            p
                        };
                        let r = p1.get_struct(None)?;
                        Ok(Reader::new(r, schema.into()).into())
                    }
                    (TypeVariant::List(element_type), dval) => {
                        let p = self.reader.get_pointer_field(offset as usize);
                        // If the type is a generic, then the default value
                        // is always an empty AnyPointer. Ignore that case.
                        let p1 = if let (true, value::List(l)) = (p.is_null(), dval) {
                            l.reader
                        } else {
                            p
                        };
                        let l = p1.get_list(element_type.expected_element_size(), None)?;
                        Ok(dynamic_list::Reader::new(l, element_type).into())
                    }
                    (TypeVariant::AnyPointer, value::AnyPointer(a)) => {
                        let p = self.reader.get_pointer_field(offset as usize);
                        let a1 = if p.is_null() {
                            a
                        } else {
                            crate::any_pointer::Reader::new(p)
                        };
                        Ok(dynamic_value::Reader::AnyPointer(a1))
                    }
                    (TypeVariant::Capability, value::Interface(())) => {
                        Ok(dynamic_value::Reader::Capability(dynamic_value::Capability))
                    }
                    _ => Err(Error::from_kind(ErrorKind::FieldAndDefaultMismatch)),
                }
            }
            field::Group(_) => {
                if let TypeVariant::Struct(schema) = ty.which() {
                    Ok(Reader::new(self.reader, schema.into()).into())
                } else {
                    Err(Error::from_kind(ErrorKind::GroupFieldButTypeIsNotStruct))
                }
            }
        }
    }

    /// Gets the field with the given name.
    pub fn get_named(self, field_name: &str) -> Result<dynamic_value::Reader<'a>> {
        self.get(self.schema.get_field_by_name(field_name)?)
    }

    /// If this struct has union fields, returns the one that is currently active.
    /// Otherwise, returns None.
    pub fn which(&self) -> Result<Option<Field>> {
        let node::Struct(st) = self.schema.get_proto().which()? else {
            return Err(Error::from_kind(ErrorKind::NotAStruct));
        };
        if st.get_discriminant_count() == 0 {
            Ok(None)
        } else {
            let discrim = self
                .reader
                .get_data_field::<u16>(st.get_discriminant_offset() as usize);
            self.schema.get_field_by_discriminant(discrim)
        }
    }

    /// Returns `false` if the field is a pointer and the pointer is null.
    pub fn has(&self, field: Field) -> Result<bool> {
        assert_eq!(self.schema.raw, field.parent.raw);
        let proto = field.get_proto();
        if has_discriminant_value(proto) {
            let node::Struct(st) = self.schema.get_proto().which()? else {
                return Err(Error::from_kind(ErrorKind::NotAStruct));
            };

            let discrim = self
                .reader
                .get_data_field::<u16>(st.get_discriminant_offset() as usize);
            if discrim != proto.get_discriminant_value() {
                // Field is not active in the union.
                return Ok(false);
            }
        }
        let slot = match proto.which()? {
            field::Group(_) => return Ok(true),
            field::Slot(s) => s,
        };
        let ty = field.get_type();
        if ty.is_pointer_type() {
            Ok(!self
                .reader
                .get_pointer_field(slot.get_offset() as usize)
                .is_null())
        } else {
            Ok(true)
        }
    }

    pub fn has_named(&self, field_name: &str) -> Result<bool> {
        let field = self.schema.get_field_by_name(field_name)?;
        self.has(field)
    }
}

/// A mutable dynamically-typed struct.
pub struct Builder<'a> {
    builder: layout::StructBuilder<'a>,
    schema: StructSchema,
}

impl<'a> From<Builder<'a>> for dynamic_value::Builder<'a> {
    fn from(x: Builder<'a>) -> dynamic_value::Builder<'a> {
        dynamic_value::Builder::Struct(x)
    }
}

impl<'a> Builder<'a> {
    pub fn new(builder: layout::StructBuilder<'a>, schema: StructSchema) -> Self {
        Self { builder, schema }
    }

    pub fn reborrow(&mut self) -> Builder<'_> {
        Builder {
            builder: self.builder.reborrow(),
            schema: self.schema,
        }
    }

    pub fn reborrow_as_reader(&self) -> Reader<'_> {
        Reader {
            reader: self.builder.as_reader(),
            schema: self.schema,
        }
    }

    pub fn into_reader(self) -> Reader<'a> {
        Reader {
            schema: self.schema,
            reader: self.builder.into_reader(),
        }
    }

    pub fn get_schema(&self) -> StructSchema {
        self.schema
    }

    pub fn get(self, field: Field) -> Result<dynamic_value::Builder<'a>> {
        assert_eq!(self.schema.raw, field.parent.raw);
        let ty = field.get_type();
        match field.get_proto().which()? {
            field::Slot(slot) => {
                let offset = slot.get_offset();
                let default_value = slot.get_default_value()?;

                match (ty.which(), default_value.which()?) {
                    (TypeVariant::Void, _) => Ok(dynamic_value::Builder::Void),
                    (TypeVariant::Bool, value::Bool(b)) => Ok(dynamic_value::Builder::Bool(
                        self.builder.get_bool_field_mask(offset as usize, b),
                    )),
                    (TypeVariant::Int8, value::Int8(x)) => Ok(dynamic_value::Builder::Int8(
                        self.builder.get_data_field_mask::<i8>(offset as usize, x),
                    )),
                    (TypeVariant::Int16, value::Int16(x)) => Ok(dynamic_value::Builder::Int16(
                        self.builder.get_data_field_mask::<i16>(offset as usize, x),
                    )),
                    (TypeVariant::Int32, value::Int32(x)) => Ok(dynamic_value::Builder::Int32(
                        self.builder.get_data_field_mask::<i32>(offset as usize, x),
                    )),
                    (TypeVariant::Int64, value::Int64(x)) => Ok(dynamic_value::Builder::Int64(
                        self.builder.get_data_field_mask::<i64>(offset as usize, x),
                    )),
                    (TypeVariant::UInt8, value::Uint8(x)) => Ok(dynamic_value::Builder::UInt8(
                        self.builder.get_data_field_mask::<u8>(offset as usize, x),
                    )),
                    (TypeVariant::UInt16, value::Uint16(x)) => Ok(dynamic_value::Builder::UInt16(
                        self.builder.get_data_field_mask::<u16>(offset as usize, x),
                    )),
                    (TypeVariant::UInt32, value::Uint32(x)) => Ok(dynamic_value::Builder::UInt32(
                        self.builder.get_data_field_mask::<u32>(offset as usize, x),
                    )),
                    (TypeVariant::UInt64, value::Uint64(x)) => Ok(dynamic_value::Builder::UInt64(
                        self.builder.get_data_field_mask::<u64>(offset as usize, x),
                    )),
                    (TypeVariant::Float32, value::Float32(x)) => {
                        Ok(dynamic_value::Builder::Float32(
                            self.builder
                                .get_data_field_mask::<f32>(offset as usize, x.to_bits()),
                        ))
                    }
                    (TypeVariant::Float64, value::Float64(x)) => {
                        Ok(dynamic_value::Builder::Float64(
                            self.builder
                                .get_data_field_mask::<f64>(offset as usize, x.to_bits()),
                        ))
                    }
                    (TypeVariant::Enum(schema), value::Enum(d)) => Ok(dynamic_value::Enum::new(
                        self.builder.get_data_field_mask::<u16>(offset as usize, d),
                        schema.into(),
                    )
                    .into()),
                    (TypeVariant::Text, dval) => {
                        let mut p = self.builder.get_pointer_field(offset as usize);
                        if p.is_null() {
                            // If the type is a generic, then the default value
                            // is always an empty AnyPointer. Ignore that case.
                            if let value::Text(t) = dval {
                                p.set_text(t?);
                            }
                        }
                        Ok(dynamic_value::Builder::Text(p.get_text(None)?))
                    }
                    (TypeVariant::Data, dval) => {
                        let mut p = self.builder.get_pointer_field(offset as usize);
                        if p.is_null() {
                            // If the type is a generic, then the default value
                            // is always an empty AnyPointer. Ignore that case.
                            if let value::Data(d) = dval {
                                p.set_data(d?);
                            }
                        }
                        Ok(dynamic_value::Builder::Data(p.get_data(None)?))
                    }
                    (TypeVariant::Struct(schema), dval) => {
                        let mut p = self.builder.get_pointer_field(offset as usize);
                        if p.is_null() {
                            // If the type is a generic, then the default value
                            // is always an empty AnyPointer. Ignore that case.
                            if let value::Struct(s) = dval {
                                p.copy_from(s.reader, false)?;
                            }
                        }
                        Ok(Builder::new(
                            p.get_struct(struct_size_from_schema(schema.into())?, None)?,
                            schema.into(),
                        )
                        .into())
                    }
                    (TypeVariant::List(element_type), dval) => {
                        let mut p = self.builder.get_pointer_field(offset as usize);
                        if p.is_null() {
                            if let value::List(l) = dval {
                                p.copy_from(l.reader, false)?;
                            }
                        }
                        let l = if let TypeVariant::Struct(ss) = element_type.which() {
                            p.get_struct_list(struct_size_from_schema(ss.into())?, None)?
                        } else {
                            p.get_list(element_type.expected_element_size(), None)?
                        };

                        Ok(dynamic_list::Builder::new(l, element_type).into())
                    }
                    (TypeVariant::AnyPointer, value::AnyPointer(_a)) => {
                        // AnyPointer fields can't have a nontrivial default.
                        Ok(crate::any_pointer::Builder::new(
                            self.builder.get_pointer_field(offset as usize),
                        )
                        .into())
                    }
                    (TypeVariant::Capability, value::Interface(())) => Ok(
                        dynamic_value::Builder::Capability(dynamic_value::Capability),
                    ),
                    _ => Err(Error::from_kind(ErrorKind::FieldAndDefaultMismatch)),
                }
            }
            field::Group(_) => {
                if let TypeVariant::Struct(schema) = ty.which() {
                    Ok(Builder::new(self.builder, schema.into()).into())
                } else {
                    Err(Error::from_kind(ErrorKind::GroupFieldButTypeIsNotStruct))
                }
            }
        }
    }

    pub fn get_named(self, field_name: &str) -> Result<dynamic_value::Builder<'a>> {
        let field = self.schema.get_field_by_name(field_name)?;
        self.get(field)
    }

    pub fn which(&self) -> Result<Option<Field>> {
        let node::Struct(st) = self.schema.get_proto().which()? else {
            return Err(Error::from_kind(ErrorKind::NotAStruct));
        };
        if st.get_discriminant_count() == 0 {
            Ok(None)
        } else {
            let discrim = self
                .builder
                .get_data_field::<u16>(st.get_discriminant_offset() as usize);
            self.schema.get_field_by_discriminant(discrim)
        }
    }

    pub fn has(&self, field: Field) -> Result<bool> {
        self.reborrow_as_reader().has(field)
    }

    pub fn has_named(&self, field_name: &str) -> Result<bool> {
        let field = self.schema.get_field_by_name(field_name)?;
        self.has(field)
    }

    pub fn set(&mut self, field: Field, value: dynamic_value::Reader<'_>) -> Result<()> {
        assert_eq!(self.schema.raw, field.parent.raw);
        self.set_in_union(field)?;
        let ty = field.get_type();
        match field.get_proto().which()? {
            field::Slot(slot) => {
                let dval = slot.get_default_value()?;
                let offset = slot.get_offset() as usize;
                match (ty.which(), value, dval.which()?) {
                    (TypeVariant::Void, _, _) => Ok(()),
                    (TypeVariant::Bool, dynamic_value::Reader::Bool(v), value::Bool(b)) => {
                        self.builder.set_bool_field_mask(offset, v, b);
                        Ok(())
                    }
                    (TypeVariant::Int8, dynamic_value::Reader::Int8(v), value::Int8(d)) => {
                        self.builder.set_data_field_mask::<i8>(offset, v, d);
                        Ok(())
                    }
                    (TypeVariant::Int16, dynamic_value::Reader::Int16(v), value::Int16(d)) => {
                        self.builder.set_data_field_mask::<i16>(offset, v, d);
                        Ok(())
                    }
                    (TypeVariant::Int32, dynamic_value::Reader::Int32(v), value::Int32(d)) => {
                        self.builder.set_data_field_mask::<i32>(offset, v, d);
                        Ok(())
                    }
                    (TypeVariant::Int64, dynamic_value::Reader::Int64(v), value::Int64(d)) => {
                        self.builder.set_data_field_mask::<i64>(offset, v, d);
                        Ok(())
                    }
                    (TypeVariant::UInt8, dynamic_value::Reader::UInt8(v), value::Uint8(d)) => {
                        self.builder.set_data_field_mask::<u8>(offset, v, d);
                        Ok(())
                    }
                    (TypeVariant::UInt16, dynamic_value::Reader::UInt16(v), value::Uint16(d)) => {
                        self.builder.set_data_field_mask::<u16>(offset, v, d);
                        Ok(())
                    }
                    (TypeVariant::UInt32, dynamic_value::Reader::UInt32(v), value::Uint32(d)) => {
                        self.builder.set_data_field_mask::<u32>(offset, v, d);
                        Ok(())
                    }
                    (TypeVariant::UInt64, dynamic_value::Reader::UInt64(v), value::Uint64(d)) => {
                        self.builder.set_data_field_mask::<u64>(offset, v, d);
                        Ok(())
                    }
                    (
                        TypeVariant::Float32,
                        dynamic_value::Reader::Float32(v),
                        value::Float32(d),
                    ) => {
                        self.builder
                            .set_data_field_mask::<f32>(offset, v, d.to_bits());
                        Ok(())
                    }
                    (
                        TypeVariant::Float64,
                        dynamic_value::Reader::Float64(v),
                        value::Float64(d),
                    ) => {
                        self.builder
                            .set_data_field_mask::<f64>(offset, v, d.to_bits());
                        Ok(())
                    }
                    (TypeVariant::Enum(_), dynamic_value::Reader::Enum(ev), value::Enum(d)) => {
                        self.builder
                            .set_data_field_mask::<u16>(offset, ev.get_value(), d);
                        Ok(())
                    }
                    (TypeVariant::Text, dynamic_value::Reader::Text(tv), _) => {
                        let mut p = self.builder.reborrow().get_pointer_field(offset);
                        p.set_text(tv);
                        Ok(())
                    }
                    (TypeVariant::Data, dynamic_value::Reader::Data(v), _) => {
                        let mut p = self.builder.reborrow().get_pointer_field(offset);
                        p.set_data(v);
                        Ok(())
                    }
                    (TypeVariant::List(_), dynamic_value::Reader::List(l), _) => {
                        let mut p = self.builder.reborrow().get_pointer_field(offset);
                        p.set_list(&l.reader, false)
                    }
                    (TypeVariant::Struct(_), dynamic_value::Reader::Struct(v), _) => {
                        let mut p = self.builder.reborrow().get_pointer_field(offset);
                        p.set_struct(&v.reader, false)
                    }
                    (TypeVariant::AnyPointer, _, _) => {
                        let mut target = crate::any_pointer::Builder::new(
                            self.builder.reborrow().get_pointer_field(offset),
                        );
                        match value {
                            dynamic_value::Reader::Text(t) => target.set_as(t),
                            dynamic_value::Reader::Data(t) => target.set_as(t),
                            dynamic_value::Reader::Struct(s) => target.set_as(s),
                            dynamic_value::Reader::List(l) => target.set_as(l),
                            dynamic_value::Reader::Capability(_) => Err(Error::from_kind(
                                ErrorKind::SettingDynamicCapabilitiesIsUnsupported,
                            )),
                            _ => Err(Error::from_kind(
                                ErrorKind::CannotSetAnyPointerFieldToAPrimitiveValue,
                            )),
                        }
                    }
                    (TypeVariant::Capability, _, _) => Err(Error::from_kind(
                        ErrorKind::SettingDynamicCapabilitiesIsUnsupported,
                    )),
                    _ => Err(Error::from_kind(ErrorKind::TypeMismatch)),
                }
            }
            field::Group(_group) => {
                let dynamic_value::Reader::Struct(src) = value else {
                    return Err(Error::from_kind(ErrorKind::NotAStruct));
                };
                let dynamic_value::Builder::Struct(mut dst) = self.reborrow().init(field)? else {
                    return Err(Error::from_kind(ErrorKind::NotAStruct));
                };
                if let Some(union_field) = src.which()? {
                    dst.set(union_field, src.get(union_field)?)?;
                }

                let non_union_fields = src.schema.get_non_union_fields()?;
                for idx in 0..non_union_fields.len() {
                    let field = non_union_fields.get(idx);
                    if src.has(field)? {
                        dst.set(field, src.get(field)?)?;
                    }
                }
                Ok(())
            }
        }
    }

    pub fn set_named(&mut self, field_name: &str, value: dynamic_value::Reader<'_>) -> Result<()> {
        let field = self.schema.get_field_by_name(field_name)?;
        self.set(field, value)
    }

    pub fn init(mut self, field: Field) -> Result<dynamic_value::Builder<'a>> {
        assert_eq!(self.schema.raw, field.parent.raw);
        self.set_in_union(field)?;
        let ty = field.get_type();
        match field.get_proto().which()? {
            field::Slot(slot) => {
                let offset = slot.get_offset() as usize;
                match ty.which() {
                    TypeVariant::Struct(ss) => Ok(Builder {
                        schema: ss.into(),
                        builder: self
                            .builder
                            .get_pointer_field(offset)
                            .init_struct(struct_size_from_schema(ss.into())?),
                    }
                    .into()),
                    TypeVariant::AnyPointer => {
                        let mut p = self.builder.get_pointer_field(offset);
                        p.clear();
                        Ok(crate::any_pointer::Builder::new(p).into())
                    }
                    _ => Err(Error::from_kind(
                        ErrorKind::InitIsOnlyValidForStructAndAnyPointerFields,
                    )),
                }
            }
            field::Group(_) => {
                self.clear(field)?;
                let TypeVariant::Struct(schema) = ty.which() else {
                    return Err(Error::from_kind(ErrorKind::NotAStruct));
                };
                Ok((Builder::new(self.builder, schema.into())).into())
            }
        }
    }

    pub fn init_named(self, field_name: &str) -> Result<dynamic_value::Builder<'a>> {
        let field = self.schema.get_field_by_name(field_name)?;
        self.init(field)
    }

    pub fn initn(mut self, field: Field, size: u32) -> Result<dynamic_value::Builder<'a>> {
        assert_eq!(self.schema.raw, field.parent.raw);
        self.set_in_union(field)?;
        let ty = field.get_type();
        match field.get_proto().which()? {
            field::Slot(slot) => {
                let offset = slot.get_offset() as usize;
                match ty.which() {
                    TypeVariant::List(element_type) => match element_type.which() {
                        TypeVariant::Struct(ss) => Ok(dynamic_list::Builder::new(
                            self.builder
                                .get_pointer_field(offset)
                                .init_struct_list(size, struct_size_from_schema(ss.into())?),
                            element_type,
                        )
                        .into()),
                        _ => Ok(dynamic_list::Builder::new(
                            self.builder
                                .get_pointer_field(offset)
                                .init_list(element_type.expected_element_size(), size),
                            element_type,
                        )
                        .into()),
                    },
                    TypeVariant::Text => Ok(self
                        .builder
                        .get_pointer_field(offset)
                        .init_text(size)
                        .into()),
                    TypeVariant::Data => Ok(self
                        .builder
                        .get_pointer_field(offset)
                        .init_data(size)
                        .into()),

                    _ => Err(Error::from_kind(
                        ErrorKind::InitnIsOnlyValidForListTextOrDataFields,
                    )),
                }
            }
            field::Group(_) => Err(Error::from_kind(
                ErrorKind::InitnIsOnlyValidForListTextOrDataFields,
            )),
        }
    }

    pub fn initn_named(self, field_name: &str, size: u32) -> Result<dynamic_value::Builder<'a>> {
        let field = self.schema.get_field_by_name(field_name)?;
        self.initn(field, size)
    }

    /// Clears a field, setting it to its default value. For pointer fields,
    /// this makes the field null.
    pub fn clear(&mut self, field: Field) -> Result<()> {
        assert_eq!(self.schema.raw, field.parent.raw);
        self.set_in_union(field)?;
        let ty = field.get_type();
        match field.get_proto().which()? {
            field::Slot(slot) => {
                let offset = slot.get_offset() as usize;
                match ty.which() {
                    TypeVariant::Void => Ok(()),
                    TypeVariant::Bool => {
                        self.builder.set_bool_field(offset, false);
                        Ok(())
                    }
                    TypeVariant::Int8 => {
                        self.builder.set_data_field::<i8>(offset, 0);
                        Ok(())
                    }
                    TypeVariant::Int16 => {
                        self.builder.set_data_field::<i16>(offset, 0);
                        Ok(())
                    }
                    TypeVariant::Int32 => {
                        self.builder.set_data_field::<i32>(offset, 0);
                        Ok(())
                    }
                    TypeVariant::Int64 => {
                        self.builder.set_data_field::<i64>(offset, 0);
                        Ok(())
                    }
                    TypeVariant::UInt8 => {
                        self.builder.set_data_field::<u8>(offset, 0);
                        Ok(())
                    }
                    TypeVariant::UInt16 => {
                        self.builder.set_data_field::<u16>(offset, 0);
                        Ok(())
                    }
                    TypeVariant::UInt32 => {
                        self.builder.set_data_field::<u32>(offset, 0);
                        Ok(())
                    }
                    TypeVariant::UInt64 => {
                        self.builder.set_data_field::<u64>(offset, 0);
                        Ok(())
                    }
                    TypeVariant::Float32 => {
                        self.builder.set_data_field::<f32>(offset, 0f32);
                        Ok(())
                    }
                    TypeVariant::Float64 => {
                        self.builder.set_data_field::<f64>(offset, 0f64);
                        Ok(())
                    }
                    TypeVariant::Enum(_) => {
                        self.builder.set_data_field::<u16>(offset, 0);
                        Ok(())
                    }
                    TypeVariant::Text
                    | TypeVariant::Data
                    | TypeVariant::Struct(_)
                    | TypeVariant::List(_)
                    | TypeVariant::AnyPointer
                    | TypeVariant::Capability => {
                        self.builder.reborrow().get_pointer_field(offset).clear();
                        Ok(())
                    }
                }
            }
            field::Group(_) => {
                let TypeVariant::Struct(schema) = ty.which() else {
                    return Err(Error::from_kind(ErrorKind::NotAStruct));
                };
                let mut group = Builder::new(self.builder.reborrow(), schema.into());

                // We clear the union field with discriminant 0 rather than the one that
                // is set because we want the union to end up with its default field active.
                if let Some(union_field) = group.schema.get_field_by_discriminant(0)? {
                    group.clear(union_field)?;
                }

                let non_union_fields = group.schema.get_non_union_fields()?;
                for idx in 0..non_union_fields.len() {
                    group.clear(non_union_fields.get(idx))?;
                }
                Ok(())
            }
        }
    }

    pub fn clear_named(&mut self, field_name: &str) -> Result<()> {
        let field = self.schema.get_field_by_name(field_name)?;
        self.clear(field)
    }

    fn set_in_union(&mut self, field: Field) -> Result<()> {
        if has_discriminant_value(field.get_proto()) {
            let node::Struct(st) = self.schema.get_proto().which()? else {
                return Err(Error::from_kind(ErrorKind::NotAStruct));
            };
            self.builder.set_data_field::<u16>(
                st.get_discriminant_offset() as usize,
                field.get_proto().get_discriminant_value(),
            );
        }
        Ok(())
    }
}

impl<'a> crate::traits::SetPointerBuilder for Reader<'a> {
    fn set_pointer_builder<'b>(
        mut pointer: crate::private::layout::PointerBuilder<'b>,
        value: Reader<'a>,
        canonicalize: bool,
    ) -> Result<()> {
        pointer.set_struct(&value.reader, canonicalize)
    }
}
