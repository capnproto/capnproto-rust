use super::data::{base64, hex};
use super::json_capnp;
use super::{DataEncoding, EncodingOptions};

pub fn serialize_json_to<'reader, W>(
    writer: &mut W,
    reader: impl Into<capnp::dynamic_value::Reader<'reader>>,
) -> capnp::Result<()>
where
    W: std::io::Write,
{
    let meta = EncodingOptions {
        prefix: &std::borrow::Cow::Borrowed(""),
        name: "",
        flatten: None,
        discriminator: None,
        data_encoding: DataEncoding::Default,
    };
    serialize_value_to(writer, reader.into(), &meta, &mut true)
}

fn serialize_value_to<W>(
    writer: &mut W,
    reader: capnp::dynamic_value::Reader<'_>,
    meta: &EncodingOptions<'_, '_>,
    first: &mut bool,
) -> capnp::Result<()>
where
    W: std::io::Write,
{
    match reader {
        capnp::dynamic_value::Reader::Void => write!(writer, "null").map_err(|e| e.into()),
        capnp::dynamic_value::Reader::Bool(value) => if value {
            write!(writer, "true")
        } else {
            write!(writer, "false")
        }
        .map_err(|e| e.into()),
        capnp::dynamic_value::Reader::Int8(value) => write_signed_number(writer, value as i64),
        capnp::dynamic_value::Reader::Int16(value) => write_signed_number(writer, value as i64),
        capnp::dynamic_value::Reader::Int32(value) => write_signed_number(writer, value as i64),
        capnp::dynamic_value::Reader::Int64(value) => write_signed_number(writer, value),
        capnp::dynamic_value::Reader::UInt8(value) => write_unsigned_number(writer, value as u64),
        capnp::dynamic_value::Reader::UInt16(value) => write_unsigned_number(writer, value as u64),
        capnp::dynamic_value::Reader::UInt32(value) => write_unsigned_number(writer, value as u64),
        capnp::dynamic_value::Reader::UInt64(value) => write_unsigned_number(writer, value),
        capnp::dynamic_value::Reader::Float32(value) => write_float_number(writer, value as f64),
        capnp::dynamic_value::Reader::Float64(value) => write_float_number(writer, value),
        capnp::dynamic_value::Reader::Enum(value) => {
            if let Some(enumerant) = value.get_enumerant()? {
                let value = enumerant
                    .get_annotations()?
                    .iter()
                    .find(|a| a.get_id() == json_capnp::name::ID)
                    .and_then(|a| {
                        a.get_value()
                            .ok()
                            .map(|v| v.downcast::<capnp::text::Reader>().to_str())
                    })
                    .unwrap_or(enumerant.get_proto().get_name()?.to_str());
                write_string(writer, value?)
            } else {
                write_unsigned_number(writer, value.get_value() as u64)
            }
        }
        capnp::dynamic_value::Reader::Text(reader) => write_string(writer, reader.to_str()?),
        capnp::dynamic_value::Reader::Data(data) => write_data(writer, data, meta.data_encoding),
        capnp::dynamic_value::Reader::Struct(reader) => write_object(writer, reader, meta, first),
        capnp::dynamic_value::Reader::List(reader) => write_array(writer, reader.iter(), meta),
        capnp::dynamic_value::Reader::AnyPointer(_) => Err(capnp::Error::unimplemented(
            "AnyPointer cannot be represented in JSON".into(),
        )),
        capnp::dynamic_value::Reader::Capability(_) => Err(capnp::Error::unimplemented(
            "Capability cannot be represented in JSON".into(),
        )),
    }
}

// TODO: use capnp::io::Write ?
fn write_unsigned_number<W: std::io::Write>(writer: &mut W, value: u64) -> capnp::Result<()> {
    write!(writer, "{value}")?;
    Ok(())
}
fn write_signed_number<W: std::io::Write>(writer: &mut W, value: i64) -> capnp::Result<()> {
    write!(writer, "{value}")?;
    Ok(())
}

fn write_float_number<W: std::io::Write>(writer: &mut W, value: f64) -> capnp::Result<()> {
    // From the C++ codec comments:
    // Inf, -inf and NaN are not allowed in the JSON spec. Storing into string.

    if value.is_finite() {
        write!(writer, "{value}")?;
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

fn write_string<W: std::io::Write>(writer: &mut W, value: &str) -> capnp::Result<()> {
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
            c => write!(writer, "{c}")?,
        }
    }
    write!(writer, "\"")?;
    Ok(())
}

fn write_array<'reader, W: std::io::Write, I>(
    writer: &mut W,
    items: I,
    meta: &EncodingOptions,
) -> capnp::Result<()>
where
    I: Iterator<Item = capnp::Result<capnp::dynamic_value::Reader<'reader>>>,
{
    write!(writer, "[")?;
    let mut first = true;
    for item in items {
        if !first {
            write!(writer, ",")?;
        }
        first = false;
        serialize_value_to(writer, item?, meta, &mut true)?;
    }
    write!(writer, "]")?;
    Ok(())
}

