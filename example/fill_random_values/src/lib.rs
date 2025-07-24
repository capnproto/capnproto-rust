use rand::distr::uniform::SampleUniform;
use rand::Rng;

use capnp::introspect::TypeVariant;
use capnp::schema;
use capnp::{dynamic_struct, dynamic_value};

capnp::generated_code!(pub mod fill_capnp);

pub struct Filler<R: Rng> {
    rng: R,
    recursion_limit: u32,
}

fn get_range<T>(r: dynamic_struct::Reader) -> ::capnp::Result<std::ops::RangeInclusive<T>>
where
    T: for<'a> ::capnp::dynamic_value::DowncastReader<'a>,
{
    Ok(r.get_named("min")?.downcast::<T>()..=r.get_named("max")?.downcast::<T>())
}

fn set_from_range<T, R>(
    rng: &mut R,
    a: ::capnp::schema::Annotation,
    mut builder: ::capnp::dynamic_struct::Builder,
    field: ::capnp::schema::Field,
) -> ::capnp::Result<()>
where
    T: for<'a> ::capnp::dynamic_value::DowncastReader<'a>
        + SampleUniform
        + PartialOrd
        + for<'a> Into<::capnp::dynamic_value::Reader<'a>>,
    R: Rng,
{
    let x: T = rng.random_range(get_range::<T>(a.get_value()?.downcast())?);
    builder.set(field, x.into())
}

impl<R: Rng> Filler<R> {
    pub fn new(rng: R, recursion_limit: u32) -> Self {
        Self {
            rng,
            recursion_limit,
        }
    }

    fn random_enum_value(&mut self, e: schema::EnumSchema) -> ::capnp::Result<dynamic_value::Enum> {
        let enumerants = e.get_enumerants()?;
        let idx = self.rng.random_range(0..enumerants.len());
        let value = enumerants.get(idx).get_ordinal();
        Ok(::capnp::dynamic_value::Enum::new(value, e))
    }

    fn fill_text(&mut self, mut builder: ::capnp::text::Builder) {
        builder.clear();
        for _ in 0..builder.len() {
            builder.push_ascii(self.rng.random_range(b'a'..=b'z'));
        }
    }

    fn fill_data(&mut self, builder: ::capnp::data::Builder) {
        for b in builder {
            *b = self.rng.random();
        }
    }

    fn fill_field(
        &mut self,
        recursion_depth: u32,
        mut builder: ::capnp::dynamic_struct::Builder,
        field: ::capnp::schema::Field,
    ) -> ::capnp::Result<()> {
        let annotations = field.get_annotations()?;
        for annotation in annotations {
            if annotation.get_id() == fill_capnp::select_from::choices::ID {
                if let TypeVariant::List(element_type) = annotation.get_type().which() {
                    if !element_type.loose_equals(field.get_type()) {
                        return Err(::capnp::Error::failed(
                            "choices annotation element type mismatch".into(),
                        ));
                    }
                } else {
                    return Err(::capnp::Error::failed(
                        "choices annotation was not of List type".into(),
                    ));
                }
                let choices: capnp::dynamic_list::Reader<'_> = annotation.get_value()?.downcast();
                let idx = self.rng.random_range(0..choices.len());
                return builder.set(field, choices.get(idx).unwrap());
            } else if annotation.get_id() == fill_capnp::int8_range::ID {
                return set_from_range::<i8, R>(&mut self.rng, annotation, builder, field);
            } else if annotation.get_id() == fill_capnp::int16_range::ID {
                return set_from_range::<i16, R>(&mut self.rng, annotation, builder, field);
            } else if annotation.get_id() == fill_capnp::int32_range::ID {
                return set_from_range::<i32, R>(&mut self.rng, annotation, builder, field);
            } else if annotation.get_id() == fill_capnp::int64_range::ID {
                return set_from_range::<i64, R>(&mut self.rng, annotation, builder, field);
            } else if annotation.get_id() == fill_capnp::uint8_range::ID {
                return set_from_range::<u8, R>(&mut self.rng, annotation, builder, field);
            } else if annotation.get_id() == fill_capnp::uint16_range::ID {
                return set_from_range::<u16, R>(&mut self.rng, annotation, builder, field);
            } else if annotation.get_id() == fill_capnp::uint32_range::ID {
                return set_from_range::<u32, R>(&mut self.rng, annotation, builder, field);
            } else if annotation.get_id() == fill_capnp::uint64_range::ID {
                return set_from_range::<u64, R>(&mut self.rng, annotation, builder, field);
            } else if annotation.get_id() == fill_capnp::float32_range::ID {
                return set_from_range::<f32, R>(&mut self.rng, annotation, builder, field);
            } else if annotation.get_id() == fill_capnp::float64_range::ID {
                return set_from_range::<f64, R>(&mut self.rng, annotation, builder, field);
            }
        }

        match field.get_type().which() {
            TypeVariant::Void => Ok(()),
            TypeVariant::Bool => builder.set(field, self.rng.random::<bool>().into()),
            TypeVariant::Int8 => builder.set(field, self.rng.random::<i8>().into()),
            TypeVariant::Int16 => builder.set(field, self.rng.random::<i16>().into()),
            TypeVariant::Int32 => builder.set(field, self.rng.random::<i32>().into()),
            TypeVariant::Int64 => builder.set(field, self.rng.random::<i64>().into()),
            TypeVariant::UInt8 => builder.set(field, self.rng.random::<u8>().into()),
            TypeVariant::UInt16 => builder.set(field, self.rng.random::<u16>().into()),
            TypeVariant::UInt32 => builder.set(field, self.rng.random::<u32>().into()),
            TypeVariant::UInt64 => builder.set(field, self.rng.random::<u64>().into()),
            TypeVariant::Float32 => builder.set(field, self.rng.random::<f32>().into()),
            TypeVariant::Float64 => builder.set(field, self.rng.random::<f64>().into()),
            TypeVariant::Text => {
                if annotations.find(fill_capnp::phone_number::ID).is_some() {
                    builder.set(
                        field,
                        format!(
                            "{:03}-555-1{:03}",
                            self.rng.random_range(0..1000),
                            self.rng.random_range(0..1000)
                        )[..]
                            .into(),
                    )
                } else {
                    let len = self.rng.random_range(0..20);
                    self.fill_text(builder.initn(field, len)?.downcast());
                    Ok(())
                }
            }
            TypeVariant::Data => {
                let len = self.rng.random_range(0..20);
                self.fill_data(builder.initn(field, len)?.downcast());
                Ok(())
            }
            TypeVariant::Enum(e) => builder.set(field, self.random_enum_value(e.into())?.into()),
            TypeVariant::Struct(_) => {
                if recursion_depth < self.recursion_limit {
                    self.fill_struct(recursion_depth + 1, builder.init(field)?.downcast())
                } else {
                    Ok(())
                }
            }
            TypeVariant::List(_) => {
                let annotations = field.get_annotations()?;
                let len;
                if let Some(len_range) = annotations.find(fill_capnp::length_range::ID) {
                    let len_range: dynamic_struct::Reader<'_> = len_range.get_value()?.downcast();
                    let min: u32 = len_range.get_named("min")?.downcast();
                    let max: u32 = len_range.get_named("max")?.downcast();
                    len = self.rng.random_range(min..=max);
                } else {
                    len = self.rng.random_range(0..10);
                }
                if recursion_depth < self.recursion_limit {
                    self.fill_list(recursion_depth + 1, builder.initn(field, len)?.downcast())
                } else {
                    Ok(())
                }
            }

            TypeVariant::AnyPointer => Ok(()),
            TypeVariant::Capability => Ok(()),
        }
    }

