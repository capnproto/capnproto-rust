mod data;
mod decode;
mod encode;

pub mod json_capnp;

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
        field: &capnp::schema::Field,
    ) -> capnp::Result<Self> {
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
                        .downcast::<capnp::text::Reader>()
                        .to_str()?;
                }
                json_capnp::base64::ID => {
                    if options.data_encoding != DataEncoding::Default {
                        return Err(capnp::Error::failed(
                            "Cannot specify both base64 and hex annotations on the same field"
                                .into(),
                        ));
                    }
                    options.data_encoding = DataEncoding::Base64;
                }
                json_capnp::hex::ID => {
                    if options.data_encoding != DataEncoding::Default {
                        return Err(capnp::Error::failed(
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
            while let capnp::introspect::TypeVariant::List(sub_element_type) = element_type.which()
            {
                element_type = sub_element_type;
            }
            if !matches!(element_type.which(), capnp::introspect::TypeVariant::Data) {
                return Err(capnp::Error::failed(
                    "base64/hex annotation can only be applied to Data fields".into(),
                ));
            }
        }
        Ok(options)
    }
}

pub fn to_json<'reader>(
    reader: impl Into<capnp::dynamic_value::Reader<'reader>>,
) -> capnp::Result<String> {
    let mut writer = std::io::Cursor::new(Vec::with_capacity(4096));
    encode::serialize_json_to(&mut writer, reader)?;
    String::from_utf8(writer.into_inner()).map_err(|e| e.into())
}

pub fn from_json<'segments>(
    json: &str,
    builder: impl Into<capnp::dynamic_value::Builder<'segments>>,
) -> capnp::Result<()> {
    let capnp::dynamic_value::Builder::Struct(builder) = builder.into() else {
        return Err(capnp::Error::failed(
            "Top-level JSON value must be an object".into(),
        ));
    };
    decode::parse(json, builder)
}
