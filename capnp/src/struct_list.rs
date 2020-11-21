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

//! List of structs.

use core::marker::PhantomData;

use crate::private::arena::{ReaderArena, BuilderArena};
use crate::private::layout::{ListReader, ListBuilder, PointerReader, PointerBuilder, InlineComposite};
use crate::traits::{FromPointerReader, FromPointerBuilder,
                    FromStructBuilder, FromStructReader, HasStructSize,
                    IndexMove, ListIter};
use crate::Result;

#[derive(Copy, Clone)]
pub struct Owned<T> where T: crate::traits::OwnedStruct {
    marker: PhantomData<T>,
}

impl<T> crate::traits::Owned for Owned<T> where T: crate::traits::OwnedStruct {
    type Reader<'a, A: ReaderArena + 'a> = Reader<'a, A, T>;
    type Builder<'a, A: BuilderArena + 'a> = Builder<'a, A, T>;
}

#[derive(Copy, Clone)]
pub struct Reader<'a, A, T> where T: crate::traits::OwnedStruct {
    marker: PhantomData<T>,
    reader: ListReader<&'a A>
}

impl <'a, A, T> Reader<'a, A, T>
where T: crate::traits::OwnedStruct + Copy,
      A: ReaderArena
{
    pub fn new<'b>(reader: ListReader<&'b A>) -> Reader<'b, A, T> {
        Reader { reader: reader, marker: PhantomData }
    }

    pub fn len(&self) -> u32 { self.reader.len() }

    pub fn iter(self) -> ListIter<Reader<'a, A, T>, <T as crate::traits::OwnedStruct>::Reader<'a, A>> {
        let len = self.len();
        ListIter::new(self, len)
    }

    pub fn reborrow<'b>(&'b self) -> Reader<'b, A, T>  {
        Reader { reader: self.reader, marker: PhantomData }
    }

    pub fn get(self, index: u32) -> <T as crate::traits::OwnedStruct>::Reader<'a, A> {
        assert!(index < self.len());
        FromStructReader::new(self.reader.get_struct_element(index))
    }
}

impl <'a, A, T> FromPointerReader<'a, A> for Reader<'a, A, T>
where T: crate::traits::OwnedStruct,
      A: ReaderArena
{
    fn get_from_pointer(reader: PointerReader<&'a A>, default: Option<&'a [crate::Word]>) -> Result<Reader<'a, A, T>> {
        Ok(Reader { reader: reader.get_list(InlineComposite, default)?,
                    marker: PhantomData })
    }
}

impl <'a, A: Copy, T: Copy> IndexMove<u32, <T as crate::traits::OwnedStruct>::Reader<'a, A>> for Reader<'a, A, T>
where T: crate::traits::OwnedStruct,
      A: ReaderArena
{
    fn index_move(&self, index: u32) -> <T as crate::traits::OwnedStruct>::Reader<'a, A> {
        self.get(index)
    }
}

impl <'a, A, T> crate::traits::IntoInternalListReader<'a, A> for Reader<'a, A, T>
where T: crate::traits::OwnedStruct,
      A: ReaderArena
{
    fn into_internal_list_reader(self) -> ListReader<&'a A> {
        self.reader
    }
}

pub struct Builder<'a, A, T> where T: crate::traits::OwnedStruct {
    marker: PhantomData<T>,
    builder: ListBuilder<&'a mut A>
}

impl <'a, A, T> Builder<'a, A, T>
where T: crate::traits::OwnedStruct,
      A: BuilderArena
{
    pub fn new(builder: ListBuilder<&'a mut A>) -> Builder<'a, A, T> {
        Builder { builder: builder, marker: PhantomData }
    }

    pub fn len(&self) -> u32 { self.builder.len() }

    pub fn into_reader(self) -> Reader<'a, A, T> {
        Reader {
            marker: PhantomData,
            reader: self.builder.into_reader(),
        }
    }

    /// Sets the list element, with the following limitation based on the fact that structs in a
    /// struct list are allocated inline: if the source struct is larger than the target struct
    /// (as can happen if it was created with a newer version of the schema), then it will be
    /// truncated, losing fields.
    pub fn set_with_caveats<'b, B>(&mut self, index: u32, value: <T as crate::traits::OwnedStruct>::Reader<'b, B>)
               -> Result<()>
    where B: BuilderArena
    {
        use crate::traits::IntoInternalStructReader;
        self.builder.reborrow().get_struct_element(index).copy_content_from(&value.into_internal_struct_reader())
    }

    pub fn reborrow<'b>(&'b mut self) -> Builder<'b, A, T> {
        Builder { builder: self.builder.reborrow(), marker: PhantomData }
    }
}

impl <'a, A, T> FromPointerBuilder<'a, A> for Builder<'a, A, T>
where T: crate::traits::OwnedStruct,
      A: BuilderArena
{
    fn init_pointer(builder: PointerBuilder<&'a mut A>, size: u32) -> Builder<'a, A, T> {
        Builder {
            marker: PhantomData,
            builder: builder.init_struct_list(
                size,
                <<T as crate::traits::OwnedStruct>::Builder<'a, A> as HasStructSize>::struct_size())
        }
    }
    fn get_from_pointer(builder: PointerBuilder<&'a mut A>, default: Option<&'a [crate::Word]>)
                        -> Result<Builder<'a, A, T>>
    {
        Ok(Builder {
            marker: PhantomData,
            builder:
            builder.get_struct_list(<<T as crate::traits::OwnedStruct>::Builder<'a, A> as HasStructSize>::struct_size(),
                                    default)?
        })
    }
}

impl <'a, A, T> Builder<'a, A, T> where T: crate::traits::OwnedStruct, A: BuilderArena {
    pub fn get(self, index: u32) -> <T as crate::traits::OwnedStruct>::Builder<'a, A> {
        assert!(index < self.len());
        FromStructBuilder::new(self.builder.get_struct_element(index))
    }
}

impl <'a, A, T> crate::traits::SetPointerBuilder for Reader<'a, A, T>
    where T: crate::traits::OwnedStruct,
          A: ReaderArena
{
    fn set_pointer_builder<'b, B>(pointer: crate::private::layout::PointerBuilder<&'b mut B>,
                                  value: Reader<'a, A, T>,
                                  canonicalize: bool) -> Result<()>
        where B: BuilderArena
    {
        pointer.set_list(&value.reader, canonicalize)
    }
}

impl <'a, A, T> ::core::iter::IntoIterator for Reader<'a, A, T>
where T: crate::traits::OwnedStruct + Copy,
      A: ReaderArena + Copy
{
    type Item = <T as crate::traits::OwnedStruct>::Reader<'a, A>;
    type IntoIter = ListIter<Reader<'a, A, T>, Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
