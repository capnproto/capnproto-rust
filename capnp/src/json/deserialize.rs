use serde_json::{Number, Value};

use crate::{
    dynamic_list, dynamic_struct, dynamic_value,
    introspect::{RawEnumSchema, Type, TypeVariant},
    schema::{EnumSchema, Field, StructSchema},
    Error, Result,
};

use super::{field_name, read_annots, DataFormat, JsonAnnots};

/// Deserialize a JSON object into a dynamic struct builder.
///
/// ```
/// # use crate::schema_capnp::value as my_capnp_struct;
/// # fn main() -> Box<dyn std::error::Error + Send + Sync + 'static> {
/// let json = serde_json::from_str(r#"{ "foo": "bar" }"#)?;
/// let mut message = capnp::message::Builder::new_default();
/// let mut root = message.init_root::<my_capnp_struct::Builder>();
/// deserialize_into(root.into(), &json);
/// # }
/// ```
pub fn deserialize_into(builder: dynamic_value::Builder, json: &Value) -> Result<()> {
    let dynamic_value::Builder::Struct(builder) = builder else {
        return Err(Error::failed("expected builder to be a struct".into()));
    };
    let Value::Object(json) = json else {
        return Err(Error::failed("expected json to be object".into()))
    };
    deserialize_struct(builder, json)
}

enum Place<'a> {
    StructField(dynamic_struct::Builder<'a>, Field),
    ListElement(dynamic_list::Builder<'a>, u32),
}

impl Place<'_> {
    fn set(&mut self, value: dynamic_value::Reader) -> Result<()> {
        match self {
            Place::StructField(builder, field) => builder.set(*field, value),
            Place::ListElement(builder, index) => builder.set(*index, value),
        }
    }

    fn builder(&mut self) -> Result<dynamic_value::Builder> {
        match self {
            Place::StructField(builder, field) => builder.reborrow().init(*field),
            Place::ListElement(builder, index) => builder.reborrow().get(*index),
        }
    }

    fn list_builder(&mut self, size: u32) -> Result<dynamic_value::Builder> {
        match self {
            Place::StructField(builder, field) => builder.reborrow().initn(*field, size),
            Place::ListElement(builder, index) => builder.reborrow().init(*index, size),
        }
    }
}

fn deserialize_value(
    mut place: Place,
    type_: Type,
    field: &JsonAnnots,
    value: &Value,
) -> Result<()> {
    match (type_.which(), value) {
        (TypeVariant::Void, Value::Null) => place.set(dynamic_value::Reader::Void),
        (TypeVariant::Bool, Value::Bool(value)) => place.set((*value).into()),
        (TypeVariant::Int8, Value::Number(value)) => place.set(i8_value(value)?),
        (TypeVariant::Int16, Value::Number(value)) => place.set(i16_value(value)?),
        (TypeVariant::Int32, Value::Number(value)) => place.set(i32_value(value)?),
        (TypeVariant::Int64, Value::Number(value)) => place.set(i64_value(value)?),
        (TypeVariant::UInt8, Value::Number(value)) => place.set(u8_value(value)?),
        (TypeVariant::UInt16, Value::Number(value)) => place.set(u16_value(value)?),
        (TypeVariant::UInt32, Value::Number(value)) => place.set(u32_value(value)?),
        (TypeVariant::UInt64, Value::Number(value)) => place.set(u64_value(value)?),
        (TypeVariant::Float32, Value::Number(value)) => place.set(f32_value(value)?),
        (TypeVariant::Float64, Value::Number(value)) => place.set(f64_value(value)?),
        (TypeVariant::Text, Value::String(value)) => place.set(value.as_str().into()),
        (TypeVariant::Data, Value::String(value)) => {
            let value = data_value(value, &field)?;
            place.set(value.as_slice().into())
        }
        (TypeVariant::Struct(_), Value::Object(value)) => {
            deserialize_struct(place.builder()?.downcast(), value)
        }
        (TypeVariant::AnyPointer, _) => Err(Error::unimplemented(
            "cannot desesrialize AnyPointer".into(),
        )),
        (TypeVariant::Capability, _) => Err(Error::unimplemented(
            "cannot desesrialize capability".into(),
        )),
        (TypeVariant::Enum(schema), Value::Number(value)) => {
            let Some(value) = value.as_u64() else {
                return Err(Error::failed(format!("expected numeric enum value to be an unsigned integer")));
            };
            let value: u16 = value
                .try_into()
                .map_err(|_| Error::failed("cannot convert numeric enum value to u16".into()))?;
            let value = dynamic_value::Enum::new(value, EnumSchema::new(schema));
            place.set(value.into())
        }
        (TypeVariant::Enum(schema), Value::String(value)) => {
            place.set(string_enumerant_value(schema, value)?)
        }
        (TypeVariant::List(_), Value::Array(value)) => deserialize_list(
            place.list_builder(value.len() as u32)?.downcast(),
            field,
            value,
        ),
        (type_, value) => {
            if let TypeVariant::Struct(schema) = type_ {
                let name = StructSchema::new(schema).proto.get_display_name()?;
                Err(Error::failed(format!(
                    "expected value of type {name}, got: {value}"
                )))
            } else {
                Err(Error::failed(format!(
                    "expected value of type {type_:?}, got: {value}"
                )))
            }
        }
    }
}

