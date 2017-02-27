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

use std::mem;
use std::ptr;

use data;
use text;
use private::capability::{ClientHook};
use private::arena::{BuilderArena, ReaderArena, NullArena, SegmentId};
use private::endian::{WireValue, Endian};
use private::mask::Mask;
use private::units::*;
use private::zero;
use {MessageSize, Result, Word};

pub use self::ElementSize::{Void, Bit, Byte, TwoBytes, FourBytes, EightBytes, Pointer, InlineComposite};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
pub enum ElementSize {
    Void = 0,
    Bit = 1,
    Byte = 2,
    TwoBytes = 3,
    FourBytes = 4,
    EightBytes = 5,
    Pointer = 6,
    InlineComposite = 7
}

impl ElementSize {
    fn from(val: u8) -> ElementSize {
        match val {
            0 => ElementSize::Void,
            1 => ElementSize::Bit,
            2 => ElementSize::Byte,
            3 => ElementSize::TwoBytes,
            4 => ElementSize::FourBytes,
            5 => ElementSize::EightBytes,
            6 => ElementSize::Pointer,
            7 => ElementSize::InlineComposite,
            _ => panic!("illegal element size: {}", val),
        }
    }
}

pub fn data_bits_per_element(size: ElementSize) -> BitCount32 {
    match size {
        Void => 0,
        Bit => 1,
        Byte => 8,
        TwoBytes => 16,
        FourBytes => 32,
        EightBytes => 64,
        Pointer => 0,
        InlineComposite => 0
    }
}

pub fn pointers_per_element(size: ElementSize) -> WirePointerCount32 {
    match size {
        Pointer => 1,
        _ => 0
    }
}

#[derive(Clone, Copy)]
pub struct StructSize {
    pub data: WordCount16,
    pub pointers: WirePointerCount16,
}

impl StructSize {
    pub fn total(&self) -> WordCount32 {
        self.data as WordCount32
            + self.pointers as WordCount32
            * WORDS_PER_POINTER as WordCount32
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
pub enum WirePointerKind {
    Struct = 0,
    List = 1,
    Far = 2,
    Other = 3
}

impl WirePointerKind {
    fn from(val: u8) -> WirePointerKind {
        match val {
            0 => WirePointerKind::Struct,
            1 => WirePointerKind::List,
            2 => WirePointerKind::Far,
            3 => WirePointerKind::Other,
            _ => panic!("illegal element size: {}", val),
        }
    }
}

#[repr(C)]
pub struct WirePointer {
    offset_and_kind: WireValue<u32>,
    upper32bits: u32,
}

#[repr(C)]
pub struct StructRef {
    data_size: WireValue<WordCount16>,
    ptr_count: WireValue<WirePointerCount16>
}

#[repr(C)]
pub struct ListRef {
    element_size_and_count: WireValue<u32>
}

#[repr(C)]
pub struct FarRef {
    segment_id: WireValue<u32>
}

#[repr(C)]
pub struct CapRef {
    index: WireValue<u32>
}

impl StructRef {
    pub fn word_size(&self) -> WordCount32 {
        self.data_size.get() as WordCount32 +
            self.ptr_count.get() as WordCount32 * WORDS_PER_POINTER as u32
    }

    #[inline]
    pub fn set_from_struct_size(&mut self, size: StructSize) {
        self.data_size.set(size.data);
        self.ptr_count.set(size.pointers);
    }

    #[inline]
    pub fn set(&mut self, ds: WordCount16, rc: WirePointerCount16) {
        self.data_size.set(ds);
        self.ptr_count.set(rc);
    }
}

impl ListRef {
    #[inline]
    pub fn element_size(&self) -> ElementSize {
        ElementSize::from(self.element_size_and_count.get() as u8 & 7)
    }

    #[inline]
    pub fn element_count(&self) -> ElementCount32 {
        self.element_size_and_count.get() >> 3
    }

    #[inline]
    pub fn inline_composite_word_count(&self) -> WordCount32 {
        self.element_count()
    }

    #[inline]
    pub fn set(&mut self, es: ElementSize, ec: ElementCount32) {
        assert!(ec < (1 << 29), "Lists are limited to 2**29 elements");
        self.element_size_and_count.set((ec << 3 ) | (es as u32));
    }

    #[inline]
    pub fn set_inline_composite(&mut self, wc: WordCount32) {
        assert!(wc < (1 << 29), "Inline composite lists are limited to 2**29 words");
        self.element_size_and_count.set((wc << 3) | (InlineComposite as u32));
    }
}

impl FarRef {
    #[inline]
    pub fn set(&mut self, si: SegmentId) { self.segment_id.set(si); }
}

impl CapRef {
    #[inline]
    pub fn set(&mut self, index: u32) { self.index.set(index); }
}

impl WirePointer {

    #[inline]
    pub fn kind(&self) -> WirePointerKind {
        WirePointerKind::from(self.offset_and_kind.get() as u8 & 3)
    }

    #[inline]
    pub fn is_positional(&self) -> bool {
        (self.offset_and_kind.get() & 2) == 0 // match Struct and List but not Far and Other.
    }

    #[inline]
    pub fn is_capability(&self) -> bool {
        self.offset_and_kind.get() == WirePointerKind::Other as u32
    }

    #[inline]
    pub fn target(&self) -> *const Word {
        let this_addr: *const Word = self as *const _ as *const _;
        unsafe { this_addr.offset((1 + ((self.offset_and_kind.get() as i32) >> 2)) as isize) }
    }

    #[inline]
    pub fn mut_target(&mut self) -> *mut Word {
        let this_addr: *mut Word = self as *mut _ as *mut _;
        unsafe { this_addr.offset((1 + ((self.offset_and_kind.get() as i32) >> 2)) as isize) }
    }

    #[inline]
    pub fn set_kind_and_target(&mut self, kind: WirePointerKind, target: *mut Word) {
        let this_addr: isize = self as *const _ as isize;
        let target_addr: isize = target as *const _ as isize;
        self.offset_and_kind.set(
            ((((target_addr - this_addr) / BYTES_PER_WORD as isize) as i32 - 1) << 2) as u32
                | (kind as u32))
    }

    #[inline]
    pub fn set_kind_with_zero_offset(&mut self, kind: WirePointerKind) {
        self.offset_and_kind.set(kind as u32)
    }

    #[inline]
    pub fn set_kind_and_target_for_empty_struct(&mut self) {
        //# This pointer points at an empty struct. Assuming the
        //# WirePointer itself is in-bounds, we can set the target to
        //# point either at the WirePointer itself or immediately after
        //# it. The latter would cause the WirePointer to be "null"
        //# (since for an empty struct the upper 32 bits are going to
        //# be zero). So we set an offset of -1, as if the struct were
        //# allocated immediately before this pointer, to distinguish
        //# it from null.

        self.offset_and_kind.set(0xfffffffc);
    }

    #[inline]
    pub fn inline_composite_list_element_count(&self) -> ElementCount32 {
        self.offset_and_kind.get() >> 2
    }

    #[inline]
    pub fn set_kind_and_inline_composite_list_element_count(&mut self,
                                                            kind: WirePointerKind,
                                                            element_count: ElementCount32) {
        self.offset_and_kind.set(( element_count << 2) | (kind as u32))
    }

    #[inline]
    pub fn far_position_in_segment(&self) -> WordCount32 {
        self.offset_and_kind.get() >> 3
    }

    #[inline]
    pub fn is_double_far(&self) -> bool {
        ((self.offset_and_kind.get() >> 2) & 1) != 0
    }

    #[inline]
    pub fn set_far(&mut self, is_double_far: bool, pos: WordCount32) {
        self.offset_and_kind
            .set(( pos << 3) | ((is_double_far as u32) << 2) | WirePointerKind::Far as u32);
    }

    #[inline]
    pub fn set_cap(&mut self, index: u32) {
        self.offset_and_kind.set(WirePointerKind::Other as u32);
        self.mut_cap_ref().set(index);
    }

    #[inline]
    pub fn struct_ref<'a>(&'a self) -> &'a StructRef {
        unsafe { mem::transmute(&self.upper32bits) }
    }

    #[inline]
    pub fn mut_struct_ref<'a>(&'a mut self) -> &'a mut StructRef {
        unsafe { mem::transmute(&mut self.upper32bits) }
    }

    #[inline]
    pub fn list_ref<'a>(&'a self) -> &'a ListRef {
        unsafe { mem::transmute(&self.upper32bits) }
    }

    #[inline]
    pub fn mut_list_ref<'a>(&'a mut self) -> &'a mut ListRef {
        unsafe { mem::transmute(&mut self.upper32bits) }
    }

    #[inline]
    pub fn far_ref<'a>(&'a self) -> &'a FarRef {
        unsafe { mem::transmute(&self.upper32bits) }
    }

    #[inline]
    pub fn mut_far_ref<'a>(&'a mut self) -> &'a mut FarRef {
        unsafe { mem::transmute(&mut self.upper32bits) }
    }

    #[inline]
    pub fn cap_ref<'a>(&'a self) -> &'a CapRef {
        unsafe { mem::transmute(&self.upper32bits) }
    }

    #[inline]
    pub fn mut_cap_ref<'a>(&'a mut self) -> &'a mut CapRef {
        unsafe { mem::transmute(&mut self.upper32bits) }
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.offset_and_kind.get() == 0 && self.upper32bits == 0
    }
}

mod wire_helpers {
    use std::{mem, ptr, slice};

    use private::capability::ClientHook;
    use private::arena::*;
    use private::layout::{
        CapTableBuilder, CapTableReader, ElementSize, ListBuilder, ListReader,
        StructBuilder, StructReader, StructSize, WirePointer, WirePointerKind};
    use private::layout::{data_bits_per_element, pointers_per_element};
    use private::layout::ElementSize::*;
    use private::units::*;
    use data;
    use text;
    use {Error, MessageSize, Result, Word};

    pub struct SegmentAnd<T> {
        #[allow(dead_code)]
        segment_id: u32,
        pub value: T,
    }

    #[inline]
    pub fn round_bytes_up_to_words(bytes: ByteCount32) -> WordCount32 {
        //# This code assumes 64-bit words.
        (bytes + 7) / BYTES_PER_WORD as u32
    }

    //# The maximum object size is 4GB - 1 byte. If measured in bits,
    //# this would overflow a 32-bit counter, so we need to accept
    //# BitCount64. However, 32 bits is enough for the returned
    //# ByteCounts and WordCounts.
    #[inline]
    pub fn round_bits_up_to_words(bits: BitCount64) -> WordCount32 {
        //# This code assumes 64-bit words.
        ((bits + 63) / (BITS_PER_WORD as u64)) as WordCount32
    }

    #[allow(dead_code)]
    #[inline]
    pub fn round_bits_up_to_bytes(bits: BitCount64) -> ByteCount32 {
        ((bits + 7) / (BITS_PER_BYTE as u64)) as ByteCount32
    }

    #[inline]
    pub fn bounds_check(arena: &ReaderArena,
                        segment_id: u32,
                        start: *const Word, end: *const Word,
                        _kind: WirePointerKind) -> Result<()> {
        arena.contains_interval(segment_id, start, end)
    }

    #[inline]
    pub fn amplified_read(arena: &ReaderArena,
                          virtual_amount: u64) -> Result<()> {
        arena.amplified_read(virtual_amount)
    }

    #[inline]
    pub unsafe fn allocate(
        arena: &BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        amount: WordCount32, kind: WirePointerKind) -> (*mut Word, *mut WirePointer, u32)
    {
        let is_null = (*reff).is_null();
        if !is_null {
            zero_object(arena, segment_id, reff)
        }

        if amount == 0 && kind == WirePointerKind::Struct {
            (*reff).set_kind_and_target_for_empty_struct();
            return (reff as *mut _, reff, segment_id);
        }

        match arena.allocate(segment_id, amount) {
            None => {

                //# Need to allocate in a different segment. We'll need to
                //# allocate an extra pointer worth of space to act as
                //# the landing pad for a far pointer.

                let amount_plus_ref = amount + POINTER_SIZE_IN_WORDS as u32;
                let (segment_id, word_idx) = arena.allocate_anywhere(amount_plus_ref);
                let (seg_start, _seg_len) = arena.get_segment_mut(segment_id);
                let ptr: *mut Word = seg_start.offset(word_idx as isize);

                //# Set up the original pointer to be a far pointer to
                //# the new segment.
                (*reff).set_far(false, word_idx);
                (*reff).mut_far_ref().segment_id.set(segment_id);

                //# Initialize the landing pad to indicate that the
                //# data immediately follows the pad.
                let reff = ptr as *mut WirePointer;

                let ptr1 = ptr.offset(POINTER_SIZE_IN_WORDS as isize);
                (*reff).set_kind_and_target(kind, ptr1);
                return (ptr1, reff, segment_id);
            }
            Some(idx) => {
                let (seg_start, _seg_len) = arena.get_segment_mut(segment_id);
                let ptr: *mut Word = seg_start.offset(idx as isize);
                (*reff).set_kind_and_target(kind, ptr);
                return (ptr, reff, segment_id);
            }
        }
    }

