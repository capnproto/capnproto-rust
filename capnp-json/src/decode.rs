// Deserialisation
use super::data::{base64, hex};
use super::json_capnp;
use super::{DataEncoding, EncodingOptions};

enum ParseError {
    UnexpectedEndOfInput,
    InvalidToken(char),
    Other(String),
}

impl From<ParseError> for capnp::Error {
    fn from(err: ParseError) -> Self {
        match err {
            ParseError::UnexpectedEndOfInput => {
                capnp::Error::failed("Unexpected end of input while parsing JSON".into())
            }
            ParseError::InvalidToken(c) => {
                capnp::Error::failed(format!("Invalid token '{c}' while parsing JSON"))
            }
            // TODO: Use better values here?
            ParseError::Other(msg) => capnp::Error::failed(msg),
        }
    }
}

use std::collections::HashMap;

// FIXME: The String valued below could be Cow<'input, str> as they only really
// need to be allocated if the input contains escaped characters. That would be
// a little more tricky lower down, but not by a lot.
enum JsonValue {
    Null,
    Boolean(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),

    DataBuffer(Vec<u8>), // HACK: This is so we have somewhere to store the data
                         // temporarily when we are decoding data fields into
                         // Readers
}

struct Parser<I>
where
    I: Iterator<Item = char>,
{
    // FIXME: By using an iter over char here, we restrict ourselves to not
    // being able to use string slices for must of the parsing. THis is piggy.
    // It would be better to just have a &str and an index probably.
    input_iter: std::iter::Peekable<std::iter::Fuse<I>>,
}

