use capnp::schema_capnp;

use crate::codegen::FormattedText::{Branch, Line};
use crate::codegen::{line, FormattedText, GeneratorContext};

use crate::codegen::get_field_name;

fn format_offset(offset: u32, typ: &str) -> String {
    match typ {
        "Bool" => {
            // Bool offset is in bits
            let byte = offset / 8;
            let bit = offset % 8;
            format!("{}:{}", byte, bit)
        }
        "Int8" | "Uint8" => format!("{}", offset),
        "Int16" | "Uint16" | "Enum" => format!("{}", offset * 2),
        "Int32" | "Uint32" | "Float32" => format!("{}", offset * 4),
        "Int64" | "Uint64" | "Float64" => format!("{}", offset * 8),
        "Void" => format!("{}", offset),
        // Pointer types: offset is in words (8 bytes each)
        _ => format!("{}", offset * 8),
    }
}

fn format_table_row(
    name: &str,
    offset: &str,
    typ: &str,
    name_width: usize,
    offset_width: usize,
    type_width: usize,
) -> FormattedText {
    Line(format!(
        "//   {:<name_width$}  {:>offset_width$}  {:<type_width$}",
        name,
        offset,
        typ,
        name_width = name_width,
        offset_width = offset_width,
        type_width = type_width,
    ))
}

fn format_table_header(name_width: usize, offset_width: usize, type_width: usize) -> FormattedText {
    format_table_row(
        "NAME",
        "OFFSET",
        "TYPE",
        name_width,
        offset_width,
        type_width,
    )
}

fn is_pointer_type(typ: &schema_capnp::type_::WhichReader<'_>) -> bool {
    matches!(
        typ,
        schema_capnp::type_::Text(())
            | schema_capnp::type_::Data(())
            | schema_capnp::type_::List(_)
            | schema_capnp::type_::Struct(_)
            | schema_capnp::type_::Interface(_)
            | schema_capnp::type_::AnyPointer(_)
    )
}

fn format_type(
    typ: &schema_capnp::type_::WhichReader<'_>,
    ctx: &GeneratorContext,
) -> capnp::Result<String> {
    match typ {
        schema_capnp::type_::Bool(()) => Ok("Bool".to_string()),
        schema_capnp::type_::Int8(()) => Ok("Int8".to_string()),
        schema_capnp::type_::Int16(()) => Ok("Int16".to_string()),
        schema_capnp::type_::Int32(()) => Ok("Int32".to_string()),
        schema_capnp::type_::Int64(()) => Ok("Int64".to_string()),
        schema_capnp::type_::Uint8(()) => Ok("Uint8".to_string()),
        schema_capnp::type_::Uint16(()) => Ok("Uint16".to_string()),
        schema_capnp::type_::Uint32(()) => Ok("Uint32".to_string()),
        schema_capnp::type_::Uint64(()) => Ok("Uint64".to_string()),
        schema_capnp::type_::Float32(()) => Ok("Float32".to_string()),
        schema_capnp::type_::Float64(()) => Ok("Float64".to_string()),
        schema_capnp::type_::Enum(_) => Ok("Enum".to_string()),
        schema_capnp::type_::Text(()) => Ok("Text".to_string()),
        schema_capnp::type_::Data(()) => Ok("Data".to_string()),
        schema_capnp::type_::List(l) => {
            let elem = format_type(&l.get_element_type()?.which()?, ctx)?;
            Ok(format!("List({})", elem))
        }
        schema_capnp::type_::Struct(st) => {
            ctx.get_capnp_name(st.get_type_id()).map(|s| s.to_string())
        }
        schema_capnp::type_::Interface(i) => {
            ctx.get_capnp_name(i.get_type_id()).map(|s| s.to_string())
        }
        schema_capnp::type_::AnyPointer(_) => Ok("AnyPointer".to_string()),
        schema_capnp::type_::Void(()) => Ok("Void".to_string()),
    }
}