    #[inline]
    pub unsafe fn follow_builder_fars(
        arena: &BuilderArena,
        reff: *mut WirePointer,
        ref_target: *mut Word,
        segment_id: u32) -> Result<(*mut Word, *mut WirePointer, u32)>
    {
        // If `ref` is a far pointer, follow it. On return, `ref` will have been updated to point at
        // a WirePointer that contains the type information about the target object, and a pointer
        // to the object contents is returned. The caller must NOT use `ref->target()` as this may
        // or may not actually return a valid pointer. `segment` is also updated to point at the
        // segment which actually contains the object.
        //
        // If `ref` is not a far pointer, this simply returns `ref_target`. Usually, `ref_target`
        // should be the same as `ref->target()`, but may not be in cases where `ref` is only a tag.

        if (*reff).kind() == WirePointerKind::Far {
            let segment_id = (*reff).far_ref().segment_id.get();
            let (seg_start, _seg_len) = arena.get_segment_mut(segment_id);
            let pad: *mut WirePointer =
                seg_start.offset((*reff).far_position_in_segment() as isize) as *mut _;
            if !(*reff).is_double_far() {
                Ok(((*pad).mut_target(), pad, segment_id))
            } else {
                //# Landing pad is another far pointer. It is followed by a
                //# tag describing the pointed-to object.
                let reff = pad.offset(1);

                let segment_id = (*pad).far_ref().segment_id.get();
                let (segment_start, _segment_len) = arena.get_segment_mut(segment_id);
                let ptr = segment_start.offset((*pad).far_position_in_segment() as isize);
                Ok((ptr, reff, segment_id))
            }
        } else {
            Ok((ref_target, reff, segment_id))
        }
    }

    #[inline]
    pub unsafe fn follow_fars(
        arena: &ReaderArena,
        reff: *const WirePointer,
        ref_target: *const Word,
        mut segment_id: u32)
        -> Result<(*const Word, *const WirePointer, u32)>
    {
        if (*reff).kind() == WirePointerKind::Far {
            segment_id = (*reff).far_ref().segment_id.get();

            let (seg_start, _seg_len) = try!(arena.get_segment(segment_id));
            let ptr: *const Word = seg_start.offset((*reff).far_position_in_segment() as isize);

            let pad_words: isize = if (*reff).is_double_far() { 2 } else { 1 };
            try!(bounds_check(arena, segment_id, ptr, ptr.offset(pad_words), WirePointerKind::Far));

            let pad: *const WirePointer = ptr as *const _;

            if !(*reff).is_double_far() {
                Ok(((*pad).target(), pad, segment_id))
            } else {
                //# Landing pad is another far pointer. It is
                //# followed by a tag describing the pointed-to
                //# object.

                let reff = pad.offset(1);

                let segment_id = (*pad).far_ref().segment_id.get();
                let (segment_start, _segment_len) = try!(arena.get_segment(segment_id));
                let ptr = segment_start.offset((*pad).far_position_in_segment() as isize);
                Ok((ptr, reff, segment_id))
            }
        } else {
            Ok((ref_target, reff, segment_id))
        }
    }

    pub unsafe fn zero_object(
        arena: &BuilderArena,
        segment_id: u32,
        reff: *mut WirePointer)
    {
        //# Zero out the pointed-to object. Use when the pointer is
        //# about to be overwritten making the target object no longer
        //# reachable.

        match (*reff).kind() {
            WirePointerKind::Struct | WirePointerKind::List | WirePointerKind::Other => {
                zero_object_helper(arena, segment_id, reff, (*reff).mut_target())
            }
            WirePointerKind::Far => {
                let segment_id = (*reff).far_ref().segment_id.get();
                let (seg_start, _seg_len) = arena.get_segment_mut(segment_id);
                let pad: *mut WirePointer =
                    seg_start.offset((*reff).far_position_in_segment() as isize) as *mut _;

                if (*reff).is_double_far() {
                    let segment_id = (*pad).far_ref().segment_id.get();

                    let (seg_start, _seg_len) = arena.get_segment_mut(segment_id);
                    let ptr: *mut Word = seg_start.offset((*pad).far_position_in_segment() as isize);
                    zero_object_helper(arena,
                                       segment_id,
                                       pad.offset(1),
                                       ptr);

                    ptr::write_bytes(pad, 0u8, 2);

                } else {
                    zero_object(arena, segment_id, pad);
                    ptr::write_bytes(pad, 0u8, 1);
                }
            }
        }
    }

    pub unsafe fn zero_object_helper(
        arena: &BuilderArena,
        segment_id: u32,
        tag: *mut WirePointer,
        ptr: *mut Word)
    {
        match (*tag).kind() {
            WirePointerKind::Other => { panic!("Don't know how to handle OTHER") }
            WirePointerKind::Struct => {
                let pointer_section: *mut WirePointer =
                    ptr.offset((*tag).struct_ref().data_size.get() as isize) as *mut _;

                let count = (*tag).struct_ref().ptr_count.get() as isize;
                for i in 0..count {
                    zero_object(arena, segment_id, pointer_section.offset(i));
                }
                ptr::write_bytes(ptr, 0u8, (*tag).struct_ref().word_size() as usize);
            }
            WirePointerKind::List => {
                match (*tag).list_ref().element_size() {
                    Void =>  { }
                    Bit | Byte | TwoBytes | FourBytes | EightBytes => {
                        ptr::write_bytes(
                            ptr, 0u8,
                            round_bits_up_to_words((
                                    (*tag).list_ref().element_count() *
                                        data_bits_per_element(
                                        (*tag).list_ref().element_size())) as u64) as usize)
                    }
                    Pointer => {
                        let count = (*tag).list_ref().element_count() as usize;
                        for i in 0..count as isize {
                            zero_object(arena, segment_id, ptr.offset(i) as *mut _);
                        }
                        ptr::write_bytes(ptr, 0u8, count);
                    }
                    InlineComposite => {
                        let element_tag: *mut WirePointer = ptr as *mut _;

                        assert!((*element_tag).kind() == WirePointerKind::Struct,
                                "Don't know how to handle non-STRUCT inline composite");

                        let data_size = (*element_tag).struct_ref().data_size.get();
                        let pointer_count = (*element_tag).struct_ref().ptr_count.get();
                        let mut pos: *mut Word = ptr.offset(1);
                        let count = (*element_tag).inline_composite_list_element_count();
                        if pointer_count > 0 {
                            for _ in 0..count {
                                pos = pos.offset(data_size as isize);
                                for _ in 0..pointer_count {
                                    zero_object(arena, segment_id, pos as *mut WirePointer);
                                    pos = pos.offset(1);
                                }
                            }
                        }
                        ptr::write_bytes(ptr, 0u8,
                                         ((*element_tag).struct_ref().word_size() * count + 1) as usize);
                    }
                }
            }
            WirePointerKind::Far => { panic!("Unexpected FAR pointer") }
        }
    }

    #[inline]
    pub unsafe fn zero_pointer_and_fars(
        arena: &BuilderArena,
        _segment_id: u32,
        reff: *mut WirePointer) -> Result<()>
    {
        // Zero out the pointer itself and, if it is a far pointer, zero the landing pad as well,
        // but do not zero the object body. Used when upgrading.

        if (*reff).kind() == WirePointerKind::Far {
            let far_segment_id = (*reff).far_ref().segment_id.get();
            let (seg_start, _seg_len) = arena.get_segment_mut(far_segment_id);
            let pad: *mut Word = seg_start.offset((*reff).far_position_in_segment() as isize);
            let num_elements = if (*reff).is_double_far() { 2 } else { 1 };
            ptr::write_bytes(pad, 0, num_elements);
        }
        ptr::write_bytes(reff, 0, 1);
        Ok(())
    }

    pub unsafe fn total_size(
        arena: &ReaderArena,
        segment_id: u32,
        reff: *const WirePointer,
        mut nesting_limit: i32) -> Result<MessageSize>
    {
        let mut result = MessageSize { word_count: 0, cap_count: 0};

        if (*reff).is_null() { return Ok(result) };

        if nesting_limit <= 0 {
            return Err(Error::failed("Message is too deeply nested.".to_string()));
        }

        nesting_limit -= 1;

        let (ptr, reff, segment_id) = try!(follow_fars(arena, reff, (*reff).target(), segment_id));

        match (*reff).kind() {
            WirePointerKind::Struct => {
                try!(bounds_check(arena, segment_id,
                                  ptr, ptr.offset((*reff).struct_ref().word_size() as isize),
                                  WirePointerKind::Struct));
                result.word_count += (*reff).struct_ref().word_size() as u64;

                let pointer_section: *const WirePointer =
                    ptr.offset((*reff).struct_ref().data_size.get() as isize) as *const _;
                let count: isize = (*reff).struct_ref().ptr_count.get() as isize;
                for i in 0..count {
                    result.plus_eq(try!(total_size(arena, segment_id, pointer_section.offset(i), nesting_limit)));
                }
            }
            WirePointerKind::List => {
                match (*reff).list_ref().element_size() {
                    Void => {}
                    Bit | Byte | TwoBytes | FourBytes | EightBytes => {
                        let total_words = round_bits_up_to_words(
                            (*reff).list_ref().element_count() as u64 *
                                data_bits_per_element((*reff).list_ref().element_size()) as u64);
                        try!(bounds_check(
                            arena, segment_id, ptr, ptr.offset(total_words as isize), WirePointerKind::List));
                        result.word_count += total_words as u64;
                    }
                    Pointer => {
                        let count = (*reff).list_ref().element_count();
                        try!(bounds_check(
                            arena, segment_id, ptr, ptr.offset((count * WORDS_PER_POINTER as u32) as isize),
                            WirePointerKind::List));

                        result.word_count += count as u64 * WORDS_PER_POINTER as u64;

                        for i in 0..count as isize {
                            result.plus_eq(
                                try!(total_size(arena, segment_id,
                                                (ptr as *const WirePointer).offset(i),
                                                nesting_limit)));
                        }
                    }
                    InlineComposite => {
                        let word_count = (*reff).list_ref().inline_composite_word_count();
                        try!(bounds_check(arena, segment_id, ptr,
                                          ptr.offset(word_count as isize + POINTER_SIZE_IN_WORDS as isize),
                                          WirePointerKind::List));

                        if word_count == 0 {
                            return Ok(result);
                        }

                        let element_tag: *const WirePointer = ptr as *const _;
                        let count = (*element_tag).inline_composite_list_element_count();

                        if (*element_tag).kind() != WirePointerKind::Struct {
                            return Err(Error::failed(
                                "Don't know how to handle non-STRUCT inline composite.".to_string()));
                        }

                        let actual_size = (*element_tag).struct_ref().word_size() as u64 * count as u64;
                        if actual_size > word_count as u64 {
                            return Err(Error::failed(
                                "InlineComposite list's elements overrun its word count.".to_string()));
                        }

                        // Count the actual size rather than the claimed word count because
                        // that's what we end up with if we make a copy.
                        result.word_count += actual_size as u64 + POINTER_SIZE_IN_WORDS as u64;

                        let data_size = (*element_tag).struct_ref().data_size.get();
                        let pointer_count = (*element_tag).struct_ref().ptr_count.get();

                        if pointer_count > 0 {
                            let mut pos: *const Word = ptr.offset(POINTER_SIZE_IN_WORDS as isize);
                            for _ in 0..count {
                                pos = pos.offset(data_size as isize);

                                for _ in 0..pointer_count {
                                    result.plus_eq(
                                        try!(total_size(arena, segment_id,
                                                        pos as *const WirePointer, nesting_limit)));
                                    pos = pos.offset(POINTER_SIZE_IN_WORDS as isize);
                                }
                            }
                        }
                    }
                }
            }
            WirePointerKind::Far => {
                return Err(Error::failed("Malformed double-far pointer.".to_string()));
            }
            WirePointerKind::Other => {
                if (*reff).is_capability() {
                    result.cap_count += 1;
                } else {
                    return Err(Error::failed("Unknown pointer type.".to_string()));
                }
            }
        }

        Ok(result)
    }