impl<I> Parser<I>
where
    I: Iterator<Item = char>,
{
    fn new(iter: I) -> Self {
        Self {
            input_iter: iter.fuse().peekable(),
        }
    }

    /// Advance past any whitespace and peek at next value
    fn peek_next(&mut self) -> Option<char> {
        self.discard_whitespace();
        self.peek()
    }

    /// Peek at the current value
    fn peek(&mut self) -> Option<char> {
        self.input_iter.peek().copied()
    }

    /// Consume the current value
    fn advance(&mut self) -> capnp::Result<char> {
        self.input_iter
            .next()
            .ok_or(ParseError::UnexpectedEndOfInput.into())
    }

    /// Consume the current value if it matches `c`, otherwise error
    fn consume(&mut self, c: char) -> capnp::Result<char> {
        match self.advance()? {
            p if p == c => Ok(p),
            p => Err(ParseError::InvalidToken(p).into()),
        }
    }

    /// Advance past any whitespace and consume the current value if it matches `c`, otherwise error
    fn consume_next(&mut self, c: char) -> capnp::Result<char> {
        self.discard_whitespace();
        match self.advance()? {
            p if p == c => Ok(p),
            p => Err(ParseError::InvalidToken(p).into()),
        }
    }

    fn discard_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance().ok();
            } else {
                break;
            }
        }
    }

    fn parse_value(&mut self) -> capnp::Result<JsonValue> {
        match self.peek_next() {
            None => Err(ParseError::UnexpectedEndOfInput.into()),
            Some('n') => {
                self.advance()?;
                self.consume('u')?;
                self.consume('l')?;
                self.consume('l')?;
                Ok(JsonValue::Null)
            }
            Some('t') => {
                self.advance()?;
                self.consume('r')?;
                self.consume('u')?;
                self.consume('e')?;
                Ok(JsonValue::Boolean(true))
            }
            Some('f') => {
                self.advance()?;
                self.consume('a')?;
                self.consume('l')?;
                self.consume('s')?;
                self.consume('e')?;
                Ok(JsonValue::Boolean(false))
            }
            Some('\"') => Ok(JsonValue::String(self.parse_string()?)),
            Some('0'..='9') | Some('-') => {
                let num_str = self.parse_number()?;
                let num = num_str
                    .parse::<f64>()
                    .map_err(|e| ParseError::Other(format!("Invalid number format: {}", e)))?;
                Ok(JsonValue::Number(num))
            }
            Some('[') => {
                self.advance()?;
                let mut items = Vec::new();
                let mut require_comma = false;
                while self.peek_next().is_some_and(|c| c != ']') {
                    if require_comma {
                        self.consume(',')?;
                    }
                    require_comma = true;
                    let item = self.parse_value()?;
                    items.push(item);
                }
                self.consume_next(']')?;
                Ok(JsonValue::Array(items))
            }
            Some('{') => {
                self.advance()?;
                let mut members = HashMap::new();
                let mut require_comma = false;
                while self.peek_next().is_some_and(|c| c != '}') {
                    if require_comma {
                        self.consume(',')?;
                    }
                    require_comma = true;
                    let key = self.parse_string()?;
                    self.consume_next(':')?;
                    let value = self.parse_value()?;
                    if members.insert(key.clone(), value).is_some() {
                        return Err(
                            ParseError::Other(format!("Duplicate key in object: {}", key)).into(),
                        );
                    }
                }
                self.consume_next('}')?;
                Ok(JsonValue::Object(members))
            }
            Some(c) => Err(ParseError::InvalidToken(c).into()),
        }
    }

    fn parse_string(&mut self) -> capnp::Result<String> {
        self.consume_next('\"')?;
        let mut result = String::new();
        loop {
            let c = self.advance()?;
            match c {
                '\"' => return Ok(result),
                '\\' => {
                    let escaped = self.advance()?;
                    match escaped {
                        '\"' => result.push('\"'),
                        '\\' => result.push('\\'),
                        '/' => result.push('/'),
                        'b' => result.push('\u{08}'),
                        'f' => result.push('\u{0C}'),
                        'n' => result.push('\n'),
                        'r' => result.push('\r'),
                        't' => result.push('\t'),
                        'u' => {
                            let mut hex = String::new();
                            for _ in 0..4 {
                                hex.push(self.advance()?);
                            }
                            let code_point = u16::from_str_radix(&hex, 16).map_err(|_| {
                                ParseError::Other(format!("Invalid unicode escape: \\u{}", hex))
                            })?;
                            if let Some(ch) = std::char::from_u32(code_point as u32) {
                                result.push(ch);
                            } else {
                                return Err(ParseError::Other(format!(
                                    "Invalid unicode code point: \\u{}",
                                    hex
                                ))
                                .into());
                            }
                        }
                        other => {
                            return Err(ParseError::Other(format!(
                                "Invalid escape character: \\{}",
                                other
                            ))
                            .into());
                        }
                    }
                }
                other => result.push(other),
            }
        }
    }

    fn parse_number(&mut self) -> capnp::Result<String> {
        let mut num_str = String::new();
        if self.peek_next().is_some_and(|c| c == '-') {
            num_str.push(self.advance()?);
        }
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            num_str.push(self.advance()?);
        }
        if self.peek().is_some_and(|c| c == '.') {
            num_str.push(self.advance()?);
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                num_str.push(self.advance()?);
            }
        }
        if self.peek().is_some_and(|c| c == 'e' || c == 'E') {
            num_str.push(self.advance()?);
            if self.peek().is_some_and(|c| c == '+' || c == '-') {
                num_str.push(self.advance()?);
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                num_str.push(self.advance()?);
            }
        }
        Ok(num_str)
    }
}

pub fn parse(json: &str, builder: capnp::dynamic_struct::Builder<'_>) -> capnp::Result<()> {
    let mut parser = Parser::new(json.chars());
    let value = parser.parse_value()?;
    let meta = EncodingOptions {
        prefix: &std::borrow::Cow::Borrowed(""),
        name: "",
        flatten: None,
        discriminator: None,
        data_encoding: DataEncoding::Default,
    };
    let JsonValue::Object(mut value) = value else {
        return Err(capnp::Error::failed(
            "Top-level JSON value must be an object".into(),
        ));
    };
    decode_struct(&mut value, builder, &meta)
}

