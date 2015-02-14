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

use capability::FromClientHook;
use private::capability::{ClientHook, PipelineHook, PipelineOp};
use private::layout::{PointerReader, PointerBuilder};
use traits::{FromPointerReader, FromPointerBuilder, SetPointerBuilder};

#[derive(Copy)]
pub struct Reader<'a> {
    reader : PointerReader<'a>
}

impl <'a> Reader<'a> {
    #[inline]
    pub fn new<'b>(reader : PointerReader<'b>) -> Reader<'b> {
        Reader { reader : reader }
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.reader.is_null()
    }

    #[inline]
    pub fn get_as<T : FromPointerReader<'a>>(&self) -> T {
        FromPointerReader::get_from_pointer(&self.reader)
    }

    pub fn get_as_capability<T : FromClientHook>(&self) -> T {
        FromClientHook::new(self.reader.get_capability())
    }

    //# Used by RPC system to implement pipelining. Applications
    //# generally shouldn't use this directly.
    pub fn get_pipelined_cap(&self, ops : &[PipelineOp]) -> Box<ClientHook+Send> {
        let mut pointer = self.reader;

        for op in ops.iter() {
            match op {
                &PipelineOp::Noop =>  { }
                &PipelineOp::GetPointerField(idx) => {
                    pointer = pointer.get_struct(::std::ptr::null()).get_pointer_field(idx as usize)
                }
            }
        }

        pointer.get_capability()
    }
}

pub struct Builder<'a> {
    builder : PointerBuilder<'a>
}

impl <'a> Builder<'a> {
    #[inline]
    pub fn new<'b>(builder : PointerBuilder<'a>) -> Builder<'a> {
        Builder { builder : builder }
    }

    pub fn get_as<T : FromPointerBuilder<'a>>(self) -> T {
        FromPointerBuilder::get_from_pointer(self.builder)
    }

    pub fn init_as<T : FromPointerBuilder<'a>>(self) -> T {
        FromPointerBuilder::init_pointer(self.builder, 0)
    }

    pub fn init_as_sized<T : FromPointerBuilder<'a>>(self, size : u32) -> T {
        FromPointerBuilder::init_pointer(self.builder, size)
    }

    pub fn set_as<To, From : SetPointerBuilder<To>>(self, value : From) {
        SetPointerBuilder::<To>::set_pointer_builder(self.builder, value);
    }

    // XXX value should be a user client.
    pub fn set_as_capability(&self, value : Box<ClientHook+Send>) {
        self.builder.set_capability(value);
    }

    #[inline]
    pub fn clear(&mut self) {
        self.builder.clear()
    }

    #[inline]
    pub fn as_reader(self) -> Reader<'a> {
        Reader { reader : self.builder.as_reader() }
    }
}

pub struct Pipeline {
    hook : Box<PipelineHook+Send>,
    ops : Vec<PipelineOp>,
}

impl Pipeline {
    pub fn new(hook : Box<PipelineHook+Send>) -> Pipeline {
        Pipeline { hook : hook, ops : Vec::new() }
    }

    pub fn noop(&self) -> Pipeline {
        Pipeline { hook : self.hook.copy(), ops : self.ops.clone() }
    }

    pub fn get_pointer_field(&self, pointer_index : u16) -> Pipeline {
        let mut new_ops = Vec::with_capacity(self.ops.len() + 1);
        for &op in self.ops.iter() {
            new_ops.push(op)
        }
        new_ops.push(PipelineOp::GetPointerField(pointer_index));
        Pipeline { hook : self.hook.copy(), ops : new_ops }
    }

    pub fn as_cap(&self) -> Box<ClientHook+Send> {
        self.hook.get_pipelined_cap(self.ops.clone())
    }
}

