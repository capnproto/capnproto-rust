// TODO: Turn these write_* into an encoding trait that can be implemented for
// different Reader types, in particular for particualr Reader types?
// In particular, encoding Data as an array of bytes is compatible with upstream
// encoder, but still dumb as bricks, probably.
//
// e.g.
// impl ToJson for crate::dynamic_value::Reader<'_> { ... }
// impl ToJson for mycrate::my_capnp::my_struct::Reader<'_> { ... } // more specific
//
// does that work in rust without specdialization?
//

// TODO: Support for discriminators

pub fn to_json(reader: crate::dynamic_value::Reader<'_>) -> crate::Result<String> {
    let mut writer = std::io::Cursor::new(Vec::with_capacity(4096));
    serialize_json_to(&mut writer, reader)?;
    String::from_utf8(writer.into_inner()).map_err(|e| e.into())
}

pub fn serialize_json_to<W>(
    writer: &mut W,
    reader: crate::dynamic_value::Reader<'_>,
) -> crate::Result<()>
where
    W: std::io::Write,
{
    let meta = EncodingOptions {
        name: "",
        flatten: None,
        discriminator: None,
        data_encoding: DataEncoding::Default,
    };
    serialize_value_to(writer, reader, &meta)
}

use crate::json_capnp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DataEncoding {
    #[default]
    Default,
    Base64,
    Hex,
}

#[derive(Debug)]
struct EncodingOptions<'schema> {
    name: &'schema str,
    flatten: Option<json_capnp::flatten_options::Reader<'schema>>,
    discriminator: Option<json_capnp::discriminator_options::Reader<'schema>>,
    data_encoding: DataEncoding,
}

impl<'schema> EncodingOptions<'schema> {
    fn from_field(field: &crate::schema::Field) -> crate::Result<Self> {
        let field_name = match field
            .get_annotations()?
            .iter()
            .find(|a| a.get_id() == json_capnp::name::ID)
        {
            Some(name_annotation) => name_annotation
                .get_value()?
                .downcast::<crate::text::Reader>()
                .to_str()?,
            None => field.get_proto().get_name()?.to_str()?,
        };
        let data_encoding = match field
            .get_annotations()?
            .iter()
            .find(|a| a.get_id() == json_capnp::base64::ID)
        {
            Some(_) => DataEncoding::Base64,
            None => match field
                .get_annotations()?
                .iter()
                .find(|a| a.get_id() == json_capnp::hex::ID)
            {
                Some(_) => DataEncoding::Hex,
                None => DataEncoding::Default,
            },
        };
        if data_encoding != DataEncoding::Default {
            let element_type = if let crate::introspect::TypeVariant::List(element_type) =
                field.get_type().which()
            {
                element_type.which()
            } else {
                field.get_type().which()
            };
            if !matches!(element_type, crate::introspect::TypeVariant::Data) {
                return Err(crate::Error::failed(
                    "base64/hex annotation can only be applied to Data fields".into(),
                ));
            }
        }
        let flatten_options = match field
            .get_annotations()?
            .iter()
            .find(|a| a.get_id() == json_capnp::flatten::ID)
        {
            Some(annotation) => Some(
                annotation
                    .get_value()?
                    .downcast_struct::<json_capnp::flatten_options::Owned>(),
            ),
            None => None,
        };
        let discriminator_options = match field
            .get_annotations()?
            .iter()
            .find(|a| a.get_id() == json_capnp::discriminator::ID)
        {
            Some(annotation) => Some(
                annotation
                    .get_value()?
                    .downcast_struct::<json_capnp::discriminator_options::Owned>(),
            ),
            None => None,
        };
        Ok(Self {
            name: field_name,
            flatten: flatten_options,
            discriminator: discriminator_options,
            data_encoding,
        })
    }
}

