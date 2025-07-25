//! Traits and types to support run-time type introspection, i.e. reflection.

use crate::private::layout::ElementSize;
use crate::schema::{EnumSchema, StructSchema};

/// A type that supports reflection. All types that can appear in a Cap'n Proto message
/// implement this trait.
pub trait Introspect {
    /// Retrieves a description of the type.
    fn introspect() -> Type;
}

/// A description of a Cap'n Proto type. The representation is
/// optimized to avoid heap allocation.
///
/// To examine a `Type`, you should call the `which()` method.
#[derive(Copy, Clone, Debug)]
pub struct Type {
    /// The type, minus any outer `List( )`.
    base: BaseType,

    /// How many times `base` is wrapped in `List( )`.
    list_count: usize,
}

impl Type {
    /// Constructs a new `Type` that is not a list.
    fn new_base(base: BaseType) -> Self {
        Self {
            base,
            list_count: 0,
        }
    }

    /// Constructs a new `Type` that is a list wrapping some other `Type`.
    pub fn list_of(mut element_type: Type) -> Self {
        element_type.list_count += 1;
        element_type
    }

    /// Unfolds a single layer of the `Type`, to allow for pattern matching.
    pub fn which(&self) -> TypeVariant {
        if self.list_count > 0 {
            TypeVariant::List(Type {
                base: self.base,
                list_count: self.list_count - 1,
            })
        } else {
            match self.base {
                BaseType::Void => TypeVariant::Void,
                BaseType::Bool => TypeVariant::Bool,
                BaseType::Int8 => TypeVariant::Int8,
                BaseType::Int16 => TypeVariant::Int16,
                BaseType::Int32 => TypeVariant::Int32,
                BaseType::Int64 => TypeVariant::Int64,
                BaseType::UInt8 => TypeVariant::UInt8,
                BaseType::UInt16 => TypeVariant::UInt16,
                BaseType::UInt32 => TypeVariant::UInt32,
                BaseType::UInt64 => TypeVariant::UInt64,
                BaseType::Float32 => TypeVariant::Float32,
                BaseType::Float64 => TypeVariant::Float64,
                BaseType::Text => TypeVariant::Text,
                BaseType::Data => TypeVariant::Data,
                BaseType::Enum(re) => TypeVariant::Enum(re),
                BaseType::Struct(rs) => TypeVariant::Struct(rs),
                BaseType::AnyPointer => TypeVariant::AnyPointer,
                BaseType::Capability => TypeVariant::Capability,
            }
        }
    }

    /// If this type T appears as List(T), then what is the expected
    /// element size of the list?
    pub(crate) fn expected_element_size(&self) -> ElementSize {
        if self.list_count > 0 {
            ElementSize::Pointer
        } else {
            match self.base {
                BaseType::Void => ElementSize::Void,
                BaseType::Bool => ElementSize::Bit,
                BaseType::Int8 | BaseType::UInt8 => ElementSize::Byte,
                BaseType::Int16 | BaseType::UInt16 | BaseType::Enum(_) => ElementSize::TwoBytes,
                BaseType::Int32 | BaseType::UInt32 | BaseType::Float32 => ElementSize::FourBytes,
                BaseType::Int64 | BaseType::UInt64 | BaseType::Float64 => ElementSize::EightBytes,
                BaseType::Text | BaseType::Data | BaseType::AnyPointer | BaseType::Capability => {
                    ElementSize::Pointer
                }
                BaseType::Struct(_) => ElementSize::InlineComposite,
            }
        }
    }

    /// Is the `Type` a pointer type?
    pub fn is_pointer_type(&self) -> bool {
        if self.list_count > 0 {
            true
        } else {
            matches!(
                self.base,
                BaseType::Text
                    | BaseType::Data
                    | BaseType::AnyPointer
                    | BaseType::Struct(_)
                    | BaseType::Capability
            )
        }
    }

