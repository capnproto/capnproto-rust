mod deserialize;
#[allow(unused)]
mod json_capnp;
pub mod serialize;

pub use deserialize::deserialize_into;
pub use serialize::serialize;

use crate::{
    schema::{AnnotationList, Field},
    Result,
};

use self::json_capnp::{discriminator_options, flatten_options};

// TODO: Support flatten, discriminator annotations

#[derive(Debug, Default)]
struct JsonAnnots<'a> {
    name: Option<&'a str>,
    data_format: Option<DataFormat>,
    flatten: Option<FlattenOptions<'a>>,
}

#[derive(Debug)]
enum DataFormat {
    Hex,
    Base64,
}

#[derive(Debug, Default)]
struct FlattenOptions<'a> {
    prefix: Option<&'a str>,
}

#[derive(Debug, Default)]
struct DiscriminatorOptions<'a> {
    name: Option<&'a str>,
    value_name: Option<&'a str>,
}

fn read_annots<'a>(list: AnnotationList) -> Result<JsonAnnots<'a>> {
    let mut out = JsonAnnots::default();
    for annot in list.iter() {
        match annot.get_id() {
            json_capnp::name::ID => {
                let value: &str = annot.get_value()?.downcast();
                out.name = Some(value);
            }
            json_capnp::flatten::ID => {
                let reader: flatten_options::Reader<'_> = annot.get_value()?.downcast();
                let mut value = FlattenOptions::default();
                if reader.has_prefix() {
                    value.prefix = Some(reader.get_prefix()?);
                }
            }
            json_capnp::discriminator::ID => {
                let reader: discriminator_options::Reader<'_> = annot.get_value()?.downcast();
                let mut value = DiscriminatorOptions::default();
                if reader.has_name() {
                    value.name = Some(reader.get_name()?);
                }
                if reader.has_value_name() {
                    value.value_name = Some(reader.get_value_name()?);
                }
            }
            json_capnp::base64::ID => out.data_format = Some(DataFormat::Base64),
            json_capnp::hex::ID => out.data_format = Some(DataFormat::Hex),
            _ => {}
        }
    }
    Ok(out)
}

fn field_name<'a>(field: &'a Field, annots: &'a JsonAnnots) -> Result<&'a str> {
    if let Some(name) = annots.name {
        Ok(name)
    } else {
        field.get_proto().get_name()
    }
}