fn decode_primitive<'json, 'meta>(
    field_value: &'json mut JsonValue,
    field_type: &'meta capnp::introspect::Type,
    field_meta: &'meta EncodingOptions,
) -> capnp::Result<capnp::dynamic_value::Reader<'json>> {
    match field_type.which() {
        capnp::introspect::TypeVariant::Void => {
            if !matches!(field_value, JsonValue::Null) {
                Err(capnp::Error::failed(format!(
                    "Expected null for void field {}",
                    field_meta.name
                )))
            } else {
                Ok(capnp::dynamic_value::Reader::Void)
            }
        }
        capnp::introspect::TypeVariant::Bool => {
            let JsonValue::Boolean(field_value) = field_value else {
                return Err(capnp::Error::failed(format!(
                    "Expected boolean for field {}",
                    field_meta.name
                )));
            };
            Ok((*field_value).into())
        }
        capnp::introspect::TypeVariant::Int8 => {
            let JsonValue::Number(field_value) = field_value else {
                return Err(capnp::Error::failed(format!(
                    "Expected number for field {}",
                    field_meta.name
                )));
            };
            Ok((*field_value as i8).into())
        }
        capnp::introspect::TypeVariant::Int16 => {
            let JsonValue::Number(field_value) = field_value else {
                return Err(capnp::Error::failed(format!(
                    "Expected number for field {}",
                    field_meta.name
                )));
            };
            Ok((*field_value as i16).into())
        }
        capnp::introspect::TypeVariant::Int32 => {
            let JsonValue::Number(field_value) = field_value else {
                return Err(capnp::Error::failed(format!(
                    "Expected number for field {}",
                    field_meta.name
                )));
            };
            Ok((*field_value as i32).into())
        }
        capnp::introspect::TypeVariant::Int64 => {
            let JsonValue::Number(field_value) = field_value else {
                return Err(capnp::Error::failed(format!(
                    "Expected number for field {}",
                    field_meta.name
                )));
            };
            Ok((*field_value as i64).into())
        }
        capnp::introspect::TypeVariant::UInt8 => {
            let JsonValue::Number(field_value) = field_value else {
                return Err(capnp::Error::failed(format!(
                    "Expected number for field {}",
                    field_meta.name
                )));
            };
            Ok((*field_value as u8).into())
        }
        capnp::introspect::TypeVariant::UInt16 => {
            let JsonValue::Number(field_value) = field_value else {
                return Err(capnp::Error::failed(format!(
                    "Expected number for field {}",
                    field_meta.name
                )));
            };
            Ok((*field_value as u16).into())
        }
        capnp::introspect::TypeVariant::UInt32 => {
            let JsonValue::Number(field_value) = field_value else {
                return Err(capnp::Error::failed(format!(
                    "Expected number for field {}",
                    field_meta.name
                )));
            };
            Ok((*field_value as u32).into())
        }
        capnp::introspect::TypeVariant::UInt64 => {
            let JsonValue::Number(field_value) = field_value else {
                return Err(capnp::Error::failed(format!(
                    "Expected number for field {}",
                    field_meta.name
                )));
            };
            Ok((*field_value as u64).into())
        }
        capnp::introspect::TypeVariant::Float32 => {
            let field_value = match field_value {
                JsonValue::Number(field_value) => *field_value as f32,
                JsonValue::String(field_value) => match field_value.as_str() {
                    "NaN" => f32::NAN,
                    "Infinity" => f32::INFINITY,
                    "-Infinity" => f32::NEG_INFINITY,
                    _ => {
                        return Err(capnp::Error::failed(format!(
                            "Expected number for field {}",
                            field_meta.name
                        )));
                    }
                },
                _ => {
                    return Err(capnp::Error::failed(format!(
                        "Expected number for field {}",
                        field_meta.name
                    )));
                }
            };
            Ok(field_value.into())
        }
        capnp::introspect::TypeVariant::Float64 => {
            let field_value = match field_value {
                JsonValue::Number(field_value) => *field_value,
                JsonValue::String(field_value) => match field_value.as_str() {
                    "NaN" => f64::NAN,
                    "Infinity" => f64::INFINITY,
                    "-Infinity" => f64::NEG_INFINITY,
                    _ => {
                        return Err(capnp::Error::failed(format!(
                            "Expected number for field {}",
                            field_meta.name
                        )));
                    }
                },
                _ => {
                    return Err(capnp::Error::failed(format!(
                        "Expected number for field {}",
                        field_meta.name
                    )));
                }
            };
            Ok(field_value.into())
        }
        capnp::introspect::TypeVariant::Text => {
            let JsonValue::String(field_value) = field_value else {
                return Err(capnp::Error::failed(format!(
                    "Expected string for field {}",
                    field_meta.name
                )));
            };
            Ok((*field_value.as_str()).into())
        }
        capnp::introspect::TypeVariant::Enum(enum_schema) => match field_value {
            JsonValue::String(field_value) => {
                let enum_schema = capnp::schema::EnumSchema::new(enum_schema);
                let Some(enum_value) = enum_schema.get_enumerants()?.iter().find(|e| {
                    e.get_proto()
                        .get_name()
                        .ok()
                        .and_then(|n| n.to_str().ok())
                        .is_some_and(|s| s == field_value)
                }) else {
                    return Err(capnp::Error::failed(format!(
                        "Invalid enum value '{}' for field {}",
                        field_value, field_meta.name
                    )));
                };

                Ok(capnp::dynamic_value::Reader::Enum(
                    capnp::dynamic_value::Enum::new(
                        enum_value.get_ordinal(),
                        enum_value.get_containing_enum(),
                    ),
                ))
            }
            JsonValue::Number(enum_value) => {
                let enum_schema = capnp::schema::EnumSchema::new(enum_schema);
                Ok(capnp::dynamic_value::Reader::Enum(
                    capnp::dynamic_value::Enum::new(*enum_value as u16, enum_schema),
                ))
            }
            _ => Err(capnp::Error::failed(format!(
                "Expected string or number for enum field {}",
                field_meta.name
            ))),
        },
        capnp::introspect::TypeVariant::Data => match field_meta.data_encoding {
            // The reason we have this ugly DataBuffer hack is to ensure that we
            // can return a Reader from this function whose lifetime is tied to
            // the field_value, as there is no other buffer we can use. We don't
            // currently support Orphans, but if we did, most of this Reader
            // dance could probably be avoided.
            DataEncoding::Default => {
                let JsonValue::Array(data_value) = field_value else {
                    return Err(capnp::Error::failed(format!(
                        "Expected array for data field {}",
                        field_meta.name
                    )));
                };
                let mut data = Vec::with_capacity(data_value.len());
                for byte_value in data_value.drain(..) {
                    let JsonValue::Number(byte_value) = byte_value else {
                        return Err(capnp::Error::failed(format!(
                            "Expected number for data byte in field {}",
                            field_meta.name
                        )));
                    };
                    data.push(byte_value as u8);
                }
                *field_value = JsonValue::DataBuffer(data);
                Ok(capnp::dynamic_value::Reader::Data(match field_value {
                    JsonValue::DataBuffer(ref data) => data.as_slice(),
                    _ => unreachable!(),
                }))
            }
            DataEncoding::Base64 => {
                let JsonValue::String(data_value) = field_value else {
                    return Err(capnp::Error::failed(format!(
                        "Expected string for base64 data field {}",
                        field_meta.name
                    )));
                };
                *field_value = JsonValue::DataBuffer(base64::decode(data_value)?);
                Ok(capnp::dynamic_value::Reader::Data(match field_value {
                    JsonValue::DataBuffer(ref data) => data.as_slice(),
                    _ => unreachable!(),
                }))
            }
            DataEncoding::Hex => {
                let JsonValue::String(data_value) = field_value else {
                    return Err(capnp::Error::failed(format!(
                        "Expected string for hex data field {}",
                        field_meta.name
                    )));
                };
                *field_value = JsonValue::DataBuffer(hex::decode(data_value)?);
                Ok(capnp::dynamic_value::Reader::Data(match field_value {
                    JsonValue::DataBuffer(ref data) => data.as_slice(),
                    _ => unreachable!(),
                }))
            }
        },
        _ => Err(capnp::Error::failed(format!(
            "Unsupported primitive type for field {}",
            field_meta.name
        ))),
    }
}