    /// Returns true if `self` is equal to `other` modulo
    /// type parameters and interface types.
    pub fn loose_equals(&self, other: Self) -> bool {
        match (self.which(), other.which()) {
            (TypeVariant::Void, TypeVariant::Void) => true,
            (TypeVariant::UInt8, TypeVariant::UInt8) => true,
            (TypeVariant::UInt16, TypeVariant::UInt16) => true,
            (TypeVariant::UInt32, TypeVariant::UInt32) => true,
            (TypeVariant::UInt64, TypeVariant::UInt64) => true,
            (TypeVariant::Int8, TypeVariant::Int8) => true,
            (TypeVariant::Int16, TypeVariant::Int16) => true,
            (TypeVariant::Int32, TypeVariant::Int32) => true,
            (TypeVariant::Int64, TypeVariant::Int64) => true,
            (TypeVariant::Float32, TypeVariant::Float32) => true,
            (TypeVariant::Float64, TypeVariant::Float64) => true,
            (TypeVariant::Text, TypeVariant::Text) => true,
            (TypeVariant::Data, TypeVariant::Data) => true,
            (TypeVariant::Enum(es1), TypeVariant::Enum(es2)) => es1 == es2,
            (TypeVariant::Struct(rbs1), TypeVariant::Struct(rbs2)) => {
                // Ignore any type parameters. The original intent was that
                // we would additionally check that the `field_types` fields
                // were equal function pointers here. However, according to
                // Miri's behavior at least, that check returns `false`
                // more than we would like it to. So we settle for being
                // a bit more accepting.
                core::ptr::eq(rbs1.generic, rbs2.generic)
            }
            (TypeVariant::List(element1), TypeVariant::List(element2)) => {
                element1.loose_equals(element2)
            }
            (TypeVariant::AnyPointer, TypeVariant::AnyPointer) => true,
            (TypeVariant::Capability, TypeVariant::Capability) => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone)]
/// A `Type` unfolded one level. Suitable for pattern matching. Can be trivially
/// converted to `Type` via the `From`/`Into` traits.
pub enum TypeVariant {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Text,
    Data,
    Struct(RawBrandedStructSchema),
    AnyPointer,
    Capability,
    Enum(RawEnumSchema),
    List(Type),
}

impl From<TypeVariant> for Type {
    fn from(tv: TypeVariant) -> Type {
        match tv {
            TypeVariant::Void => Type::new_base(BaseType::Void),
            TypeVariant::Bool => Type::new_base(BaseType::Bool),
            TypeVariant::Int8 => Type::new_base(BaseType::Int8),
            TypeVariant::Int16 => Type::new_base(BaseType::Int16),
            TypeVariant::Int32 => Type::new_base(BaseType::Int32),
            TypeVariant::Int64 => Type::new_base(BaseType::Int64),
            TypeVariant::UInt8 => Type::new_base(BaseType::UInt8),
            TypeVariant::UInt16 => Type::new_base(BaseType::UInt16),
            TypeVariant::UInt32 => Type::new_base(BaseType::UInt32),
            TypeVariant::UInt64 => Type::new_base(BaseType::UInt64),
            TypeVariant::Float32 => Type::new_base(BaseType::Float32),
            TypeVariant::Float64 => Type::new_base(BaseType::Float64),
            TypeVariant::Text => Type::new_base(BaseType::Text),
            TypeVariant::Data => Type::new_base(BaseType::Data),
            TypeVariant::Struct(rbs) => Type::new_base(BaseType::Struct(rbs)),
            TypeVariant::AnyPointer => Type::new_base(BaseType::AnyPointer),
            TypeVariant::Capability => Type::new_base(BaseType::Capability),
            TypeVariant::Enum(es) => Type::new_base(BaseType::Enum(es)),
            TypeVariant::List(list) => Type::list_of(list),
        }
    }
}

/// A Cap'n Proto type, excluding `List`.
#[derive(Copy, Clone, Debug)]
enum BaseType {
    Void,
    Bool,
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    Text,
    Data,
    Struct(RawBrandedStructSchema),
    AnyPointer,
    Capability,
    Enum(RawEnumSchema),
}

macro_rules! primitive_introspect(
    ($t:ty, $v:ident) => (
        impl Introspect for $t {
            fn introspect() -> Type { Type::new_base(BaseType::$v) }
        }
    )
);

primitive_introspect!((), Void);
primitive_introspect!(bool, Bool);
primitive_introspect!(i8, Int8);
primitive_introspect!(i16, Int16);
primitive_introspect!(i32, Int32);
primitive_introspect!(i64, Int64);
primitive_introspect!(u8, UInt8);
primitive_introspect!(u16, UInt16);
primitive_introspect!(u32, UInt32);
primitive_introspect!(u64, UInt64);
primitive_introspect!(f32, Float32);
primitive_introspect!(f64, Float64);

/// Type information that gets included in the generated code for every
/// user-defined Cap'n Proto struct.
#[derive(Copy, Clone)]
pub struct RawStructSchema {
    /// The Node (as defined in schema.capnp), as a single segment message.
    pub encoded_node: &'static [crate::Word],