    pub unsafe fn transfer_pointer(
        arena: &BuilderArena,
        dst_segment_id: u32, dst: *mut WirePointer,
        src_segment_id: u32, src: *mut WirePointer)
    {
        //# Make *dst point to the same object as *src. Both must
        //# reside in the same message, but can be in different
        //# segments. Not always-inline because this is rarely used.
        //
        //# Caller MUST zero out the source pointer after calling this,
        //# to make sure no later code mistakenly thinks the source
        //# location still owns the object. transferPointer() doesn't
        //# do this zeroing itself because many callers transfer
        //# several pointers in a loop then zero out the whole section.

        assert!((*dst).is_null());
        // We expect the caller to ensure the target is already null so won't leak.

        if (*src).is_null() {
            ptr::write_bytes(dst, 0, 1);
        } else if (*src).is_positional() {
            transfer_pointer_split(arena, dst_segment_id, dst, src_segment_id, src, (*src).mut_target());
        } else {
            ptr::copy_nonoverlapping(src, dst, 1);
        }
    }

    pub unsafe fn transfer_pointer_split(
        arena: &BuilderArena,
        dst_segment_id: u32, dst: *mut WirePointer,
        src_segment_id: u32, src_tag: *mut WirePointer,
        src_ptr: *mut Word)
    {
        // Like the other transfer_pointer, but splits src into a tag and a
        // target. Particularly useful for OrphanBuilder.

        if dst_segment_id == src_segment_id {
            // Same segment, so create a direct pointer.

            if (*src_tag).kind() == WirePointerKind::Struct && (*src_tag).struct_ref().word_size() == 0 {
                (*dst).set_kind_and_target_for_empty_struct();
            } else {
                (*dst).set_kind_and_target((*src_tag).kind(), src_ptr);
            }
            // We can just copy the upper 32 bits. (Use memcpy() to comply with aliasing rules.)
            ptr::copy_nonoverlapping(&(*src_tag).upper32bits, &mut (*dst).upper32bits, 1);
        } else {
            // Need to create a far pointer. Try to allocate it in the same segment as the source,
            // so that it doesn't need to be a double-far.

            match arena.allocate(src_segment_id, 1) {
                None => {
                    //# Darn, need a double-far.
                    let (far_segment_id, word_idx) = arena.allocate_anywhere(2);
                    let (seg_start, _seg_len) = arena.get_segment_mut(far_segment_id);
                    let landing_pad: *mut WirePointer = seg_start.offset(word_idx as isize) as *mut _;

                    let (src_seg_start, _seg_len) = arena.get_segment_mut(src_segment_id);

                    (*landing_pad).set_far(
                        false,
                        ((src_ptr as usize - src_seg_start as usize) / BYTES_PER_WORD) as u32);
                    (*landing_pad).mut_far_ref().segment_id.set(src_segment_id);

                    let landing_pad1 = landing_pad.offset(1);
                    (*landing_pad1).set_kind_with_zero_offset((*src_tag).kind());

                    ptr::copy_nonoverlapping(
                        &(*src_tag).upper32bits,
                        &mut (*landing_pad1).upper32bits,
                        1);

                    (*dst).set_far(true, word_idx);
                    (*dst).mut_far_ref().set(far_segment_id);
                }
                Some(landing_pad_word) => {
                    //# Simple landing pad is just a pointer.
                    let (seg_start, seg_len) = arena.get_segment_mut(src_segment_id);
                    assert!(landing_pad_word < seg_len);
                    let landing_pad: *mut WirePointer = seg_start.offset(landing_pad_word as isize) as *mut _;
                    (*landing_pad).set_kind_and_target((*src_tag).kind(), src_ptr);
                    ptr::copy_nonoverlapping(&(*src_tag).upper32bits,
                                             &mut (*landing_pad).upper32bits, 1);

                    (*dst).set_far(false, landing_pad_word);
                    (*dst).mut_far_ref().set(src_segment_id);
                }
            }
        }
    }