fn decode_list(
    mut field_values: Vec<JsonValue>,
    mut list_builder: capnp::dynamic_list::Builder,
    field_meta: &EncodingOptions,
) -> capnp::Result<()> {
    match list_builder.element_type().which() {
        capnp::introspect::TypeVariant::Struct(_sub_element_schema) => {
            for (i, item_value) in field_values.drain(..).enumerate() {
                let JsonValue::Object(mut item_value) = item_value else {
                    return Err(capnp::Error::failed(format!(
                        "Expected object for struct list field {}",
                        field_meta.name
                    )));
                };
                let struct_builder = list_builder
                    .reborrow()
                    .get(i as u32)?
                    .downcast::<capnp::dynamic_struct::Builder>();
                decode_struct(&mut item_value, struct_builder, field_meta)?;
            }
            Ok(())
        }
        capnp::introspect::TypeVariant::List(_sub_element_type) => {
            for (i, item_value) in field_values.drain(..).enumerate() {
                let JsonValue::Array(item_value) = item_value else {
                    return Err(capnp::Error::failed(format!(
                        "Expected array for list field {}",
                        field_meta.name
                    )));
                };
                let sub_element_builder = list_builder
                    .reborrow()
                    .init(i as u32, item_value.len() as u32)?
                    .downcast::<capnp::dynamic_list::Builder>();
                decode_list(item_value, sub_element_builder, field_meta)?;
            }
            Ok(())
        }
        _ => {
            for (i, mut item_value) in field_values.drain(..).enumerate() {
                list_builder.set(
                    i as u32,
                    decode_primitive(&mut item_value, &list_builder.element_type(), field_meta)?,
                )?;
            }
            Ok(())
        }
    }
}