fn deserialize_struct(
    mut builder: dynamic_struct::Builder,
    value: &serde_json::Map<String, Value>,
) -> Result<()> {
    for field in builder.get_schema().get_fields()?.iter() {
        let annots = read_annots(field.get_annotations()?)?;
        let key = field_name(&field, &annots)?;

        let value = match value.get(key) {
            Some(Value::Null) | None => {
                builder.clear(field)?;
                continue;
            }
            Some(value) => value,
        };

        deserialize_value(
            Place::StructField(builder.reborrow(), field),
            field.get_type(),
            &annots,
            value,
        )?;
    }

    Ok(())
}

fn deserialize_list(
    mut builder: dynamic_list::Builder,
    field: &JsonAnnots,
    value: &[Value],
) -> Result<()> {
    let type_ = builder.element_type();
    for (i, value) in value.iter().enumerate() {
        deserialize_value(
            Place::ListElement(builder.reborrow(), i as u32),
            type_,
            field,
            value,
        )?;
    }
    Ok(())
}

macro_rules! numerics {
    ($($fn:ident $a:ident -> $b:ident -> $c:ident),+,) => {
        $(
            fn $fn(value: &Number) -> Result<dynamic_value::Reader> {
                let Some(value) = value.$a() else {
                    return Err(Error::failed(format!("{} failed", stringify!($a))));
                };
                let value: $b = value.try_into().map_err(|_| {
                    Error::failed(format!("cannot convert to {}", stringify!($b)))
                })?;
                Ok(dynamic_value::Reader::$c(value))
            }
        )+
    }
}

numerics! {
    i8_value as_i64 -> i8 -> Int8,
    i16_value as_i64 -> i16 -> Int16,
    i32_value as_i64 -> i32 -> Int32,
    i64_value as_i64 -> i64 -> Int64,
    u8_value as_u64 -> u8 -> UInt8,
    u16_value as_u64 -> u16 -> UInt16,
    u32_value as_u64 -> u32 -> UInt32,
    u64_value as_u64 -> u64 -> UInt64,
    f64_value as_f64 -> f64 -> Float64,
}

fn f32_value(value: &Number) -> Result<dynamic_value::Reader> {
    let Some(value) = value.as_f64() else {
        return Err(Error::failed(format!("expected unsigned integer, got {value:?}")));
    };
    Ok(dynamic_value::Reader::Float32(value as f32))
}

fn data_value(value: &str, annots: &JsonAnnots) -> Result<Vec<u8>> {
    match annots.data_format {
        Some(DataFormat::Hex) => {
            hex::decode(value).map_err(|err| Error::failed(format!("invalid hex data: {err}")))
        }
        Some(DataFormat::Base64) => {
            use base64::{engine::general_purpose, Engine as _};
            general_purpose::STANDARD
                .decode(value)
                .map_err(|err| Error::failed(format!("invalid base64 data: {err}")))
        }
        None => Err(Error::failed(
            "cannot deserialize data field without Jason.hex or Json.base64 annotation".into(),
        )),
    }
}

fn string_enumerant_value(schema: RawEnumSchema, value: &str) -> Result<dynamic_value::Reader> {
    let schema = EnumSchema::new(schema);
    for possible_enumerant in schema.get_enumerants()?.iter() {
        let annots = read_annots(possible_enumerant.get_annotations()?)?;
        let possible_name = if let Some(name) = annots.name {
            name
        } else {
            possible_enumerant.get_proto().get_name()?
        };

        if value == possible_name {
            return Ok(dynamic_value::Reader::Enum(dynamic_value::Enum::new(
                possible_enumerant.get_ordinal(),
                schema,
            )));
        }
    }
    Err(Error::failed(format!("unrecognized enumerant {value}")))
}