    fn fill_list_element(
        &mut self,
        recursion_depth: u32,
        mut builder: ::capnp::dynamic_list::Builder,
        index: u32,
    ) -> ::capnp::Result<()> {
        match builder.element_type().which() {
            TypeVariant::Void => Ok(()),
            TypeVariant::Bool => builder.set(index, self.rng.random::<bool>().into()),
            TypeVariant::Int8 => builder.set(index, self.rng.random::<i8>().into()),
            TypeVariant::Int16 => builder.set(index, self.rng.random::<i16>().into()),
            TypeVariant::Int32 => builder.set(index, self.rng.random::<i32>().into()),
            TypeVariant::Int64 => builder.set(index, self.rng.random::<i64>().into()),
            TypeVariant::UInt8 => builder.set(index, self.rng.random::<u8>().into()),
            TypeVariant::UInt16 => builder.set(index, self.rng.random::<u16>().into()),
            TypeVariant::UInt32 => builder.set(index, self.rng.random::<u32>().into()),
            TypeVariant::UInt64 => builder.set(index, self.rng.random::<u64>().into()),
            TypeVariant::Float32 => builder.set(index, self.rng.random::<f32>().into()),
            TypeVariant::Float64 => builder.set(index, self.rng.random::<f64>().into()),
            TypeVariant::Enum(e) => builder.set(index, self.random_enum_value(e.into())?.into()),
            TypeVariant::Text => {
                let len = self.rng.random_range(0..20);
                self.fill_text(builder.init(index, len)?.downcast());
                Ok(())
            }
            TypeVariant::Data => {
                let len = self.rng.random_range(0..20);
                self.fill_data(builder.init(index, len)?.downcast());
                Ok(())
            }
            TypeVariant::Struct(_) => {
                self.fill_struct(recursion_depth + 1, builder.get(index)?.downcast())
            }
            TypeVariant::List(_) => {
                self.fill_list(recursion_depth + 1, builder.get(index)?.downcast())
            }
            TypeVariant::AnyPointer => Ok(()),
            TypeVariant::Capability => Ok(()),
        }
    }

    fn fill_list(
        &mut self,
        recursion_depth: u32,
        mut builder: ::capnp::dynamic_list::Builder,
    ) -> ::capnp::Result<()> {
        for idx in 0..builder.len() {
            self.fill_list_element(recursion_depth, builder.reborrow(), idx)?;
        }
        Ok(())
    }

    fn fill_struct(
        &mut self,
        recursion_depth: u32,
        mut builder: ::capnp::dynamic_struct::Builder,
    ) -> ::capnp::Result<()> {
        let schema = builder.get_schema();
        let non_union_fields = schema.get_non_union_fields()?;
        for field in non_union_fields {
            if field.get_type().is_pointer_type() {
                // maybe decide not to touch the field.
            }
            self.fill_field(recursion_depth, builder.reborrow(), field)?;
        }

        let union_fields = schema.get_union_fields()?;
        if !union_fields.is_empty() {
            let disc = self.rng.random_range(0..union_fields.len());
            self.fill_field(recursion_depth, builder, union_fields.get(disc))?;
        }
        Ok(())
    }

    pub fn fill(&mut self, builder: ::capnp::dynamic_struct::Builder) -> ::capnp::Result<()> {
        self.fill_struct(0, builder)
    }
}
