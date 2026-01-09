// Copyright (c) 2017 Sandstorm Development Group, Inc. and contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use capnp::{any_pointer, message};

use crate::codegen::FormattedText::{Branch, Indent, Line};
use crate::codegen::{fmt, indent, line, FormattedText, GeneratorContext};
use crate::codegen_types::{Leaf, RustTypeInfo};
use capnp::schema_capnp::type_;

#[derive(Clone, Copy)]
pub struct WordArrayDeclarationOptions {
    pub public: bool,
}

fn word_array_declaration_aux<T: ::capnp::traits::SetterInput<impl ::capnp::traits::Owned>>(
    ctx: &GeneratorContext,
    name: &str,
    value: T,
    total_size: ::capnp::MessageSize,
    options: WordArrayDeclarationOptions,
) -> ::capnp::Result<FormattedText> {
    let allocator =
        message::HeapAllocator::new().first_segment_words(total_size.word_count as u32 + 1);
    let mut message = message::Builder::new(allocator);
    message.set_root(value)?;
    let words = message.get_segments_for_output()[0];
    let mut words_lines = Vec::new();
    for index in 0..(words.len() / 8) {
        let bytes = &words[(index * 8)..(index + 1) * 8];
        words_lines.push(Line(fmt!(
            ctx,
            "{capnp}::word({}, {}, {}, {}, {}, {}, {}, {}),",
            bytes[0],
            bytes[1],
            bytes[2],
            bytes[3],
            bytes[4],
            bytes[5],
            bytes[6],
            bytes[7]
        )));
    }

    // `static` instead of `const` because these arrays can be large
    // and consts get inlined at each usage.
    let vis = if options.public { "pub " } else { "" };
    Ok(Branch(vec![
        Line(fmt!(
            ctx,
            "{}static {}: [{capnp}::Word; {}] = [",
            vis,
            name,
            words.len() / 8
        )),
        indent(Branch(words_lines)),
        line("];"),
    ]))
}

pub fn word_array_declaration(
    ctx: &GeneratorContext,
    name: &str,
    value: any_pointer::Reader,
    options: WordArrayDeclarationOptions,
) -> ::capnp::Result<FormattedText> {
    word_array_declaration_aux(ctx, name, value, value.target_size()?, options)
}

pub fn node_word_array_declaration(
    ctx: &GeneratorContext,
    name: &str,
    value: capnp::schema_capnp::node::Reader,
    options: WordArrayDeclarationOptions,
) -> ::capnp::Result<FormattedText> {
    word_array_declaration_aux(ctx, name, value, value.total_size()?, options)
}

pub fn generate_pointer_constant(
    ctx: &GeneratorContext,
    styled_name: &str,
    typ: type_::Reader,
    value: any_pointer::Reader,
) -> ::capnp::Result<FormattedText> {
    Ok(Branch(vec![
        Line(fmt!(
            ctx,
            "pub static {}: {capnp}::constant::Reader<{}> = {{",
            styled_name,
            typ.type_string(ctx, Leaf::Owned)?
        )),
        Indent(Box::new(Branch(vec![
            word_array_declaration(
                ctx,
                "WORDS",
                value,
                WordArrayDeclarationOptions { public: false },
            )?,
            Line(fmt!(
                ctx,
                "static ARENA: {capnp}::private::arena::GeneratedCodeArena = {capnp}::private::arena::GeneratedCodeArena::new(&WORDS);"
            )),
            Line(fmt!(
                ctx,
                "{capnp}::constant::Reader::new(&ARENA)"
            )),
        ]))),
        line("};"),
    ]))
}