fn write_object<'reader, W: std::io::Write>(
    writer: &mut W,
    reader: capnp::dynamic_struct::Reader<'reader>,
    meta: &EncodingOptions<'_, '_>,
    first: &mut bool,
) -> capnp::Result<()> {
    let (flatten, field_prefix) = if let Some(flatten_options) = &meta.flatten {
        (
            true,
            std::borrow::Cow::Owned(format!(
                "{}{}",
                meta.prefix,
                flatten_options.get_prefix()?.to_str()?
            )),
        )
    } else {
        (false, std::borrow::Cow::Borrowed(""))
    };

    let mut my_first = true;

    let first = if !flatten {
        write!(writer, "{{")?;
        &mut my_first
    } else {
        first
    };
    for field in reader.get_schema().get_non_union_fields()? {
        if !reader.has(field)? {
            continue;
        }
        let field_meta = EncodingOptions::from_field(&field_prefix, &field)?;
        if field_meta.flatten.is_none() {
            if !*first {
                write!(writer, ",")?;
            }
            *first = false;
            write_string(
                writer,
                format!("{}{}", field_prefix, field_meta.name).as_str(),
            )?;
            write!(writer, ":")?;
        }
        let field_value = reader.get(field)?;
        serialize_value_to(writer, field_value, &field_meta, first)?;
    }

    // Comment copied verbatim from the Cap'n Proto C++ implementation:
    // There are two cases of unions:
    // * Named unions, which are special cases of named groups. In this case, the union may be
    //   annotated by annotating the field. In this case, we receive a non-null `discriminator`
    //   as a constructor parameter, and schemaProto.getAnnotations() must be empty because
    //   it's not possible to annotate a group's type (because the type is anonymous).
    // * Unnamed unions, of which there can only be one in any particular scope. In this case,
    //   the parent struct type itself is annotated.
    // So if we received `null` as the constructor parameter, check for annotations on the struct
    // type.
    let struct_discriminator = reader
        .get_schema()
        .get_annotations()?
        .iter()
        .find(|a| a.get_id() == json_capnp::discriminator::ID)
        .and_then(|annotation| {
            annotation
                .get_value()
                .ok()
                .map(|v| v.downcast_struct::<json_capnp::discriminator_options::Owned>())
        });
    let discriminator = meta.discriminator.or(struct_discriminator);

    if let Some(active_union_member) = reader.which()? {
        let active_union_member_meta =
            EncodingOptions::from_field(&field_prefix, &active_union_member)?;
        if reader.has(active_union_member)? {
            let mut value_name = active_union_member_meta.name;
            let mut suppress_void = false;
            if let Some(discriminator) = discriminator {
                let discriminator_name = if discriminator.has_name() {
                    Some(discriminator.get_name()?.to_str()?)
                } else if flatten {
                    Some(meta.name)
                } else {
                    // https://github.com/capnproto/capnproto/issues/2461
                    // The discriminator is not output even if the annoyation is
                    // present if:
                    //  - it doesn't have an explicit name, and
                    //  - the group is _not_ being flattened.
                    None
                };
                if discriminator.has_value_name() {
                    value_name = discriminator.get_value_name()?.to_str()?;
                }

                if let Some(discriminator_name) = discriminator_name {
                    if !*first {
                        write!(writer, ",")?;
                    }
                    *first = false;
                    suppress_void = true;
                    write_string(
                        writer,
                        format!("{field_prefix}{discriminator_name}").as_str(),
                    )?;
                    write!(writer, ":")?;
                    write_string(writer, active_union_member_meta.name)?;
                }
            }
            let field_value = reader.get(active_union_member)?;
            if !suppress_void || !matches!(field_value, capnp::dynamic_value::Reader::Void) {
                if active_union_member_meta.flatten.is_none() {
                    if !*first {
                        write!(writer, ",")?;
                    }
                    *first = false;
                    write_string(writer, format!("{field_prefix}{value_name}").as_str())?;
                    write!(writer, ":")?;
                }
                serialize_value_to(writer, field_value, &active_union_member_meta, first)?;
            }
        }
    }
    if !flatten {
        write!(writer, "}}")?;
    }
    Ok(())
}

fn write_data<W: std::io::Write>(
    writer: &mut W,
    data: capnp::data::Reader<'_>,
    encoding: DataEncoding,
) -> capnp::Result<()> {
    match encoding {
        DataEncoding::Default => {
            write!(writer, "[")?;
            let mut first = true;
            for byte in data.iter() {
                if !first {
                    write!(writer, ",")?;
                }
                first = false;
                write!(writer, "{byte}")?;
            }
            write!(writer, "]")?;
            Ok(())
        }
        DataEncoding::Base64 => write_string(writer, base64::encode(data).as_str()),
        DataEncoding::Hex => write_string(writer, hex::encode(data).as_str()),
    }
}