    #[inline]
    pub unsafe fn init_struct_pointer<'a>(
        arena: &'a BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        cap_table: CapTableBuilder,
        size: StructSize) -> StructBuilder<'a>
    {
        let (ptr, reff, segment_id) = allocate(
            arena,
            reff,
            segment_id,
            size.total(),
            WirePointerKind::Struct);
        (*reff).mut_struct_ref().set_from_struct_size(size);

        StructBuilder {
            arena: arena,
            segment_id: segment_id,
            cap_table: cap_table,
            data: ptr as *mut _,
            pointers: ptr.offset((size.data as usize) as isize) as *mut _,
            data_size: size.data as WordCount32 * (BITS_PER_WORD as BitCount32),
            pointer_count: size.pointers,
        }
    }

    #[inline]
    pub unsafe fn get_writable_struct_pointer<'a>(
        arena: &'a BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        cap_table: CapTableBuilder,
        size: StructSize,
        default_value: *const Word) -> Result<StructBuilder<'a>>
    {
        let ref_target = (*reff).mut_target();

        if (*reff).is_null() {
            if default_value.is_null() || (*(default_value as *const WirePointer)).is_null() {
                return Ok(init_struct_pointer(arena, reff, segment_id, cap_table, size));
            }
            unimplemented!()
        }

        let (old_ptr, old_ref, old_segment_id) = try!(follow_builder_fars(arena, reff, ref_target, segment_id));
        if (*old_ref).kind() != WirePointerKind::Struct {
            return Err(Error::failed(
                "Message contains non-struct pointer where struct pointer was expected.".to_string()));
        }

        let old_data_size = (*old_ref).struct_ref().data_size.get();
        let old_pointer_count = (*old_ref).struct_ref().ptr_count.get();
        let old_pointer_section: *mut WirePointer = old_ptr.offset(old_data_size as isize) as *mut _;

        if old_data_size < size.data || old_pointer_count < size.pointers {
            //# The space allocated for this struct is too small.
            //# Unlike with readers, we can't just run with it and do
            //# bounds checks at access time, because how would we
            //# handle writes? Instead, we have to copy the struct to a
            //# new space now.

            let new_data_size = ::std::cmp::max(old_data_size, size.data);
            let new_pointer_count = ::std::cmp::max(old_pointer_count, size.pointers);
            let total_size = new_data_size as u32 + new_pointer_count as u32 * WORDS_PER_POINTER as u32;

            //# Don't let allocate() zero out the object just yet.
            try!(zero_pointer_and_fars(arena, segment_id, reff));

            let (ptr, reff, segment_id) = allocate(arena, reff, segment_id, total_size, WirePointerKind::Struct);
            (*reff).mut_struct_ref().set(new_data_size, new_pointer_count);

            // Copy data section.
            // Note: copy_nonoverlapping's third argument is an element count, not a byte count.
            ptr::copy_nonoverlapping(old_ptr, ptr, old_data_size as usize);

            //# Copy pointer section.
            let new_pointer_section: *mut WirePointer = ptr.offset(new_data_size as isize) as *mut _;
            for i in 0..old_pointer_count as isize {
                transfer_pointer(arena, segment_id, new_pointer_section.offset(i),
                                 old_segment_id, old_pointer_section.offset(i));
            }

            ptr::write_bytes(old_ptr, 0, old_data_size as usize + old_pointer_count as usize);

            Ok(StructBuilder {
                arena: arena,
                segment_id: segment_id,
                cap_table: cap_table,
                data: ptr as *mut _,
                pointers: new_pointer_section,
                data_size: new_data_size as u32 * BITS_PER_WORD as u32,
                pointer_count: new_pointer_count
            })
        } else {
            Ok(StructBuilder {
                arena: arena,
                segment_id: old_segment_id,
                cap_table: cap_table,
                data: old_ptr as *mut _,
                pointers: old_pointer_section,
                data_size: old_data_size as u32 * BITS_PER_WORD as u32,
                pointer_count: old_pointer_count
            })
        }
    }

    #[inline]
    pub unsafe fn init_list_pointer<'a>(
        arena: &'a BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        cap_table: CapTableBuilder,
        element_count: ElementCount32,
        element_size: ElementSize) -> ListBuilder<'a>
    {
        assert!(element_size != InlineComposite,
                "Should have called initStructListPointer() instead");

        let data_size = data_bits_per_element(element_size);
        let pointer_count = pointers_per_element(element_size);
        let step = data_size + pointer_count * BITS_PER_POINTER as u32;
        let word_count = round_bits_up_to_words(element_count as ElementCount64 * (step as u64));
        let (ptr, reff, segment_id) = allocate(arena, reff, segment_id, word_count, WirePointerKind::List);

        (*reff).mut_list_ref().set(element_size, element_count);

        ListBuilder {
            arena: arena,
            segment_id: segment_id,
            cap_table: cap_table,
            ptr: ptr as *mut _,
            step: step,
            element_count: element_count,
            struct_data_size: data_size,
            struct_pointer_count: pointer_count as u16
        }
    }

    #[inline]
    pub unsafe fn init_struct_list_pointer<'a>(
        arena: &'a BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        cap_table: CapTableBuilder,
        element_count: ElementCount32,
        element_size: StructSize) -> ListBuilder<'a>
    {
        let words_per_element = element_size.total();

        //# Allocate the list, prefixed by a single WirePointer.
        let word_count: WordCount32 = element_count * words_per_element;
        let (ptr, reff, segment_id) = allocate(arena,
                                               reff,
                                               segment_id,
                                               POINTER_SIZE_IN_WORDS as u32 + word_count,
                                               WirePointerKind::List);
        let ptr = ptr as *mut WirePointer;

        //# Initialize the pointer.
        (*reff).mut_list_ref().set_inline_composite(word_count);
        (*ptr).set_kind_and_inline_composite_list_element_count(WirePointerKind::Struct, element_count);
        (*ptr).mut_struct_ref().set_from_struct_size(element_size);

        let ptr1 = ptr.offset(POINTER_SIZE_IN_WORDS as isize);

        ListBuilder {
            arena: arena,
            segment_id: segment_id,
            cap_table: cap_table,
            ptr: ptr1 as *mut _,
            step: words_per_element * BITS_PER_WORD as u32,
            element_count: element_count,
            struct_data_size: element_size.data as u32 * (BITS_PER_WORD as u32),
            struct_pointer_count: element_size.pointers
        }
    }

    #[inline]
    pub unsafe fn get_writable_list_pointer<'a>(
        arena: &'a BuilderArena,
        orig_ref: *mut WirePointer,
        orig_segment_id: u32,
        cap_table: CapTableBuilder,
        element_size: ElementSize,
        default_value: *const Word) -> Result<ListBuilder<'a>>
    {
        assert!(element_size != InlineComposite,
                "Use get_struct_list_{element,field}() for structs");

        let orig_ref_target = (*orig_ref).mut_target();

        if (*orig_ref).is_null() {
            if default_value.is_null() || (*(default_value as *const WirePointer)).is_null() {
                    return Ok(ListBuilder::new_default());
                }
            unimplemented!()
        }

        // We must verify that the pointer has the right size. Unlike in
        // get_writable_struct_list_pointer(), we never need to "upgrade" the data, because this
        // method is called only for non-struct lists, and there is no allowed upgrade path *to* a
        // non-struct list, only *from* them.

        let (mut ptr, reff, segment_id) =
            try!(follow_builder_fars(arena, orig_ref, orig_ref_target, orig_segment_id));

        if (*reff).kind() != WirePointerKind::List {
            return Err(Error::failed(
                "Called get_list_{{field,element}}() but existing pointer is not a list.".to_string()));
        }

        let old_size = (*reff).list_ref().element_size();

        if old_size == InlineComposite {
            // The existing element size is InlineComposite, which means that it is at least two
            // words, which makes it bigger than the expected element size. Since fields can only
            // grow when upgraded, the existing data must have been written with a newer version of
            // the protocol. We therefore never need to upgrade the data in this case, but we do
            // need to validate that it is a valid upgrade from what we expected.

            // Read the tag to get the actual element count.
            let tag: *const WirePointer = ptr as *const _;

            if (*tag).kind() != WirePointerKind::Struct {
                return Err(Error::failed(
                    "InlineComposite list with non-STRUCT elements not supported.".to_string()));
            }

            ptr = ptr.offset(POINTER_SIZE_IN_WORDS as isize);

            let data_size = (*tag).struct_ref().data_size.get();
            let pointer_count = (*tag).struct_ref().ptr_count.get();

            match element_size {
                Void => {} // Anything is a valid upgrade from Void.
                Bit => {
                    return Err(Error::failed(
                        "Found struct list where bit list was expected.".to_string()));
                }
                Byte | TwoBytes | FourBytes | EightBytes => {
                    if data_size < 1 {
                        return Err(Error::failed(
                            "Existing list value is incompatible with expected type.".to_string()));
                    }
                }
                Pointer => {
                    if pointer_count < 1 {
                        return Err(Error::failed(
                            "Existing list value is incompatible with expected type.".to_string()));
                    }
                    // Adjust the pointer to point at the reference segment.
                    ptr = ptr.offset(data_size as isize);
                }
                InlineComposite => {
                    unreachable!()
                }
            }
            // OK, looks valid.

            Ok(ListBuilder {
                arena: arena,
                segment_id: segment_id,
                cap_table: cap_table,
                ptr: ptr as *mut _,
                element_count: (*tag).inline_composite_list_element_count(),
                step: (*tag).struct_ref().word_size() * BITS_PER_WORD as u32,
                struct_data_size: data_size as u32 * BITS_PER_WORD as u32,
                struct_pointer_count: pointer_count
            })
        } else {
            let data_size = data_bits_per_element(old_size);
            let pointer_count = pointers_per_element(old_size);

            if data_size < data_bits_per_element(element_size) ||
                pointer_count < pointers_per_element(element_size) {
                return Err(Error::failed(
                    "Existing list value is incompatible with expected type.".to_string()));
            }

            let step = data_size + pointer_count * BITS_PER_POINTER as u32;

            Ok(ListBuilder {
                arena: arena,
                segment_id: segment_id,
                cap_table: cap_table,
                ptr: ptr as *mut _,
                step: step,
                element_count: (*reff).list_ref().element_count(),
                struct_data_size: data_size,
                struct_pointer_count: pointer_count as u16
            })
        }
    }

    #[inline]
    pub unsafe fn get_writable_struct_list_pointer<'a>(
        arena: &'a BuilderArena,
        orig_ref: *mut WirePointer,
        orig_segment_id: u32,
        cap_table: CapTableBuilder,
        element_size: StructSize,
        default_value: *const Word) -> Result<ListBuilder<'a>>
    {
        let orig_ref_target = (*orig_ref).mut_target();

        if (*orig_ref).is_null() {
            if default_value.is_null() || (*(default_value as *const WirePointer)).is_null() {
                return Ok(ListBuilder::new_default());
            }
            unimplemented!()
        }

        // We must verify that the pointer has the right size and potentially upgrade it if not.

        let (mut old_ptr, old_ref, old_segment_id) =
            try!(follow_builder_fars(arena, orig_ref, orig_ref_target, orig_segment_id));

        if (*old_ref).kind() != WirePointerKind::List {
            return Err(Error::failed(
                "Called getList{{Field,Element}} but existing pointer is not a list.".to_string()));
        }

        let old_size = (*old_ref).list_ref().element_size();

        if old_size == InlineComposite {
            // Existing list is InlineComposite, but we need to verify that the sizes match.

            let old_tag: *const WirePointer = old_ptr as *const _;
            old_ptr = old_ptr.offset(POINTER_SIZE_IN_WORDS as isize);
            if (*old_tag).kind() != WirePointerKind::Struct {
                return Err(Error::failed(
                    "InlineComposite list with non-STRUCT elements not supported.".to_string()));
            }

            let old_data_size = (*old_tag).struct_ref().data_size.get();
            let old_pointer_count = (*old_tag).struct_ref().ptr_count.get();
            let old_step = old_data_size as u32 + old_pointer_count as u32 * WORDS_PER_POINTER as u32;
            let element_count = (*old_tag).inline_composite_list_element_count();

            if old_data_size >= element_size.data && old_pointer_count >= element_size.pointers {
                // Old size is at least as large as we need. Ship it.
                return Ok(ListBuilder {
                    arena: arena,
                    segment_id: old_segment_id,
                    cap_table: cap_table,
                    ptr: old_ptr as *mut _,
                    element_count: element_count,
                    step: old_step * BITS_PER_WORD as u32,
                    struct_data_size: old_data_size as u32 * BITS_PER_WORD as u32,
                    struct_pointer_count: old_pointer_count
                });
            }

            // The structs in this list are smaller than expected, probably written using an older
            // version of the protocol. We need to make a copy and expand them.

            let new_data_size = ::std::cmp::max(old_data_size, element_size.data);
            let new_pointer_count = ::std::cmp::max(old_pointer_count, element_size.pointers);
            let new_step = new_data_size as u32 + new_pointer_count as u32 * WORDS_PER_POINTER as u32;
            let total_size = new_step * element_count;

            // Don't let allocate() zero out the object just yet.
            try!(zero_pointer_and_fars(arena, orig_segment_id, orig_ref));

            let (mut new_ptr, new_ref, new_segment_id) =
                allocate(arena, orig_ref, orig_segment_id,
                         total_size + POINTER_SIZE_IN_WORDS as u32, WirePointerKind::List);
            (*new_ref).mut_list_ref().set_inline_composite(total_size);

            let new_tag: *mut WirePointer = new_ptr as *mut _;
            (*new_tag).set_kind_and_inline_composite_list_element_count(WirePointerKind::Struct, element_count);
            (*new_tag).mut_struct_ref().set(new_data_size, new_pointer_count);
            new_ptr = new_ptr.offset(POINTER_SIZE_IN_WORDS as isize);

            let mut src: *mut Word = old_ptr as *mut _;
            let mut dst: *mut Word = new_ptr;
            for _ in 0..element_count {
                // Copy data section.
                ptr::copy_nonoverlapping(src, dst, old_data_size as usize);

                // Copy pointer section
                let new_pointer_section: *mut WirePointer = dst.offset(new_data_size as isize) as *mut _;
                let old_pointer_section: *mut WirePointer = src.offset(old_data_size as isize) as *mut _;
                for jj in 0..(old_pointer_count as isize) {
                    transfer_pointer(arena, new_segment_id,
                                     new_pointer_section.offset(jj),
                                     old_segment_id, old_pointer_section.offset(jj));
                }

                dst = dst.offset(new_step as isize);
                src = src.offset(old_step as isize);
            }

            ptr::write_bytes(old_ptr.offset(-1), 0,
                             (old_step as u64 * element_count as u64) as usize);

            Ok(ListBuilder {
                arena: arena,
                segment_id: new_segment_id,
                cap_table: cap_table,
                ptr: new_ptr as *mut _,
                element_count: element_count,
                step: new_step * BITS_PER_WORD as u32,
                struct_data_size: new_data_size as u32 * BITS_PER_WORD as u32,
                struct_pointer_count: new_pointer_count,
            })
        } else {
            // We're upgrading from a non-struct list.

            let old_data_size = data_bits_per_element(old_size);
            let old_pointer_count = pointers_per_element(old_size);
            let old_step = old_data_size + old_pointer_count * BITS_PER_POINTER as u32;
            let element_count = (*old_ref).list_ref().element_count();

            if old_size == ElementSize::Void {
                // Nothing to copy, just allocate a new list.
                return Ok(init_struct_list_pointer(
                    arena, orig_ref, orig_segment_id, cap_table, element_count, element_size));
            } else {
                // Upgrade to an inline composite list.

                if old_size == ElementSize::Bit {
                    return Err(Error::failed(
                        "Found bit list where struct list was expected; upgrading boolean \
                         lists to struct lists is no longer supported.".to_string()));
                }

                let mut new_data_size = element_size.data;
                let mut new_pointer_count = element_size.pointers;

                if old_size == ElementSize::Pointer {
                    new_pointer_count = ::std::cmp::max(new_pointer_count, 1);
                } else {
                    // Old list contains data elements, so we need at least one word of data.
                    new_data_size = ::std::cmp::max(new_data_size, 1);
                }

                let new_step = new_data_size as u32 + new_pointer_count as u32 * WORDS_PER_POINTER as u32;
                let total_words = element_count * new_step;

                // Don't let allocate() zero out the object just yet.
                try!(zero_pointer_and_fars(arena, orig_segment_id, orig_ref));

                let (mut new_ptr, new_ref, new_segment_id) =
                    allocate(arena, orig_ref, orig_segment_id,
                             total_words + POINTER_SIZE_IN_WORDS as u32, WirePointerKind::List);
                (*new_ref).mut_list_ref().set_inline_composite(total_words);

                let tag: *mut WirePointer = new_ptr as *mut _;
                (*tag).set_kind_and_inline_composite_list_element_count(WirePointerKind::Struct, element_count);
                (*tag).mut_struct_ref().set(new_data_size, new_pointer_count);
                new_ptr = new_ptr.offset(POINTER_SIZE_IN_WORDS as isize);

                if old_size == ElementSize::Pointer {
                    let mut dst: *mut Word = new_ptr.offset(new_data_size as isize);
                    let mut src: *mut WirePointer = old_ptr as *mut _;
                    for _ in 0..element_count {
                        transfer_pointer(arena, new_segment_id, dst as *mut _, old_segment_id, src);
                        dst = dst.offset(new_step as isize / WORDS_PER_POINTER as isize);
                        src = src.offset(1);
                    }
                } else {
                    let mut dst: *mut Word = new_ptr;
                    let mut src: *mut u8 = old_ptr as *mut u8;
                    let old_byte_step = old_data_size / BITS_PER_BYTE as u32;
                    for _ in 0..element_count {
                        ptr::copy_nonoverlapping(src, dst as *mut _, old_byte_step as usize);
                        src = src.offset(old_byte_step as isize);
                        dst = dst.offset(new_step as isize);
                    }
                }

                // Zero out old location.
                ptr::write_bytes(old_ptr as *mut u8, 0,
                                 round_bits_up_to_bytes(old_step as u64 * element_count as u64) as usize);

                Ok(ListBuilder {
                    arena: arena,
                    segment_id: new_segment_id,
                    cap_table: cap_table,
                    ptr: new_ptr as *mut _,
                    element_count: element_count,
                    step: new_step * BITS_PER_WORD as u32,
                    struct_data_size: new_data_size as u32 * BITS_PER_WORD as u32,
                    struct_pointer_count: new_pointer_count
                })
            }
        }
    }

    #[inline]
    pub unsafe fn init_text_pointer<'a>(
        arena: &'a BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        size: ByteCount32) -> SegmentAnd<text::Builder<'a>>
    {
        //# The byte list must include a NUL terminator.
        let byte_size = size + 1;

        //# Allocate the space.
        let (ptr, reff, segment_id) =
            allocate(arena, reff, segment_id, round_bytes_up_to_words(byte_size), WirePointerKind::List);

        //# Initialize the pointer.
        (*reff).mut_list_ref().set(Byte, byte_size);

        return SegmentAnd {
            segment_id: segment_id,
            value: text::Builder::new(slice::from_raw_parts_mut(ptr as *mut _, size as usize), 0)
                .expect("empty text builder should be valid utf-8")
        }
    }

    #[inline]
    pub unsafe fn set_text_pointer<'a>(
        arena: &'a BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        value: &str) -> SegmentAnd<text::Builder<'a>>
    {
        let value_bytes = value.as_bytes();
        // TODO make sure the string is not longer than 2 ** 29.
        let mut allocation = init_text_pointer(arena, reff, segment_id, value_bytes.len() as u32);
        allocation.value.push_str(value);
        allocation
    }

    #[inline]
    pub unsafe fn get_writable_text_pointer<'a>(
        arena: &'a BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        _default_value: *const Word,
        default_size: ByteCount32) -> Result<text::Builder<'a>>
    {
        if (*reff).is_null() {
            if default_size == 0 {
                return text::Builder::new(slice::from_raw_parts_mut(ptr::null_mut(), 0), 0);
            } else {
                let _builder = init_text_pointer(arena, reff, segment_id, default_size).value;
                unimplemented!()
            }
        }
        let ref_target = (*reff).mut_target();
        let (ptr, reff, _segment_id) = try!(follow_builder_fars(arena, reff, ref_target, segment_id));
        let cptr: *mut u8 = ptr as *mut _;

        if (*reff).kind() != WirePointerKind::List {
            return Err(Error::failed(
                "Called getText{{Field,Element}}() but existing pointer is not a list.".to_string()));
        }
        if (*reff).list_ref().element_size() != Byte {
            return Err(Error::failed(
                "Called getText{{Field,Element}}() but existing list pointer is not byte-sized.".to_string()));
        }

        let count = (*reff).list_ref().element_count();
        if count <= 0 || *cptr.offset((count - 1) as isize) != 0 {
            return Err(Error::failed(
                "Text blob missing NUL terminator.".to_string()));
        }

        // Subtract 1 from the size for the NUL terminator.
        return text::Builder::new(slice::from_raw_parts_mut(cptr, (count - 1) as usize), count - 1);
    }

    #[inline]
    pub unsafe fn init_data_pointer<'a>(
        arena: &'a BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        size: ByteCount32) -> SegmentAnd<data::Builder<'a>>
    {
        //# Allocate the space.
        let (ptr, reff, segment_id) =
            allocate(arena, reff, segment_id, round_bytes_up_to_words(size), WirePointerKind::List);

        //# Initialize the pointer.
        (*reff).mut_list_ref().set(Byte, size);

        SegmentAnd { segment_id: segment_id, value: data::new_builder(ptr as *mut _, size) }
    }

    #[inline]
    pub unsafe fn set_data_pointer<'a>(
        arena: &'a BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        value: &[u8]) -> SegmentAnd<data::Builder<'a>>
    {
        let allocation = init_data_pointer(arena, reff, segment_id, value.len() as u32);
        ptr::copy_nonoverlapping(value.as_ptr(), allocation.value.as_mut_ptr(),
                                 value.len());
        allocation
    }

    #[inline]
    pub unsafe fn get_writable_data_pointer<'a>(
        arena: &'a BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        default_value: *const Word,
        default_size: ByteCount32) -> Result<data::Builder<'a>>
    {
        if (*reff).is_null() {
            if default_size == 0 {
                return Ok(data::new_builder(ptr::null_mut(), 0));
            } else {
                let builder = init_data_pointer(arena, reff, segment_id, default_size).value;
                ptr::copy_nonoverlapping(default_value as *const _,
                                         builder.as_mut_ptr() as *mut _,
                                         default_size as usize);
                return Ok(builder);
            }
        }
        let ref_target = (*reff).mut_target();
        let (ptr, reff, _segment_id) = try!(follow_builder_fars(arena, reff, ref_target, segment_id));

        if (*reff).kind() != WirePointerKind::List {
            return Err(Error::failed(
                "Called getData{{Field,Element}}() but existing pointer is not a list.".to_string()));
        }
        if (*reff).list_ref().element_size() != Byte {
            return Err(Error::failed(
                "Called getData{{Field,Element}}() but existing list pointer is not byte-sized.".to_string()));
        }

        Ok(data::new_builder(ptr as *mut _, (*reff).list_ref().element_count()))
    }

    pub unsafe fn set_struct_pointer<'a>(
        arena: &BuilderArena,
        segment_id: u32,
        cap_table: CapTableBuilder,
        reff: *mut WirePointer,
        value: StructReader) -> Result<SegmentAnd<*mut Word>>
    {
        let data_size: WordCount32 = round_bits_up_to_words(value.data_size as u64);
        let total_size: WordCount32 = data_size + value.pointer_count as u32 * WORDS_PER_POINTER as u32;

        let (ptr, reff, segment_id) =
            allocate(arena, reff, segment_id, total_size, WirePointerKind::Struct);
        (*reff).mut_struct_ref().set(data_size as u16, value.pointer_count);

        if value.data_size == 1 {
            *(ptr as *mut u8) = value.get_bool_field(0) as u8
        } else {
            ptr::copy_nonoverlapping::<Word>(value.data as *const _, ptr,
                                             value.data_size as usize / BITS_PER_WORD);
        }

        let pointer_section: *mut WirePointer = ptr.offset(data_size as isize) as *mut _;
        for i in 0..value.pointer_count as isize {
            try!(copy_pointer(arena, segment_id, cap_table, pointer_section.offset(i),
                              value.arena,
                              value.segment_id, value.cap_table, value.pointers.offset(i),
                              value.nesting_limit));
        }

        Ok(SegmentAnd { segment_id: segment_id, value: ptr })
    }

    pub fn set_capability_pointer(
        _arena: &BuilderArena,
        _segment_id: u32,
        mut cap_table: CapTableBuilder,
        reff: *mut WirePointer,
        cap: Box<ClientHook>)
    {
        // TODO if ref is not null, zero object.
        unsafe { (*reff).set_cap(cap_table.inject_cap(cap) as u32); }
    }

    pub unsafe fn set_list_pointer<'a>(
        arena: &'a BuilderArena,
        segment_id: u32,
        cap_table: CapTableBuilder,
        reff: *mut WirePointer,
        value: ListReader) -> Result<SegmentAnd<*mut Word>>
    {
        let total_size = round_bits_up_to_words((value.element_count * value.step) as u64);

        if value.step <= BITS_PER_WORD as u32 {
            //# List of non-structs.
            let (ptr, reff, segment_id) =
                allocate(arena, reff, segment_id, total_size, WirePointerKind::List);

            if value.struct_pointer_count == 1 {
                //# List of pointers.
                (*reff).mut_list_ref().set(Pointer, value.element_count);
                for i in 0.. value.element_count as isize {
                    try!(copy_pointer(arena, segment_id, cap_table,
                                      (ptr as *mut _).offset(i),
                                      value.arena,
                                      value.segment_id, value.cap_table,
                                      (value.ptr as *const _).offset(i),
                                      value.nesting_limit));
                }
            } else {
                //# List of data.
                let element_size = match value.step {
                    0 => Void,
                    1 => Bit,
                    8 => Byte,
                    16 => TwoBytes,
                    32 => FourBytes,
                    64 => EightBytes,
                    _ => { panic!("invalid list step size: {}", value.step) }
                };

                (*reff).mut_list_ref().set(element_size, value.element_count);
                ptr::copy_nonoverlapping(value.ptr as *const Word, ptr, total_size as usize);
            }

            Ok(SegmentAnd { segment_id: segment_id, value: ptr })
        } else {
            //# List of structs.
            let (ptr, reff, segment_id) =
                allocate(arena, reff, segment_id,
                         total_size + POINTER_SIZE_IN_WORDS as u32, WirePointerKind::List);
            (*reff).mut_list_ref().set_inline_composite(total_size);

            let data_size = round_bits_up_to_words(value.struct_data_size as u64);
            let pointer_count = value.struct_pointer_count;

            let tag: *mut WirePointer = ptr as *mut _;
            (*tag).set_kind_and_inline_composite_list_element_count(WirePointerKind::Struct, value.element_count);
            (*tag).mut_struct_ref().set(data_size as u16, pointer_count);
            let mut dst = ptr.offset(POINTER_SIZE_IN_WORDS as isize);

            let mut src: *const Word = value.ptr as *const _;
            for _ in 0.. value.element_count {
                ptr::copy_nonoverlapping(src, dst,
                                         value.struct_data_size as usize / BITS_PER_WORD);
                dst = dst.offset(data_size as isize);
                src = src.offset(data_size as isize);

                for _ in 0..pointer_count {
                    try!(copy_pointer(arena, segment_id, cap_table, dst as *mut _,
                                      value.arena, value.segment_id, value.cap_table, src as *const _,
                                      value.nesting_limit));
                    dst = dst.offset(POINTER_SIZE_IN_WORDS as isize);
                    src = src.offset(POINTER_SIZE_IN_WORDS as isize);
                }
            }
            Ok(SegmentAnd { segment_id: segment_id, value: ptr })
        }
    }

    pub unsafe fn copy_pointer(
        dst_arena: &BuilderArena,
        dst_segment_id: u32, dst_cap_table: CapTableBuilder,
        dst: *mut WirePointer,
        src_arena: &ReaderArena,
        src_segment_id: u32, src_cap_table: CapTableReader,
        src: *const WirePointer,
        nesting_limit: i32) -> Result<SegmentAnd<*mut Word>>
    {
        let src_target = (*src).target();

        if (*src).is_null() {
            ptr::write_bytes(dst, 0, 1);
            return Ok(SegmentAnd { segment_id: dst_segment_id, value: ptr::null_mut() });
        }

        let (mut ptr, src, src_segment_id) =
            try!(follow_fars(src_arena, src, src_target, src_segment_id));

        match (*src).kind() {
            WirePointerKind::Struct => {
                if nesting_limit <= 0 {
                    return Err(Error::failed(
                        "Message is too deeply-nested or contains cycles. See ReaderOptions.".to_string()));
                }

                try!(bounds_check(src_arena, src_segment_id,
                                  ptr, ptr.offset((*src).struct_ref().word_size() as isize),
                                  WirePointerKind::Struct));

                return set_struct_pointer(
                    dst_arena,
                    dst_segment_id, dst_cap_table, dst,
                    StructReader {
                        arena: src_arena,
                        segment_id: src_segment_id,
                        cap_table: src_cap_table,
                        data: ptr as *mut _,
                        pointers: ptr.offset((*src).struct_ref().data_size.get() as isize) as *mut _,
                        data_size: (*src).struct_ref().data_size.get() as u32 * BITS_PER_WORD as u32,
                        pointer_count: (*src).struct_ref().ptr_count.get(),
                        nesting_limit: nesting_limit - 1
                    });
            }
            WirePointerKind::List => {
                let element_size = (*src).list_ref().element_size();
                if nesting_limit <= 0 {
                    return Err(Error::failed(
                        "Message is too deeply-nested or contains cycles. See ReaderOptions.".to_string()));
                }

                if element_size == InlineComposite {
                    let word_count = (*src).list_ref().inline_composite_word_count();
                    let tag: *const WirePointer = ptr as *const _;
                    ptr = ptr.offset(POINTER_SIZE_IN_WORDS as isize);

                    try!(bounds_check(
                        src_arena, src_segment_id, ptr.offset(-1), ptr.offset(word_count as isize),
                        WirePointerKind::List));

                    if (*tag).kind() != WirePointerKind::Struct {
                        return Err(Error::failed(
                            "InlineComposite lists of non-STRUCT type are not supported.".to_string()));
                    }

                    let element_count = (*tag).inline_composite_list_element_count();
                    let words_per_element = (*tag).struct_ref().word_size();

                    if words_per_element as u64 * element_count as u64 > word_count as u64 {
                        return Err(Error::failed(
                            "InlineComposite list's elements overrun its word count.".to_string()));
                    }

                    if words_per_element == 0 {
                        // Watch out for lists of zero-sized structs, which can claim to be
                        // arbitrarily large without having sent actual data.
                        try!(amplified_read(src_arena, element_count as u64));
                    }

                    return set_list_pointer(
                        dst_arena,
                        dst_segment_id, dst_cap_table, dst,
                        ListReader {
                            arena: src_arena,
                            segment_id: src_segment_id,
                            cap_table: src_cap_table,
                            ptr: ptr as *mut _,
                            element_count: element_count,
                            step: words_per_element * BITS_PER_WORD as u32,
                            struct_data_size: (*tag).struct_ref().data_size.get() as u32 * BITS_PER_WORD as u32,
                            struct_pointer_count: (*tag).struct_ref().ptr_count.get(),
                            nesting_limit: nesting_limit - 1
                        })
                } else {
                    let data_size = data_bits_per_element(element_size);
                    let pointer_count = pointers_per_element(element_size);
                    let step = data_size + pointer_count * BITS_PER_POINTER as u32;
                    let element_count = (*src).list_ref().element_count();
                    let word_count = round_bits_up_to_words(element_count as u64 * step as u64);

                    try!(bounds_check(src_arena,
                                      src_segment_id, ptr,
                                      ptr.offset(word_count as isize), WirePointerKind::List));

                    if element_size == Void {
                        // Watch out for lists of void, which can claim to be arbitrarily large
                        // without having sent actual data.
                        try!(amplified_read(src_arena, element_count as u64));
                    }

                    return set_list_pointer(
                        dst_arena,
                        dst_segment_id, dst_cap_table, dst,
                        ListReader {
                            arena: src_arena,
                            segment_id: src_segment_id,
                            cap_table : src_cap_table,
                            ptr: ptr as *mut _,
                            element_count: element_count,
                            step: step,
                            struct_data_size: data_size,
                            struct_pointer_count: pointer_count as u16,
                            nesting_limit: nesting_limit - 1
                        })
                }
            }
            WirePointerKind::Far => {
                Err(Error::failed("Malformed double-far pointer.".to_string()))
            }
            WirePointerKind::Other => {
                if !(*src).is_capability() {
                    return Err(Error::failed("Unknown pointer type.".to_string()));
                }
                match src_cap_table.extract_cap((*src).cap_ref().index.get() as usize) {
                    Some(cap) => {
                        set_capability_pointer(dst_arena, dst_segment_id, dst_cap_table, dst, cap);
                        return Ok(SegmentAnd { segment_id: dst_segment_id, value: ptr::null_mut() });
                    }
                    None => {
                        return Err(Error::failed(
                            "Message contained invalid capability pointer.".to_string()));
                    }
                }
            }
        }
    }

    #[inline]
    pub unsafe fn read_struct_pointer<'a>(
        arena: &'a ReaderArena,
        segment_id: u32,
        cap_table: CapTableReader,
        reff: *const WirePointer,
        default_value: *const Word,
        nesting_limit: i32) -> Result<StructReader<'a>>
    {
        let ref_target: *const Word = (*reff).target();

        if (*reff).is_null() {
            if default_value.is_null() || (*(default_value as *const WirePointer)).is_null() {
                    return Ok(StructReader::new_default());
            }
            //segment = ::std::ptr::null();
            //reff = default_value as *const WirePointer;
            unimplemented!()
        }

        if nesting_limit <= 0 {
            return Err(Error::failed("Message is too deeply-nested or contains cycles.".to_string()));
        }

        let (ptr, reff, segment_id) = try!(follow_fars(arena, reff, ref_target, segment_id));

        let data_size_words = (*reff).struct_ref().data_size.get();

        if (*reff).kind() != WirePointerKind::Struct {
            return Err(Error::failed(
                "Message contains non-struct pointer where struct pointer was expected.".to_string()));
        }

        try!(bounds_check(arena, segment_id, ptr,
                          ptr.offset((*reff).struct_ref().word_size() as isize),
                          WirePointerKind::Struct));

        Ok(StructReader {
            arena: arena,
            segment_id: segment_id,
            cap_table: cap_table,
            data: ptr as *const _,
            pointers: ptr.offset(data_size_words as isize) as *const _,
            data_size: data_size_words as u32 * BITS_PER_WORD as BitCount32,
            pointer_count: (*reff).struct_ref().ptr_count.get(),
            nesting_limit: nesting_limit - 1,
        })
     }

    #[inline]
    pub unsafe fn read_capability_pointer(
        _arena: &ReaderArena,
        _segment_id: u32,
        cap_table: CapTableReader,
        reff: *const WirePointer,
        _nesting_limit: i32) -> Result<Box<ClientHook>>
    {
        if (*reff).is_null() {
            Err(Error::failed(
                "Message contains null capability pointer.".to_string()))
        } else if !(*reff).is_capability() {
            Err(Error::failed(
                "Message contains non-capability pointer where capability pointer was expected.".to_string()))
        } else {
            let n = (*reff).cap_ref().index.get() as usize;
            match cap_table.extract_cap(n) {
                Some(client_hook) => { Ok(client_hook) }
                None => {
                    Err(Error::failed(
                        format!("Message contains invalid capability pointer. Index: {}", n)))
                }
            }
        }
    }

    #[inline]
    pub unsafe fn read_list_pointer<'a>(
        arena: &'a ReaderArena,
        segment_id: u32,
        cap_table: CapTableReader,
        reff: *const WirePointer,
        default_value: *const Word,
        expected_element_size: ElementSize,
        nesting_limit: i32) -> Result<ListReader<'a>>
    {
        let ref_target: *const Word = (*reff).target();

        if (*reff).is_null() {
            if default_value.is_null() || (*(default_value as *const WirePointer)).is_null() {
                    return Ok(ListReader::new_default());
                }
            panic!("list default values unimplemented");
        }

        if nesting_limit <= 0 {
            return Err(Error::failed("nesting limit exceeded".to_string()));
        }

        let (mut ptr, reff, segment_id) = try!(follow_fars(arena, reff, ref_target, segment_id));

        if (*reff).kind() != WirePointerKind::List {
            return Err(Error::failed(
                "Message contains non-list pointer where list pointer was expected".to_string()));
        }

        let list_ref = (*reff).list_ref();

        let element_size = list_ref.element_size();
        match element_size {
            InlineComposite => {
                let word_count = list_ref.inline_composite_word_count();

                let tag: *const WirePointer = mem::transmute(ptr);

                ptr = ptr.offset(1);

                try!(bounds_check(arena, segment_id, ptr.offset(-1),
                                  ptr.offset(word_count as isize),
                                  WirePointerKind::List));

                if (*tag).kind() != WirePointerKind::Struct {
                    return Err(Error::failed(
                        "InlineComposite lists of non-STRUCT type are not supported.".to_string()));
                }

                let size = (*tag).inline_composite_list_element_count();
                let struct_ref = (*tag).struct_ref();
                let words_per_element = struct_ref.word_size();

                if size as u64 * words_per_element as u64 > word_count as u64 {
                    return Err(Error::failed(
                         "InlineComposite list's elements overrun its word count.".to_string()));
                }

                if words_per_element == 0 {
                    // Watch out for lists of zero-sized structs, which can claim to be
                    // arbitrarily large without having sent actual data.
                    try!(amplified_read(arena, size as u64));
                }

                // If a struct list was not expected, then presumably a non-struct list was upgraded
                // to a struct list. We need to manipulate the pointer to point at the first field
                // of the struct. Together with the "stepBits", this will allow the struct list to
                // be accessed as if it were a primitive list without branching.

                // Check whether the size is compatible.
                match expected_element_size {
                    Void => {}
                    Bit => {
                        return Err(Error::failed(
                            "Found struct list where bit list was expected.".to_string()));
                    }
                    Byte | TwoBytes | FourBytes | EightBytes => {
                        if struct_ref.data_size.get() <= 0 {
                            return Err(Error::failed(
                                "Expected a primitive list, but got a list of pointer-only structs".to_string()));
                        }
                    }
                    Pointer => {
                        // We expected a list of pointers but got a list of structs. Assuming the
                        // first field in the struct is the pointer we were looking for, we want to
                        // munge the pointer to point at the first element's pointer section.
                        ptr = ptr.offset(struct_ref.data_size.get() as isize);
                        if struct_ref.ptr_count.get() <= 0 {
                            return Err(Error::failed(
                                "Expected a pointer list, but got a list of data-only structs".to_string()));
                        }
                    }
                    InlineComposite => {}
                }

                Ok(ListReader {
                    arena: arena,
                    segment_id: segment_id,
                    cap_table: cap_table,
                    ptr: ptr as *const _,
                    element_count: size,
                    step: words_per_element * BITS_PER_WORD as u32,
                    struct_data_size: struct_ref.data_size.get() as u32 * (BITS_PER_WORD as u32),
                    struct_pointer_count: struct_ref.ptr_count.get(),
                    nesting_limit: nesting_limit - 1,
                })
            }
            _ => {
                // This is a primitive or pointer list, but all such lists can also be interpreted
                // as struct lists. We need to compute the data size and pointer count for such
                // structs.
                let data_size = data_bits_per_element(list_ref.element_size());
                let pointer_count = pointers_per_element(list_ref.element_size());
                let element_count = list_ref.element_count();
                let step = data_size + pointer_count * BITS_PER_POINTER as u32;

                let word_count = round_bits_up_to_words(list_ref.element_count() as u64 * step as u64);
                try!(bounds_check(
                    arena, segment_id, ptr, ptr.offset(word_count as isize), WirePointerKind::List));

                if element_size == Void {
                    // Watch out for lists of void, which can claim to be arbitrarily large
                    // without having sent actual data.
                    try!(amplified_read(arena, element_count as u64));
                }

                // Verify that the elements are at least as large as the expected type. Note that if
                // we expected InlineComposite, the expected sizes here will be zero, because bounds
                // checking will be performed at field access time. So this check here is for the
                // case where we expected a list of some primitive or pointer type.

                let expected_data_bits_per_element = data_bits_per_element(expected_element_size);
                let expected_pointers_per_element = pointers_per_element(expected_element_size);

                if expected_data_bits_per_element > data_size ||
                    expected_pointers_per_element > pointer_count {
                    return Err(Error::failed(
                        "Message contains list with incompatible element type.".to_string()));
                }

                Ok(ListReader {
                    arena: arena,
                    segment_id: segment_id,
                    cap_table: cap_table,
                    ptr: ptr as *const _,
                    element_count: list_ref.element_count(),
                    step: step,
                    struct_data_size: data_size,
                    struct_pointer_count: pointer_count as u16,
                    nesting_limit: nesting_limit - 1,
                })
            }
        }
    }

    #[inline]
    pub unsafe fn read_text_pointer<'a>(
        arena: &'a ReaderArena,
        segment_id: u32,
        reff: *const WirePointer,
        default_value: *const Word,
        default_size: ByteCount32) -> Result<text::Reader<'a>>
    {
        if (*reff).is_null() {
            //   TODO?       if default_value.is_null() { default_value = &"" }
            return text::new_reader(
                slice::from_raw_parts(mem::transmute(default_value), default_size as usize));
        }

        let ref_target = (*reff).target();
        let (ptr, reff, segment_id) = try!(follow_fars(arena, reff, ref_target, segment_id));
        let list_ref = (*reff).list_ref();
        let size = list_ref.element_count();

        if (*reff).kind() != WirePointerKind::List {
            return Err(Error::failed(
                "Message contains non-list pointer where text was expected.".to_string()));
        }

        if list_ref.element_size() != Byte {
            return Err(Error::failed(
                "Message contains list pointer of non-bytes where text was expected.".to_string()));
        }

        try!(bounds_check(arena, segment_id, ptr,
                          ptr.offset(round_bytes_up_to_words(size) as isize),
                          WirePointerKind::List));

        if size <= 0 {
            return Err(Error::failed("Message contains text that is not NUL-terminated.".to_string()));
        }

        let str_ptr = ptr as *const u8;

        if (*str_ptr.offset((size - 1) as isize)) != 0u8 {
            return Err(Error::failed(
                "Message contains text that is not NUL-terminated".to_string()));
        }

        Ok(try!(text::new_reader(slice::from_raw_parts(str_ptr, size as usize -1))))
    }

    #[inline]
    pub unsafe fn read_data_pointer<'a>(
        arena: &'a ReaderArena,
        segment_id: u32,
        reff: *const WirePointer,
        default_value: *const Word,
        default_size: ByteCount32) -> Result<data::Reader<'a>>
    {
        if (*reff).is_null() {
            return Ok(data::new_reader(default_value as *const _, default_size));
        }

        let ref_target = (*reff).target();

        let (ptr, reff, segment_id) = try!(follow_fars(arena, reff, ref_target, segment_id));

        let list_ref = (*reff).list_ref();

        let size: u32 = list_ref.element_count();

        if (*reff).kind() != WirePointerKind::List {
            return Err(Error::failed(
                "Message contains non-list pointer where data was expected.".to_string()));
        }

        if list_ref.element_size() != Byte {
            return Err(Error::failed(
                "Message contains list pointer of non-bytes where data was expected.".to_string()));
        }

        try!(bounds_check(arena, segment_id, ptr,
                          ptr.offset(round_bytes_up_to_words(size) as isize),
                          WirePointerKind::List));

        Ok(data::new_reader(ptr as *const _, size))
    }
}

