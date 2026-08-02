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
        let proto = crate::any_pointer::Reader::new(
            layout::PointerReader::get_root_from_arena(raw.generic.arena).unwrap(),
        )
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

    /// Looks up a field by name using binary search. Returns `None` if no matching field is found.
    pub fn find_field_by_name(&self, name: &str) -> Result<Option<Field>> {
        let fields = self.get_fields()?;
        let mut lower: usize = 0;
        let mut upper: usize = self.raw.generic.members_by_name.len();

        while lower < upper {
            let mid: usize = (lower + upper) / 2;
            let candidate_index = self.raw.generic.members_by_name[mid];
            let candidate_name = fields.get(candidate_index).get_proto().get_name()?;

            use core::cmp::Ordering;
            match (&name).partial_cmp(&candidate_name) {
                Some(Ordering::Equal) => return Ok(Some(fields.get(candidate_index))),
                Some(Ordering::Greater) => lower = mid + 1,
                Some(Ordering::Less) => upper = mid,
                None => unreachable!(),
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
            write!(error, "{name}");
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

impl ::core::cmp::PartialEq for StructSchema {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl ::core::cmp::Eq for StructSchema {}

impl ::core::hash::Hash for StructSchema {
    fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl ::core::fmt::Debug for StructSchema {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        // Two schemas with the same display name are unequal if their brandings
        // differ, so also include the type id.
        match self.proto.get_display_name().map(|n| n.to_str()) {
            Ok(Ok(name)) => write!(f, "StructSchema({name}, {:?})", self.raw.type_id),
            _ => write!(f, "StructSchema({:?})", self.raw),
        }
    }
}

/// A field of a struct, with generics applied.
#[derive(Clone, Copy)]
pub struct Field {
    proto: field::Reader<'static>,
    index: u16,
    ty: introspect::Type,
    pub(crate) parent: StructSchema,
}

impl Field {
    pub fn get_proto(self) -> field::Reader<'static> {
        self.proto
    }

    pub fn get_type(&self) -> introspect::Type {
        self.ty
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

impl ::core::cmp::PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        self.parent == other.parent && self.index == other.index
    }
}
impl ::core::cmp::Eq for Field {}
impl ::core::hash::Hash for Field {
    fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        self.parent.hash(state);
        self.index.hash(state);
    }
}

impl ::core::fmt::Debug for Field {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self.proto.get_name().map(|n| n.to_str()) {
            Ok(Ok(name)) => write!(f, "Field({name}, {:?})", self.parent),
            _ => write!(f, "Field(index {}, {:?})", self.index, self.parent),
        }
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
        self.fields.len().try_into().unwrap()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(self, index: u16) -> Field {
        Field {
            proto: self.fields.get(index as u32),
            index,
            ty: (self.parent.raw.field_types)(index),
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
        self.indices.len().try_into().unwrap()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(self, index: u16) -> Field {
        let index = self.indices[index as usize];
        Field {
            proto: self.fields.get(index as u32),
            index,
            ty: (self.parent.raw.field_types)(index),
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
        let proto = crate::any_pointer::Reader::new(
            layout::PointerReader::get_root_from_arena(raw.arena).unwrap(),
        )
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

impl ::core::cmp::PartialEq for EnumSchema {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl ::core::cmp::Eq for EnumSchema {}

impl ::core::hash::Hash for EnumSchema {
    fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl ::core::fmt::Debug for EnumSchema {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self.proto.get_display_name().map(|n| n.to_str()) {
            Ok(Ok(name)) => write!(f, "EnumSchema({name})"),
            _ => write!(f, "EnumSchema({:?})", self.raw),
        }
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

impl ::core::cmp::PartialEq for Enumerant {
    fn eq(&self, other: &Self) -> bool {
        self.parent == other.parent && self.ordinal == other.ordinal
    }
}
impl ::core::cmp::Eq for Enumerant {}
impl ::core::hash::Hash for Enumerant {
    fn hash<H: ::core::hash::Hasher>(&self, state: &mut H) {
        self.parent.hash(state);
        self.ordinal.hash(state);
    }
}

impl ::core::fmt::Debug for Enumerant {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self.proto.get_name().map(|n| n.to_str()) {
            Ok(Ok(name)) => write!(f, "Enumerant({name}, {:?})", self.parent),
            _ => write!(f, "Enumerant(ordinal {}, {:?})", self.ordinal, self.parent),
        }
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
        self.enumerants.len().try_into().unwrap()
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

#[cfg(test)]
mod tests {
    use crate::introspect::Introspect;

    #[cfg(feature = "std")]
    #[test]
    fn fields_can_be_hashed() {
        let crate::introspect::TypeVariant::Struct(struct_schema) =
            crate::schema_capnp::node::Owned::introspect().which()
        else {
            panic!("Expected a struct schema");
        };

        let struct_schema = crate::schema::StructSchema::new(struct_schema);

        let display_name = struct_schema.get_field_by_name("displayName").unwrap();
        let id = struct_schema.get_field_by_name("id").unwrap();

        let mut map = std::collections::HashMap::new();
        map.insert(display_name, 1);
        map.insert(id, 2);

        assert_eq!(map.get(&display_name), Some(&1));
        assert_eq!(map.get(&id), Some(&2));
        assert_eq!(
            map.get(&struct_schema.get_field_by_name("displayName").unwrap()),
            Some(&1)
        );
        assert_eq!(
            map.get(&struct_schema.get_field_by_name("id").unwrap()),
            Some(&2)
        );
    }

    #[test]
    fn fields_can_be_compared() {
        let crate::introspect::TypeVariant::Struct(struct_schema) =
            crate::schema_capnp::node::Owned::introspect().which()
        else {
            panic!("Expected a struct schema");
        };

        let struct_schema = crate::schema::StructSchema::new(struct_schema);

        let display_name = struct_schema.get_field_by_name("displayName").unwrap();
        let id = struct_schema.get_field_by_name("id").unwrap();

        assert_eq!(display_name, display_name);
        assert_eq!(
            display_name,
            struct_schema.get_field_by_name("displayName").unwrap()
        );
        assert_eq!(id, id);
        assert_eq!(id, struct_schema.get_field_by_name("id").unwrap());

        assert_ne!(display_name, id);
    }

    #[cfg(feature = "std")]
    #[test]
    fn schemas_can_be_hashed() {
        let node_schema = {
            let crate::introspect::TypeVariant::Struct(schema) =
                crate::schema_capnp::node::Owned::introspect().which()
            else {
                panic!("Expected a struct schema");
            };

            crate::schema::StructSchema::new(schema)
        };
        let cgr_schema = {
            let crate::introspect::TypeVariant::Struct(schema) =
                crate::schema_capnp::code_generator_request::Owned::introspect().which()
            else {
                panic!("Expected a struct schema");
            };
            crate::schema::StructSchema::new(schema)
        };

        let mut map = std::collections::HashMap::new();
        map.insert(node_schema, 1);
        map.insert(cgr_schema, 2);

        assert_eq!(map.get(&node_schema), Some(&1));
        assert_eq!(map.get(&cgr_schema), Some(&2));
    }

    #[test]
    fn schemas_can_be_compared() {
        let node_schema = {
            let crate::introspect::TypeVariant::Struct(schema) =
                crate::schema_capnp::node::Owned::introspect().which()
            else {
                panic!("Expected a struct schema");
            };

            crate::schema::StructSchema::new(schema)
        };
        let cgr_schema = {
            let crate::introspect::TypeVariant::Struct(schema) =
                crate::schema_capnp::code_generator_request::Owned::introspect().which()
            else {
                panic!("Expected a struct schema");
            };
            crate::schema::StructSchema::new(schema)
        };

        assert_eq!(node_schema, node_schema);
        assert_eq!(cgr_schema, cgr_schema);
        assert_ne!(node_schema, cgr_schema);
    }

    #[test]
    fn enum_schemas_can_be_compared() {
        let crate::introspect::TypeVariant::Enum(raw) =
            crate::schema_capnp::ElementSize::introspect().which()
        else {
            panic!("Expected an enum schema");
        };
        let schema = crate::schema::EnumSchema::new(raw);

        assert_eq!(schema, crate::schema::EnumSchema::new(raw));

        let enumerants = schema.get_enumerants().unwrap();
        assert_eq!(enumerants.get(0), enumerants.get(0));
        assert_ne!(enumerants.get(0), enumerants.get(1));
    }

    #[cfg(feature = "std")]
    #[test]
    fn enumerants_can_be_hashed() {
        let crate::introspect::TypeVariant::Enum(raw) =
            crate::schema_capnp::ElementSize::introspect().which()
        else {
            panic!("Expected an enum schema");
        };
        let schema = crate::schema::EnumSchema::new(raw);
        let enumerants = schema.get_enumerants().unwrap();

        let mut map = std::collections::HashMap::new();
        map.insert(enumerants.get(0), 0);
        map.insert(enumerants.get(1), 1);

        assert_eq!(map.get(&enumerants.get(0)), Some(&0));
        assert_eq!(map.get(&enumerants.get(1)), Some(&1));
    }

    #[test]
    fn type_variants_can_be_compared() {
        use crate::introspect::TypeVariant;

        assert_eq!(u32::introspect().which(), TypeVariant::UInt32);
        assert_ne!(u32::introspect().which(), TypeVariant::Int32);
        assert_eq!(
            crate::schema_capnp::node::Owned::introspect().which(),
            crate::schema_capnp::node::Owned::introspect().which()
        );
        assert_ne!(
            crate::schema_capnp::node::Owned::introspect().which(),
            crate::schema_capnp::code_generator_request::Owned::introspect().which()
        );
    }
}