pub(crate) fn format_struct_layout(
    node_id: u64,
    struct_reader: schema_capnp::node::struct_::Reader<'_>,
    ctx: &GeneratorContext,
) -> capnp::Result<FormattedText> {
    let name = ctx.get_capnp_name(node_id)?;
    let fields = struct_reader.get_fields()?;
    let data_size = struct_reader.get_data_word_count();
    let pointer_size = struct_reader.get_pointer_count();
    let is_union = struct_reader.get_is_group();
    let kind = if is_union { "union" } else { "struct" };

    // (name, is_pointer, offset, type_desc)
    let mut field_infos: Vec<(&str, bool, u32, String)> = Vec::new();

    for field in fields {
        match field.which()? {
            schema_capnp::field::Slot(s) => {
                let name = get_field_name(field)?;
                let offset = s.get_offset();
                let typ = s.get_type()?.which()?;
                let type_desc = format_type(&typ, ctx)?;
                let is_pointer = is_pointer_type(&typ);
                field_infos.push((name, is_pointer, offset, type_desc));
            }
            schema_capnp::field::Group(g) => {
                if let Some(group_node) = ctx.node_map.get(&g.get_type_id()) {
                    if let schema_capnp::node::Struct(struct_node) = group_node.which()? {
                        for member in struct_node.get_fields()? {
                            if let schema_capnp::field::Slot(s) = member.which()? {
                                let name = get_field_name(member)?;
                                let offset = s.get_offset();
                                let typ = s.get_type()?.which()?;
                                let type_desc = format_type(&typ, ctx)?;
                                let is_pointer = is_pointer_type(&typ);
                                field_infos.push((name, is_pointer, offset, type_desc));
                            }
                        }
                    }
                }
            }
        }
    }

    let data_fields: Vec<_> = field_infos
        .iter()
        .filter(|(_, is_ptr, _, _)| !is_ptr)
        .collect();
    let ptr_fields: Vec<_> = field_infos
        .iter()
        .filter(|(_, is_ptr, _, _)| *is_ptr)
        .collect();

    // Collect discriminants (for anonymous unions)
    // (name, offset)
    let mut discriminants: Vec<(&str, u32)> = Vec::new();

    // Top-level struct discriminant (unnamed)
    if struct_reader.get_discriminant_count() > 0 {
        discriminants.push(("(unnamed)", struct_reader.get_discriminant_offset()));
    }

    // Discriminants from groups (named unions that contain anonymous unions)
    for field in fields {
        if let schema_capnp::field::Group(g) = field.which()? {
            if let Some(group_node) = ctx.node_map.get(&g.get_type_id()) {
                if let schema_capnp::node::Struct(struct_node) = group_node.which()? {
                    if struct_node.get_discriminant_count() > 0 {
                        let group_name = get_field_name(field)?;
                        discriminants.push((group_name, struct_node.get_discriminant_offset()));
                    }
                }
            }
        }
    }

    let name_width = field_infos
        .iter()
        .map(|(n, _, _, _)| n.len())
        .chain(discriminants.iter().map(|(n, _)| n.len()))
        .max()
        .unwrap_or(0)
        .max("NAME".len());

    // Calculate offset width from formatted offset strings
    let offset_width = field_infos
        .iter()
        .map(|(_, _, off, typ)| format_offset(*off, typ).len())
        .chain(discriminants.iter().map(|(_, off)| {
            // Discriminants are UInt16 (16-bit), offset is in 16-bit units
            // Convert to bytes: offset * 2
            let bytes = off * 2;
            bytes.to_string().len()
        }))
        .max()
        .unwrap_or(0)
        .max("OFFSET".len());

    let type_width = field_infos
        .iter()
        .map(|(_, _, _, t)| t.len())
        .max()
        .unwrap_or(0)
        .max("TYPE".len());

    let mut lines: Vec<FormattedText> = Vec::new();

    lines.push(Line(format!(
        "// {} {}: size in words: {} data, {} pointers",
        kind, name, data_size, pointer_size
    )));

    if !data_fields.is_empty() || !ptr_fields.is_empty() || !discriminants.is_empty() {
        lines.push(format_table_header(name_width, offset_width, type_width));
    }

    if !data_fields.is_empty() {
        lines.push(line("// Data:"));
        for (name, _, offset, typ) in &data_fields {
            let offset_str = format_offset(*offset, typ);
            lines.push(format_table_row(
                name,
                &offset_str,
                typ,
                name_width,
                offset_width,
                type_width,
            ));
        }
    }

    if !discriminants.is_empty() {
        lines.push(line("// Discriminants:"));
        for (disc_name, offset) in &discriminants {
            // Discriminant offset is in 16-bit units, convert to bytes
            let offset_str = format!("{}", offset * 2);
            lines.push(format_table_row(
                disc_name,
                &offset_str,
                "UInt16",
                name_width,
                offset_width,
                type_width,
            ));
        }
    }

    if !ptr_fields.is_empty() {
        lines.push(line("// Pointers:"));
        for (name, _, offset, typ) in &ptr_fields {
            // Pointer offset is in words (8 bytes each)
            let offset_str = format!("{}", offset * 8);
            lines.push(format_table_row(
                name,
                &offset_str,
                typ,
                name_width,
                offset_width,
                type_width,
            ));
        }
    }

    Ok(Branch(lines))
}