static ZERO: u64 = 0;
fn zero_pointer() -> *const WirePointer { &ZERO as *const _ as *const _ }

static NULL_ARENA: NullArena = NullArena;

pub type CapTable = Vec<Option<Box<ClientHook>>>;

#[derive(Copy, Clone)]
pub enum CapTableReader {
    // At one point, we had a `Dummy` variant here, but that ended up
    // making values of this type take 16 bytes of memory. Now we instead
    // represent a null CapTableReader with `Plain(ptr::null())`.

    Plain(*const Vec<Option<Box<ClientHook>>>),
}

impl CapTableReader {
    pub fn extract_cap(&self, index: usize) -> Option<Box<ClientHook>> {
        match self {
            &CapTableReader::Plain(hooks) => {
                if hooks.is_null() { return None }
                let hooks: &Vec<Option<Box<ClientHook>>> = unsafe { &*hooks };
                if index >= hooks.len() { None }
                else {
                    match hooks[index] {
                        None => None,
                        Some(ref hook) => Some(hook.add_ref())
                    }
                }
            }
        }
    }
}

#[derive(Copy, Clone)]
pub enum CapTableBuilder {
    // At one point, we had a `Dummy` variant here, but that ended up
    // making values of this type take 16 bytes of memory. Now we instead
    // represent a null CapTableBuilder with `Plain(ptr::null_mut())`.

