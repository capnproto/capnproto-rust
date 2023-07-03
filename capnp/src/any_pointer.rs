// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
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

//! Untyped pointer that can be cast to any struct, list, or capability type.

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};

#[cfg(feature = "alloc")]
use crate::capability::FromClientHook;
#[cfg(feature = "alloc")]
use crate::private::capability::{ClientHook, PipelineHook, PipelineOp};
use crate::private::layout::{PointerBuilder, PointerReader};
use crate::traits::{FromPointerBuilder, FromPointerReader, SetPointerBuilder};
use crate::Result;

#[derive(Copy, Clone)]
pub struct Owned(());

impl crate::traits::Owned for Owned {
    type Reader<'a> = Reader<'a>;
    type Builder<'a> = Builder<'a>;
}

impl crate::introspect::Introspect for Owned {
    fn introspect() -> crate::introspect::Type {
        crate::introspect::TypeVariant::AnyPointer.into()
    }
}

impl crate::traits::Pipelined for Owned {
    type Pipeline = Pipeline;
}

#[derive(Copy, Clone)]
pub struct Reader<'a> {
    pub(crate) reader: PointerReader<'a>,
}

impl<'a> Reader<'a> {
    pub fn new(reader: PointerReader<'_>) -> Reader<'_> {
        Reader { reader }
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.reader.is_null()
    }

    /// Gets the total size of the target and all of its children. Does not count far pointer overhead.
    pub fn target_size(&self) -> Result<crate::MessageSize> {
        self.reader.total_size()
    }

    #[inline]
    pub fn get_as<T: FromPointerReader<'a>>(&self) -> Result<T> {
        FromPointerReader::get_from_pointer(&self.reader, None)
    }

    #[cfg(feature = "alloc")]
    pub fn get_as_capability<T: FromClientHook>(&self) -> Result<T> {
        Ok(FromClientHook::new(self.reader.get_capability()?))
    }

    //# Used by RPC system to implement pipelining. Applications
    //# generally shouldn't use this directly.
    #[cfg(feature = "alloc")]
    pub fn get_pipelined_cap(&self, ops: &[PipelineOp]) -> Result<Box<dyn ClientHook>> {
        let mut pointer = self.reader;

        for op in ops {
            match *op {
                PipelineOp::Noop => {}
                PipelineOp::GetPointerField(idx) => {
                    pointer = pointer.get_struct(None)?.get_pointer_field(idx as usize);
                }
            }
        }

        pointer.get_capability()
    }
}

impl<'a> FromPointerReader<'a> for Reader<'a> {
    fn get_from_pointer(
        reader: &PointerReader<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Reader<'a>> {
        if default.is_some() {
            panic!("Unsupported: any_pointer with a default value.");
        }
        Ok(Reader { reader: *reader })
    }
}

impl<'a> crate::traits::SetPointerBuilder for Reader<'a> {
    fn set_pointer_builder<'b>(
        mut pointer: crate::private::layout::PointerBuilder<'b>,
        value: Reader<'a>,
        canonicalize: bool,
    ) -> Result<()> {
        pointer.copy_from(value.reader, canonicalize)
    }
}

#[cfg(feature = "alloc")]
impl<'a> crate::traits::Imbue<'a> for Reader<'a> {
    fn imbue(&mut self, cap_table: &'a crate::private::layout::CapTable) {
        self.reader
            .imbue(crate::private::layout::CapTableReader::Plain(cap_table));
    }
}

pub struct Builder<'a> {
    builder: PointerBuilder<'a>,
}

impl<'a> Builder<'a> {
    pub fn new(builder: PointerBuilder<'a>) -> Builder<'a> {
        Builder { builder }
    }

    pub fn reborrow(&mut self) -> Builder<'_> {
        Builder {
            builder: self.builder.reborrow(),
        }
    }

    pub fn is_null(&self) -> bool {
        self.builder.is_null()
    }

    /// Gets the total size of the target and all of its children. Does not count far pointer overhead.
    pub fn target_size(&self) -> Result<crate::MessageSize> {
        self.builder.as_reader().total_size()
    }

