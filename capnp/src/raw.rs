// Copyright (c) 2018 the capnproto-rust contributors
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

//! Functions providing low level access to encoded data.

use crate::traits::{IntoInternalListReader, IntoInternalStructReader};

/// Gets a slice view of the data section of a struct.
pub fn get_struct_data_section<'a, T>(value: T) -> &'a [u8]
where
    T: IntoInternalStructReader<'a>,
{
    value
        .into_internal_struct_reader()
        .get_data_section_as_blob()
}

/// Gets the pointer section as a list.
pub fn get_struct_pointer_section<'a, T>(value: T) -> crate::any_pointer_list::Reader<'a>
where
    T: IntoInternalStructReader<'a>,
{
    crate::any_pointer_list::Reader::new(
        value
            .into_internal_struct_reader()
            .get_pointer_section_as_list(),
    )
}

/// Gets the size of the elements in a list.
pub fn get_list_element_size<'a, T>(value: T) -> crate::private::layout::ElementSize
where
    T: IntoInternalListReader<'a>,
{
    value.into_internal_list_reader().get_element_size()
}

/// Gets the number of bits between successive elements in a list.
pub fn get_list_step_size_in_bits<'a, T>(value: T) -> u32
where
    T: IntoInternalListReader<'a>,
{
    value.into_internal_list_reader().get_step_size_in_bits()
}

/// Gets a slice view of a list, excluding any tag word.
pub fn get_list_bytes<'a, T>(value: T) -> &'a [u8]
where
    T: IntoInternalListReader<'a>,
{
    value.into_internal_list_reader().into_raw_bytes()
}
