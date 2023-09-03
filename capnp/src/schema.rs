//! Convenience wrappers of the datatypes defined in schema.capnp.

use crate::dynamic_value;
use crate::introspect::{self, RawBrandedStructSchema, RawEnumSchema};
use crate::private::layout;
use crate::schema_capnp::{annotation, enumerant, field, node};
use crate::struct_list;
use crate::traits::{IndexMove, ListIter, ShortListIter};
use crate::Result;

/// A struct node, with generics applied.
#[derive(Clone, Copy)]
pub struct StructSchema {
    pub(crate) raw: RawBrandedStructSchema,
    pub(crate) proto: node::Reader<'static>,
}

impl StructSchema {
    pub fn new(raw: RawBrandedStructSchema) -> Self {
        let proto =
            crate::any_pointer::Reader::new(unsafe {
                layout::PointerReader::get_root_unchecked(
                    raw.generic.encoded_node.as_ptr() as *const u8
                )
            })
            .get_as()
            .unwrap();
        Self { raw, proto }
    }

    pub fn get_proto(&self) -> node::Reader<'static> {
        self.proto
    }

    pub fn get_fields(self) -> crate::Result<FieldList> {
        if let node::Struct(s) = self.proto.which()? {
            Ok(FieldList {
                fields: s.get_fields()?,
                parent: self,
            })
        } else {
            panic!()
        }
    }

    pub fn get_field_by_discriminant(self, discriminant: u16) -> Result<Option<Field>> {
        match self
            .raw
            .generic
            .members_by_discriminant
            .get(discriminant as usize)
        {
            None => Ok(None),
            Some(&idx) => Ok(Some(self.get_fields()?.get(idx))),
        }
    }

    /// Looks up a field by name. Returns `None` if no matching field is found.
    pub fn find_field_by_name(&self, name: &str) -> Result<Option<Field>> {
        for field in self.get_fields()? {
            if field.get_proto().get_name()? == name {
                return Ok(Some(field));
            }
        }
        Ok(None)
    }

    /// Like `find_field_by_name()`, but returns an error if the field is not found.
    pub fn get_field_by_name(&self, name: &str) -> Result<Field> {
        if let Some(field) = self.find_field_by_name(name)? {
            Ok(field)
        } else {
            let mut error = crate::Error::from_kind(crate::ErrorKind::FieldNotFound);
            write!(error, "{}", name);
            Err(error)
        }
    }

    pub fn get_union_fields(self) -> Result<FieldSubset> {
        if let node::Struct(s) = self.proto.which()? {
            Ok(FieldSubset {
                fields: s.get_fields()?,
                indices: self.raw.generic.members_by_discriminant,
                parent: self,
            })
        } else {
            panic!()
        }
    }

    pub fn get_non_union_fields(self) -> Result<FieldSubset> {
        if let node::Struct(s) = self.proto.which()? {
            Ok(FieldSubset {
                fields: s.get_fields()?,
                indices: self.raw.generic.nonunion_members,
                parent: self,
            })
        } else {
            panic!()
        }
    }

    pub fn get_annotations(self) -> Result<AnnotationList> {
        Ok(AnnotationList {
            annotations: self.proto.get_annotations()?,
            child_index: None,
            get_annotation_type: self.raw.annotation_types,
        })
    }
}

impl From<RawBrandedStructSchema> for StructSchema {
    fn from(rs: RawBrandedStructSchema) -> StructSchema {
        StructSchema::new(rs)
    }
}

/// A field of a struct, with generics applied.
#[derive(Clone, Copy)]
pub struct Field {
    proto: field::Reader<'static>,
    index: u16,
    pub(crate) parent: StructSchema,
}

impl Field {
    pub fn get_proto(self) -> field::Reader<'static> {
        self.proto
    }

    pub fn get_type(&self) -> introspect::Type {
        (self.parent.raw.field_types)(self.index)
    }

    pub fn get_index(&self) -> u16 {
        self.index
    }

    pub fn get_annotations(self) -> Result<AnnotationList> {
        Ok(AnnotationList {
            annotations: self.proto.get_annotations()?,
            child_index: Some(self.index),
            get_annotation_type: self.parent.raw.annotation_types,
        })
    }
}

/// A list of fields of a struct, with generics applied.
#[derive(Clone, Copy)]
pub struct FieldList {
    pub(crate) fields: crate::struct_list::Reader<'static, field::Owned>,
    pub(crate) parent: StructSchema,
}