    Plain(*mut Vec<Option<Box<ClientHook>>>),
}

impl CapTableBuilder {
    pub fn as_reader(self) -> CapTableReader {
        match self {
            CapTableBuilder::Plain(hooks) => CapTableReader::Plain(hooks),
        }
    }

    pub fn extract_cap(&self, index: usize) -> Option<Box<ClientHook>> {
        match self {
            &CapTableBuilder::Plain(hooks) => {
                if hooks.is_null() { return None }
                let hooks: &Vec<Option<Box<ClientHook>>> = unsafe { &*hooks };
                if index >= hooks.len() { None }
                else {
                    match hooks[index] {
                        None => None,
                        Some(ref hook) => Some(hook.add_ref())
                    }
                }
            }
        }
    }

    pub fn inject_cap(&mut self, cap: Box<ClientHook>) -> usize {
        match self {
            &mut CapTableBuilder::Plain(hooks) => {
                if hooks.is_null() {
                    panic!("Called inject_cap() on a null capability table. You need \
                            to call imbue_mut() on this message before adding capabilities.");
                }
                let hooks: &mut Vec<Option<Box<ClientHook>>> = unsafe { &mut *hooks };
                hooks.push(Some(cap));
                hooks.len() - 1
            }
        }
    }

    pub fn drop_cap(&mut self, index: usize) {
        match self {
            &mut CapTableBuilder::Plain(hooks) => {
                if hooks.is_null() {
                    panic!("Called drop_cap() on a null capability table. You need \
                            to call imbue_mut() on this message before adding capabilities.");
                }
                let hooks: &mut Vec<Option<Box<ClientHook>>> = unsafe { &mut *hooks };
                if index < hooks.len() { hooks[index] = None; }
            }
        }
    }
}