fn decode_struct(
    value: &mut HashMap<String, JsonValue>,
    mut builder: capnp::dynamic_struct::Builder<'_>,
    meta: &EncodingOptions,
) -> capnp::Result<()> {
    let field_prefix = if let Some(flatten_options) = &meta.flatten {
        std::borrow::Cow::Owned(format!(
            "{}{}",
            meta.prefix,
            flatten_options.get_prefix()?.to_str()?
        ))
    } else {
        std::borrow::Cow::Borrowed("")
    };

    fn decode_member(
        mut builder: capnp::dynamic_struct::Builder<'_>,
        field: capnp::schema::Field,
        field_meta: &EncodingOptions,
        value: &mut HashMap<String, JsonValue>,
        value_name: &str,
    ) -> capnp::Result<()> {
        match field.get_type().which() {
            capnp::introspect::TypeVariant::Struct(_struct_schema) => {
                let struct_builder = builder
                    .reborrow()
                    .init(field)?
                    .downcast::<capnp::dynamic_struct::Builder>();
                if field_meta.flatten.is_none() {
                    let field_value = match value.remove(value_name) {
                        Some(v) => v,
                        None => return Ok(()),
                    };

                    let JsonValue::Object(mut field_value) = field_value else {
                        return Err(capnp::Error::failed(format!(
                            "Expected object for field {}",
                            field_meta.name
                        )));
                    };
                    decode_struct(&mut field_value, struct_builder, field_meta)?;
                } else {
                    // Flattened struct; pass the JsonValue at this level down
                    decode_struct(value, struct_builder, field_meta)?;
                }
            }
            capnp::introspect::TypeVariant::List(_element_type) => {
                let Some(field_value) = value.remove(value_name) else {
                    return Ok(());
                };

                let JsonValue::Array(field_value) = field_value else {
                    return Err(capnp::Error::failed(format!(
                        "Expected array for field {}",
                        field_meta.name
                    )));
                };
                let list_builder = builder
                    .reborrow()
                    .initn(field, field_value.len() as u32)?
                    .downcast::<capnp::dynamic_list::Builder>();
                decode_list(field_value, list_builder, field_meta)?;
            }

            capnp::introspect::TypeVariant::AnyPointer => {
                return Err(capnp::Error::unimplemented(
                    "AnyPointer cannot be represented in JSON".into(),
                ))
            }
            capnp::introspect::TypeVariant::Capability => {
                return Err(capnp::Error::unimplemented(
                    "Capability cannot be represented in JSON".into(),
                ))
            }

            _ => {
                let Some(mut field_value) = value.remove(value_name) else {
                    return Ok(());
                };

                builder.set(
                    field,
                    decode_primitive(&mut field_value, &field.get_type(), field_meta)?,
                )?;
            }
        }
        Ok(())
    }

    for field in builder.get_schema().get_non_union_fields()? {
        let field_meta = EncodingOptions::from_field(meta.prefix, &field)?;
        let field_name = format!("{}{}", field_prefix, field_meta.name);

        decode_member(builder.reborrow(), field, &field_meta, value, &field_name)?;
    }

    let struct_discriminator = builder
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

    // FIXME: refactor this to only loop through union memberes once; each
    // iteration check if it matches the discriminant, *or* the requisite
    // named field is present, then decode and break;
    let discriminant = match discriminator {
        Some(discriminator) => {
            let discriminator_name = if discriminator.has_name() {
                discriminator.get_name()?.to_str()?
            } else {
                meta.name
            };
            let field_name = format!("{}{}", field_prefix, discriminator_name);
            if let Some(JsonValue::String(discriminant)) = value.remove(&field_name) {
                Some(discriminant)
            } else {
                None
            }
        }
        None => {
            // find the first field that exists matching a union field?
            let mut discriminant = None;
            for field in builder.get_schema().get_union_fields()? {
                let field_meta = EncodingOptions::from_field(meta.prefix, &field)?;
                let field_name = format!("{}{}", field_prefix, field_meta.name);
                if value.contains_key(&field_name) {
                    discriminant = Some(field_meta.name.to_string());
                    break;
                }
            }
            discriminant
        }
    };
    if let Some(discriminant) = discriminant {
        for field in builder.get_schema().get_union_fields()? {
            let field_meta = EncodingOptions::from_field(meta.prefix, &field)?;
            if field_meta.name != discriminant {
                continue;
            }
            let value_name = if let Some(discriminator) = discriminator {
                if discriminator.has_value_name() {
                    discriminator.get_value_name()?.to_str()?
                } else {
                    field_meta.name
                }
            } else {
                field_meta.name
            };
            decode_member(builder.reborrow(), field, &field_meta, value, value_name)?;
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_parse_string() -> capnp::Result<()> {
        let json = r#""Hello, World!""#;

        let mut parser = Parser::new(json.chars());
        let value = parser.parse_value()?;

        assert!(matches!(value, JsonValue::String(s) if s == "Hello, World!"));
        Ok(())
    }

    #[test]
    fn test_parse_string_with_special_chars() -> capnp::Result<()> {
        let json = r#""Hełło,\nWorld!\"†ęś†: \u0007""#;

        let mut parser = Parser::new(json.chars());
        let value = parser.parse_value()?;

        assert!(matches!(value, JsonValue::String(s) if s == "Hełło,\nWorld!\"†ęś†: \u{0007}"));

        let json =
            r#"{"value":"tab: \t, newline: \n, carriage return: \r, quote: \", backslash: \\"}"#;
        let mut parser = Parser::new(json.chars());
        let value = parser.parse_value()?;
        let JsonValue::Object(map) = value else {
            panic!("Expected object at top level");
        };
        let Some(JsonValue::String(s)) = map.get("value") else {
            panic!("Expected string value for 'value' key");
        };
        assert_eq!(
            s,
            "tab: \t, newline: \n, carriage return: \r, quote: \", backslash: \\"
        );
        Ok(())
    }
}