    /// Indices (not ordinals) of fields that don't have a discriminant value.
    pub nonunion_members: &'static [u16],

    /// Map from discriminant value to field index.
    pub members_by_discriminant: &'static [u16],

    /// Indices of fields, sorted by their respective names.
    pub members_by_name: &'static [u16],
}

/// A RawStructSchema with branding information, i.e. resolution of type parameters.
/// To use one of this, you will usually want to convert it to a `schema::StructSchema`,
/// which can be done via `into()`.
#[derive(Copy, Clone)]
pub struct RawBrandedStructSchema {
    /// The unbranded base schema.
    pub generic: &'static RawStructSchema,

    /// Map from field index (not ordinal) to Type.
    pub field_types: fn(u16) -> Type,

    /// Map from (maybe field index, annotation index) to the Type
    /// of the value held by that annotation.
    pub annotation_types: fn(Option<u16>, u32) -> Type,
}

impl core::fmt::Debug for RawBrandedStructSchema {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(
            f,
            "RawBrandedStructSchema({:?}, {:?})",
            self.generic as *const _, self.field_types as *const fn(u16) -> Type
        )
    }
}

impl From<StructSchema> for RawBrandedStructSchema {
    fn from(value: StructSchema) -> Self {
        value.raw
    }
}

/// Type information that gets included in the generated code for every
/// user-defined Cap'n Proto enum.
///
/// To use one of these, you will usually want to convert it to a `schema::EnumSchema`,
/// which can be done via `into()`.
#[derive(Clone, Copy)]
pub struct RawEnumSchema {
    /// The Node (as defined in schema.capnp), as a single segment message.
    pub encoded_node: &'static [crate::Word],

    /// Map from (maybe enumerant index, annotation index) to the Type
    /// of the value held by that annotation.
    pub annotation_types: fn(Option<u16>, u32) -> Type,
}

impl core::cmp::PartialEq for RawEnumSchema {
    fn eq(&self, other: &Self) -> bool {
        ::core::ptr::eq(self.encoded_node, other.encoded_node)
    }
}

impl core::cmp::Eq for RawEnumSchema {}

impl core::fmt::Debug for RawEnumSchema {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::result::Result<(), core::fmt::Error> {
        write!(f, "RawEnumSchema({:?})", self.encoded_node as *const _)
    }
}

impl From<EnumSchema> for RawEnumSchema {
    fn from(value: EnumSchema) -> Self {
        value.raw
    }
}

/**
Function intended to be called by generated `get_field_types()` methods.
Defined here so that we can use inline format args syntax, which did
not exist before Rust edition 2021. Not intended to be called directly by
end users.
 */
pub fn panic_invalid_field_index(index: u16) -> ! {
    panic!("invalid field index {index}")
}

/**
Function intended to be called by generated `get_annotation_types()` methods.
Defined here so that we can use inline format args syntax, which did
not exist before Rust edition 2021. Not intended to be called directly by
end users.
 */
pub fn panic_invalid_annotation_indices(child_index: Option<u16>, index: u32) -> ! {
    panic!("invalid annotation indices ({child_index:?}, {index})")
}
