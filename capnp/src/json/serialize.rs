use serde_json::Value;

use crate::{
    dynamic_list, dynamic_struct, dynamic_value,
    json::{field_name, read_annots, DataFormat, JsonAnnots},
    Error, Result,
};

/// Serialize a dynamic struct reader into a JSON object.
///
/// ```
/// # use crate::schema_capnp::value as my_capnp_struct;
/// # fn main() -> Box<dyn std::error::Error + Send + Sync + 'static> {
/// let mut message = capnp::message::Builder::new_default();
/// let mut root = message.init_root::<my_capnp_struct::Builder>();
/// root.set_text("hello world");
///
/// let json = serialize_value(root.into(), Default::default())?;
/// # }
/// ```
pub fn serialize(reader: dynamic_struct::Reader, opts: Opts) -> Result<Value> {
    serialize_struct(reader, &opts)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Opts {
    pub on_unsupported: OnUnsupported,
    pub on_enumerant_not_in_schema: OnEnumerantNotInSchema,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum OnUnsupported {
    #[default]
    Skip,
    Error,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum OnEnumerantNotInSchema {
    #[default]
    UseNumber,
    Skip,
    Error,
}

impl Opts {
    fn unsupported_result(&self, what: &str) -> Result<Option<Value>> {
        match self.on_unsupported {
            OnUnsupported::Skip => Ok(None),
            OnUnsupported::Error => Err(Error::unimplemented(format!(
                "cannot serialize {what} to json"
            ))),
        }
    }
}

// NOTE: serialize_foo return Ok(None) if we should skip

fn serialize_value(
    reader: dynamic_value::Reader,
    field: &JsonAnnots,
    opts: &Opts,
) -> Result<Option<Value>> {
    match reader {
        dynamic_value::Reader::Void => Ok(Some(Value::Null)),
        dynamic_value::Reader::Bool(v) => Ok(Some(v.into())),
        dynamic_value::Reader::Int8(v) => Ok(Some(v.into())),
        dynamic_value::Reader::Int16(v) => Ok(Some(v.into())),
        dynamic_value::Reader::Int32(v) => Ok(Some(v.into())),
        dynamic_value::Reader::Int64(v) => Ok(Some(v.into())),
        dynamic_value::Reader::UInt8(v) => Ok(Some(v.into())),
        dynamic_value::Reader::UInt16(v) => Ok(Some(v.into())),
        dynamic_value::Reader::UInt32(v) => Ok(Some(v.into())),
        dynamic_value::Reader::UInt64(v) => Ok(Some(v.into())),
        dynamic_value::Reader::Float32(v) => Ok(Some(v.into())),
        dynamic_value::Reader::Float64(v) => Ok(Some(v.into())),
        dynamic_value::Reader::Enum(v) => serialize_enum(v, opts),
        dynamic_value::Reader::Text(v) => Ok(Some(v.into())),
        dynamic_value::Reader::Data(v) => serialize_data(v, field, opts),
        dynamic_value::Reader::Struct(v) => serialize_struct(v, opts).map(Some),
        dynamic_value::Reader::List(v) => serialize_list(v, field, opts),
        dynamic_value::Reader::AnyPointer(_) => opts.unsupported_result("anypointer"),
        dynamic_value::Reader::Capability(_) => opts.unsupported_result("capability"),
    }
}

fn serialize_struct(reader: dynamic_struct::Reader, opts: &Opts) -> Result<Value> {
    let mut out = serde_json::Map::new();
    for field in reader.get_schema().get_fields()?.into_iter() {
        let annots = read_annots(field.get_annotations()?)?;

        let value = if reader.has(field)? {
            match serialize_value(reader.get(field)?, &annots, &opts)? {
                Some(value) => value,
                None => continue, // We should skip this field
            }
        } else {
            // This is a pointer field with the null value
            Value::Null
        };

        let key = field_name(&field, &annots)?.to_string();

        out.insert(key, value);
    }
    Ok(Value::Object(out))
}

fn serialize_data(data: &[u8], field: &JsonAnnots, opts: &Opts) -> Result<Option<Value>> {
    match field.data_format {
        Some(DataFormat::Hex) => Ok(Some(Value::String(hex::encode(data)))),
        Some(DataFormat::Base64) => {
            use base64::{engine::general_purpose, Engine as _};
            let value = general_purpose::STANDARD.encode(data);
            Ok(Some(Value::String(value)))
        }
        None => opts.unsupported_result("data field with Json.hex or Json.base64 annotation"),
    }
}

fn serialize_enum(value: dynamic_value::Enum, opts: &Opts) -> Result<Option<Value>> {
    if let Some(enumerant) = value.get_enumerant()? {
        let proto = enumerant.get_proto();
        let annots = read_annots(enumerant.get_annotations()?)?;
        if let Some(name) = annots.name {
            Ok(Some(Value::String(name.to_string())))
        } else {
            Ok(Some(Value::String(proto.get_name()?.to_string())))
        }
    } else {
        let value = value.get_value();
        match opts.on_enumerant_not_in_schema {
            OnEnumerantNotInSchema::UseNumber => Ok(Some(value.into())),
            OnEnumerantNotInSchema::Skip => Ok(None),
            OnEnumerantNotInSchema::Error => Err(Error::failed(format!(
                "enumerant not in schema: value={value}"
            ))),
        }
    }
}

fn serialize_list(
    reader: dynamic_list::Reader,
    field: &JsonAnnots,
    opts: &Opts,
) -> Result<Option<Value>> {
    let mut out = Vec::with_capacity(reader.len() as usize);
    for item in reader.iter() {
        match serialize_value(item?, field, opts)? {
            Some(value) => out.push(value),
            None => {
                // In this case we can't serialize the type and we're configured
                // to skip on unsupported.
                return Ok(None);
            }
        }
    }
    Ok(Some(Value::Array(out)))
}