fn serialize_value_to<W>(
    writer: &mut W,
    reader: crate::dynamic_value::Reader<'_>,
    meta: &EncodingOptions<'_>,
) -> crate::Result<()>
where
    W: std::io::Write,
{
    match reader {
        crate::dynamic_value::Reader::Void => write!(writer, "null").map_err(|e| e.into()),
        crate::dynamic_value::Reader::Bool(value) => if value {
            write!(writer, "true")
        } else {
            write!(writer, "false")
        }
        .map_err(|e| e.into()),
        crate::dynamic_value::Reader::Int8(value) => write_signed_number(writer, value as i64),
        crate::dynamic_value::Reader::Int16(value) => write_signed_number(writer, value as i64),
        crate::dynamic_value::Reader::Int32(value) => write_signed_number(writer, value as i64),
        crate::dynamic_value::Reader::Int64(value) => write_signed_number(writer, value),
        crate::dynamic_value::Reader::UInt8(value) => write_unsigned_number(writer, value as u64),
        crate::dynamic_value::Reader::UInt16(value) => write_unsigned_number(writer, value as u64),
        crate::dynamic_value::Reader::UInt32(value) => write_unsigned_number(writer, value as u64),
        crate::dynamic_value::Reader::UInt64(value) => write_unsigned_number(writer, value),
        crate::dynamic_value::Reader::Float32(value) => write_float_number(writer, value as f64),
        crate::dynamic_value::Reader::Float64(value) => write_float_number(writer, value),
        crate::dynamic_value::Reader::Enum(value) => {
            if let Some(enumerant) = value.get_enumerant()? {
                write_string(writer, enumerant.get_proto().get_name()?.to_str()?)
            } else {
                write_unsigned_number(writer, value.get_value() as u64)
            }
        }
        crate::dynamic_value::Reader::Text(reader) => write_string(writer, reader.to_str()?),
        crate::dynamic_value::Reader::Data(data) => write_data(writer, data, meta.data_encoding),
        crate::dynamic_value::Reader::Struct(reader) => write_object(writer, reader, meta),
        crate::dynamic_value::Reader::List(reader) => write_array(writer, reader.iter(), meta),
        crate::dynamic_value::Reader::AnyPointer(_) => Err(crate::Error::unimplemented(
            "AnyPointer cannot be represented in JSON".into(),
        )),
        crate::dynamic_value::Reader::Capability(_) => Err(crate::Error::unimplemented(
            "Capability cannot be represented in JSON".into(),
        )),
    }
}

// TODO: use crate::io::Write ?
fn write_unsigned_number<W: std::io::Write>(writer: &mut W, value: u64) -> crate::Result<()> {
    write!(writer, "{}", value)?;
    Ok(())
}
fn write_signed_number<W: std::io::Write>(writer: &mut W, value: i64) -> crate::Result<()> {
    write!(writer, "{}", value)?;
    Ok(())
}

fn write_float_number<W: std::io::Write>(writer: &mut W, value: f64) -> crate::Result<()> {
    if value.is_finite() {
        write!(writer, "{}", value)?;
    } else if value.is_nan() {
        write_string(writer, "NaN")?;
    } else if value.is_infinite() {
        if value.is_sign_positive() {
            write_string(writer, "Infinity")?;
        } else {
            write_string(writer, "-Infinity")?;
        }
    }
    Ok(())
}

fn write_string<W: std::io::Write>(writer: &mut W, value: &str) -> crate::Result<()> {
    write!(writer, "\"")?;
    for c in value.chars() {
        match c {
            '\"' => write!(writer, "\\\"")?,
            '\\' => write!(writer, "\\\\")?,
            '\n' => write!(writer, "\\n")?,
            '\r' => write!(writer, "\\r")?,
            '\t' => write!(writer, "\\t")?,
            '\u{08}' => write!(writer, "\\b")?,
            '\u{0C}' => write!(writer, "\\f")?,
            c if c.is_control() => write!(writer, "\\u{:04x}", c as u32)?,
            c => write!(writer, "{}", c)?,
        }
    }
    write!(writer, "\"")?;
    Ok(())
}

