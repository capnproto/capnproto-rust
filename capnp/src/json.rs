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

pub fn to_json<'reader>(
    reader: impl Into<crate::dynamic_value::Reader<'reader>>,
) -> crate::Result<String> {
    let mut writer = std::io::Cursor::new(Vec::with_capacity(4096));
    encode::serialize_json_to(&mut writer, reader)?;
    String::from_utf8(writer.into_inner()).map_err(|e| e.into())
}

pub fn from_json<'segments>(
    json: &str,
    builder: impl Into<crate::dynamic_value::Builder<'segments>>,
) -> crate::Result<()> {
    let crate::dynamic_value::Builder::Struct(builder) = builder.into() else {
        return Err(crate::Error::failed(
            "Top-level JSON value must be an object".into(),
        ));
    };
    decode::parse(json, builder)
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
struct EncodingOptions<'schema, 'prefix> {
    prefix: &'prefix std::borrow::Cow<'schema, str>,
    name: &'schema str,
    flatten: Option<json_capnp::flatten_options::Reader<'schema>>,
    discriminator: Option<json_capnp::discriminator_options::Reader<'schema>>,
    data_encoding: DataEncoding,
}

impl<'schema, 'prefix> EncodingOptions<'schema, 'prefix> {
    fn from_field(
        prefix: &'prefix std::borrow::Cow<'schema, str>,
        field: &crate::schema::Field,
    ) -> crate::Result<Self> {
        let mut options = Self {
            prefix,
            name: field.get_proto().get_name()?.to_str()?,
            flatten: None,
            discriminator: None,
            data_encoding: DataEncoding::Default,
        };

        for anno in field.get_annotations()?.iter() {
            match anno.get_id() {
                json_capnp::name::ID => {
                    options.name = anno
                        .get_value()?
                        .downcast::<crate::text::Reader>()
                        .to_str()?;
                }
                json_capnp::base64::ID => {
                    if options.data_encoding != DataEncoding::Default {
                        return Err(crate::Error::failed(
                            "Cannot specify both base64 and hex annotations on the same field"
                                .into(),
                        ));
                    }
                    options.data_encoding = DataEncoding::Base64;
                }
                json_capnp::hex::ID => {
                    if options.data_encoding != DataEncoding::Default {
                        return Err(crate::Error::failed(
                            "Cannot specify both base64 and hex annotations on the same field"
                                .into(),
                        ));
                    }
                    options.data_encoding = DataEncoding::Hex;
                }
                json_capnp::flatten::ID => {
                    options.flatten = Some(
                        anno.get_value()?
                            .downcast_struct::<json_capnp::flatten_options::Owned>(),
                    );
                }
                json_capnp::discriminator::ID => {
                    options.discriminator = Some(
                        anno.get_value()?
                            .downcast_struct::<json_capnp::discriminator_options::Owned>(),
                    );
                }
                _ => {}
            }
        }
        if options.data_encoding != DataEncoding::Default {
            let mut element_type = field.get_type();
            while let crate::introspect::TypeVariant::List(sub_element_type) = element_type.which()
            {
                element_type = sub_element_type;
            }
            if !matches!(element_type.which(), crate::introspect::TypeVariant::Data) {
                return Err(crate::Error::failed(
                    "base64/hex annotation can only be applied to Data fields".into(),
                ));
            }
        }
        Ok(options)
    }
}

// Serialisation

pub mod encode {
    use super::*;

    pub fn serialize_json_to<'reader, W>(
        writer: &mut W,
        reader: impl Into<crate::dynamic_value::Reader<'reader>>,
    ) -> crate::Result<()>
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
        serialize_value_to(writer, reader.into(), &meta)
    }

    fn serialize_value_to<W>(
        writer: &mut W,
        reader: crate::dynamic_value::Reader<'_>,
        meta: &EncodingOptions<'_, '_>,
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
            crate::dynamic_value::Reader::UInt8(value) => {
                write_unsigned_number(writer, value as u64)
            }
            crate::dynamic_value::Reader::UInt16(value) => {
                write_unsigned_number(writer, value as u64)
            }
            crate::dynamic_value::Reader::UInt32(value) => {
                write_unsigned_number(writer, value as u64)
            }
            crate::dynamic_value::Reader::UInt64(value) => write_unsigned_number(writer, value),
            crate::dynamic_value::Reader::Float32(value) => {
                write_float_number(writer, value as f64)
            }
            crate::dynamic_value::Reader::Float64(value) => write_float_number(writer, value),
            crate::dynamic_value::Reader::Enum(value) => {
                if let Some(enumerant) = value.get_enumerant()? {
                    let value = enumerant
                        .get_annotations()?
                        .iter()
                        .find(|a| a.get_id() == json_capnp::name::ID)
                        .and_then(|a| {
                            a.get_value()
                                .ok()
                                .map(|v| v.downcast::<crate::text::Reader>().to_str())
                        })
                        .unwrap_or(enumerant.get_proto().get_name()?.to_str());
                    write_string(writer, value?)
                } else {
                    write_unsigned_number(writer, value.get_value() as u64)
                }
            }
            crate::dynamic_value::Reader::Text(reader) => write_string(writer, reader.to_str()?),
            crate::dynamic_value::Reader::Data(data) => {
                write_data(writer, data, meta.data_encoding)
            }
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
        // From the C++ codec comments:
        // Inf, -inf and NaN are not allowed in the JSON spec. Storing into string.

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
        meta: &EncodingOptions<'_, '_>,
    ) -> crate::Result<()> {
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

        if !flatten {
            write!(writer, "{{")?;
        }
        let mut first = true;
        for field in reader.get_schema().get_fields()? {
            if !reader.has(field)? {
                continue;
            }
            let field_meta = EncodingOptions::from_field(&field_prefix, &field)?;
            let mut value_name = field_meta.name;
            if field.get_proto().get_discriminant_value()
                != crate::schema_capnp::field::NO_DISCRIMINANT
            {
                if let Some(active_union_member) = reader.which()? {
                    let active_union_member_meta =
                        EncodingOptions::from_field(&field_prefix, &active_union_member)?;
                    if field.get_proto().get_discriminant_value()
                        != active_union_member.get_proto().get_discriminant_value()
                    {
                        // Skip union members that are not set.
                        continue;
                    }
                    let discriminator = match meta.discriminator {
                        Some(ref d) => Some(d),
                        None => struct_discriminator.as_ref(),
                    };
                    if let Some(discriminator) = discriminator {
                        // write out the discriminator
                        if !first {
                            write!(writer, ",")?;
                        }
                        first = false;
                        let discriminator_name = if discriminator.has_name() {
                            discriminator.get_name()?.to_str()?
                        } else {
                            meta.name
                        };
                        if discriminator.has_value_name() {
                            value_name = discriminator.get_value_name()?.to_str()?;
                        }

                        write_string(
                            writer,
                            format!("{}{}", field_prefix, discriminator_name).as_str(),
                        )?;
                        write!(writer, ":")?;
                        write_string(writer, active_union_member_meta.name)?;
                    }
                }
            }
            if !first {
                write!(writer, ",")?;
            }
            first = false;
            if field_meta.flatten.is_none() {
                write_string(writer, format!("{}{}", field_prefix, value_name).as_str())?;
                write!(writer, ":")?;
            }
            let field_value = reader.get(field)?;
            serialize_value_to(writer, field_value, &field_meta)?;
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
            DataEncoding::Base64 => write_string(writer, base64::encode(data).as_str()),
            DataEncoding::Hex => write_string(writer, hex::encode(data).as_str()),
        }
    }
}