    pub fn get_as<T: FromPointerBuilder<'a>>(self) -> Result<T> {
        FromPointerBuilder::get_from_pointer(self.builder, None)
    }

    pub fn init_as<T: FromPointerBuilder<'a>>(self) -> T {
        FromPointerBuilder::init_pointer(self.builder, 0)
    }

    pub fn initn_as<T: FromPointerBuilder<'a>>(self, size: u32) -> T {
        FromPointerBuilder::init_pointer(self.builder, size)
    }

    pub fn set_as<From: SetPointerBuilder>(&mut self, value: From) -> Result<()> {
        SetPointerBuilder::set_pointer_builder(self.builder.reborrow(), value, false)
    }

    // XXX value should be a user client.
    #[cfg(feature = "alloc")]
    pub fn set_as_capability(&mut self, value: Box<dyn ClientHook>) {
        self.builder.set_capability(value);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.builder.clear()
    }

    pub fn into_reader(self) -> Reader<'a> {
        Reader {
            reader: self.builder.into_reader(),
        }
    }
}

impl<'a> FromPointerBuilder<'a> for Builder<'a> {
    fn init_pointer(mut builder: PointerBuilder<'a>, _len: u32) -> Builder<'a> {
        if !builder.is_null() {
            builder.clear();
        }
        Builder { builder }
    }
    fn get_from_pointer(
        builder: PointerBuilder<'a>,
        default: Option<&'a [crate::Word]>,
    ) -> Result<Builder<'a>> {
        if default.is_some() {
            panic!("AnyPointer defaults are unsupported")
        }
        Ok(Builder { builder })
    }
}

#[cfg(feature = "alloc")]
impl<'a> crate::traits::ImbueMut<'a> for Builder<'a> {
    fn imbue_mut(&mut self, cap_table: &'a mut crate::private::layout::CapTable) {
        self.builder
            .imbue(crate::private::layout::CapTableBuilder::Plain(cap_table));
    }
}

pub struct Pipeline {
    // XXX this should not be public
    #[cfg(feature = "alloc")]
    pub hook: Box<dyn PipelineHook>,

    #[cfg(feature = "alloc")]
    ops: Vec<PipelineOp>,
}

impl Pipeline {
    #[cfg(feature = "alloc")]
    pub fn new(hook: Box<dyn PipelineHook>) -> Self {
        Self {
            hook,
            ops: Vec::new(),
        }
    }

    #[cfg(feature = "alloc")]
    pub fn noop(&self) -> Self {
        Self {
            hook: self.hook.add_ref(),
            ops: self.ops.clone(),
        }
    }

    #[cfg(not(feature = "alloc"))]
    pub fn noop(&self) -> Self {
        Self {}
    }

    #[cfg(feature = "alloc")]
    pub fn get_pointer_field(&self, pointer_index: u16) -> Self {
        let mut new_ops = Vec::with_capacity(self.ops.len() + 1);
        for op in &self.ops {
            new_ops.push(*op)
        }
        new_ops.push(PipelineOp::GetPointerField(pointer_index));
        Self {
            hook: self.hook.add_ref(),
            ops: new_ops,
        }
    }

    #[cfg(not(feature = "alloc"))]
    pub fn get_pointer_field(&self, _pointer_index: u16) -> Self {
        Self {}
    }

    #[cfg(feature = "alloc")]
    pub fn as_cap(&self) -> Box<dyn ClientHook> {
        self.hook.get_pipelined_cap(&self.ops)
    }
}

impl crate::capability::FromTypelessPipeline for Pipeline {
    fn new(typeless: Pipeline) -> Self {
        typeless
    }
}

impl<'a> From<Reader<'a>> for crate::dynamic_value::Reader<'a> {
    fn from(a: Reader<'a>) -> crate::dynamic_value::Reader<'a> {
        crate::dynamic_value::Reader::AnyPointer(a)
    }
}

impl<'a> From<Builder<'a>> for crate::dynamic_value::Builder<'a> {
    fn from(a: Builder<'a>) -> crate::dynamic_value::Builder<'a> {
        crate::dynamic_value::Builder::AnyPointer(a)
    }
}

#[cfg(feature = "alloc")]
#[test]
fn init_clears_value() {
    let mut message = crate::message::Builder::new_default();
    {
        let root: crate::any_pointer::Builder = message.init_root();
        let mut list: crate::primitive_list::Builder<u16> = root.initn_as(10);
        for idx in 0..10 {
            list.set(idx, idx as u16);
        }
    }

    {
        let root: crate::any_pointer::Builder = message.init_root();
        assert!(root.is_null());
    }

    let mut output: Vec<u8> = Vec::new();
    crate::serialize::write_message(&mut output, &message).unwrap();
    assert_eq!(output.len(), 40);
    for byte in &output[8..] {
        // Everything not in the message header is zero.
        assert_eq!(*byte, 0u8);
    }
}