fn write_array<'reader, W: std::io::Write, I>(
    writer: &mut W,
    items: I,
    meta: &EncodingOptions,
) -> crate::Result<()>
where
    I: Iterator<Item = crate::Result<crate::dynamic_value::Reader<'reader>>>,
{
    write!(writer, "[")?;
    let mut first = true;
    for item in items {
        if !first {
            write!(writer, ",")?;
        }
        first = false;
        serialize_value_to(writer, item?, meta)?;
    }
    write!(writer, "]")?;
    Ok(())
}

fn write_object<'reader, W: std::io::Write>(
    writer: &mut W,
    reader: crate::dynamic_struct::Reader<'reader>,
    meta: &EncodingOptions<'_>,
) -> crate::Result<()> {
    let (flatten, field_prefix) = if let Some(flatten_options) = &meta.flatten {
        (true, flatten_options.get_prefix()?.to_str()?)
    } else {
        (false, "")
    };

    if !flatten {
        write!(writer, "{{")?;
    }
    let mut first = true;
    for field in reader.get_schema().get_fields()? {
        if !reader.has(field)? {
            continue;
        }
        if field.get_proto().get_discriminant_value() != crate::schema_capnp::field::NO_DISCRIMINANT
        {
            if let Some(active_union_member) = reader.which()? {
                if field.get_proto().get_discriminant_value()
                    != active_union_member.get_proto().get_discriminant_value()
                {
                    // Skip union members that are not set.
                    continue;
                }
                // TODO: there's a specific annotation 'discriminator' which says to
                // write out the discriminant field name as a field in the object,
                // allowing to use flatten on the union member (only for named union
                // fields).
            }
        }
        let meta = EncodingOptions::from_field(&field)?;
        if !first {
            write!(writer, ",")?;
        }
        first = false;
        if meta.flatten.is_none() {
            write_string(writer, format!("{}{}", field_prefix, meta.name).as_str())?;
            write!(writer, ":")?;
        }
        let field_value = reader.get(field)?;
        serialize_value_to(writer, field_value, &meta)?;
    }
    if !flatten {
        write!(writer, "}}")?;
    }
    Ok(())
}

fn write_data<W: std::io::Write>(
    writer: &mut W,
    data: crate::data::Reader<'_>,
    encoding: DataEncoding,
) -> crate::Result<()> {
    match encoding {
        DataEncoding::Default => {
            write!(writer, "[")?;
            let mut first = true;
            for byte in data.iter() {
                if !first {
                    write!(writer, ",")?;
                }
                first = false;
                write!(writer, "{}", byte)?;
            }
            write!(writer, "]")?;
            Ok(())
        }
        DataEncoding::Base64 => write_string(writer, encode_base64(data).as_str()),
        DataEncoding::Hex => write_string(writer, encode_hex(data).as_str()),
    }
}

fn encode_base64(data: &[u8]) -> String {
    // We don't want to pull in base64 crate just for this. So hand-rolling a
    // base64 encoder.
    const BASE64_CHARS: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        #[allow(clippy::get_first)]
        let b0 = chunk.get(0).copied().unwrap_or(0);
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        let c0 = BASE64_CHARS[((n >> 18) & 0x3F) as usize];
        let c1 = BASE64_CHARS[((n >> 12) & 0x3F) as usize];
        let c2 = if chunk.len() > 1 {
            BASE64_CHARS[((n >> 6) & 0x3F) as usize]
        } else {
            b'='
        };
        let c3 = if chunk.len() > 2 {
            BASE64_CHARS[(n & 0x3F) as usize]
        } else {
            b'='
        };
        encoded.push(c0 as char);
        encoded.push(c1 as char);
        encoded.push(c2 as char);
        encoded.push(c3 as char);
    }
    encoded
}

fn encode_hex(data: &[u8]) -> String {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(data.len() * 2);
    for &byte in data {
        let high = HEX_CHARS[(byte >> 4) as usize];
        let low = HEX_CHARS[(byte & 0x0F) as usize];
        encoded.push(high as char);
        encoded.push(low as char);
    }
    encoded
}