// Deserialisation

pub mod decode {
    use super::*;

    enum ParseError {
        UnexpectedEndOfInput,
        InvalidToken(char),
        Other(String),
    }

    impl From<ParseError> for crate::Error {
        fn from(err: ParseError) -> Self {
            match err {
                ParseError::UnexpectedEndOfInput => {
                    crate::Error::failed("Unexpected end of input while parsing JSON".into())
                }
                ParseError::InvalidToken(c) => {
                    crate::Error::failed(format!("Invalid token '{c}' while parsing JSON"))
                }
                // TODO: Use better values here?
                ParseError::Other(msg) => crate::Error::failed(msg),
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

        fn advance(&mut self) -> crate::Result<char> {
            self.input_iter
                .next()
                .ok_or(ParseError::UnexpectedEndOfInput.into())
        }

        fn peek(&mut self) -> Option<char> {
            self.input_iter.peek().copied()
        }

        fn consume(&mut self, c: char) -> crate::Result<char> {
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

        fn discard_peek(&mut self) -> Option<char> {
            self.discard_whitespace();
            self.peek()
        }

        fn parse_value(&mut self) -> crate::Result<JsonValue> {
            match self.discard_peek() {
                None => Err(ParseError::UnexpectedEndOfInput.into()),
                Some('n') => {
                    self.consume('n')?;
                    self.consume('u')?;
                    self.consume('l')?;
                    self.consume('l')?;
                    Ok(JsonValue::Null)
                }
                Some('t') => {
                    self.consume('t')?;
                    self.consume('r')?;
                    self.consume('u')?;
                    self.consume('e')?;
                    Ok(JsonValue::Boolean(true))
                }
                Some('f') => {
                    self.consume('f')?;
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
                    self.consume('[')?;
                    let mut items = Vec::new();
                    let mut require_comma = false;
                    while self.discard_peek().is_some_and(|c| c != ']') {
                        if require_comma {
                            self.consume(',')?;
                        }
                        require_comma = true;
                        let item = self.parse_value()?;
                        items.push(item);
                    }
                    self.consume(']')?;
                    Ok(JsonValue::Array(items))
                }
                Some('{') => {
                    self.consume('{')?;
                    let mut members = HashMap::new();
                    let mut require_comma = false;
                    while self.discard_peek().is_some_and(|c| c != '}') {
                        if require_comma {
                            self.consume(',')?;
                        }
                        require_comma = true;
                        let key = self.parse_string()?;
                        self.consume(':')?;
                        let value = self.parse_value()?;
                        if members.insert(key.clone(), value).is_some() {
                            return Err(ParseError::Other(format!(
                                "Duplicate key in object: {}",
                                key
                            ))
                            .into());
                        }
                    }
                    self.consume('}')?;
                    Ok(JsonValue::Object(members))
                }
                Some(c) => Err(ParseError::InvalidToken(c).into()),
            }
        }

        fn parse_string(&mut self) -> crate::Result<String> {
            self.consume('\"')?;
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

        fn parse_number(&mut self) -> crate::Result<String> {
            let mut num_str = String::new();
            if self.discard_peek().is_some_and(|c| c == '-') {
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

    pub fn parse(json: &str, builder: crate::dynamic_struct::Builder<'_>) -> crate::Result<()> {
        let mut parser = Parser::new(json.chars());
        let value = parser.parse_value()?;
        let meta = EncodingOptions {
            prefix: &std::borrow::Cow::Borrowed(""),
            name: "",
            flatten: None,
            discriminator: None,
            data_encoding: DataEncoding::Default,
        };
        let JsonValue::Object(value) = value else {
            return Err(crate::Error::failed(
                "Top-level JSON value must be an object".into(),
            ));
        };
        decode_struct(value, builder, &meta)
    }

    fn decode_primitive<'json, 'meta>(
        field_value: &'json mut JsonValue,
        field_type: &'meta crate::introspect::Type,
        field_meta: &'meta EncodingOptions,
    ) -> crate::Result<crate::dynamic_value::Reader<'json>> {
        match field_type.which() {
            crate::introspect::TypeVariant::Void => {
                if !matches!(field_value, JsonValue::Null) {
                    Err(crate::Error::failed(format!(
                        "Expected null for void field {}",
                        field_meta.name
                    )))
                } else {
                    Ok(crate::dynamic_value::Reader::Void)
                }
            }
            crate::introspect::TypeVariant::Bool => {
                let JsonValue::Boolean(field_value) = field_value else {
                    return Err(crate::Error::failed(format!(
                        "Expected boolean for field {}",
                        field_meta.name
                    )));
                };
                Ok((*field_value).into())
            }
            crate::introspect::TypeVariant::Int8 => {
                let JsonValue::Number(field_value) = field_value else {
                    return Err(crate::Error::failed(format!(
                        "Expected number for field {}",
                        field_meta.name
                    )));
                };
                Ok((*field_value as i8).into())
            }
            crate::introspect::TypeVariant::Int16 => {
                let JsonValue::Number(field_value) = field_value else {
                    return Err(crate::Error::failed(format!(
                        "Expected number for field {}",
                        field_meta.name
                    )));
                };
                Ok((*field_value as i16).into())
            }
            crate::introspect::TypeVariant::Int32 => {
                let JsonValue::Number(field_value) = field_value else {
                    return Err(crate::Error::failed(format!(
                        "Expected number for field {}",
                        field_meta.name
                    )));
                };
                Ok((*field_value as i32).into())
            }
            crate::introspect::TypeVariant::Int64 => {
                let JsonValue::Number(field_value) = field_value else {
                    return Err(crate::Error::failed(format!(
                        "Expected number for field {}",
                        field_meta.name
                    )));
                };
                Ok((*field_value as i64).into())
            }
            crate::introspect::TypeVariant::UInt8 => {
                let JsonValue::Number(field_value) = field_value else {
                    return Err(crate::Error::failed(format!(
                        "Expected number for field {}",
                        field_meta.name
                    )));
                };
                Ok((*field_value as u8).into())
            }
            crate::introspect::TypeVariant::UInt16 => {
                let JsonValue::Number(field_value) = field_value else {
                    return Err(crate::Error::failed(format!(
                        "Expected number for field {}",
                        field_meta.name
                    )));
                };
                Ok((*field_value as u16).into())
            }
            crate::introspect::TypeVariant::UInt32 => {
                let JsonValue::Number(field_value) = field_value else {
                    return Err(crate::Error::failed(format!(
                        "Expected number for field {}",
                        field_meta.name
                    )));
                };
                Ok((*field_value as u32).into())
            }
            crate::introspect::TypeVariant::UInt64 => {
                let JsonValue::Number(field_value) = field_value else {
                    return Err(crate::Error::failed(format!(
                        "Expected number for field {}",
                        field_meta.name
                    )));
                };
                Ok((*field_value as u64).into())
            }
            crate::introspect::TypeVariant::Float32 => {
                let field_value = match field_value {
                    JsonValue::Number(field_value) => *field_value as f32,
                    JsonValue::String(field_value) => match field_value.as_str() {
                        "NaN" => f32::NAN,
                        "Infinity" => f32::INFINITY,
                        "-Infinity" => f32::NEG_INFINITY,
                        _ => {
                            return Err(crate::Error::failed(format!(
                                "Expected number for field {}",
                                field_meta.name
                            )));
                        }
                    },
                    _ => {
                        return Err(crate::Error::failed(format!(
                            "Expected number for field {}",
                            field_meta.name
                        )));
                    }
                };
                Ok(field_value.into())
            }
            crate::introspect::TypeVariant::Float64 => {
                let field_value = match field_value {
                    JsonValue::Number(field_value) => *field_value,
                    JsonValue::String(field_value) => match field_value.as_str() {
                        "NaN" => f64::NAN,
                        "Infinity" => f64::INFINITY,
                        "-Infinity" => f64::NEG_INFINITY,
                        _ => {
                            return Err(crate::Error::failed(format!(
                                "Expected number for field {}",
                                field_meta.name
                            )));
                        }
                    },
                    _ => {
                        return Err(crate::Error::failed(format!(
                            "Expected number for field {}",
                            field_meta.name
                        )));
                    }
                };
                Ok(field_value.into())
            }
            crate::introspect::TypeVariant::Text => {
                let JsonValue::String(field_value) = field_value else {
                    return Err(crate::Error::failed(format!(
                        "Expected string for field {}",
                        field_meta.name
                    )));
                };
                Ok((*field_value.as_str()).into())
            }
            crate::introspect::TypeVariant::Enum(enum_schema) => match field_value {
                JsonValue::String(field_value) => {
                    let enum_schema = crate::schema::EnumSchema::new(enum_schema);
                    let Some(enum_value) = enum_schema.get_enumerants()?.iter().find(|e| {
                        e.get_proto()
                            .get_name()
                            .ok()
                            .and_then(|n| n.to_str().ok())
                            .is_some_and(|s| s == field_value)
                    }) else {
                        return Err(crate::Error::failed(format!(
                            "Invalid enum value '{}' for field {}",
                            field_value, field_meta.name
                        )));
                    };

                    Ok(crate::dynamic_value::Reader::Enum(
                        crate::dynamic_value::Enum::new(
                            enum_value.get_ordinal(),
                            enum_value.get_containing_enum(),
                        ),
                    ))
                }
                JsonValue::Number(enum_value) => {
                    let enum_schema = crate::schema::EnumSchema::new(enum_schema);
                    Ok(crate::dynamic_value::Reader::Enum(
                        crate::dynamic_value::Enum::new(*enum_value as u16, enum_schema),
                    ))
                }
                _ => Err(crate::Error::failed(format!(
                    "Expected string or number for enum field {}",
                    field_meta.name
                ))),
            },
            crate::introspect::TypeVariant::Data => match field_meta.data_encoding {
                // The reason we have this ugly DataBuffer hack is to ensure that we
                // can return a Reader from this function whose lifetime is tied to
                // the field_value, as there is no other buffer we can use. We don't
                // currently support Orphans, but if we did, most of this Reader
                // dance could probably be avoided.
                DataEncoding::Default => {
                    let JsonValue::Array(data_value) = field_value else {
                        return Err(crate::Error::failed(format!(
                            "Expected array for data field {}",
                            field_meta.name
                        )));
                    };
                    let mut data = Vec::with_capacity(data_value.len());
                    for byte_value in data_value.drain(..) {
                        let JsonValue::Number(byte_value) = byte_value else {
                            return Err(crate::Error::failed(format!(
                                "Expected number for data byte in field {}",
                                field_meta.name
                            )));
                        };
                        data.push(byte_value as u8);
                    }
                    *field_value = JsonValue::DataBuffer(data);
                    Ok(crate::dynamic_value::Reader::Data(match field_value {
                        JsonValue::DataBuffer(ref data) => data.as_slice(),
                        _ => unreachable!(),
                    }))
                }
                DataEncoding::Base64 => {
                    let JsonValue::String(data_value) = field_value else {
                        return Err(crate::Error::failed(format!(
                            "Expected string for base64 data field {}",
                            field_meta.name
                        )));
                    };
                    *field_value = JsonValue::DataBuffer(base64::decode(data_value)?);
                    Ok(crate::dynamic_value::Reader::Data(match field_value {
                        JsonValue::DataBuffer(ref data) => data.as_slice(),
                        _ => unreachable!(),
                    }))
                }
                DataEncoding::Hex => {
                    let JsonValue::String(data_value) = field_value else {
                        return Err(crate::Error::failed(format!(
                            "Expected string for hex data field {}",
                            field_meta.name
                        )));
                    };
                    *field_value = JsonValue::DataBuffer(hex::decode(data_value)?);
                    Ok(crate::dynamic_value::Reader::Data(match field_value {
                        JsonValue::DataBuffer(ref data) => data.as_slice(),
                        _ => unreachable!(),
                    }))
                }
            },
            _ => Err(crate::Error::failed(format!(
                "Unsupported primitive type for field {}",
                field_meta.name
            ))),
        }
    }

    fn decode_list(
        mut field_values: Vec<JsonValue>,
        mut list_builder: crate::dynamic_list::Builder,
        field_meta: &EncodingOptions,
    ) -> crate::Result<()> {
        match list_builder.element_type().which() {
            crate::introspect::TypeVariant::Struct(_sub_element_schema) => {
                for (i, item_value) in field_values.drain(..).enumerate() {
                    let JsonValue::Object(item_value) = item_value else {
                        return Err(crate::Error::failed(format!(
                            "Expected object for struct list field {}",
                            field_meta.name
                        )));
                    };
                    let struct_builder = list_builder
                        .reborrow()
                        .get(i as u32)?
                        .downcast::<crate::dynamic_struct::Builder>();
                    decode_struct(item_value, struct_builder, field_meta)?;
                }
                Ok(())
            }
            crate::introspect::TypeVariant::List(_sub_element_type) => {
                for (i, item_value) in field_values.drain(..).enumerate() {
                    let JsonValue::Array(item_value) = item_value else {
                        return Err(crate::Error::failed(format!(
                            "Expected array for list field {}",
                            field_meta.name
                        )));
                    };
                    let sub_element_builder = list_builder
                        .reborrow()
                        .init(i as u32, item_value.len() as u32)?
                        .downcast::<crate::dynamic_list::Builder>();
                    decode_list(item_value, sub_element_builder, field_meta)?;
                }
                Ok(())
            }
            _ => {
                for (i, mut item_value) in field_values.drain(..).enumerate() {
                    list_builder.set(
                        i as u32,
                        decode_primitive(
                            &mut item_value,
                            &list_builder.element_type(),
                            field_meta,
                        )?,
                    )?;
                }
                Ok(())
            }
        }
    }

    fn decode_struct(
        mut value: HashMap<String, JsonValue>,
        mut builder: crate::dynamic_struct::Builder<'_>,
        meta: &EncodingOptions,
    ) -> crate::Result<()> {
        for field in builder.get_schema().get_fields()? {
            let field_meta = EncodingOptions::from_field(meta.prefix, &field)?;
            let mut field_value = match value.remove(field_meta.name) {
                Some(v) => v,
                None => continue,
            };

            // TODO: Handle (un)flattening, unions, discriminators, etc.

            match field.get_type().which() {
                crate::introspect::TypeVariant::Struct(_struct_schema) => {
                    let JsonValue::Object(field_value) = field_value else {
                        return Err(crate::Error::failed(format!(
                            "Expected object for field {}",
                            field_meta.name
                        )));
                    };
                    let struct_builder = builder
                        .reborrow()
                        .init(field)?
                        .downcast::<crate::dynamic_struct::Builder>();
                    decode_struct(field_value, struct_builder, &field_meta)?;
                }
                crate::introspect::TypeVariant::List(_element_type) => {
                    let JsonValue::Array(field_value) = field_value else {
                        return Err(crate::Error::failed(format!(
                            "Expected array for field {}",
                            field_meta.name
                        )));
                    };
                    let list_builder = builder
                        .reborrow()
                        .initn(field, field_value.len() as u32)?
                        .downcast::<crate::dynamic_list::Builder>();
                    decode_list(field_value, list_builder, &field_meta)?;
                }

                crate::introspect::TypeVariant::AnyPointer => {
                    return Err(crate::Error::unimplemented(
                        "AnyPointer cannot be represented in JSON".into(),
                    ))
                }
                crate::introspect::TypeVariant::Capability => {
                    return Err(crate::Error::unimplemented(
                        "Capability cannot be represented in JSON".into(),
                    ))
                }

                _ => {
                    builder.set(
                        field,
                        decode_primitive(&mut field_value, &field.get_type(), &field_meta)?,
                    )?;
                }
            }
        }

        Ok(())
    }
}

mod base64 {
    const BASE64_CHARS: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(data: &[u8]) -> String {
        // We don't want to pull in base64 crate just for this. So hand-rolling a
        // base64 encoder.
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

    pub fn decode(data: &str) -> crate::Result<Vec<u8>> {
        let bytes = data.as_bytes();
        if bytes.len() % 4 != 0 {
            return Err(crate::Error::failed(
                "Base64 string length must be a multiple of 4".into(),
            ));
        }
        let mut decoded = Vec::with_capacity(bytes.len() / 4 * 3);
        for chunk in bytes.chunks(4) {
            let mut n: u32 = 0;
            let mut padding = 0;
            for &c in chunk {
                n <<= 6;
                match c {
                    b'A'..=b'Z' => n |= (c - b'A') as u32,
                    b'a'..=b'z' => n |= (c - b'a' + 26) as u32,
                    b'0'..=b'9' => n |= (c - b'0' + 52) as u32,
                    b'+' => n |= 62,
                    b'/' => n |= 63,
                    b'=' => {
                        n |= 0;
                        padding += 1;
                    }
                    _ => {
                        return Err(crate::Error::failed(format!(
                            "Invalid base64 character: {}",
                            c as char
                        )));
                    }
                }
            }
            decoded.push(((n >> 16) & 0xFF) as u8);
            if padding < 2 {
                decoded.push(((n >> 8) & 0xFF) as u8);
            }
            if padding < 1 {
                decoded.push((n & 0xFF) as u8);
            }
        }
        Ok(decoded)
    }
}

mod hex {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    fn hex_char_to_value(c: u8) -> crate::Result<u8> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(crate::Error::failed(format!(
                "Invalid hex character: {}",
                c as char
            ))),
        }
    }

    pub fn encode(data: &[u8]) -> String {
        let mut encoded = String::with_capacity(data.len() * 2);
        for &byte in data {
            let high = HEX_CHARS[(byte >> 4) as usize];
            let low = HEX_CHARS[(byte & 0x0F) as usize];
            encoded.push(high as char);
            encoded.push(low as char);
        }
        encoded
    }

    pub fn decode(data: &str) -> crate::Result<Vec<u8>> {
        if data.len() % 2 != 0 {
            return Err(crate::Error::failed(
                "Hex string must have even length".into(),
            ));
        }
        let mut decoded = Vec::with_capacity(data.len() / 2);
        let bytes = data.as_bytes();
        for i in (0..data.len()).step_by(2) {
            let high = hex_char_to_value(bytes[i])?;
            let low = hex_char_to_value(bytes[i + 1])?;
            decoded.push((high << 4) | low);
        }
        Ok(decoded)
    }
}