#[derive(Clone, Copy)]
pub struct PointerReader<'a>  {
    arena: &'a ReaderArena,
    cap_table: CapTableReader,
    pointer: *const WirePointer,
    segment_id: u32,
    nesting_limit: i32,
}

impl <'a> PointerReader<'a> {
    pub fn new_default<'b>() -> PointerReader<'b> {
        PointerReader {
            arena: &NULL_ARENA,
            segment_id: 0,
            cap_table: CapTableReader::Plain(ptr::null()),
            pointer: ptr::null(),
            nesting_limit: 0x7fffffff }
    }

    pub fn get_root(arena: &'a ReaderArena,
                    segment_id: u32,
                    location: *const Word,
                    nesting_limit: i32) -> Result<Self>
    {
        try!(wire_helpers::bounds_check(arena, segment_id, location,
                                        unsafe { location.offset(POINTER_SIZE_IN_WORDS as isize) },
                                        WirePointerKind::Struct));

        Ok(PointerReader {
            arena: arena,
            segment_id: segment_id,
            cap_table: CapTableReader::Plain(ptr::null()),
            pointer: location as *const _,
            nesting_limit: nesting_limit,
        })
    }

    pub fn borrow<'b>(&'b self) -> PointerReader<'b> {
        PointerReader { arena: self.arena, .. *self }
    }

    pub fn get_root_unchecked<'b>(location: *const Word) -> PointerReader<'b> {
        PointerReader {
            arena: &NULL_ARENA,
            segment_id: 0,
            cap_table: CapTableReader::Plain(ptr::null()),
            pointer: location as *mut _,
            nesting_limit: 0x7fffffff }
    }

    pub fn imbue(&mut self, cap_table: CapTableReader) {
        self.cap_table = cap_table;
    }

    pub fn is_null(&self) -> bool {
        self.pointer.is_null() || unsafe { (*self.pointer).is_null() }
    }

    pub fn total_size(&self) -> Result<MessageSize> {
        if self.pointer.is_null() {
            Ok( MessageSize { word_count: 0, cap_count: 0 } )
        } else {
            unsafe { wire_helpers::total_size(self.arena, self.segment_id, self.pointer, self.nesting_limit) }
        }
    }

    pub fn get_struct(self, default_value: *const Word) -> Result<StructReader<'a>> {
        let reff: *const WirePointer = if self.pointer.is_null() { zero_pointer() } else { self.pointer };
        unsafe {
            wire_helpers::read_struct_pointer(self.arena,
                                              self.segment_id, self.cap_table, reff,
                                              default_value, self.nesting_limit)
        }
    }

    pub fn get_list(self, expected_element_size: ElementSize,
                    default_value: *const Word) -> Result<ListReader<'a>> {
        let reff = if self.pointer.is_null() { zero_pointer() } else { self.pointer };
        unsafe {
            wire_helpers::read_list_pointer(
                self.arena,
                self.segment_id,
                self.cap_table,
                reff,
                default_value,
                expected_element_size, self.nesting_limit)
        }
    }

    pub fn get_text(self, default_value: *const Word, default_size: ByteCount32) -> Result<text::Reader<'a>> {
        let reff = if self.pointer.is_null() { zero_pointer() } else { self.pointer };
        unsafe {
            wire_helpers::read_text_pointer(self.arena, self.segment_id, reff, default_value, default_size)
        }
    }

    pub fn get_data(&self, default_value: *const Word, default_size: ByteCount32) -> Result<data::Reader<'a>> {
        let reff = if self.pointer.is_null() { zero_pointer() } else { self.pointer };
        unsafe {
            wire_helpers::read_data_pointer(self.arena, self.segment_id, reff, default_value, default_size)
        }
    }

    pub fn get_capability(&self) -> Result<Box<ClientHook>> {
        let reff: *const WirePointer = if self.pointer.is_null() { zero_pointer() } else { self.pointer };
        unsafe {
            wire_helpers::read_capability_pointer(
                self.arena, self.segment_id, self.cap_table, reff, self.nesting_limit)
        }
    }
}

#[derive(Clone, Copy)]
pub struct PointerBuilder<'a> {
    arena: &'a BuilderArena,
    segment_id: u32,
    cap_table: CapTableBuilder,
    pointer: *mut WirePointer
}

impl <'a> PointerBuilder<'a> {
    #[inline]
    pub fn get_root(
        arena: &'a BuilderArena,
        segment_id: u32,
        location: *mut Word)
        -> Self
    {
        PointerBuilder {
            arena: arena,
            cap_table: CapTableBuilder::Plain(ptr::null_mut()),
            segment_id: segment_id,
            pointer: location as *mut _,
        }
    }

    pub fn borrow<'b>(&'b mut self) -> PointerBuilder<'b> {
        PointerBuilder { arena: self.arena, .. *self }
    }

    pub fn imbue(&mut self, cap_table: CapTableBuilder) {
        self.cap_table = cap_table;
    }

    pub fn is_null(&self) -> bool {
        unsafe { (*self.pointer).is_null() }
    }

    pub fn get_struct(self, size: StructSize, default_value: *const Word) -> Result<StructBuilder<'a>> {
        unsafe {
            wire_helpers::get_writable_struct_pointer(
                self.arena,
                self.pointer,
                self.segment_id,
                self.cap_table,
                size,
                default_value)
        }
    }

    pub fn get_list(self, element_size: ElementSize, default_value: *const Word)
                    -> Result<ListBuilder<'a>>
    {
        unsafe {
            wire_helpers::get_writable_list_pointer(
                self.arena, self.pointer, self.segment_id, self.cap_table, element_size, default_value)
        }
    }

    pub fn get_struct_list(self, element_size: StructSize,
                           default_value: *const Word) -> Result<ListBuilder<'a>>
    {
        unsafe {
            wire_helpers::get_writable_struct_list_pointer(
                self.arena, self.pointer, self.segment_id, self.cap_table, element_size, default_value)
        }
    }

    pub fn get_text(self, default_value: *const Word, default_size: ByteCount32)
                    -> Result<text::Builder<'a>>
    {
        unsafe {
            wire_helpers::get_writable_text_pointer(
                self.arena,
                self.pointer, self.segment_id, default_value, default_size)
        }
    }

    pub fn get_data(self, default_value: *const Word, default_size: ByteCount32)
                    -> Result<data::Builder<'a>>
    {
        unsafe {
            wire_helpers::get_writable_data_pointer(
                self.arena, self.pointer, self.segment_id, default_value, default_size)
        }
    }

    pub fn get_capability(&self) -> Result<Box<ClientHook>> {
        unsafe {
            wire_helpers::read_capability_pointer(
                self.arena.as_reader(),
                self.segment_id, self.cap_table.as_reader(), self.pointer, ::std::i32::MAX)
        }
    }

    pub fn init_struct(self, size: StructSize) -> StructBuilder<'a> {
        unsafe {
            wire_helpers::init_struct_pointer(self.arena, self.pointer, self.segment_id, self.cap_table, size)
        }
    }

    pub fn init_list(self, element_size: ElementSize, element_count: ElementCount32) -> ListBuilder<'a> {
        unsafe {
            wire_helpers::init_list_pointer(
                self.arena, self.pointer, self.segment_id, self.cap_table, element_count, element_size)
        }
    }

    pub fn init_struct_list(self, element_count: ElementCount32, element_size: StructSize)
                            -> ListBuilder<'a> {
        unsafe {
            wire_helpers::init_struct_list_pointer(
                self.arena,
                self.pointer, self.segment_id,
                self.cap_table, element_count, element_size)
        }
    }

    pub fn init_text(self, size: ByteCount32) -> text::Builder<'a> {
        unsafe {
            wire_helpers::init_text_pointer(self.arena, self.pointer, self.segment_id, size).value
        }
    }

    pub fn init_data(self, size: ByteCount32) -> data::Builder<'a> {
        unsafe {
            wire_helpers::init_data_pointer(self.arena, self.pointer, self.segment_id, size).value
        }
    }

    pub fn set_struct(&self, value: &StructReader) -> Result<()> {
        unsafe {
            try!(wire_helpers::set_struct_pointer(
                self.arena,
                self.segment_id, self.cap_table, self.pointer, *value));
            Ok(())
        }
    }

    pub fn set_list(&self, value: &ListReader) -> Result<()> {
        unsafe {
            try!(wire_helpers::set_list_pointer(self.arena, self.segment_id,
                                                self.cap_table, self.pointer, *value));
            Ok(())
        }
    }

    pub fn set_text(&self, value: &str) {
        unsafe {
            wire_helpers::set_text_pointer(self.arena, self.pointer, self.segment_id, value);
        }
    }

    pub fn set_data(&self, value: &[u8]) {
        unsafe {
            wire_helpers::set_data_pointer(self.arena, self.pointer, self.segment_id, value);
        }
    }

    pub fn set_capability(&self, cap: Box<ClientHook>) {
        wire_helpers::set_capability_pointer(
            self.arena, self.segment_id, self.cap_table, self.pointer, cap);
    }

    pub fn copy_from(&mut self, other: PointerReader) -> Result<()> {
        if other.pointer.is_null()  {
            if !self.pointer.is_null() {
                unsafe {
                    wire_helpers::zero_object(self.arena, self.segment_id, self.pointer);
                    *self.pointer = mem::zeroed();
                }
            }
        } else {
            unsafe {
                try!(wire_helpers::copy_pointer(self.arena, self.segment_id, self.cap_table, self.pointer,
                                                other.arena,
                                                other.segment_id, other.cap_table, other.pointer,
                                                other.nesting_limit));
            }
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        unsafe {
            wire_helpers::zero_object(self.arena, self.segment_id, self.pointer);
            ptr::write_bytes(self.pointer, 0, 1);
        }
    }

    pub fn as_reader(self) -> PointerReader<'a> {
        PointerReader {
            arena: self.arena.as_reader(),
            segment_id: self.segment_id,
            cap_table: self.cap_table.as_reader(),
            pointer: self.pointer,
            nesting_limit: 0x7fffffff
        }
    }
}

#[derive(Clone, Copy)]
pub struct StructReader<'a> {
    arena: &'a ReaderArena,
    cap_table: CapTableReader,
    data: *const u8,
    pointers: *const WirePointer,
    segment_id: u32,
    data_size: BitCount32,
    pointer_count: WirePointerCount16,
    nesting_limit: i32,
}

impl <'a> StructReader<'a> {
    pub fn new_default<'b>() -> StructReader<'b> {
        StructReader {
            arena: &NULL_ARENA,
            segment_id: 0,
            cap_table: CapTableReader::Plain(ptr::null()),
            data: ptr::null(),
            pointers: ptr::null(),
            data_size: 0,
            pointer_count: 0,
            nesting_limit: 0x7fffffff}
    }

    pub fn imbue(&mut self, cap_table: CapTableReader) {
        self.cap_table = cap_table
    }

    pub fn get_data_section_size(&self) -> BitCount32 { self.data_size }

    pub fn get_pointer_section_size(&self) -> WirePointerCount16 { self.pointer_count }

    pub fn get_data_section_as_blob(&self) -> usize { panic!("unimplemented") }

    #[inline]
    pub fn get_data_field<T: Endian + zero::Zero>(&self, offset: ElementCount) -> T {
        // We need to check the offset because the struct may have
        // been created with an old version of the protocol that did
        // not contain the field.
        if (offset + 1) * bits_per_element::<T>() <= self.data_size as usize {
            unsafe {
                let dwv: *const WireValue<T> = self.data as *const _;
                (*dwv.offset(offset as isize)).get()
            }
        } else {
            return T::zero();
        }
    }

    #[inline]
    pub fn get_bool_field(&self, offset: ElementCount) -> bool {
        let boffset: BitCount32 = offset as BitCount32;
        if boffset < self.data_size {
            unsafe {
                let b: *const u8 = self.data.offset((boffset as usize / BITS_PER_BYTE) as isize);
                ((*b) & (1u8 << (boffset % BITS_PER_BYTE as u32) as usize)) != 0
            }
        } else {
            false
        }
    }

    #[inline]
    pub fn get_data_field_mask<T:Endian + zero::Zero + Mask>(&self,
                                                             offset: ElementCount,
                                                             mask: <T as Mask>::T) -> T {
        Mask::mask(self.get_data_field(offset), mask)
    }

    #[inline]
    pub fn get_bool_field_mask(&self,
                               offset: ElementCount,
                               mask: bool) -> bool {
       self.get_bool_field(offset) ^ mask
    }