impl FieldList {
    pub fn len(&self) -> u16 {
        self.fields.len() as u16
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(self, index: u16) -> Field {
        Field {
            proto: self.fields.get(index as u32),
            index,
            parent: self.parent,
        }
    }

    pub fn iter(self) -> ShortListIter<Self, Field> {
        ShortListIter::new(self, self.len())
    }
}

impl IndexMove<u16, Field> for FieldList {
    fn index_move(&self, index: u16) -> Field {
        self.get(index)
    }
}

impl ::core::iter::IntoIterator for FieldList {
    type Item = Field;
    type IntoIter = ShortListIter<FieldList, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A list of a subset of fields of a struct, with generics applied.
#[derive(Clone, Copy)]
pub struct FieldSubset {
    fields: struct_list::Reader<'static, field::Owned>,
    indices: &'static [u16],
    parent: StructSchema,
}

impl FieldSubset {
    pub fn len(&self) -> u16 {
        self.indices.len() as u16
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(self, index: u16) -> Field {
        let index = self.indices[index as usize];
        Field {
            proto: self.fields.get(index as u32),
            index,
            parent: self.parent,
        }
    }

    pub fn iter(self) -> ShortListIter<Self, Field> {
        ShortListIter::new(self, self.len())
    }
}

impl IndexMove<u16, Field> for FieldSubset {
    fn index_move(&self, index: u16) -> Field {
        self.get(index)
    }
}

impl ::core::iter::IntoIterator for FieldSubset {
    type Item = Field;
    type IntoIter = ShortListIter<FieldSubset, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An enum, with generics applied. (Generics may affect types of annotations.)
#[derive(Clone, Copy)]
pub struct EnumSchema {
    pub(crate) raw: RawEnumSchema,
    pub(crate) proto: node::Reader<'static>,
}

impl EnumSchema {
    pub fn new(raw: RawEnumSchema) -> Self {
        let proto = crate::any_pointer::Reader::new(unsafe {
            layout::PointerReader::get_root_unchecked(raw.encoded_node.as_ptr() as *const u8)
        })
        .get_as()
        .unwrap();
        Self { raw, proto }
    }

    pub fn get_proto(self) -> node::Reader<'static> {
        self.proto
    }

    pub fn get_enumerants(self) -> crate::Result<EnumerantList> {
        if let node::Enum(s) = self.proto.which()? {
            Ok(EnumerantList {
                enumerants: s.get_enumerants()?,
                parent: self,
            })
        } else {
            panic!()
        }
    }

    pub fn get_annotations(self) -> Result<AnnotationList> {
        Ok(AnnotationList {
            annotations: self.proto.get_annotations()?,
            child_index: None,
            get_annotation_type: self.raw.annotation_types,
        })
    }
}

impl From<RawEnumSchema> for EnumSchema {
    fn from(re: RawEnumSchema) -> EnumSchema {
        EnumSchema::new(re)
    }
}

/// An enumerant, with generics applied. (Generics may affect types of annotations.)
#[derive(Clone, Copy)]
pub struct Enumerant {
    ordinal: u16,
    parent: EnumSchema,
    proto: enumerant::Reader<'static>,
}

impl Enumerant {
    pub fn get_containing_enum(self) -> EnumSchema {
        self.parent
    }

    pub fn get_ordinal(self) -> u16 {
        self.ordinal
    }

    pub fn get_proto(self) -> enumerant::Reader<'static> {
        self.proto
    }

    pub fn get_annotations(self) -> Result<AnnotationList> {
        Ok(AnnotationList {
            annotations: self.proto.get_annotations()?,
            child_index: Some(self.ordinal),
            get_annotation_type: self.parent.raw.annotation_types,
        })
    }
}

/// A list of enumerants.
#[derive(Clone, Copy)]
pub struct EnumerantList {
    enumerants: struct_list::Reader<'static, enumerant::Owned>,
    parent: EnumSchema,
}

impl EnumerantList {
    pub fn len(&self) -> u16 {
        self.enumerants.len() as u16
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(self, ordinal: u16) -> Enumerant {
        Enumerant {
            proto: self.enumerants.get(ordinal as u32),
            ordinal,
            parent: self.parent,
        }
    }

    pub fn iter(self) -> ShortListIter<Self, Enumerant> {
        ShortListIter::new(self, self.len())
    }
}

impl IndexMove<u16, Enumerant> for EnumerantList {
    fn index_move(&self, index: u16) -> Enumerant {
        self.get(index)
    }
}

impl ::core::iter::IntoIterator for EnumerantList {
    type Item = Enumerant;
    type IntoIter = ShortListIter<Self, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// An annotation.
#[derive(Clone, Copy)]
pub struct Annotation {
    proto: annotation::Reader<'static>,
    ty: introspect::Type,
}

impl Annotation {
    /// Gets the value held in this annotation.
    pub fn get_value(self) -> Result<dynamic_value::Reader<'static>> {
        dynamic_value::Reader::new(self.proto.get_value()?, self.ty)
    }

    /// Gets the ID of the annotation node.
    pub fn get_id(&self) -> u64 {
        self.proto.get_id()
    }

    /// Gets the type of the value held in this annotation.
    pub fn get_type(&self) -> introspect::Type {
        self.ty
    }
}

/// A list of annotations.
#[derive(Clone, Copy)]
pub struct AnnotationList {
    annotations: struct_list::Reader<'static, annotation::Owned>,
    child_index: Option<u16>,
    get_annotation_type: fn(Option<u16>, u32) -> introspect::Type,
}

impl AnnotationList {
    pub fn len(&self) -> u32 {
        self.annotations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(self, index: u32) -> Annotation {
        let proto = self.annotations.get(index);
        let ty = (self.get_annotation_type)(self.child_index, index);
        Annotation { proto, ty }
    }

    /// Returns the first annotation in the list that matches `id`.
    /// Otherwise returns `None`.
    pub fn find(self, id: u64) -> Option<Annotation> {
        self.iter().find(|&annotation| annotation.get_id() == id)
    }

    pub fn iter(self) -> ListIter<Self, Annotation> {
        ListIter::new(self, self.len())
    }
}

impl IndexMove<u32, Annotation> for AnnotationList {
    fn index_move(&self, index: u32) -> Annotation {
        self.get(index)
    }
}

impl ::core::iter::IntoIterator for AnnotationList {
    type Item = Annotation;
    type IntoIter = ListIter<Self, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