    #[inline]
    pub fn get_pointer_field(&self, ptr_index: WirePointerCount) -> PointerReader<'a> {
        if ptr_index < self.pointer_count as WirePointerCount {
            PointerReader {
                arena: self.arena,
                segment_id: self.segment_id,
                cap_table: self.cap_table,
                pointer: unsafe { self.pointers.offset(ptr_index as isize) },
                nesting_limit: self.nesting_limit
            }
        } else {
            PointerReader::new_default()
        }
    }

    pub fn total_size(&self) -> Result<MessageSize> {
        let mut result = MessageSize {
            word_count: wire_helpers::round_bits_up_to_words(self.data_size as u64) as u64 +
                self.pointer_count as u64 * WORDS_PER_POINTER as u64,
            cap_count: 0 };

        for i in 0.. self.pointer_count as isize {
            unsafe {
                result.plus_eq(try!(wire_helpers::total_size(
                    self.arena, self.segment_id, self.pointers.offset(i),
                    self.nesting_limit)));
            }
        }

        // TODO when we have read limiting: segment->unread()

        Ok(result)
    }
}

#[derive(Clone, Copy)]
pub struct StructBuilder<'a> {
    arena: &'a BuilderArena,
    cap_table: CapTableBuilder,
    data: *mut u8,
    pointers: *mut WirePointer,
    segment_id: u32,
    data_size: BitCount32,
    pointer_count: WirePointerCount16,
}

impl <'a> StructBuilder<'a> {
    pub fn as_reader(self) -> StructReader<'a> {
        StructReader {
            arena: self.arena.as_reader(),
            cap_table: self.cap_table.as_reader(),
            data: self.data,
            pointers: self.pointers,
            pointer_count: self.pointer_count,
            segment_id: self.segment_id,
            data_size: self.data_size,
            nesting_limit: 0x7fffffff,
        }
    }

    pub fn imbue(&mut self, cap_table: CapTableBuilder) {
        self.cap_table = cap_table
    }

    #[inline]
    pub fn set_data_field<T:Endian>(&self, offset: ElementCount, value: T) {
        unsafe {
            let ptr: *mut WireValue<T> = self.data as *mut _;
            (*ptr.offset(offset as isize)).set(value)
        }
    }

    #[inline]
    pub fn set_data_field_mask<T:Endian + Mask>(&self,
                                                offset: ElementCount,
                                                value: T,
                                                mask: <T as Mask>::T) {
        self.set_data_field(offset, Mask::mask(value, mask));
    }

    #[inline]
    pub fn get_data_field<T: Endian>(&self, offset: ElementCount) -> T {
        unsafe {
            let ptr: *mut WireValue<T> = self.data as *mut _;
            (*ptr.offset(offset as isize)).get()
        }
    }

    #[inline]
    pub fn get_data_field_mask<T:Endian + Mask>(&self,
                                                offset: ElementCount,
                                                mask: <T as Mask>::T) -> T {
        Mask::mask(self.get_data_field(offset), mask)
    }


    #[inline]
    pub fn set_bool_field(&self, offset: ElementCount, value: bool) {
        //# This branch should be compiled out whenever this is
        //# inlined with a constant offset.
        let boffset: BitCount0 = offset;
        let b = unsafe { self.data.offset((boffset / BITS_PER_BYTE) as isize)};
        let bitnum = boffset % BITS_PER_BYTE;
        unsafe { (*b) = ( (*b) & !(1 << bitnum)) | ((value as u8) << bitnum) }
    }

    #[inline]
    pub fn set_bool_field_mask(&self,
                               offset: ElementCount,
                               value: bool,
                               mask: bool) {
       self.set_bool_field(offset , value ^ mask);
    }

    #[inline]
    pub fn get_bool_field(&self, offset: ElementCount) -> bool {
        let boffset: BitCount0 = offset;
        let b = unsafe { self.data.offset((boffset / BITS_PER_BYTE) as isize) };
        unsafe { ((*b) & (1 << (boffset % BITS_PER_BYTE ))) != 0 }
    }

    #[inline]
    pub fn get_bool_field_mask(&self,
                               offset: ElementCount,
                               mask: bool) -> bool {
       self.get_bool_field(offset) ^ mask
    }


    #[inline]
    pub fn get_pointer_field(self, ptr_index: WirePointerCount) -> PointerBuilder<'a> {
        PointerBuilder {
            arena: self.arena,
            segment_id: self.segment_id,
            cap_table: self.cap_table,
            pointer: unsafe { self.pointers.offset(ptr_index as isize) }
        }
    }

}

#[derive(Clone, Copy)]
pub struct ListReader<'a> {
    arena: &'a ReaderArena,
    cap_table: CapTableReader,
    ptr: *const u8,
    segment_id: u32,
    element_count: ElementCount32,
    step: BitCount32,
    struct_data_size: BitCount32,
    nesting_limit: i32,
    struct_pointer_count: WirePointerCount16,
}

impl <'a> ListReader<'a> {
    pub fn new_default<'b>() -> ListReader<'b> {
        ListReader {
            arena: &NULL_ARENA,
            segment_id: 0,
            cap_table: CapTableReader::Plain(ptr::null()),
            ptr: ptr::null(),
            element_count: 0,
            step: 0,
            struct_data_size: 0,
            struct_pointer_count: 0,
            nesting_limit: 0x7fffffff,
        }
    }

    pub fn imbue(&mut self, cap_table: CapTableReader) {
        self.cap_table = cap_table
    }

    #[inline]
    pub fn len(&self) -> ElementCount32 { self.element_count }

    #[inline]
    pub fn get_struct_element(&self, index: ElementCount32) -> StructReader<'a> {
        let index_byte: ByteCount32 =
            ((index as ElementCount64 * (self.step as BitCount64)) / BITS_PER_BYTE as u64) as u32;

        let struct_data: *const u8 = unsafe { self.ptr.offset(index_byte as isize) };

        let struct_pointers: *const WirePointer = unsafe {
            struct_data.offset((self.struct_data_size as usize / BITS_PER_BYTE) as isize) as *const _
        };

        StructReader {
            arena: self.arena,
            segment_id: self.segment_id,
            cap_table: self.cap_table,
            data: struct_data,
            pointers: struct_pointers,
            data_size: self.struct_data_size,
            pointer_count: self.struct_pointer_count,
            nesting_limit: self.nesting_limit - 1,
        }
    }

    #[inline]
    pub fn get_pointer_element(self, index: ElementCount32) -> PointerReader<'a> {
        let offset = (index as u64 * self.step as u64 / BITS_PER_BYTE as u64) as u32;
        PointerReader {
            arena: self.arena,
            segment_id: self.segment_id,
            cap_table: self.cap_table,
            pointer: unsafe { self.ptr.offset(offset as isize) as *mut _ },
            nesting_limit: self.nesting_limit
        }
    }
}

#[derive(Clone, Copy)]
pub struct ListBuilder<'a> {
    arena: &'a BuilderArena,
    cap_table: CapTableBuilder,
    ptr: *mut u8,
    segment_id: u32,
    element_count: ElementCount32,
    step: BitCount32,
    struct_data_size: BitCount32,
    struct_pointer_count: WirePointerCount16
}

impl <'a> ListBuilder<'a> {

    #[inline]
    pub fn new_default<'b>() -> ListBuilder<'b> {
        ListBuilder {
            arena: &NULL_ARENA,
            segment_id: 0,
            cap_table: CapTableBuilder::Plain(ptr::null_mut()),
            ptr: ptr::null_mut(),
            element_count: 0,
            step: 0,
            struct_data_size: 0,
            struct_pointer_count: 0,
        }
    }

    pub fn as_reader(self) -> ListReader<'a> {
        ListReader {
            arena: self.arena.as_reader(),
            segment_id: self.segment_id,
            cap_table: self.cap_table.as_reader(),
            ptr: self.ptr as *const _,
            element_count: self.element_count,
            step: self.step,
            struct_data_size: self.struct_data_size,
            struct_pointer_count: self.struct_pointer_count,
            nesting_limit: 0x7fffffff,
        }
    }

    pub fn borrow<'b>(&'b mut self) -> ListBuilder<'b> {
        ListBuilder { arena: self.arena, ..*self }
    }

    pub fn imbue(&mut self, cap_table: CapTableBuilder) {
        self.cap_table = cap_table
    }

    #[inline]
    pub fn len(&self) -> ElementCount32 { self.element_count }

    #[inline]
    pub fn get_struct_element(self, index: ElementCount32) -> StructBuilder<'a> {
        let index_byte = ((index as u64 * self.step as u64) / BITS_PER_BYTE as u64) as u32;
        let struct_data = unsafe{ self.ptr.offset(index_byte as isize)};
        let struct_pointers = unsafe {
            struct_data.offset(((self.struct_data_size as usize) / BITS_PER_BYTE) as isize) as *mut _
        };
        StructBuilder {
            arena: self.arena,
            segment_id: self.segment_id,
            cap_table: self.cap_table,
            data: struct_data,
            pointers: struct_pointers,
            data_size: self.struct_data_size,
            pointer_count: self.struct_pointer_count,
        }
    }

    #[inline]
    pub fn get_pointer_element(self, index: ElementCount32) -> PointerBuilder<'a> {
        let offset = (index as u64 * self.step as u64 / BITS_PER_BYTE as u64) as u32;
        PointerBuilder {
            arena: self.arena,
            segment_id: self.segment_id,
            cap_table: self.cap_table,
            pointer: unsafe { self.ptr.offset(offset as isize) } as *mut _,
        }
    }
}


pub trait PrimitiveElement: Endian {
    #[inline]
    fn get(list_reader: &ListReader, index: ElementCount32) -> Self {
        let offset = (index as u64 * list_reader.step as u64 / BITS_PER_BYTE as u64) as u32;
        unsafe {
            let ptr: *const u8 = list_reader.ptr.offset(offset as isize);
            (*(ptr as *const WireValue<Self>)).get()
        }
    }

    #[inline]
    fn get_from_builder(list_builder: &ListBuilder, index: ElementCount32) -> Self {
        let offset = (index as u64 * list_builder.step as u64 / BITS_PER_BYTE as u64) as u32;
        unsafe {
            let ptr: *mut WireValue<Self> = list_builder.ptr.offset(offset as isize) as *mut _;
            (*ptr).get()
        }
    }

    #[inline]
    fn set(list_builder: &ListBuilder, index: ElementCount32, value: Self) {
        let offset = (index as u64 * list_builder.step as u64 / BITS_PER_BYTE as u64) as u32;
        unsafe {
            let ptr: *mut WireValue<Self> = list_builder.ptr.offset(offset as isize) as *mut _;
            (*ptr).set(value);
        }
    }

    fn element_size() -> ElementSize {
        match mem::size_of::<Self>() {
            0 => Void,
            1 => Byte,
            2 => TwoBytes,
            4 => FourBytes,
            8 => EightBytes,
            _ => unreachable!(),
        }
    }
}

impl PrimitiveElement for u8 { }
impl PrimitiveElement for u16 { }
impl PrimitiveElement for u32 { }
impl PrimitiveElement for u64 { }
impl PrimitiveElement for i8 { }
impl PrimitiveElement for i16 { }
impl PrimitiveElement for i32 { }
impl PrimitiveElement for i64 { }
impl PrimitiveElement for f32 { }
impl PrimitiveElement for f64 { }

impl PrimitiveElement for bool {
    #[inline]
    fn get(list: &ListReader, index: ElementCount32) -> bool {
        let bindex = index as u64 * list.step as u64;
        unsafe {
            let b: *const u8 = list.ptr.offset((bindex / BITS_PER_BYTE as u64) as isize);
            ((*b) & (1 << (bindex % BITS_PER_BYTE as u64))) != 0
        }
    }
    #[inline]
    fn get_from_builder(list: &ListBuilder, index: ElementCount32) -> bool {
        let bindex = index as u64 * list.step as u64;
        let b = unsafe { list.ptr.offset((bindex / BITS_PER_BYTE as u64) as isize) };
        unsafe { ((*b) & (1 << (bindex % BITS_PER_BYTE as u64 ))) != 0 }
    }
    #[inline]
    fn set(list: &ListBuilder, index: ElementCount32, value: bool) {
        let bindex = index as u64 * list.step as u64;
        let b = unsafe { list.ptr.offset((bindex / BITS_PER_BYTE as u64) as isize) };

        let bitnum = bindex % BITS_PER_BYTE as u64;
        unsafe { (*b) = ((*b) & !(1 << bitnum)) | ((value as u8) << bitnum) }
    }
    fn element_size() -> ElementSize { Bit }
}

impl PrimitiveElement for () {
    #[inline]
    fn get(_list: &ListReader, _index: ElementCount32) -> () { () }

    #[inline]
    fn get_from_builder(_list: &ListBuilder, _index: ElementCount32) -> () { () }

    #[inline]
    fn set(_list: &ListBuilder, _index: ElementCount32, _value: ()) { }
}
