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

#[cfg(feature = "alloc")]
use alloc::{boxed::Box, vec::Vec};
use core::cell::Cell;
use core::mem;
use core::ptr;

use crate::data;
use crate::private::arena::{BuilderArena, NullArena, ReaderArena, SegmentId};
#[cfg(feature = "alloc")]
use crate::private::capability::ClientHook;
use crate::private::mask::Mask;
use crate::private::primitive::{Primitive, WireValue};
use crate::private::units::*;
use crate::private::zero;
use crate::text;
use crate::{Error, ErrorKind, MessageSize, Result};

pub use self::ElementSize::{
    Bit, Byte, EightBytes, FourBytes, InlineComposite, Pointer, TwoBytes, Void,
};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ElementSize {
    Void = 0,
    Bit = 1,
    Byte = 2,
    TwoBytes = 3,
    FourBytes = 4,
    EightBytes = 5,
    Pointer = 6,
    InlineComposite = 7,
}

impl ElementSize {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Void,
            1 => Self::Bit,
            2 => Self::Byte,
            3 => Self::TwoBytes,
            4 => Self::FourBytes,
            5 => Self::EightBytes,
            6 => Self::Pointer,
            7 => Self::InlineComposite,
            _ => panic!("illegal element size: {val}"),
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
        InlineComposite => 0,
    }
}

pub fn pointers_per_element(size: ElementSize) -> WirePointerCount32 {
    match size {
        Pointer => 1,
        _ => 0,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct StructSize {
    pub data: WordCount16,
    pub pointers: WirePointerCount16,
}

impl StructSize {
    pub fn total(&self) -> WordCount32 {
        u32::from(self.data) + u32::from(self.pointers) * WORDS_PER_POINTER as WordCount32
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
pub enum WirePointerKind {
    Struct = 0,
    List = 1,
    Far = 2,
    Other = 3,
}

pub enum PointerType {
    Null,
    Struct,
    List,
    Capability,
}

impl WirePointerKind {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Struct,
            1 => Self::List,
            2 => Self::Far,
            3 => Self::Other,
            _ => panic!("illegal element size: {val}"),
        }
    }
}

#[repr(C)]
pub struct WirePointer {
    offset_and_kind: WireValue<u32>,
    upper32bits: WireValue<u32>,
}

#[test]
#[cfg(feature = "unaligned")]
fn wire_pointer_align() {
    // We cast *u8 to *WirePointer, so we need to make sure its alignment allows that.
    assert_eq!(core::mem::align_of::<WirePointer>(), 1);
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
    pub unsafe fn target(ptr: *const Self) -> *const u8 {
        let this_addr: *const u8 = ptr as *const _;
        unsafe { this_addr.offset(8 * (1 + (((*ptr).offset_and_kind.get() as i32) >> 2)) as isize) }
    }

    // At one point, we had `&self` here instead of `ptr: *const Self`, but miri
    // flagged that as running afoul of "stacked borrow" rules.
    #[inline]
    fn target_from_segment(
        ptr: *const Self,
        arena: &dyn ReaderArena,
        segment_id: u32,
    ) -> Result<*const u8> {
        let this_addr: *const u8 = ptr as *const _;
        unsafe {
            let offset = 1 + (((*ptr).offset_and_kind.get() as i32) >> 2);
            arena.check_offset(segment_id, this_addr, offset)
        }
    }

    // At one point, we had `&mut self` here instead of `ptr: *mut Self`, but miri
    // flagged that as running afoul of "stacked borrow" rules.
    #[inline]
    fn mut_target(ptr: *mut Self) -> *mut u8 {
        let this_addr: *mut u8 = ptr as *mut _;
        unsafe {
            this_addr.wrapping_offset(
                BYTES_PER_WORD as isize
                    * (1 + (((*ptr).offset_and_kind.get() as i32) >> 2)) as isize,
            )
        }
    }

    #[inline]
    pub fn set_kind_and_target(&mut self, kind: WirePointerKind, target: *mut u8) {
        let this_addr: isize = self as *const _ as isize;
        let target_addr: isize = target as *const _ as isize;
        self.offset_and_kind.set(
            ((((target_addr - this_addr) / BYTES_PER_WORD as isize) as i32 - 1) << 2) as u32
                | (kind as u32),
        )
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
    pub fn set_kind_and_inline_composite_list_element_count(
        &mut self,
        kind: WirePointerKind,
        element_count: ElementCount32,
    ) {
        self.offset_and_kind
            .set((element_count << 2) | (kind as u32))
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
            .set((pos << 3) | (u32::from(is_double_far) << 2) | WirePointerKind::Far as u32);
    }

    #[inline]
    pub fn set_cap(&mut self, index: u32) {
        self.offset_and_kind.set(WirePointerKind::Other as u32);
        self.upper32bits.set(index);
    }

    #[inline]
    pub fn struct_data_size(&self) -> WordCount16 {
        self.upper32bits.get() as WordCount16
    }

    #[inline]
    pub fn struct_ptr_count(&self) -> WordCount16 {
        (self.upper32bits.get() >> 16) as WordCount16
    }

    #[inline]
    pub fn struct_word_size(&self) -> WordCount32 {
        u32::from(self.struct_data_size())
            + u32::from(self.struct_ptr_count()) * WORDS_PER_POINTER as u32
    }

    #[inline]
    pub fn set_struct_size(&mut self, size: StructSize) {
        self.upper32bits
            .set(u32::from(size.data) | (u32::from(size.pointers) << 16))
    }

    #[inline]
    pub fn set_struct_size_from_pieces(&mut self, ds: WordCount16, rc: WirePointerCount16) {
        self.set_struct_size(StructSize {
            data: ds,
            pointers: rc,
        })
    }

    #[inline]
    pub fn list_element_size(&self) -> ElementSize {
        ElementSize::from(self.upper32bits.get() as u8 & 7)
    }

    #[inline]
    pub fn list_element_count(&self) -> ElementCount32 {
        self.upper32bits.get() >> 3
    }

    #[inline]
    pub fn list_inline_composite_word_count(&self) -> WordCount32 {
        self.list_element_count()
    }

    #[inline]
    pub fn set_list_size_and_count(&mut self, es: ElementSize, ec: ElementCount32) {
        assert!(ec < (1 << 29), "Lists are limited to 2**29 elements");
        self.upper32bits.set((ec << 3) | (es as u32));
    }

    #[inline]
    pub fn set_list_inline_composite(&mut self, wc: WordCount32) {
        assert!(
            wc < (1 << 29),
            "Inline composite lists are limited to 2**29 words"
        );
        self.upper32bits.set((wc << 3) | (InlineComposite as u32));
    }

    #[inline]
    pub fn far_segment_id(&self) -> SegmentId {
        self.upper32bits.get() as SegmentId
    }

    #[inline]
    pub fn set_far_segment_id(&mut self, si: SegmentId) {
        self.upper32bits.set(si)
    }

    #[inline]
    pub fn cap_index(&self) -> u32 {
        self.upper32bits.get()
    }

    #[inline]
    pub fn set_cap_index(&mut self, index: u32) {
        self.upper32bits.set(index)
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.offset_and_kind.get() == 0 && self.upper32bits.get() == 0
    }
}

mod wire_helpers {
    #[cfg(feature = "alloc")]
    use alloc::boxed::Box;
    use core::{ptr, slice};

    use crate::data;
    use crate::private::arena::*;
    #[cfg(feature = "alloc")]
    use crate::private::capability::ClientHook;
    use crate::private::layout::ElementSize::*;
    use crate::private::layout::{data_bits_per_element, pointers_per_element};
    use crate::private::layout::{CapTableBuilder, CapTableReader};
    use crate::private::layout::{
        ElementSize, ListBuilder, ListReader, StructBuilder, StructReader, StructSize, WirePointer,
        WirePointerKind,
    };
    use crate::private::units::*;
    use crate::text;
    use crate::{Error, ErrorKind, MessageSize, Result};

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
    pub fn bounds_check(
        arena: &dyn ReaderArena,
        segment_id: u32,
        start: *const u8,
        size_in_words: usize,
        _kind: WirePointerKind,
    ) -> Result<()> {
        arena.contains_interval(segment_id, start, size_in_words)
    }

    #[inline]
    pub fn amplified_read(arena: &dyn ReaderArena, virtual_amount: u64) -> Result<()> {
        arena.amplified_read(virtual_amount)
    }

    #[inline]
    pub unsafe fn allocate(
        arena: &mut dyn BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        amount: WordCount32,
        kind: WirePointerKind,
    ) -> (*mut u8, *mut WirePointer, u32) {
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
                let ptr = seg_start.offset(word_idx as isize * BYTES_PER_WORD as isize);

                //# Set up the original pointer to be a far pointer to
                //# the new segment.
                (*reff).set_far(false, word_idx);
                (*reff).set_far_segment_id(segment_id);

                //# Initialize the landing pad to indicate that the
                //# data immediately follows the pad.
                let reff = ptr as *mut WirePointer;

                let ptr1 = ptr.add(BYTES_PER_WORD);
                (*reff).set_kind_and_target(kind, ptr1);
                (ptr1, reff, segment_id)
            }
            Some(idx) => {
                let (seg_start, _seg_len) = arena.get_segment_mut(segment_id);
                let ptr = (seg_start).offset(idx as isize * BYTES_PER_WORD as isize);
                (*reff).set_kind_and_target(kind, ptr);
                (ptr, reff, segment_id)
            }
        }
    }

    #[inline]
    pub unsafe fn follow_builder_fars(
        arena: &mut dyn BuilderArena,
        reff: *mut WirePointer,
        ref_target: *mut u8,
        segment_id: u32,
    ) -> Result<(*mut u8, *mut WirePointer, u32)> {
        // If `ref` is a far pointer, follow it. On return, `ref` will have been updated to point at
        // a WirePointer that contains the type information about the target object, and a pointer
        // to the object contents is returned. The caller must NOT use `ref->target()` as this may
        // or may not actually return a valid pointer. `segment` is also updated to point at the
        // segment which actually contains the object.
        //
        // If `ref` is not a far pointer, this simply returns `ref_target`. Usually, `ref_target`
        // should be the same as `ref->target()`, but may not be in cases where `ref` is only a tag.

        if (*reff).kind() == WirePointerKind::Far {
            let segment_id = (*reff).far_segment_id();
            let (seg_start, _seg_len) = arena.get_segment_mut(segment_id);
            let pad: *mut WirePointer =
                (seg_start as *mut WirePointer).offset((*reff).far_position_in_segment() as isize);
            if !(*reff).is_double_far() {
                Ok((WirePointer::mut_target(pad), pad, segment_id))
            } else {
                //# Landing pad is another far pointer. It is followed by a
                //# tag describing the pointed-to object.
                let reff = pad.offset(1);

                let segment_id = (*pad).far_segment_id();
                let (segment_start, _segment_len) = arena.get_segment_mut(segment_id);
                let ptr = segment_start
                    .offset((*pad).far_position_in_segment() as isize * BYTES_PER_WORD as isize);
                Ok((ptr, reff, segment_id))
            }
        } else {
            Ok((ref_target, reff, segment_id))
        }
    }

    /// Follows a WirePointer to get a triple containing:
    ///   - the pointed-to object
    ///   - the resolved WirePointer, whose kind is something other than WirePointerKind::Far
    ///   - the segment on which the pointed-to object lives
    #[inline]
    pub unsafe fn follow_fars(
        arena: &dyn ReaderArena,
        reff: *const WirePointer,
        segment_id: u32,
    ) -> Result<(*const u8, *const WirePointer, u32)> {
        if (*reff).kind() == WirePointerKind::Far {
            let far_segment_id = (*reff).far_segment_id();

            let (seg_start, _seg_len) = arena.get_segment(far_segment_id)?;
            let ptr = seg_start
                .offset((*reff).far_position_in_segment() as isize * BYTES_PER_WORD as isize);

            let pad_words: usize = if (*reff).is_double_far() { 2 } else { 1 };
            bounds_check(arena, far_segment_id, ptr, pad_words, WirePointerKind::Far)?;

            let pad: *const WirePointer = ptr as *const _;

            if !(*reff).is_double_far() {
                Ok((
                    WirePointer::target_from_segment(pad, arena, far_segment_id)?,
                    pad,
                    far_segment_id,
                ))
            } else {
                // Landing pad is another far pointer. It is followed by a tag describing the
                // pointed-to object.

                let tag = pad.offset(1);
                let double_far_segment_id = (*pad).far_segment_id();
                let (segment_start, _segment_len) = arena.get_segment(double_far_segment_id)?;
                let ptr = segment_start
                    .offset((*pad).far_position_in_segment() as isize * BYTES_PER_WORD as isize);
                Ok((ptr, tag, double_far_segment_id))
            }
        } else {
            Ok((
                WirePointer::target_from_segment(reff, arena, segment_id)?,
                reff,
                segment_id,
            ))
        }
    }

    pub unsafe fn zero_object(
        arena: &mut dyn BuilderArena,
        segment_id: u32,
        reff: *mut WirePointer,
    ) {
        //# Zero out the pointed-to object. Use when the pointer is
        //# about to be overwritten making the target object no longer
        //# reachable.

        match (*reff).kind() {
            WirePointerKind::Struct | WirePointerKind::List | WirePointerKind::Other => {
                zero_object_helper(arena, segment_id, reff, WirePointer::mut_target(reff))
            }
            WirePointerKind::Far => {
                let segment_id = (*reff).far_segment_id();
                let (seg_start, _seg_len) = arena.get_segment_mut(segment_id);
                let pad: *mut WirePointer = (seg_start as *mut WirePointer)
                    .offset((*reff).far_position_in_segment() as isize);

                if (*reff).is_double_far() {
                    let segment_id = (*pad).far_segment_id();

                    let (seg_start, _seg_len) = arena.get_segment_mut(segment_id);
                    let ptr = seg_start.offset(
                        (*pad).far_position_in_segment() as isize * BYTES_PER_WORD as isize,
                    );
                    zero_object_helper(arena, segment_id, pad.offset(1), ptr);

                    ptr::write_bytes(pad, 0u8, 2);
                } else {
                    zero_object(arena, segment_id, pad);
                    ptr::write_bytes(pad, 0u8, 1);
                }
            }
        }
    }

    pub unsafe fn zero_object_helper(
        arena: &mut dyn BuilderArena,
        segment_id: u32,
        tag: *mut WirePointer,
        ptr: *mut u8,
    ) {
        match (*tag).kind() {
            WirePointerKind::Other => {
                panic!("Don't know how to handle OTHER")
            }
            WirePointerKind::Struct => {
                let pointer_section: *mut WirePointer = ptr
                    .offset((*tag).struct_data_size() as isize * BYTES_PER_WORD as isize)
                    as *mut _;

                let count = (*tag).struct_ptr_count() as isize;
                for i in 0..count {
                    zero_object(arena, segment_id, pointer_section.offset(i));
                }
                ptr::write_bytes(
                    ptr,
                    0u8,
                    (*tag).struct_word_size() as usize * BYTES_PER_WORD,
                );
            }
            WirePointerKind::List => match (*tag).list_element_size() {
                Void => {}
                Bit | Byte | TwoBytes | FourBytes | EightBytes => ptr::write_bytes(
                    ptr,
                    0u8,
                    BYTES_PER_WORD
                        * round_bits_up_to_words(
                            u64::from((*tag).list_element_count())
                                * u64::from(data_bits_per_element((*tag).list_element_size())),
                        ) as usize,
                ),
                Pointer => {
                    let count = (*tag).list_element_count() as usize;
                    for i in 0..count as isize {
                        zero_object(
                            arena,
                            segment_id,
                            ptr.offset(i * BYTES_PER_WORD as isize) as *mut _,
                        );
                    }
                    ptr::write_bytes(ptr, 0u8, count * BYTES_PER_WORD);
                }
                InlineComposite => {
                    let element_tag: *mut WirePointer = ptr as *mut _;

                    assert!(
                        (*element_tag).kind() == WirePointerKind::Struct,
                        "Don't know how to handle non-STRUCT inline composite"
                    );

                    let data_size = (*element_tag).struct_data_size();
                    let pointer_count = (*element_tag).struct_ptr_count();
                    let mut pos = ptr.add(BYTES_PER_WORD);
                    let count = (*element_tag).inline_composite_list_element_count();
                    if pointer_count > 0 {
                        for _ in 0..count {
                            pos = pos.offset(data_size as isize * BYTES_PER_WORD as isize);
                            for _ in 0..pointer_count {
                                zero_object(arena, segment_id, pos as *mut WirePointer);
                                pos = pos.add(BYTES_PER_WORD);
                            }
                        }
                    }
                    ptr::write_bytes(
                        ptr,
                        0u8,
                        BYTES_PER_WORD * ((*element_tag).struct_word_size() * count + 1) as usize,
                    );
                }
            },
            WirePointerKind::Far => {
                panic!("Unexpected FAR pointer")
            }
        }
    }

    #[inline]
    pub unsafe fn zero_pointer_and_fars(
        arena: &mut dyn BuilderArena,
        _segment_id: u32,
        reff: *mut WirePointer,
    ) -> Result<()> {
        // Zero out the pointer itself and, if it is a far pointer, zero the landing pad as well,
        // but do not zero the object body. Used when upgrading.

        if (*reff).kind() == WirePointerKind::Far {
            let far_segment_id = (*reff).far_segment_id();
            let (seg_start, _seg_len) = arena.get_segment_mut(far_segment_id);
            let pad = seg_start
                .offset((*reff).far_position_in_segment() as isize * BYTES_PER_WORD as isize);
            let num_elements = if (*reff).is_double_far() { 2 } else { 1 };
            ptr::write_bytes(pad, 0, num_elements * BYTES_PER_WORD);
        }
        ptr::write_bytes(reff, 0, 1);
        Ok(())
    }

    pub unsafe fn total_size(
        arena: &dyn ReaderArena,
        segment_id: u32,
        reff: *const WirePointer,
        mut nesting_limit: i32,
    ) -> Result<MessageSize> {
        let mut result = MessageSize {
            word_count: 0,
            cap_count: 0,
        };

        if (*reff).is_null() {
            return Ok(result);
        };

        if nesting_limit <= 0 {
            return Err(Error::from_kind(ErrorKind::MessageIsTooDeeplyNested));
        }

        nesting_limit -= 1;

        let (ptr, reff, segment_id) = follow_fars(arena, reff, segment_id)?;

        match (*reff).kind() {
            WirePointerKind::Struct => {
                bounds_check(
                    arena,
                    segment_id,
                    ptr,
                    (*reff).struct_word_size() as usize,
                    WirePointerKind::Struct,
                )?;
                result.word_count += u64::from((*reff).struct_word_size());

                let pointer_section: *const WirePointer = ptr
                    .offset((*reff).struct_data_size() as isize * BYTES_PER_WORD as isize)
                    as *const _;
                let count: isize = (*reff).struct_ptr_count() as isize;
                for i in 0..count {
                    result +=
                        total_size(arena, segment_id, pointer_section.offset(i), nesting_limit)?;
                }
            }
            WirePointerKind::List => {
                match (*reff).list_element_size() {
                    Void => {}
                    Bit | Byte | TwoBytes | FourBytes | EightBytes => {
                        let total_words = round_bits_up_to_words(
                            u64::from((*reff).list_element_count())
                                * u64::from(data_bits_per_element((*reff).list_element_size())),
                        );
                        bounds_check(
                            arena,
                            segment_id,
                            ptr,
                            total_words as usize,
                            WirePointerKind::List,
                        )?;
                        result.word_count += u64::from(total_words);
                    }
                    Pointer => {
                        let count = (*reff).list_element_count();
                        bounds_check(
                            arena,
                            segment_id,
                            ptr,
                            count as usize * WORDS_PER_POINTER,
                            WirePointerKind::List,
                        )?;

                        result.word_count += u64::from(count) * WORDS_PER_POINTER as u64;

                        for i in 0..count as isize {
                            result += total_size(
                                arena,
                                segment_id,
                                (ptr as *const WirePointer).offset(i),
                                nesting_limit,
                            )?;
                        }
                    }
                    InlineComposite => {
                        let word_count = (*reff).list_inline_composite_word_count();
                        bounds_check(
                            arena,
                            segment_id,
                            ptr,
                            word_count as usize + POINTER_SIZE_IN_WORDS,
                            WirePointerKind::List,
                        )?;

                        let element_tag: *const WirePointer = ptr as *const _;
                        let count = (*element_tag).inline_composite_list_element_count();

                        if (*element_tag).kind() != WirePointerKind::Struct {
                            return Err(Error::from_kind(
                                ErrorKind::CantHandleNonStructInlineComposite,
                            ));
                        }

                        let actual_size =
                            u64::from((*element_tag).struct_word_size()) * u64::from(count);
                        if actual_size > u64::from(word_count) {
                            return Err(Error::from_kind(
                                ErrorKind::InlineCompositeListsElementsOverrunItsWordCount,
                            ));
                        }

                        // Count the actual size rather than the claimed word count because
                        // that's what we end up with if we make a copy.
                        result.word_count += actual_size + POINTER_SIZE_IN_WORDS as u64;

                        let data_size = (*element_tag).struct_data_size();
                        let pointer_count = (*element_tag).struct_ptr_count();

                        if pointer_count > 0 {
                            let mut pos = ptr.add(BYTES_PER_WORD);
                            for _ in 0..count {
                                pos = pos.offset(data_size as isize * BYTES_PER_WORD as isize);

                                for _ in 0..pointer_count {
                                    result += total_size(
                                        arena,
                                        segment_id,
                                        pos as *const WirePointer,
                                        nesting_limit,
                                    )?;
                                    pos = pos.add(BYTES_PER_WORD);
                                }
                            }
                        }
                    }
                }
            }
            WirePointerKind::Far => {
                return Err(Error::from_kind(ErrorKind::MalformedDoubleFarPointer));
            }
            WirePointerKind::Other => {
                if (*reff).is_capability() {
                    result.cap_count += 1;
                } else {
                    return Err(Error::from_kind(ErrorKind::UnknownPointerType));
                }
            }
        }

        Ok(result)
    }

    // Helper for copy_message().
    unsafe fn copy_struct(
        arena: &mut dyn BuilderArena,
        segment_id: u32,
        cap_table: CapTableBuilder,
        dst: *mut u8,
        src: *const u8,
        data_size: isize,
        pointer_count: isize,
    ) {
        ptr::copy_nonoverlapping(src, dst, data_size as usize * BYTES_PER_WORD);

        let src_refs: *const WirePointer = (src as *const WirePointer).offset(data_size);
        let dst_refs: *mut WirePointer = (dst as *mut WirePointer).offset(data_size);

        for ii in 0..pointer_count {
            copy_message(
                arena,
                segment_id,
                cap_table,
                dst_refs.offset(ii),
                src_refs.offset(ii),
            );
        }
    }

    // Copies from a trusted message.
    // Returns (new_dst_ptr, new_dst, new_segment_id).
    pub unsafe fn copy_message(
        arena: &mut dyn BuilderArena,
        segment_id: u32,
        cap_table: CapTableBuilder,
        dst: *mut WirePointer,
        src: *const WirePointer,
    ) -> (*mut u8, *mut WirePointer, u32) {
        match (*src).kind() {
            WirePointerKind::Struct => {
                if (*src).is_null() {
                    ptr::write_bytes(dst, 0, 1);
                    (ptr::null_mut(), dst, segment_id)
                } else {
                    let src_ptr = WirePointer::target(src);
                    let (dst_ptr, dst, segment_id) = allocate(
                        arena,
                        dst,
                        segment_id,
                        (*src).struct_word_size(),
                        WirePointerKind::Struct,
                    );
                    copy_struct(
                        arena,
                        segment_id,
                        cap_table,
                        dst_ptr,
                        src_ptr,
                        (*src).struct_data_size() as isize,
                        (*src).struct_ptr_count() as isize,
                    );
                    (*dst).set_struct_size_from_pieces(
                        (*src).struct_data_size(),
                        (*src).struct_ptr_count(),
                    );
                    (dst_ptr, dst, segment_id)
                }
            }
            WirePointerKind::List => match (*src).list_element_size() {
                ElementSize::Void
                | ElementSize::Bit
                | ElementSize::Byte
                | ElementSize::TwoBytes
                | ElementSize::FourBytes
                | ElementSize::EightBytes => {
                    let word_count = round_bits_up_to_words(
                        u64::from((*src).list_element_count())
                            * u64::from(data_bits_per_element((*src).list_element_size())),
                    );
                    let src_ptr = WirePointer::target(src);
                    let (dst_ptr, dst, segment_id) =
                        allocate(arena, dst, segment_id, word_count, WirePointerKind::List);
                    ptr::copy_nonoverlapping(
                        src_ptr,
                        dst_ptr,
                        word_count as usize * BYTES_PER_WORD,
                    );
                    (*dst).set_list_size_and_count(
                        (*src).list_element_size(),
                        (*src).list_element_count(),
                    );
                    (dst_ptr, dst, segment_id)
                }

                ElementSize::Pointer => {
                    let src_refs: *const WirePointer = WirePointer::target(src) as _;
                    let (dst_refs, dst, segment_id) = allocate(
                        arena,
                        dst,
                        segment_id,
                        (*src).list_element_count(),
                        WirePointerKind::List,
                    );
                    for ii in 0..((*src).list_element_count() as isize) {
                        copy_message(
                            arena,
                            segment_id,
                            cap_table,
                            dst_refs.offset(ii * BYTES_PER_WORD as isize) as *mut WirePointer,
                            src_refs.offset(ii),
                        );
                    }
                    (*dst)
                        .set_list_size_and_count(ElementSize::Pointer, (*src).list_element_count());
                    (dst_refs, dst, segment_id)
                }
                ElementSize::InlineComposite => {
                    let src_ptr = WirePointer::target(src);
                    let (dst_ptr, dst, segment_id) = allocate(
                        arena,
                        dst,
                        segment_id,
                        (*src).list_inline_composite_word_count() + 1,
                        WirePointerKind::List,
                    );

                    (*dst).set_list_inline_composite((*src).list_inline_composite_word_count());

                    let src_tag: *const WirePointer = src_ptr as _;
                    ptr::copy_nonoverlapping(src_tag, dst_ptr as *mut WirePointer, 1);

                    let mut src_element = src_ptr.add(BYTES_PER_WORD);
                    let mut dst_element = dst_ptr.add(BYTES_PER_WORD);

                    if (*src_tag).kind() != WirePointerKind::Struct {
                        panic!("unsupported INLINE_COMPOSITE list");
                    }
                    for _ in 0..(*src_tag).inline_composite_list_element_count() {
                        copy_struct(
                            arena,
                            segment_id,
                            cap_table,
                            dst_element,
                            src_element,
                            (*src_tag).struct_data_size() as isize,
                            (*src_tag).struct_ptr_count() as isize,
                        );
                        src_element = src_element.offset(
                            BYTES_PER_WORD as isize * (*src_tag).struct_word_size() as isize,
                        );
                        dst_element = dst_element.offset(
                            BYTES_PER_WORD as isize * (*src_tag).struct_word_size() as isize,
                        );
                    }
                    (dst_ptr, dst, segment_id)
                }
            },
            WirePointerKind::Other => {
                panic!("Unchecked message contained an OTHER pointer.")
            }
            WirePointerKind::Far => {
                panic!("Unchecked message contained a far pointer.")
            }
        }
    }

    pub unsafe fn transfer_pointer(
        arena: &mut dyn BuilderArena,
        dst_segment_id: u32,
        dst: *mut WirePointer,
        src_segment_id: u32,
        src: *mut WirePointer,
    ) {
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
            transfer_pointer_split(
                arena,
                dst_segment_id,
                dst,
                src_segment_id,
                src,
                WirePointer::mut_target(src),
            );
        } else {
            ptr::copy_nonoverlapping(src, dst, 1);
        }
    }

    pub unsafe fn transfer_pointer_split(
        arena: &mut dyn BuilderArena,
        dst_segment_id: u32,
        dst: *mut WirePointer,
        src_segment_id: u32,
        src_tag: *mut WirePointer,
        src_ptr: *mut u8,
    ) {
        // Like the other transfer_pointer, but splits src into a tag and a
        // target. Particularly useful for OrphanBuilder.

        if dst_segment_id == src_segment_id {
            // Same segment, so create a direct pointer.

            if (*src_tag).kind() == WirePointerKind::Struct && (*src_tag).struct_word_size() == 0 {
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
                    let landing_pad: *mut WirePointer =
                        (seg_start as *mut WirePointer).offset(word_idx as isize);

                    let (src_seg_start, _seg_len) = arena.get_segment_mut(src_segment_id);

                    (*landing_pad).set_far(
                        false,
                        ((src_ptr as usize - src_seg_start as usize) / BYTES_PER_WORD) as u32,
                    );
                    (*landing_pad).set_far_segment_id(src_segment_id);

                    let landing_pad1 = landing_pad.offset(1);
                    (*landing_pad1).set_kind_with_zero_offset((*src_tag).kind());

                    ptr::copy_nonoverlapping(
                        &(*src_tag).upper32bits,
                        &mut (*landing_pad1).upper32bits,
                        1,
                    );

                    (*dst).set_far(true, word_idx);
                    (*dst).set_far_segment_id(far_segment_id);
                }
                Some(landing_pad_word) => {
                    //# Simple landing pad is just a pointer.
                    let (seg_start, seg_len) = arena.get_segment_mut(src_segment_id);
                    assert!(landing_pad_word < seg_len);
                    let landing_pad: *mut WirePointer =
                        (seg_start as *mut WirePointer).offset(landing_pad_word as isize);
                    (*landing_pad).set_kind_and_target((*src_tag).kind(), src_ptr);
                    ptr::copy_nonoverlapping(
                        &(*src_tag).upper32bits,
                        &mut (*landing_pad).upper32bits,
                        1,
                    );

                    (*dst).set_far(false, landing_pad_word);
                    (*dst).set_far_segment_id(src_segment_id);
                }
            }
        }
    }

    #[inline]
    pub unsafe fn init_struct_pointer(
        arena: &mut dyn BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        cap_table: CapTableBuilder,
        size: StructSize,
    ) -> StructBuilder<'_> {
        let (ptr, reff, segment_id) = allocate(
            arena,
            reff,
            segment_id,
            size.total(),
            WirePointerKind::Struct,
        );
        (*reff).set_struct_size(size);

        StructBuilder {
            arena,
            segment_id,
            cap_table,
            data: ptr as *mut _,
            pointers: ptr.offset((size.data as usize) as isize * BYTES_PER_WORD as isize) as *mut _,
            data_size: u32::from(size.data) * (BITS_PER_WORD as BitCount32),
            pointer_count: size.pointers,
        }
    }

    #[inline]
    pub unsafe fn get_writable_struct_pointer<'a>(
        arena: &'a mut dyn BuilderArena,
        mut reff: *mut WirePointer,
        mut segment_id: u32,
        cap_table: CapTableBuilder,
        size: StructSize,
        default: Option<&'a [crate::Word]>,
    ) -> Result<StructBuilder<'a>> {
        let mut ref_target = WirePointer::mut_target(reff);

        if (*reff).is_null() {
            match default {
                None => {
                    return Ok(init_struct_pointer(
                        arena, reff, segment_id, cap_table, size,
                    ))
                }
                Some(d) if (*(d.as_ptr() as *const WirePointer)).is_null() => {
                    return Ok(init_struct_pointer(
                        arena, reff, segment_id, cap_table, size,
                    ))
                }
                Some(d) => {
                    let (new_ref_target, new_reff, new_segment_id) = copy_message(
                        arena,
                        segment_id,
                        cap_table,
                        reff,
                        d.as_ptr() as *const WirePointer,
                    );
                    reff = new_reff;
                    segment_id = new_segment_id;
                    ref_target = new_ref_target;
                }
            }
        }

        let (old_ptr, old_ref, old_segment_id) =
            follow_builder_fars(arena, reff, ref_target, segment_id)?;
        if (*old_ref).kind() != WirePointerKind::Struct {
            return Err(Error::from_kind(
                ErrorKind::MessageContainsNonStructPointerWhereStructPointerWasExpected,
            ));
        }

        let old_data_size = (*old_ref).struct_data_size();
        let old_pointer_count = (*old_ref).struct_ptr_count();
        let old_pointer_section: *mut WirePointer =
            old_ptr.offset(old_data_size as isize * BYTES_PER_WORD as isize) as *mut _;

        if old_data_size < size.data || old_pointer_count < size.pointers {
            //# The space allocated for this struct is too small.
            //# Unlike with readers, we can't just run with it and do
            //# bounds checks at access time, because how would we
            //# handle writes? Instead, we have to copy the struct to a
            //# new space now.

            let new_data_size = ::core::cmp::max(old_data_size, size.data);
            let new_pointer_count = ::core::cmp::max(old_pointer_count, size.pointers);
            let total_size =
                u32::from(new_data_size) + u32::from(new_pointer_count) * WORDS_PER_POINTER as u32;

            //# Don't let allocate() zero out the object just yet.
            zero_pointer_and_fars(arena, segment_id, reff)?;

            let (ptr, reff, segment_id) =
                allocate(arena, reff, segment_id, total_size, WirePointerKind::Struct);
            (*reff).set_struct_size_from_pieces(new_data_size, new_pointer_count);

            // Copy data section.
            // Note: copy_nonoverlapping's third argument is an element count, not a byte count.
            ptr::copy_nonoverlapping(old_ptr, ptr, old_data_size as usize * BYTES_PER_WORD);

            //# Copy pointer section.
            let new_pointer_section: *mut WirePointer =
                ptr.offset(new_data_size as isize * BYTES_PER_WORD as isize) as *mut _;
            for i in 0..old_pointer_count as isize {
                transfer_pointer(
                    arena,
                    segment_id,
                    new_pointer_section.offset(i),
                    old_segment_id,
                    old_pointer_section.offset(i),
                );
            }

            ptr::write_bytes(
                old_ptr,
                0,
                (old_data_size as usize + old_pointer_count as usize) * BYTES_PER_WORD,
            );

            Ok(StructBuilder {
                arena,
                segment_id,
                cap_table,
                data: ptr as *mut _,
                pointers: new_pointer_section,
                data_size: u32::from(new_data_size) * BITS_PER_WORD as u32,
                pointer_count: new_pointer_count,
            })
        } else {
            Ok(StructBuilder {
                arena,
                segment_id: old_segment_id,
                cap_table,
                data: old_ptr,
                pointers: old_pointer_section,
                data_size: u32::from(old_data_size) * BITS_PER_WORD as u32,
                pointer_count: old_pointer_count,
            })
        }
    }

    #[inline]
    pub unsafe fn init_list_pointer(
        arena: &mut dyn BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        cap_table: CapTableBuilder,
        element_count: ElementCount32,
        element_size: ElementSize,
    ) -> ListBuilder<'_> {
        assert!(
            element_size != InlineComposite,
            "Should have called initStructListPointer() instead"
        );

        let data_size = data_bits_per_element(element_size);
        let pointer_count = pointers_per_element(element_size);
        let step = data_size + pointer_count * BITS_PER_POINTER as u32;
        let word_count = round_bits_up_to_words(u64::from(element_count) * u64::from(step));
        let (ptr, reff, segment_id) =
            allocate(arena, reff, segment_id, word_count, WirePointerKind::List);

        (*reff).set_list_size_and_count(element_size, element_count);

        ListBuilder {
            arena,
            segment_id,
            cap_table,
            ptr,
            step,
            element_count,
            element_size,
            struct_data_size: data_size,
            struct_pointer_count: pointer_count as u16,
        }
    }

    #[inline]
    pub unsafe fn init_struct_list_pointer(
        arena: &mut dyn BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        cap_table: CapTableBuilder,
        element_count: ElementCount32,
        element_size: StructSize,
    ) -> ListBuilder<'_> {
        let words_per_element = element_size.total();

        //# Allocate the list, prefixed by a single WirePointer.
        let word_count: WordCount32 = element_count * words_per_element;
        let (ptr, reff, segment_id) = allocate(
            arena,
            reff,
            segment_id,
            POINTER_SIZE_IN_WORDS as u32 + word_count,
            WirePointerKind::List,
        );
        let ptr = ptr as *mut WirePointer;

        //# Initialize the pointer.
        (*reff).set_list_inline_composite(word_count);
        (*ptr).set_kind_and_inline_composite_list_element_count(
            WirePointerKind::Struct,
            element_count,
        );
        (*ptr).set_struct_size(element_size);

        let ptr1 = ptr.add(POINTER_SIZE_IN_WORDS);

        ListBuilder {
            arena,
            segment_id,
            cap_table,
            ptr: ptr1 as *mut _,
            step: words_per_element * BITS_PER_WORD as u32,
            element_count,
            element_size: ElementSize::InlineComposite,
            struct_data_size: u32::from(element_size.data) * (BITS_PER_WORD as u32),
            struct_pointer_count: element_size.pointers,
        }
    }

    #[inline]
    pub unsafe fn get_writable_list_pointer(
        arena: &mut dyn BuilderArena,
        mut orig_ref: *mut WirePointer,
        mut orig_segment_id: u32,
        cap_table: CapTableBuilder,
        element_size: ElementSize,
        default_value: *const u8,
    ) -> Result<ListBuilder<'_>> {
        assert!(
            element_size != InlineComposite,
            "Use get_writable_struct_list_pointer() for struct lists"
        );

        let mut orig_ref_target = WirePointer::mut_target(orig_ref);

        if (*orig_ref).is_null() {
            if default_value.is_null() || (*(default_value as *const WirePointer)).is_null() {
                return Ok(ListBuilder::new_default(arena));
            }
            let (new_orig_ref_target, new_orig_ref, new_orig_segment_id) = copy_message(
                arena,
                orig_segment_id,
                cap_table,
                orig_ref,
                default_value as *const WirePointer,
            );
            orig_ref_target = new_orig_ref_target;
            orig_ref = new_orig_ref;
            orig_segment_id = new_orig_segment_id;
        }

        // We must verify that the pointer has the right size. Unlike in
        // get_writable_struct_list_pointer(), we never need to "upgrade" the data, because this
        // method is called only for non-struct lists, and there is no allowed upgrade path *to* a
        // non-struct list, only *from* them.

        let (mut ptr, reff, segment_id) =
            follow_builder_fars(arena, orig_ref, orig_ref_target, orig_segment_id)?;

        if (*reff).kind() != WirePointerKind::List {
            return Err(Error::from_kind(ErrorKind::ExistingPointerIsNotAList));
        }

        let old_size = (*reff).list_element_size();

        if old_size == InlineComposite {
            // The existing element size is InlineComposite, which means that it is at least two
            // words, which makes it bigger than the expected element size. Since fields can only
            // grow when upgraded, the existing data must have been written with a newer version of
            // the protocol. We therefore never need to upgrade the data in this case, but we do
            // need to validate that it is a valid upgrade from what we expected.

            // Read the tag to get the actual element count.
            let tag: *const WirePointer = ptr as *const _;

            if (*tag).kind() != WirePointerKind::Struct {
                return Err(Error::from_kind(
                    ErrorKind::InlineCompositeListWithNonStructElementsNotSupported,
                ));
            }

            ptr = ptr.add(BYTES_PER_WORD);

            let data_size = (*tag).struct_data_size();
            let pointer_count = (*tag).struct_ptr_count();

            match element_size {
                Void => {} // Anything is a valid upgrade from Void.
                Bit => {
                    return Err(Error::from_kind(
                        ErrorKind::FoundStructListWhereBitListWasExpected,
                    ));
                }
                Byte | TwoBytes | FourBytes | EightBytes => {
                    if data_size < 1 {
                        return Err(Error::from_kind(
                            ErrorKind::ExistingListValueIsIncompatibleWithExpectedType,
                        ));
                    }
                }
                Pointer => {
                    if pointer_count < 1 {
                        return Err(Error::from_kind(
                            ErrorKind::ExistingListValueIsIncompatibleWithExpectedType,
                        ));
                    }
                    // Adjust the pointer to point at the reference segment.
                    ptr = ptr.offset(data_size as isize * BYTES_PER_WORD as isize);
                }
                InlineComposite => {
                    unreachable!()
                }
            }
            // OK, looks valid.

            Ok(ListBuilder {
                arena,
                segment_id,
                cap_table,
                ptr: ptr as *mut _,
                element_count: (*tag).inline_composite_list_element_count(),
                element_size: ElementSize::InlineComposite,
                step: (*tag).struct_word_size() * BITS_PER_WORD as u32,
                struct_data_size: u32::from(data_size) * BITS_PER_WORD as u32,
                struct_pointer_count: pointer_count,
            })
        } else {
            let data_size = data_bits_per_element(old_size);
            let pointer_count = pointers_per_element(old_size);

            if data_size < data_bits_per_element(element_size)
                || pointer_count < pointers_per_element(element_size)
            {
                return Err(Error::from_kind(
                    ErrorKind::ExistingListValueIsIncompatibleWithExpectedType,
                ));
            }

            let step = data_size + pointer_count * BITS_PER_POINTER as u32;

            Ok(ListBuilder {
                arena,
                segment_id,
                cap_table,
                ptr: ptr as *mut _,
                step,
                element_count: (*reff).list_element_count(),
                element_size: old_size,
                struct_data_size: data_size,
                struct_pointer_count: pointer_count as u16,
            })
        }
    }

    #[inline]
    pub unsafe fn get_writable_struct_list_pointer(
        arena: &mut dyn BuilderArena,
        mut orig_ref: *mut WirePointer,
        mut orig_segment_id: u32,
        cap_table: CapTableBuilder,
        element_size: StructSize,
        default_value: *const u8,
    ) -> Result<ListBuilder<'_>> {
        let mut orig_ref_target = WirePointer::mut_target(orig_ref);

        if (*orig_ref).is_null() {
            if default_value.is_null() || (*(default_value as *const WirePointer)).is_null() {
                return Ok(ListBuilder::new_default(arena));
            }
            let (new_orig_ref_target, new_orig_ref, new_orig_segment_id) = copy_message(
                arena,
                orig_segment_id,
                cap_table,
                orig_ref,
                default_value as *const WirePointer,
            );
            orig_ref_target = new_orig_ref_target;
            orig_ref = new_orig_ref;
            orig_segment_id = new_orig_segment_id;
        }

        // We must verify that the pointer has the right size and potentially upgrade it if not.

        let (mut old_ptr, old_ref, old_segment_id) =
            follow_builder_fars(arena, orig_ref, orig_ref_target, orig_segment_id)?;

        if (*old_ref).kind() != WirePointerKind::List {
            return Err(Error::from_kind(ErrorKind::ExistingPointerIsNotAList));
        }

        let old_size = (*old_ref).list_element_size();

        if old_size == InlineComposite {
            // Existing list is InlineComposite, but we need to verify that the sizes match.

            let old_tag: *const WirePointer = old_ptr as *const _;
            old_ptr = old_ptr.add(BYTES_PER_WORD);
            if (*old_tag).kind() != WirePointerKind::Struct {
                return Err(Error::from_kind(
                    ErrorKind::InlineCompositeListWithNonStructElementsNotSupported,
                ));
            }

            let old_data_size = (*old_tag).struct_data_size();
            let old_pointer_count = (*old_tag).struct_ptr_count();
            let old_step =
                u32::from(old_data_size) + u32::from(old_pointer_count) * WORDS_PER_POINTER as u32;
            let element_count = (*old_tag).inline_composite_list_element_count();

            if old_data_size >= element_size.data && old_pointer_count >= element_size.pointers {
                // Old size is at least as large as we need. Ship it.
                return Ok(ListBuilder {
                    arena,
                    segment_id: old_segment_id,
                    cap_table,
                    ptr: old_ptr as *mut _,
                    element_count,
                    element_size: ElementSize::InlineComposite,
                    step: old_step * BITS_PER_WORD as u32,
                    struct_data_size: u32::from(old_data_size) * BITS_PER_WORD as u32,
                    struct_pointer_count: old_pointer_count,
                });
            }

            // The structs in this list are smaller than expected, probably written using an older
            // version of the protocol. We need to make a copy and expand them.

            let new_data_size = ::core::cmp::max(old_data_size, element_size.data);
            let new_pointer_count = ::core::cmp::max(old_pointer_count, element_size.pointers);
            let new_step =
                u32::from(new_data_size) + u32::from(new_pointer_count) * WORDS_PER_POINTER as u32;
            let total_size = new_step * element_count;

            // Don't let allocate() zero out the object just yet.
            zero_pointer_and_fars(arena, orig_segment_id, orig_ref)?;

            let (mut new_ptr, new_ref, new_segment_id) = allocate(
                arena,
                orig_ref,
                orig_segment_id,
                total_size + POINTER_SIZE_IN_WORDS as u32,
                WirePointerKind::List,
            );
            (*new_ref).set_list_inline_composite(total_size);

            let new_tag: *mut WirePointer = new_ptr as *mut _;
            (*new_tag).set_kind_and_inline_composite_list_element_count(
                WirePointerKind::Struct,
                element_count,
            );
            (*new_tag).set_struct_size_from_pieces(new_data_size, new_pointer_count);
            new_ptr = new_ptr.add(BYTES_PER_WORD);

            let mut src = old_ptr as *mut WirePointer;
            let mut dst = new_ptr as *mut WirePointer;
            for _ in 0..element_count {
                // Copy data section.
                ptr::copy_nonoverlapping(src, dst, old_data_size as usize);

                // Copy pointer section
                let new_pointer_section = dst.offset(new_data_size as isize);
                let old_pointer_section = src.offset(old_data_size as isize);
                for jj in 0..(old_pointer_count as isize) {
                    transfer_pointer(
                        arena,
                        new_segment_id,
                        new_pointer_section.offset(jj),
                        old_segment_id,
                        old_pointer_section.offset(jj),
                    );
                }

                dst = dst.offset(new_step as isize);
                src = src.offset(old_step as isize);
            }

            ptr::write_bytes(
                old_ptr.offset(-(BYTES_PER_WORD as isize)),
                0,
                (u64::from(old_step) * u64::from(element_count)) as usize * BYTES_PER_WORD,
            );

            Ok(ListBuilder {
                arena,
                segment_id: new_segment_id,
                cap_table,
                ptr: new_ptr,
                element_count,
                element_size: ElementSize::InlineComposite,
                step: new_step * BITS_PER_WORD as u32,
                struct_data_size: u32::from(new_data_size) * BITS_PER_WORD as u32,
                struct_pointer_count: new_pointer_count,
            })
        } else {
            // We're upgrading from a non-struct list.

            let old_data_size = data_bits_per_element(old_size);
            let old_pointer_count = pointers_per_element(old_size);
            let old_step = old_data_size + old_pointer_count * BITS_PER_POINTER as u32;
            let element_count = (*old_ref).list_element_count();

            if old_size == ElementSize::Void {
                // Nothing to copy, just allocate a new list.
                Ok(init_struct_list_pointer(
                    arena,
                    orig_ref,
                    orig_segment_id,
                    cap_table,
                    element_count,
                    element_size,
                ))
            } else {
                // Upgrade to an inline composite list.

                if old_size == ElementSize::Bit {
                    return Err(Error::from_kind(
                        ErrorKind::FoundBitListWhereStructListWasExpected,
                    ));
                }

                let mut new_data_size = element_size.data;
                let mut new_pointer_count = element_size.pointers;

                if old_size == ElementSize::Pointer {
                    new_pointer_count = ::core::cmp::max(new_pointer_count, 1);
                } else {
                    // Old list contains data elements, so we need at least one word of data.
                    new_data_size = ::core::cmp::max(new_data_size, 1);
                }

                let new_step = u32::from(new_data_size)
                    + u32::from(new_pointer_count) * WORDS_PER_POINTER as u32;
                let total_words = element_count * new_step;

                // Don't let allocate() zero out the object just yet.
                zero_pointer_and_fars(arena, orig_segment_id, orig_ref)?;

                let (mut new_ptr, new_ref, new_segment_id) = allocate(
                    arena,
                    orig_ref,
                    orig_segment_id,
                    total_words + POINTER_SIZE_IN_WORDS as u32,
                    WirePointerKind::List,
                );
                (*new_ref).set_list_inline_composite(total_words);

                let tag: *mut WirePointer = new_ptr as *mut _;
                (*tag).set_kind_and_inline_composite_list_element_count(
                    WirePointerKind::Struct,
                    element_count,
                );
                (*tag).set_struct_size_from_pieces(new_data_size, new_pointer_count);
                new_ptr = new_ptr.add(BYTES_PER_WORD);

                if old_size == ElementSize::Pointer {
                    let mut dst = new_ptr.offset(new_data_size as isize * BYTES_PER_WORD as isize);
                    let mut src: *mut WirePointer = old_ptr as *mut _;
                    for _ in 0..element_count {
                        transfer_pointer(arena, new_segment_id, dst as *mut _, old_segment_id, src);
                        dst = dst.offset(new_step as isize * BYTES_PER_WORD as isize);
                        src = src.offset(1);
                    }
                } else {
                    let mut dst = new_ptr;
                    let mut src: *mut u8 = old_ptr;
                    let old_byte_step = old_data_size / BITS_PER_BYTE as u32;
                    for _ in 0..element_count {
                        ptr::copy_nonoverlapping(src, dst, old_byte_step as usize);
                        src = src.offset(old_byte_step as isize);
                        dst = dst.offset(new_step as isize * BYTES_PER_WORD as isize);
                    }
                }

                // Zero out old location.
                ptr::write_bytes(
                    old_ptr,
                    0,
                    round_bits_up_to_bytes(u64::from(old_step) * u64::from(element_count)) as usize,
                );

                Ok(ListBuilder {
                    arena,
                    segment_id: new_segment_id,
                    cap_table,
                    ptr: new_ptr,
                    element_count,
                    element_size: ElementSize::InlineComposite,
                    step: new_step * BITS_PER_WORD as u32,
                    struct_data_size: u32::from(new_data_size) * BITS_PER_WORD as u32,
                    struct_pointer_count: new_pointer_count,
                })
            }
        }
    }

    #[inline]
    pub unsafe fn init_text_pointer(
        arena: &mut dyn BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        size: ByteCount32,
    ) -> SegmentAnd<text::Builder<'_>> {
        //# The byte list must include a NUL terminator.
        let byte_size = size + 1;

        //# Allocate the space.
        let (ptr, reff, segment_id) = allocate(
            arena,
            reff,
            segment_id,
            round_bytes_up_to_words(byte_size),
            WirePointerKind::List,
        );

        //# Initialize the pointer.
        (*reff).set_list_size_and_count(Byte, byte_size);

        SegmentAnd {
            segment_id,
            value: text::Builder::new(slice::from_raw_parts_mut(ptr, size as usize)),
        }
    }

    #[inline]
    pub unsafe fn set_text_pointer<'a>(
        arena: &'a mut dyn BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        value: crate::text::Reader<'_>,
    ) -> SegmentAnd<text::Builder<'a>> {
        let value_bytes = value.as_bytes();
        // TODO make sure the string is not longer than 2 ** 29.
        let mut allocation = init_text_pointer(arena, reff, segment_id, value_bytes.len() as u32);
        allocation
            .value
            .reborrow()
            .as_bytes_mut()
            .copy_from_slice(value_bytes);
        allocation
    }

    #[inline]
    pub unsafe fn get_writable_text_pointer<'a>(
        arena: &'a mut dyn BuilderArena,
        mut reff: *mut WirePointer,
        mut segment_id: u32,
        default: Option<&'a [crate::Word]>,
    ) -> Result<text::Builder<'a>> {
        let ref_target = if (*reff).is_null() {
            match default {
                None => return Ok(text::Builder::new(&mut [])),
                Some(d) => {
                    let (new_ref_target, new_reff, new_segment_id) = copy_message(
                        arena,
                        segment_id,
                        Default::default(),
                        reff,
                        d.as_ptr() as *const _,
                    );
                    reff = new_reff;
                    segment_id = new_segment_id;
                    new_ref_target
                }
            }
        } else {
            WirePointer::mut_target(reff)
        };

        let (ptr, reff, _segment_id) = follow_builder_fars(arena, reff, ref_target, segment_id)?;

        if (*reff).kind() != WirePointerKind::List {
            return Err(Error::from_kind(ErrorKind::ExistingPointerIsNotAList));
        }
        if (*reff).list_element_size() != Byte {
            return Err(Error::from_kind(
                ErrorKind::ExistingListPointerIsNotByteSized,
            ));
        }

        let count = (*reff).list_element_count();
        if count == 0 || *ptr.offset((count - 1) as isize) != 0 {
            return Err(Error::from_kind(ErrorKind::TextBlobMissingNULTerminator));
        }

        // Subtract 1 from the size for the NUL terminator.
        Ok(text::Builder::with_pos(
            slice::from_raw_parts_mut(ptr, (count - 1) as usize),
            (count - 1) as usize,
        ))
    }

    #[inline]
    pub unsafe fn init_data_pointer(
        arena: &mut dyn BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        size: ByteCount32,
    ) -> SegmentAnd<data::Builder<'_>> {
        //# Allocate the space.
        let (ptr, reff, segment_id) = allocate(
            arena,
            reff,
            segment_id,
            round_bytes_up_to_words(size),
            WirePointerKind::List,
        );

        //# Initialize the pointer.
        (*reff).set_list_size_and_count(Byte, size);

        SegmentAnd {
            segment_id,
            value: data::builder_from_raw_parts(ptr, size),
        }
    }

    #[inline]
    pub unsafe fn set_data_pointer<'a>(
        arena: &'a mut dyn BuilderArena,
        reff: *mut WirePointer,
        segment_id: u32,
        value: &[u8],
    ) -> SegmentAnd<data::Builder<'a>> {
        let allocation = init_data_pointer(arena, reff, segment_id, value.len() as u32);
        ptr::copy_nonoverlapping(value.as_ptr(), allocation.value.as_mut_ptr(), value.len());
        allocation
    }

    #[inline]
    pub unsafe fn get_writable_data_pointer<'a>(
        arena: &'a mut dyn BuilderArena,
        mut reff: *mut WirePointer,
        mut segment_id: u32,
        default: Option<&'a [crate::Word]>,
    ) -> Result<data::Builder<'a>> {
        let ref_target = if (*reff).is_null() {
            match default {
                None => return Ok(&mut []),
                Some(d) => {
                    let (new_ref_target, new_reff, new_segment_id) = copy_message(
                        arena,
                        segment_id,
                        Default::default(),
                        reff,
                        d.as_ptr() as *const _,
                    );
                    reff = new_reff;
                    segment_id = new_segment_id;
                    new_ref_target
                }
            }
        } else {
            WirePointer::mut_target(reff)
        };

        let (ptr, reff, _segment_id) = follow_builder_fars(arena, reff, ref_target, segment_id)?;

        if (*reff).kind() != WirePointerKind::List {
            return Err(Error::from_kind(ErrorKind::ExistingPointerIsNotAList));
        }
        if (*reff).list_element_size() != Byte {
            return Err(Error::from_kind(
                ErrorKind::ExistingListPointerIsNotByteSized,
            ));
        }

        Ok(data::builder_from_raw_parts(
            ptr,
            (*reff).list_element_count(),
        ))
    }

    pub unsafe fn set_struct_pointer(
        arena: &mut dyn BuilderArena,
        segment_id: u32,
        cap_table: CapTableBuilder,
        reff: *mut WirePointer,
        value: StructReader,
        canonicalize: bool,
    ) -> Result<SegmentAnd<*mut u8>> {
        let mut data_size: ByteCount32 = round_bits_up_to_bytes(u64::from(value.data_size));
        let mut ptr_count = value.pointer_count;

        if canonicalize {
            // StructReaders should not have bitwidths other than 1, but let's be safe
            if !(value.data_size == 1 || value.data_size % BITS_PER_BYTE as u32 == 0) {
                return Err(Error::from_kind(
                    ErrorKind::StructReaderHadBitwidthOtherThan1,
                ));
            }

            if value.data_size == 1 {
                if !value.get_bool_field(0) {
                    data_size = 0;
                }
            } else {
                'chop: while data_size != 0 {
                    let end = data_size;
                    let mut window = data_size % BYTES_PER_WORD as u32;
                    if window == 0 {
                        window = BYTES_PER_WORD as u32;
                    }
                    let start = end - window;
                    let last_word = &value.get_data_section_as_blob()[start as usize..end as usize];
                    if last_word == [0; 8] {
                        data_size -= window;
                    } else {
                        break 'chop;
                    }
                }
            }

            while ptr_count != 0 && value.get_pointer_field(ptr_count as usize - 1).is_null() {
                ptr_count -= 1;
            }
        }

        let data_words = round_bytes_up_to_words(data_size);
        let total_size: WordCount32 = data_words + u32::from(ptr_count) * WORDS_PER_POINTER as u32;

        let (ptr, reff, segment_id) =
            allocate(arena, reff, segment_id, total_size, WirePointerKind::Struct);
        (*reff).set_struct_size_from_pieces(data_words as u16, ptr_count);

        if value.data_size == 1 {
            // Data size could be made 0 by truncation
            if data_size != 0 {
                *ptr = u8::from(value.get_bool_field(0))
            }
        } else {
            ptr::copy_nonoverlapping::<u8>(value.data, ptr, data_size as usize);
        }

        let pointer_section: *mut WirePointer =
            ptr.offset(data_words as isize * BYTES_PER_WORD as isize) as *mut _;
        for i in 0..ptr_count as isize {
            copy_pointer(
                arena,
                segment_id,
                cap_table,
                pointer_section.offset(i),
                value.arena,
                value.segment_id,
                value.cap_table,
                value.pointers.offset(i),
                value.nesting_limit,
                canonicalize,
            )?;
        }

        Ok(SegmentAnd {
            segment_id,
            value: ptr,
        })
    }

    #[cfg(feature = "alloc")]
    pub fn set_capability_pointer(
        _arena: &mut dyn BuilderArena,
        _segment_id: u32,
        mut cap_table: CapTableBuilder,
        reff: *mut WirePointer,
        cap: Box<dyn ClientHook>,
    ) {
        // TODO if ref is not null, zero object.
        unsafe {
            (*reff).set_cap(cap_table.inject_cap(cap) as u32);
        }
    }

    pub unsafe fn set_list_pointer(
        arena: &mut dyn BuilderArena,
        segment_id: u32,
        cap_table: CapTableBuilder,
        reff: *mut WirePointer,
        value: ListReader,
        canonicalize: bool,
    ) -> Result<SegmentAnd<*mut u8>> {
        let total_size =
            round_bits_up_to_words(u64::from(value.element_count) * u64::from(value.step));

        if value.element_size != ElementSize::InlineComposite {
            //# List of non-structs.
            let (ptr, reff, segment_id) =
                allocate(arena, reff, segment_id, total_size, WirePointerKind::List);

            if value.struct_pointer_count == 1 {
                //# List of pointers.
                (*reff).set_list_size_and_count(Pointer, value.element_count);
                for i in 0..value.element_count as isize {
                    copy_pointer(
                        arena,
                        segment_id,
                        cap_table,
                        (ptr as *mut WirePointer).offset(i),
                        value.arena,
                        value.segment_id,
                        value.cap_table,
                        (value.ptr as *const WirePointer).offset(i),
                        value.nesting_limit,
                        canonicalize,
                    )?;
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
                    _ => {
                        panic!("invalid list step size: {}", value.step)
                    }
                };

                (*reff).set_list_size_and_count(element_size, value.element_count);

                // Be careful to avoid coping any bytes past the end of the list.
                // TODO(perf) Is ptr::copy_nonoverlapping faster if word-aligned?
                // If so, then perhaps we should only drop to the byte-index level
                // in the canonicalize=true case.
                let whole_byte_size =
                    u64::from(value.element_count) * u64::from(value.step) / BITS_PER_BYTE as u64;
                ptr::copy_nonoverlapping(value.ptr, ptr, whole_byte_size as usize);
                let leftover_bits =
                    u64::from(value.element_count) * u64::from(value.step) % BITS_PER_BYTE as u64;
                if leftover_bits > 0 {
                    let mask: u8 = (1 << leftover_bits as u8) - 1;

                    *ptr.offset(whole_byte_size as isize) =
                        mask & (*value.ptr.offset(whole_byte_size as isize))
                }
            }

            Ok(SegmentAnd {
                segment_id,
                value: ptr,
            })
        } else {
            //# List of structs.

            let decl_data_size = value.struct_data_size / BITS_PER_WORD as u32;
            let decl_pointer_count = value.struct_pointer_count;

            let mut data_size = 0;
            let mut ptr_count = 0;
            let mut total_size = total_size;

            if canonicalize {
                for ec in 0..value.element_count {
                    let se = value.get_struct_element(ec);
                    let mut local_data_size = decl_data_size;
                    'data_chop: while local_data_size != 0 {
                        let end = local_data_size * BYTES_PER_WORD as u32;
                        let window = BYTES_PER_WORD as u32;
                        let start = end - window;
                        let last_word =
                            &se.get_data_section_as_blob()[start as usize..end as usize];
                        if last_word != [0; 8] {
                            break 'data_chop;
                        } else {
                            local_data_size -= 1;
                        }
                    }
                    if local_data_size > data_size {
                        data_size = local_data_size;
                    }
                    let mut local_ptr_count = decl_pointer_count;
                    while local_ptr_count != 0
                        && se.get_pointer_field(local_ptr_count as usize - 1).is_null()
                    {
                        local_ptr_count -= 1;
                    }
                    if local_ptr_count > ptr_count {
                        ptr_count = local_ptr_count;
                    }
                }
                total_size = (data_size + u32::from(ptr_count)) * value.element_count;
            } else {
                data_size = decl_data_size;
                ptr_count = decl_pointer_count;
            }

            let (ptr, reff, segment_id) = allocate(
                arena,
                reff,
                segment_id,
                total_size + POINTER_SIZE_IN_WORDS as u32,
                WirePointerKind::List,
            );
            (*reff).set_list_inline_composite(total_size);

            let tag: *mut WirePointer = ptr as *mut _;
            (*tag).set_kind_and_inline_composite_list_element_count(
                WirePointerKind::Struct,
                value.element_count,
            );
            (*tag).set_struct_size_from_pieces(data_size as u16, ptr_count);
            let mut dst = ptr.add(BYTES_PER_WORD);

            let mut src: *const u8 = value.ptr;
            for _ in 0..value.element_count {
                ptr::copy_nonoverlapping(src, dst, data_size as usize * BYTES_PER_WORD);
                dst = dst.offset(data_size as isize * BYTES_PER_WORD as isize);
                src = src.offset(decl_data_size as isize * BYTES_PER_WORD as isize);

                for _ in 0..ptr_count {
                    copy_pointer(
                        arena,
                        segment_id,
                        cap_table,
                        dst as *mut _,
                        value.arena,
                        value.segment_id,
                        value.cap_table,
                        src as *const WirePointer,
                        value.nesting_limit,
                        canonicalize,
                    )?;
                    dst = dst.add(BYTES_PER_WORD);
                    src = src.add(BYTES_PER_WORD);
                }

                src =
                    src.offset((decl_pointer_count - ptr_count) as isize * BYTES_PER_WORD as isize);
            }
            Ok(SegmentAnd {
                segment_id,
                value: ptr,
            })
        }
    }

    pub unsafe fn copy_pointer(
        dst_arena: &mut dyn BuilderArena,
        dst_segment_id: u32,
        dst_cap_table: CapTableBuilder,
        dst: *mut WirePointer,
        src_arena: &dyn ReaderArena,
        src_segment_id: u32,
        src_cap_table: CapTableReader,
        src: *const WirePointer,
        nesting_limit: i32,
        canonicalize: bool,
    ) -> Result<SegmentAnd<*mut u8>> {
        if (*src).is_null() {
            ptr::write_bytes(dst, 0, 1);
            return Ok(SegmentAnd {
                segment_id: dst_segment_id,
                value: ptr::null_mut(),
            });
        }

        let (mut ptr, src, src_segment_id) = follow_fars(src_arena, src, src_segment_id)?;

        match (*src).kind() {
            WirePointerKind::Struct => {
                if nesting_limit <= 0 {
                    return Err(Error::from_kind(
                        ErrorKind::MessageIsTooDeeplyNestedOrContainsCycles,
                    ));
                }

                bounds_check(
                    src_arena,
                    src_segment_id,
                    ptr,
                    (*src).struct_word_size() as usize,
                    WirePointerKind::Struct,
                )?;

                set_struct_pointer(
                    dst_arena,
                    dst_segment_id,
                    dst_cap_table,
                    dst,
                    StructReader {
                        arena: src_arena,
                        segment_id: src_segment_id,
                        cap_table: src_cap_table,
                        data: ptr,
                        pointers: ptr
                            .offset((*src).struct_data_size() as isize * BYTES_PER_WORD as isize)
                            as *const _,
                        data_size: u32::from((*src).struct_data_size()) * BITS_PER_WORD as u32,
                        pointer_count: (*src).struct_ptr_count(),
                        nesting_limit: nesting_limit - 1,
                    },
                    canonicalize,
                )
            }
            WirePointerKind::List => {
                let element_size = (*src).list_element_size();
                if nesting_limit <= 0 {
                    return Err(Error::from_kind(
                        ErrorKind::MessageIsTooDeeplyNestedOrContainsCycles,
                    ));
                }

                if element_size == InlineComposite {
                    let word_count = (*src).list_inline_composite_word_count();
                    let tag: *const WirePointer = ptr as *const _;
                    ptr = ptr.add(BYTES_PER_WORD);

                    bounds_check(
                        src_arena,
                        src_segment_id,
                        ptr.offset(-(BYTES_PER_WORD as isize)),
                        word_count as usize + 1,
                        WirePointerKind::List,
                    )?;

                    if (*tag).kind() != WirePointerKind::Struct {
                        return Err(Error::from_kind(
                            ErrorKind::InlineCompositeListsOfNonStructTypeAreNotSupported,
                        ));
                    }

                    let element_count = (*tag).inline_composite_list_element_count();
                    let words_per_element = (*tag).struct_word_size();

                    if u64::from(words_per_element) * u64::from(element_count)
                        > u64::from(word_count)
                    {
                        return Err(Error::from_kind(
                            ErrorKind::InlineCompositeListsElementsOverrunItsWordCount,
                        ));
                    }

                    if words_per_element == 0 {
                        // Watch out for lists of zero-sized structs, which can claim to be
                        // arbitrarily large without having sent actual data.
                        amplified_read(src_arena, u64::from(element_count))?;
                    }

                    set_list_pointer(
                        dst_arena,
                        dst_segment_id,
                        dst_cap_table,
                        dst,
                        ListReader {
                            arena: src_arena,
                            segment_id: src_segment_id,
                            cap_table: src_cap_table,
                            ptr: ptr as *const _,
                            element_count,
                            element_size,
                            step: words_per_element * BITS_PER_WORD as u32,
                            struct_data_size: u32::from((*tag).struct_data_size())
                                * BITS_PER_WORD as u32,
                            struct_pointer_count: (*tag).struct_ptr_count(),
                            nesting_limit: nesting_limit - 1,
                        },
                        canonicalize,
                    )
                } else {
                    let data_size = data_bits_per_element(element_size);
                    let pointer_count = pointers_per_element(element_size);
                    let step = data_size + pointer_count * BITS_PER_POINTER as u32;
                    let element_count = (*src).list_element_count();
                    let word_count =
                        round_bits_up_to_words(u64::from(element_count) * u64::from(step));

                    bounds_check(
                        src_arena,
                        src_segment_id,
                        ptr,
                        word_count as usize,
                        WirePointerKind::List,
                    )?;

                    if element_size == Void {
                        // Watch out for lists of void, which can claim to be arbitrarily large
                        // without having sent actual data.
                        amplified_read(src_arena, u64::from(element_count))?;
                    }

                    set_list_pointer(
                        dst_arena,
                        dst_segment_id,
                        dst_cap_table,
                        dst,
                        ListReader {
                            arena: src_arena,
                            segment_id: src_segment_id,
                            cap_table: src_cap_table,
                            ptr: ptr as *const _,
                            element_count,
                            element_size,
                            step,
                            struct_data_size: data_size,
                            struct_pointer_count: pointer_count as u16,
                            nesting_limit: nesting_limit - 1,
                        },
                        canonicalize,
                    )
                }
            }
            WirePointerKind::Far => Err(Error::from_kind(ErrorKind::MalformedDoubleFarPointer)),
            WirePointerKind::Other => {
                if !(*src).is_capability() {
                    return Err(Error::from_kind(ErrorKind::UnknownPointerType));
                }
                if canonicalize {
                    return Err(Error::from_kind(
                        ErrorKind::CannotCreateACanonicalMessageWithACapability,
                    ));
                }
                #[cfg(feature = "alloc")]
                match src_cap_table.extract_cap((*src).cap_index() as usize) {
                    Some(cap) => {
                        set_capability_pointer(dst_arena, dst_segment_id, dst_cap_table, dst, cap);
                        Ok(SegmentAnd {
                            segment_id: dst_segment_id,
                            value: ptr::null_mut(),
                        })
                    }
                    None => Err(Error::from_kind(
                        ErrorKind::MessageContainsInvalidCapabilityPointer,
                    )),
                }
                #[cfg(not(feature = "alloc"))]
                return Err(Error::from_kind(ErrorKind::UnknownPointerType));
            }
        }
    }

    #[inline]
    pub unsafe fn read_struct_pointer<'a>(
        mut arena: &'a dyn ReaderArena,
        mut segment_id: u32,
        cap_table: CapTableReader,
        mut reff: *const WirePointer,
        default: Option<&'a [crate::Word]>,
        nesting_limit: i32,
    ) -> Result<StructReader<'a>> {
        if (*reff).is_null() {
            match default {
                None => return Ok(StructReader::new_default()),
                Some(d) if (*(d.as_ptr() as *const WirePointer)).is_null() => {
                    return Ok(StructReader::new_default())
                }
                Some(d) => {
                    reff = d.as_ptr() as *const _;
                    arena = &super::NULL_ARENA;
                    segment_id = 0;
                }
            }
        }

        if nesting_limit <= 0 {
            return Err(Error::from_kind(
                ErrorKind::MessageIsTooDeeplyNestedOrContainsCycles,
            ));
        }

        let (ptr, reff, segment_id) = follow_fars(arena, reff, segment_id)?;

        let data_size_words = (*reff).struct_data_size();

        if (*reff).kind() != WirePointerKind::Struct {
            return Err(Error::from_kind(
                ErrorKind::MessageContainsNonStructPointerWhereStructPointerWasExpected,
            ));
        }

        bounds_check(
            arena,
            segment_id,
            ptr,
            (*reff).struct_word_size() as usize,
            WirePointerKind::Struct,
        )?;

        Ok(StructReader {
            arena,
            segment_id,
            cap_table,
            data: ptr,
            pointers: ptr.offset(data_size_words as isize * BYTES_PER_WORD as isize) as *const _,
            data_size: u32::from(data_size_words) * BITS_PER_WORD as BitCount32,
            pointer_count: (*reff).struct_ptr_count(),
            nesting_limit: nesting_limit - 1,
        })
    }

    #[inline]
    #[cfg(feature = "alloc")]
    pub unsafe fn read_capability_pointer(
        _arena: &dyn ReaderArena,
        _segment_id: u32,
        cap_table: CapTableReader,
        reff: *const WirePointer,
        _nesting_limit: i32,
    ) -> Result<Box<dyn ClientHook>> {
        if (*reff).is_null() {
            Err(Error::from_kind(
                ErrorKind::MessageContainsNullCapabilityPointer,
            ))
        } else if !(*reff).is_capability() {
            Err(Error::from_kind(
                ErrorKind::MessageContainsNonCapabilityPointerWhereCapabilityPointerWasExpected,
            ))
        } else {
            let n = (*reff).cap_index() as usize;
            match cap_table.extract_cap(n) {
                Some(client_hook) => Ok(client_hook),
                None => Err(Error::from_kind(
                    ErrorKind::MessageContainsInvalidCapabilityPointer,
                )),
            }
        }
    }

    #[inline]
    pub unsafe fn read_list_pointer(
        mut arena: &dyn ReaderArena,
        mut segment_id: u32,
        cap_table: CapTableReader,
        mut reff: *const WirePointer,
        default_value: *const u8,
        expected_element_size: Option<ElementSize>,
        nesting_limit: i32,
    ) -> Result<ListReader<'_>> {
        if (*reff).is_null() {
            if default_value.is_null() || (*(default_value as *const WirePointer)).is_null() {
                return Ok(ListReader::new_default());
            }
            reff = default_value as *const _;
            arena = &super::NULL_ARENA;
            segment_id = 0;
        }

        if nesting_limit <= 0 {
            return Err(Error::from_kind(ErrorKind::NestingLimitExceeded));
        }
        let (mut ptr, reff, segment_id) = follow_fars(arena, reff, segment_id)?;

        if (*reff).kind() != WirePointerKind::List {
            return Err(Error::from_kind(
                ErrorKind::MessageContainsNonListPointerWhereListPointerWasExpected,
            ));
        }

        let element_size = (*reff).list_element_size();
        match element_size {
            InlineComposite => {
                let word_count = (*reff).list_inline_composite_word_count();

                let tag: *const WirePointer = ptr as *const WirePointer;

                ptr = ptr.add(BYTES_PER_WORD);

                bounds_check(
                    arena,
                    segment_id,
                    ptr.offset(-(BYTES_PER_WORD as isize)),
                    word_count as usize + 1,
                    WirePointerKind::List,
                )?;

                if (*tag).kind() != WirePointerKind::Struct {
                    return Err(Error::from_kind(
                        ErrorKind::InlineCompositeListsOfNonStructTypeAreNotSupported,
                    ));
                }

                let size = (*tag).inline_composite_list_element_count();
                let data_size = (*tag).struct_data_size();
                let ptr_count = (*tag).struct_ptr_count();
                let words_per_element = (*tag).struct_word_size();

                if u64::from(size) * u64::from(words_per_element) > u64::from(word_count) {
                    return Err(Error::from_kind(
                        ErrorKind::InlineCompositeListsElementsOverrunItsWordCount,
                    ));
                }

                if words_per_element == 0 {
                    // Watch out for lists of zero-sized structs, which can claim to be
                    // arbitrarily large without having sent actual data.
                    amplified_read(arena, u64::from(size))?;
                }

                // If a struct list was not expected, then presumably a non-struct list was upgraded
                // to a struct list. We need to manipulate the pointer to point at the first field
                // of the struct. Together with the `step` field, this will allow the struct list to
                // be accessed as if it were a primitive list without branching.

                // Check whether the size is compatible.
                match expected_element_size {
                    None | Some(Void | InlineComposite) => (),
                    Some(Bit) => {
                        return Err(Error::from_kind(
                            ErrorKind::FoundStructListWhereBitListWasExpected,
                        ));
                    }
                    Some(Byte | TwoBytes | FourBytes | EightBytes) => {
                        if data_size == 0 {
                            return Err(Error::from_kind(
                                ErrorKind::ExpectedAPrimitiveListButGotAListOfPointerOnlyStructs,
                            ));
                        }
                    }
                    Some(Pointer) => {
                        if ptr_count == 0 {
                            return Err(Error::from_kind(
                                ErrorKind::ExpectedAPointerListButGotAListOfDataOnlyStructs,
                            ));
                        }
                    }
                }

                Ok(ListReader {
                    arena,
                    segment_id,
                    cap_table,
                    ptr: ptr as *const _,
                    element_count: size,
                    element_size,
                    step: words_per_element * BITS_PER_WORD as u32,
                    struct_data_size: u32::from(data_size) * (BITS_PER_WORD as u32),
                    struct_pointer_count: ptr_count,
                    nesting_limit: nesting_limit - 1,
                })
            }
            _ => {
                // This is a primitive or pointer list, but all such lists can also be interpreted
                // as struct lists. We need to compute the data size and pointer count for such
                // structs.
                let data_size = data_bits_per_element((*reff).list_element_size());
                let pointer_count = pointers_per_element((*reff).list_element_size());
                let element_count = (*reff).list_element_count();
                let step = data_size + pointer_count * BITS_PER_POINTER as u32;

                let word_count = round_bits_up_to_words(u64::from(element_count) * u64::from(step));
                bounds_check(
                    arena,
                    segment_id,
                    ptr,
                    word_count as usize,
                    WirePointerKind::List,
                )?;

                if element_size == Void {
                    // Watch out for lists of void, which can claim to be arbitrarily large
                    // without having sent actual data.
                    amplified_read(arena, u64::from(element_count))?;
                }

                if let Some(expected_element_size) = expected_element_size {
                    if element_size == ElementSize::Bit && expected_element_size != ElementSize::Bit
                    {
                        return Err(Error::from_kind(
                            ErrorKind::FoundBitListWhereStructListWasExpected,
                        ));
                    }

                    // Verify that the elements are at least as large as the expected type. Note that if
                    // we expected InlineComposite, the expected sizes here will be zero, because bounds
                    // checking will be performed at field access time. So this check here is for the
                    // case where we expected a list of some primitive or pointer type.

                    let expected_data_bits_per_element =
                        data_bits_per_element(expected_element_size);
                    let expected_pointers_per_element = pointers_per_element(expected_element_size);

                    if expected_data_bits_per_element > data_size
                        || expected_pointers_per_element > pointer_count
                    {
                        return Err(Error::from_kind(
                            ErrorKind::MessageContainsListWithIncompatibleElementType,
                        ));
                    }
                }

                Ok(ListReader {
                    arena,
                    segment_id,
                    cap_table,
                    ptr: ptr as *const _,
                    element_count,
                    element_size,
                    step,
                    struct_data_size: data_size,
                    struct_pointer_count: pointer_count as u16,
                    nesting_limit: nesting_limit - 1,
                })
            }
        }
    }

    #[inline]
    pub unsafe fn read_text_pointer<'a>(
        mut arena: &'a dyn ReaderArena,
        mut segment_id: u32,
        mut reff: *const WirePointer,
        default: Option<&[crate::Word]>,
    ) -> Result<text::Reader<'a>> {
        if (*reff).is_null() {
            match default {
                None => return Ok("".into()),
                Some(d) => {
                    reff = d.as_ptr() as *const WirePointer;
                    arena = &super::NULL_ARENA;
                    segment_id = 0;
                }
            }
        }

        let (ptr, reff, segment_id) = follow_fars(arena, reff, segment_id)?;
        let size = (*reff).list_element_count();

        if (*reff).kind() != WirePointerKind::List {
            return Err(Error::from_kind(
                ErrorKind::MessageContainsNonListPointerWhereTextWasExpected,
            ));
        }

        if (*reff).list_element_size() != Byte {
            return Err(Error::from_kind(
                ErrorKind::MessageContainsListPointerOfNonBytesWhereTextWasExpected,
            ));
        }

        bounds_check(
            arena,
            segment_id,
            ptr,
            round_bytes_up_to_words(size) as usize,
            WirePointerKind::List,
        )?;

        if size == 0 {
            return Err(Error::from_kind(
                ErrorKind::MessageContainsTextThatIsNotNULTerminated,
            ));
        }

        let str_ptr = ptr;

        if (*str_ptr.offset((size - 1) as isize)) != 0u8 {
            return Err(Error::from_kind(
                ErrorKind::MessageContainsTextThatIsNotNULTerminated,
            ));
        }

        Ok(text::Reader(slice::from_raw_parts(
            str_ptr,
            size as usize - 1,
        )))
    }

    #[inline]
    pub unsafe fn read_data_pointer<'a>(
        mut arena: &'a dyn ReaderArena,
        mut segment_id: u32,
        mut reff: *const WirePointer,
        default: Option<&'a [crate::Word]>,
    ) -> Result<data::Reader<'a>> {
        if (*reff).is_null() {
            match default {
                None => return Ok(&[]),
                Some(d) => {
                    reff = d.as_ptr() as *const WirePointer;
                    arena = &super::NULL_ARENA;
                    segment_id = 0;
                }
            }
        }

        let (ptr, reff, segment_id) = follow_fars(arena, reff, segment_id)?;

        let size: u32 = (*reff).list_element_count();

        if (*reff).kind() != WirePointerKind::List {
            return Err(Error::from_kind(
                ErrorKind::MessageContainsNonListPointerWhereDataWasExpected,
            ));
        }

        if (*reff).list_element_size() != Byte {
            return Err(Error::from_kind(
                ErrorKind::MessageContainsListPointerOfNonBytesWhereDataWasExpected,
            ));
        }

        bounds_check(
            arena,
            segment_id,
            ptr,
            round_bytes_up_to_words(size) as usize,
            WirePointerKind::List,
        )?;

        Ok(data::reader_from_raw_parts(ptr as *const _, size))
    }
}

static ZERO: u64 = 0;
fn zero_pointer() -> *const WirePointer {
    &ZERO as *const _ as *const _
}

static NULL_ARENA: NullArena = NullArena;

#[cfg(feature = "alloc")]
pub type CapTable = Vec<Option<Box<dyn ClientHook>>>;

#[cfg(not(feature = "alloc"))]
pub struct CapTable;

#[derive(Copy, Clone)]
pub enum CapTableReader {
    // At one point, we had a `Dummy` variant here, but that ended up
    // making values of this type take 16 bytes of memory. Now we instead
    // represent a null CapTableReader with `Plain(ptr::null())`.
    Plain(*const CapTable),
}

impl Default for CapTableReader {
    fn default() -> Self {
        CapTableReader::Plain(ptr::null())
    }
}

#[cfg(feature = "alloc")]
impl CapTableReader {
    pub fn extract_cap(&self, index: usize) -> Option<Box<dyn ClientHook>> {
        match *self {
            Self::Plain(hooks) => {
                if hooks.is_null() {
                    return None;
                }
                let hooks: &Vec<Option<Box<dyn ClientHook>>> = unsafe { &*hooks };
                if index >= hooks.len() {
                    None
                } else {
                    hooks[index].as_ref().map(|hook| hook.add_ref())
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
    Plain(*mut CapTable),
}

impl Default for CapTableBuilder {
    fn default() -> Self {
        CapTableBuilder::Plain(ptr::null_mut())
    }
}

impl CapTableBuilder {
    pub fn into_reader(self) -> CapTableReader {
        match self {
            Self::Plain(hooks) => CapTableReader::Plain(hooks),
        }
    }

    #[cfg(feature = "alloc")]
    pub fn extract_cap(&self, index: usize) -> Option<Box<dyn ClientHook>> {
        match *self {
            Self::Plain(hooks) => {
                if hooks.is_null() {
                    return None;
                }
                let hooks: &Vec<Option<Box<dyn ClientHook>>> = unsafe { &*hooks };
                if index >= hooks.len() {
                    None
                } else {
                    hooks[index].as_ref().map(|hook| hook.add_ref())
                }
            }
        }
    }

    #[cfg(feature = "alloc")]
    pub fn inject_cap(&mut self, cap: Box<dyn ClientHook>) -> usize {
        match *self {
            Self::Plain(hooks) => {
                if hooks.is_null() {
                    panic!(
                        "Called inject_cap() on a null capability table. You need \
                            to call imbue_mut() on this message before adding capabilities."
                    );
                }
                let hooks: &mut Vec<Option<Box<dyn ClientHook>>> = unsafe { &mut *hooks };
                hooks.push(Some(cap));
                hooks.len() - 1
            }
        }
    }

    #[cfg(feature = "alloc")]
    pub fn drop_cap(&mut self, index: usize) {
        match *self {
            Self::Plain(hooks) => {
                if hooks.is_null() {
                    panic!(
                        "Called drop_cap() on a null capability table. You need \
                            to call imbue_mut() on this message before adding capabilities."
                    );
                }
                let hooks: &mut Vec<Option<Box<dyn ClientHook>>> = unsafe { &mut *hooks };
                if index < hooks.len() {
                    hooks[index] = None;
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct PointerReader<'a> {
    arena: &'a dyn ReaderArena,
    cap_table: CapTableReader,
    pointer: *const WirePointer,
    segment_id: u32,
    nesting_limit: i32,
}

impl<'a> PointerReader<'a> {
    pub fn new_default<'b>() -> PointerReader<'b> {
        PointerReader {
            arena: &NULL_ARENA,
            segment_id: 0,
            cap_table: Default::default(),
            pointer: ptr::null(),
            nesting_limit: 0x7fffffff,
        }
    }

    pub fn get_root(
        arena: &'a dyn ReaderArena,
        segment_id: u32,
        location: *const u8,
        nesting_limit: i32,
    ) -> Result<Self> {
        wire_helpers::bounds_check(
            arena,
            segment_id,
            location as *const _,
            POINTER_SIZE_IN_WORDS,
            WirePointerKind::Struct,
        )?;

        Ok(PointerReader {
            arena,
            segment_id,
            cap_table: Default::default(),
            pointer: location as *const _,
            nesting_limit,
        })
    }

    pub fn reborrow(&self) -> PointerReader<'_> {
        PointerReader {
            arena: self.arena,
            ..*self
        }
    }

    pub unsafe fn get_root_unchecked<'b>(location: *const u8) -> PointerReader<'b> {
        PointerReader {
            arena: &NULL_ARENA,
            segment_id: 0,
            cap_table: Default::default(),
            pointer: location as *const _,
            nesting_limit: 0x7fffffff,
        }
    }

    pub fn imbue(&mut self, cap_table: CapTableReader) {
        self.cap_table = cap_table;
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.pointer.is_null() || unsafe { (*self.pointer).is_null() }
    }

    pub fn total_size(&self) -> Result<MessageSize> {
        if self.pointer.is_null() {
            Ok(MessageSize {
                word_count: 0,
                cap_count: 0,
            })
        } else {
            unsafe {
                wire_helpers::total_size(
                    self.arena,
                    self.segment_id,
                    self.pointer,
                    self.nesting_limit,
                )
            }
        }
    }

    pub fn get_struct(self, default: Option<&'a [crate::Word]>) -> Result<StructReader<'a>> {
        let reff: *const WirePointer = if self.pointer.is_null() {
            zero_pointer()
        } else {
            self.pointer
        };
        unsafe {
            wire_helpers::read_struct_pointer(
                self.arena,
                self.segment_id,
                self.cap_table,
                reff,
                default,
                self.nesting_limit,
            )
        }
    }

    pub fn get_list(
        self,
        expected_element_size: ElementSize,
        default: Option<&'a [crate::Word]>,
    ) -> Result<ListReader<'a>> {
        let default_value: *const u8 = match default {
            None => core::ptr::null(),
            Some(d) => d.as_ptr() as *const u8,
        };
        let reff = if self.pointer.is_null() {
            zero_pointer()
        } else {
            self.pointer
        };
        unsafe {
            wire_helpers::read_list_pointer(
                self.arena,
                self.segment_id,
                self.cap_table,
                reff,
                default_value,
                Some(expected_element_size),
                self.nesting_limit,
            )
        }
    }

    fn get_list_any_size(self, default_value: *const u8) -> Result<ListReader<'a>> {
        let reff = if self.pointer.is_null() {
            zero_pointer()
        } else {
            self.pointer
        };
        unsafe {
            wire_helpers::read_list_pointer(
                self.arena,
                self.segment_id,
                self.cap_table,
                reff,
                default_value,
                None,
                self.nesting_limit,
            )
        }
    }

    pub fn get_text(self, default: Option<&[crate::Word]>) -> Result<text::Reader<'a>> {
        let reff = if self.pointer.is_null() {
            zero_pointer()
        } else {
            self.pointer
        };
        unsafe { wire_helpers::read_text_pointer(self.arena, self.segment_id, reff, default) }
    }

    pub fn get_data(&self, default: Option<&'a [crate::Word]>) -> Result<data::Reader<'a>> {
        let reff = if self.pointer.is_null() {
            zero_pointer()
        } else {
            self.pointer
        };
        unsafe { wire_helpers::read_data_pointer(self.arena, self.segment_id, reff, default) }
    }

    #[cfg(feature = "alloc")]
    pub fn get_capability(&self) -> Result<Box<dyn ClientHook>> {
        let reff: *const WirePointer = if self.pointer.is_null() {
            zero_pointer()
        } else {
            self.pointer
        };
        unsafe {
            wire_helpers::read_capability_pointer(
                self.arena,
                self.segment_id,
                self.cap_table,
                reff,
                self.nesting_limit,
            )
        }
    }

    pub fn get_pointer_type(&self) -> Result<PointerType> {
        if self.is_null() {
            Ok(PointerType::Null)
        } else {
            let (_, reff, _) =
                unsafe { wire_helpers::follow_fars(self.arena, self.pointer, self.segment_id)? };

            match unsafe { (*reff).kind() } {
                WirePointerKind::Far => Err(Error::from_kind(ErrorKind::UnexepectedFarPointer)),
                WirePointerKind::Struct => Ok(PointerType::Struct),
                WirePointerKind::List => Ok(PointerType::List),
                WirePointerKind::Other => {
                    if unsafe { (*reff).is_capability() } {
                        Ok(PointerType::Capability)
                    } else {
                        Err(Error::from_kind(ErrorKind::UnknownPointerType))
                    }
                }
            }
        }
    }

    pub fn is_canonical(&self, read_head: &Cell<*const u8>) -> Result<bool> {
        if self.pointer.is_null() || unsafe { !(*self.pointer).is_positional() } {
            return Ok(false);
        }

        match self.get_pointer_type()? {
            PointerType::Null => Ok(true),
            PointerType::Struct => {
                let mut data_trunc = false;
                let mut ptr_trunc = false;
                let st = self.get_struct(None)?;
                if st.get_data_section_size() == 0 && st.get_pointer_section_size() == 0 {
                    Ok(self.pointer as *const _ == st.get_location())
                } else {
                    let result =
                        st.is_canonical(read_head, read_head, &mut data_trunc, &mut ptr_trunc)?;
                    Ok(result && data_trunc && ptr_trunc)
                }
            }
            PointerType::List => unsafe {
                self.get_list_any_size(ptr::null())?
                    .is_canonical(read_head, self.pointer)
            },
            PointerType::Capability => Ok(false),
        }
    }
}

pub struct PointerBuilder<'a> {
    arena: &'a mut dyn BuilderArena,
    segment_id: u32,
    cap_table: CapTableBuilder,
    pointer: *mut WirePointer,
}

impl<'a> PointerBuilder<'a> {
    #[inline]
    pub fn get_root(arena: &'a mut dyn BuilderArena, segment_id: u32, location: *mut u8) -> Self {
        PointerBuilder {
            arena,
            cap_table: Default::default(),
            segment_id,
            pointer: location as *mut _,
        }
    }

    #[inline]
    pub fn reborrow(&mut self) -> PointerBuilder<'_> {
        PointerBuilder {
            arena: self.arena,
            ..*self
        }
    }

    pub fn imbue(&mut self, cap_table: CapTableBuilder) {
        self.cap_table = cap_table;
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        unsafe { (*self.pointer).is_null() }
    }

    pub fn get_struct(
        self,
        size: StructSize,
        default: Option<&'a [crate::Word]>,
    ) -> Result<StructBuilder<'a>> {
        unsafe {
            wire_helpers::get_writable_struct_pointer(
                self.arena,
                self.pointer,
                self.segment_id,
                self.cap_table,
                size,
                default,
            )
        }
    }

    pub fn get_list(
        self,
        element_size: ElementSize,
        default: Option<&'a [crate::Word]>,
    ) -> Result<ListBuilder<'a>> {
        let default_value: *const u8 = match default {
            None => core::ptr::null(),
            Some(d) => d.as_ptr() as *const u8,
        };
        unsafe {
            wire_helpers::get_writable_list_pointer(
                self.arena,
                self.pointer,
                self.segment_id,
                self.cap_table,
                element_size,
                default_value,
            )
        }
    }

    pub fn get_struct_list(
        self,
        element_size: StructSize,
        default: Option<&'a [crate::Word]>,
    ) -> Result<ListBuilder<'a>> {
        let default_value: *const u8 = match default {
            None => core::ptr::null(),
            Some(d) => d.as_ptr() as *const u8,
        };
        unsafe {
            wire_helpers::get_writable_struct_list_pointer(
                self.arena,
                self.pointer,
                self.segment_id,
                self.cap_table,
                element_size,
                default_value,
            )
        }
    }

    pub fn get_text(self, default: Option<&'a [crate::Word]>) -> Result<text::Builder<'a>> {
        unsafe {
            wire_helpers::get_writable_text_pointer(
                self.arena,
                self.pointer,
                self.segment_id,
                default,
            )
        }
    }

    pub fn get_data(self, default: Option<&'a [crate::Word]>) -> Result<data::Builder<'a>> {
        unsafe {
            wire_helpers::get_writable_data_pointer(
                self.arena,
                self.pointer,
                self.segment_id,
                default,
            )
        }
    }

    #[cfg(feature = "alloc")]
    pub fn get_capability(&self) -> Result<Box<dyn ClientHook>> {
        unsafe {
            wire_helpers::read_capability_pointer(
                self.arena.as_reader(),
                self.segment_id,
                self.cap_table.into_reader(),
                self.pointer,
                ::core::i32::MAX,
            )
        }
    }

    pub fn init_struct(self, size: StructSize) -> StructBuilder<'a> {
        unsafe {
            wire_helpers::init_struct_pointer(
                self.arena,
                self.pointer,
                self.segment_id,
                self.cap_table,
                size,
            )
        }
    }

    pub fn init_list(
        self,
        element_size: ElementSize,
        element_count: ElementCount32,
    ) -> ListBuilder<'a> {
        unsafe {
            wire_helpers::init_list_pointer(
                self.arena,
                self.pointer,
                self.segment_id,
                self.cap_table,
                element_count,
                element_size,
            )
        }
    }

    pub fn init_struct_list(
        self,
        element_count: ElementCount32,
        element_size: StructSize,
    ) -> ListBuilder<'a> {
        unsafe {
            wire_helpers::init_struct_list_pointer(
                self.arena,
                self.pointer,
                self.segment_id,
                self.cap_table,
                element_count,
                element_size,
            )
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

    pub fn set_struct(&mut self, value: &StructReader, canonicalize: bool) -> Result<()> {
        unsafe {
            wire_helpers::set_struct_pointer(
                self.arena,
                self.segment_id,
                self.cap_table,
                self.pointer,
                *value,
                canonicalize,
            )?;
            Ok(())
        }
    }

    pub fn set_list(&mut self, value: &ListReader, canonicalize: bool) -> Result<()> {
        unsafe {
            wire_helpers::set_list_pointer(
                self.arena,
                self.segment_id,
                self.cap_table,
                self.pointer,
                *value,
                canonicalize,
            )?;
            Ok(())
        }
    }

    pub fn set_text(&mut self, value: crate::text::Reader<'_>) {
        unsafe {
            wire_helpers::set_text_pointer(self.arena, self.pointer, self.segment_id, value);
        }
    }

    pub fn set_data(&mut self, value: &[u8]) {
        unsafe {
            wire_helpers::set_data_pointer(self.arena, self.pointer, self.segment_id, value);
        }
    }

    #[cfg(feature = "alloc")]
    pub fn set_capability(&mut self, cap: Box<dyn ClientHook>) {
        wire_helpers::set_capability_pointer(
            self.arena,
            self.segment_id,
            self.cap_table,
            self.pointer,
            cap,
        );
    }

    pub fn copy_from(&mut self, other: PointerReader, canonicalize: bool) -> Result<()> {
        if other.pointer.is_null() {
            if !self.pointer.is_null() {
                unsafe {
                    wire_helpers::zero_object(self.arena, self.segment_id, self.pointer);
                    *self.pointer = mem::zeroed();
                }
            }
        } else {
            unsafe {
                wire_helpers::copy_pointer(
                    self.arena,
                    self.segment_id,
                    self.cap_table,
                    self.pointer,
                    other.arena,
                    other.segment_id,
                    other.cap_table,
                    other.pointer,
                    other.nesting_limit,
                    canonicalize,
                )?;
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

    pub fn as_reader(&self) -> PointerReader<'_> {
        PointerReader {
            arena: self.arena.as_reader(),
            segment_id: self.segment_id,
            cap_table: self.cap_table.into_reader(),
            pointer: self.pointer,
            nesting_limit: 0x7fffffff,
        }
    }

    pub fn into_reader(self) -> PointerReader<'a> {
        PointerReader {
            arena: self.arena.as_reader(),
            segment_id: self.segment_id,
            cap_table: self.cap_table.into_reader(),
            pointer: self.pointer,
            nesting_limit: 0x7fffffff,
        }
    }
}

#[derive(Clone, Copy)]
pub struct StructReader<'a> {
    arena: &'a dyn ReaderArena,
    cap_table: CapTableReader,
    data: *const u8,
    pointers: *const WirePointer,
    segment_id: u32,
    data_size: BitCount32,
    pointer_count: WirePointerCount16,
    nesting_limit: i32,
}

impl<'a> StructReader<'a> {
    pub fn new_default<'b>() -> StructReader<'b> {
        StructReader {
            arena: &NULL_ARENA,
            segment_id: 0,
            cap_table: Default::default(),
            data: ptr::null(),
            pointers: ptr::null(),
            data_size: 0,
            pointer_count: 0,
            nesting_limit: 0x7fffffff,
        }
    }

    pub fn imbue(&mut self, cap_table: CapTableReader) {
        self.cap_table = cap_table
    }

    pub fn get_data_section_size(&self) -> BitCount32 {
        self.data_size
    }

    pub fn get_pointer_section_size(&self) -> WirePointerCount16 {
        self.pointer_count
    }

    pub fn get_pointer_section_as_list(&self) -> ListReader<'a> {
        ListReader {
            arena: self.arena,
            segment_id: self.segment_id,
            cap_table: self.cap_table,
            ptr: self.pointers as *const _,
            element_count: u32::from(self.pointer_count),
            element_size: ElementSize::Pointer,
            step: BITS_PER_WORD as BitCount32,
            struct_data_size: 0,
            struct_pointer_count: 0,
            nesting_limit: self.nesting_limit,
        }
    }

    pub fn get_data_section_as_blob(&self) -> &'a [u8] {
        if self.data_size == 0 {
            // Explictly handle this case to avoid forming a slice to a null pointer,
            // which would be undefined behavior.
            &[]
        } else {
            unsafe {
                ::core::slice::from_raw_parts(self.data, self.data_size as usize / BITS_PER_BYTE)
            }
        }
    }

    #[inline]
    pub fn get_data_field<T: Primitive + zero::Zero>(&self, offset: ElementCount) -> T {
        // We need to check the offset because the struct may have
        // been created with an old version of the protocol that did
        // not contain the field.
        if (offset + 1) * bits_per_element::<T>() <= self.data_size as usize {
            let dwv: *const <T as Primitive>::Raw = self.data as *const _;
            unsafe { <T as Primitive>::get(&*dwv.add(offset)) }
        } else {
            T::zero()
        }
    }

    #[inline]
    pub fn get_bool_field(&self, offset: ElementCount) -> bool {
        let boffset: BitCount32 = offset as BitCount32;
        if boffset < self.data_size {
            unsafe {
                let b: *const u8 = self.data.add(boffset as usize / BITS_PER_BYTE);
                ((*b) & (1u8 << (boffset % BITS_PER_BYTE as u32) as usize)) != 0
            }
        } else {
            false
        }
    }

    #[inline]
    pub fn get_data_field_mask<T: Primitive + zero::Zero + Mask>(
        &self,
        offset: ElementCount,
        mask: <T as Mask>::T,
    ) -> T {
        Mask::mask(self.get_data_field(offset), mask)
    }

    #[inline]
    pub fn get_bool_field_mask(&self, offset: ElementCount, mask: bool) -> bool {
        self.get_bool_field(offset) ^ mask
    }

    #[inline]
    pub fn get_pointer_field(&self, ptr_index: WirePointerCount) -> PointerReader<'a> {
        if ptr_index < self.pointer_count as WirePointerCount {
            PointerReader {
                arena: self.arena,
                segment_id: self.segment_id,
                cap_table: self.cap_table,
                pointer: unsafe { self.pointers.add(ptr_index) },
                nesting_limit: self.nesting_limit,
            }
        } else {
            PointerReader::new_default()
        }
    }

    #[inline]
    pub fn is_pointer_field_null(&self, ptr_index: WirePointerCount) -> bool {
        if ptr_index < self.pointer_count as WirePointerCount {
            unsafe { (*self.pointers.add(ptr_index)).is_null() }
        } else {
            true
        }
    }

    pub fn total_size(&self) -> Result<MessageSize> {
        let mut result = MessageSize {
            word_count: u64::from(wire_helpers::round_bits_up_to_words(u64::from(
                self.data_size,
            ))) + u64::from(self.pointer_count) * WORDS_PER_POINTER as u64,
            cap_count: 0,
        };

        for i in 0..self.pointer_count as isize {
            unsafe {
                result += wire_helpers::total_size(
                    self.arena,
                    self.segment_id,
                    self.pointers.offset(i),
                    self.nesting_limit,
                )?;
            }
        }

        // TODO when we have read limiting: segment->unread()

        Ok(result)
    }

    fn get_location(&self) -> *const u8 {
        self.data
    }

    pub fn is_canonical(
        &self,
        read_head: &Cell<*const u8>,
        ptr_head: &Cell<*const u8>,
        data_trunc: &mut bool,
        ptr_trunc: &mut bool,
    ) -> Result<bool> {
        if self.get_location() != read_head.get() {
            return Ok(false);
        }

        if self.get_data_section_size() % BITS_PER_WORD as u32 != 0 {
            // legacy non-word-size struct
            return Ok(false);
        }

        let data_size = self.get_data_section_size() / BITS_PER_WORD as u32;

        // mark whether the struct is properly truncated
        if data_size != 0 {
            *data_trunc = self.get_data_field::<u64>((data_size - 1) as usize) != 0;
        } else {
            *data_trunc = true;
        }

        if self.pointer_count != 0 {
            *ptr_trunc = !self
                .get_pointer_field(self.pointer_count as usize - 1)
                .is_null();
        } else {
            *ptr_trunc = true;
        }

        read_head.set(unsafe {
            (read_head.get()).offset(
                (data_size as isize + self.pointer_count as isize) * (BYTES_PER_WORD as isize),
            )
        });

        for ptr_idx in 0..self.pointer_count {
            if !self
                .get_pointer_field(ptr_idx as usize)
                .is_canonical(ptr_head)?
            {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

pub struct StructBuilder<'a> {
    arena: &'a mut dyn BuilderArena,
    cap_table: CapTableBuilder,
    data: *mut u8,
    pointers: *mut WirePointer,
    segment_id: u32,
    data_size: BitCount32,
    pointer_count: WirePointerCount16,
}

impl<'a> StructBuilder<'a> {
    #[inline]
    pub fn reborrow(&mut self) -> StructBuilder<'_> {
        StructBuilder {
            arena: self.arena,
            ..*self
        }
    }

    pub fn as_reader(&self) -> StructReader<'_> {
        StructReader {
            arena: self.arena.as_reader(),
            cap_table: self.cap_table.into_reader(),
            data: self.data,
            pointers: self.pointers,
            pointer_count: self.pointer_count,
            segment_id: self.segment_id,
            data_size: self.data_size,
            nesting_limit: 0x7fffffff,
        }
    }

    pub fn into_reader(self) -> StructReader<'a> {
        StructReader {
            arena: self.arena.as_reader(),
            cap_table: self.cap_table.into_reader(),
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
    pub fn set_data_field<T: Primitive>(&self, offset: ElementCount, value: T) {
        let ptr: *mut <T as Primitive>::Raw = self.data as *mut _;
        unsafe { <T as Primitive>::set(&mut *ptr.add(offset), value) }
    }

    #[inline]
    pub fn set_data_field_mask<T: Primitive + Mask>(
        &self,
        offset: ElementCount,
        value: T,
        mask: <T as Mask>::T,
    ) {
        self.set_data_field(offset, Mask::mask(value, mask));
    }

    #[inline]
    pub fn get_data_field<T: Primitive>(&self, offset: ElementCount) -> T {
        let ptr: *const <T as Primitive>::Raw = self.data as *const _;
        unsafe { <T as Primitive>::get(&*ptr.add(offset)) }
    }

    #[inline]
    pub fn get_data_field_mask<T: Primitive + Mask>(
        &self,
        offset: ElementCount,
        mask: <T as Mask>::T,
    ) -> T {
        Mask::mask(self.get_data_field(offset), mask)
    }

    #[inline]
    pub fn set_bool_field(&self, offset: ElementCount, value: bool) {
        //# This branch should be compiled out whenever this is
        //# inlined with a constant offset.
        let boffset: BitCount0 = offset;
        let b = unsafe { self.data.add(boffset / BITS_PER_BYTE) };
        let bitnum = boffset % BITS_PER_BYTE;
        unsafe { (*b) = ((*b) & !(1 << bitnum)) | (u8::from(value) << bitnum) }
    }

    #[inline]
    pub fn set_bool_field_mask(&self, offset: ElementCount, value: bool, mask: bool) {
        self.set_bool_field(offset, value ^ mask);
    }

    #[inline]
    pub fn get_bool_field(&self, offset: ElementCount) -> bool {
        let boffset: BitCount0 = offset;
        let b = unsafe { self.data.add(boffset / BITS_PER_BYTE) };
        unsafe { ((*b) & (1 << (boffset % BITS_PER_BYTE))) != 0 }
    }

    #[inline]
    pub fn get_bool_field_mask(&self, offset: ElementCount, mask: bool) -> bool {
        self.get_bool_field(offset) ^ mask
    }

    #[inline]
    pub fn get_pointer_field(self, ptr_index: WirePointerCount) -> PointerBuilder<'a> {
        PointerBuilder {
            arena: self.arena,
            segment_id: self.segment_id,
            cap_table: self.cap_table,
            pointer: unsafe { self.pointers.add(ptr_index) },
        }
    }

    #[inline]
    pub fn get_pointer_field_mut(&mut self, ptr_index: WirePointerCount) -> PointerBuilder<'_> {
        PointerBuilder {
            arena: self.arena,
            segment_id: self.segment_id,
            cap_table: self.cap_table,
            pointer: unsafe { self.pointers.add(ptr_index) },
        }
    }

    #[inline]
    pub fn is_pointer_field_null(&self, ptr_index: WirePointerCount) -> bool {
        unsafe { (*self.pointers.add(ptr_index)).is_null() }
    }

    pub fn copy_content_from(&mut self, other: &StructReader) -> Result<()> {
        use core::cmp::min;
        // Determine the amount of data the builders have in common.
        let shared_data_size = min(self.data_size, other.data_size);
        let shared_pointer_count = min(self.pointer_count, other.pointer_count);

        if (shared_data_size > 0 && other.data == self.data)
            || (shared_pointer_count > 0 && other.pointers == self.pointers)
        {
            // At least one of the section pointers is pointing to ourself. Verify that the other is too
            // (but ignore empty sections).
            if (shared_data_size == 0 || other.data == self.data)
                && (shared_pointer_count == 0 || other.pointers == self.pointers)
            {
                return Err(Error::from_kind(
                    ErrorKind::OnlyOneOfTheSectionPointersIsPointingToOurself,
                ));
            }

            // So `other` appears to be a reader for this same struct. No copying is needed.
            return Ok(());
        }

        unsafe {
            if self.data_size > shared_data_size {
                // Since the target is larger than the source, make sure to zero out the extra bits that the
                // source doesn't have.
                if self.data_size == 1 {
                    self.set_bool_field(0, false);
                } else {
                    let unshared = self
                        .data
                        .offset((shared_data_size / BITS_PER_BYTE as u32) as isize);
                    ptr::write_bytes(
                        unshared,
                        0,
                        ((self.data_size - shared_data_size) / BITS_PER_BYTE as u32) as usize,
                    );
                }
            }

            // Copy over the shared part.
            if shared_data_size == 1 {
                self.set_bool_field(0, other.get_bool_field(0));
            } else {
                ptr::copy_nonoverlapping(
                    other.data,
                    self.data,
                    (shared_data_size / BITS_PER_BYTE as u32) as usize,
                );
            }

            // Zero out all pointers in the target.
            for i in 0..self.pointer_count as isize {
                wire_helpers::zero_object(
                    self.arena,
                    self.segment_id,
                    self.pointers.offset(i) as *mut _,
                );
            }
            ptr::write_bytes(self.pointers, 0u8, self.pointer_count as usize);

            for i in 0..shared_pointer_count as isize {
                wire_helpers::copy_pointer(
                    self.arena,
                    self.segment_id,
                    self.cap_table,
                    self.pointers.offset(i),
                    other.arena,
                    other.segment_id,
                    other.cap_table,
                    other.pointers.offset(i),
                    other.nesting_limit,
                    false,
                )?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct ListReader<'a> {
    arena: &'a dyn ReaderArena,
    cap_table: CapTableReader,
    ptr: *const u8,
    segment_id: u32,
    element_count: ElementCount32,
    step: BitCount32,
    struct_data_size: BitCount32,
    nesting_limit: i32,
    struct_pointer_count: WirePointerCount16,
    element_size: ElementSize,
}

impl<'a> ListReader<'a> {
    pub fn new_default<'b>() -> ListReader<'b> {
        ListReader {
            arena: &NULL_ARENA,
            segment_id: 0,
            cap_table: Default::default(),
            ptr: ptr::null(),
            element_count: 0,
            element_size: ElementSize::Void,
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
    pub fn len(&self) -> ElementCount32 {
        self.element_count
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn get_step_size_in_bits(&self) -> u32 {
        self.step
    }

    pub(crate) fn get_element_size(&self) -> ElementSize {
        self.element_size
    }

    pub(crate) fn into_raw_bytes(self) -> &'a [u8] {
        if self.element_count == 0 {
            // Explictly handle this case to avoid forming a slice to a null pointer,
            // which would be undefined behavior.
            &[]
        } else {
            let num_bytes = wire_helpers::round_bits_up_to_bytes(
                u64::from(self.step) * u64::from(self.element_count),
            ) as usize;
            unsafe { ::core::slice::from_raw_parts(self.ptr, num_bytes) }
        }
    }

    #[inline]
    pub fn get_struct_element(&self, index: ElementCount32) -> StructReader<'a> {
        let index_byte: ByteCount32 =
            ((u64::from(index) * u64::from(self.step)) / BITS_PER_BYTE as u64) as u32;

        let struct_data: *const u8 = unsafe { self.ptr.offset(index_byte as isize) };

        let struct_pointers: *const WirePointer =
            unsafe { struct_data.add(self.struct_data_size as usize / BITS_PER_BYTE) as *const _ };

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
        let offset = (self.struct_data_size as u64 / BITS_PER_BYTE as u64
            + u64::from(index) * u64::from(self.step) / BITS_PER_BYTE as u64)
            as isize;
        PointerReader {
            arena: self.arena,
            segment_id: self.segment_id,
            cap_table: self.cap_table,
            pointer: unsafe { self.ptr.offset(offset) } as *const _,
            nesting_limit: self.nesting_limit,
        }
    }

    pub unsafe fn is_canonical(
        &self,
        read_head: &Cell<*const u8>,
        reff: *const WirePointer,
    ) -> Result<bool> {
        match self.element_size {
            ElementSize::InlineComposite => {
                read_head.set(unsafe { read_head.get().add(BYTES_PER_WORD) }); // tag word
                if self.ptr as *const _ != read_head.get() {
                    return Ok(false);
                }
                if self.struct_data_size % BITS_PER_WORD as u32 != 0 {
                    return Ok(false);
                }
                let struct_size = (self.struct_data_size / BITS_PER_WORD as u32)
                    + u32::from(self.struct_pointer_count);
                let word_count = unsafe { (*reff).list_inline_composite_word_count() };
                if struct_size * self.element_count != word_count {
                    return Ok(false);
                }
                if struct_size == 0 {
                    return Ok(true);
                }
                let list_end = unsafe {
                    read_head
                        .get()
                        .add((self.element_count * struct_size) as usize * BYTES_PER_WORD)
                };
                let pointer_head = Cell::new(list_end);
                let mut list_data_trunc = false;
                let mut list_ptr_trunc = false;
                for idx in 0..self.element_count {
                    let mut data_trunc = false;
                    let mut ptr_trunc = false;
                    if !self.get_struct_element(idx).is_canonical(
                        read_head,
                        &pointer_head,
                        &mut data_trunc,
                        &mut ptr_trunc,
                    )? {
                        return Ok(false);
                    }
                    list_data_trunc |= data_trunc;
                    list_ptr_trunc |= ptr_trunc;
                }
                assert_eq!(read_head.get(), list_end);
                read_head.set(pointer_head.get());
                Ok(list_data_trunc && list_ptr_trunc)
            }
            ElementSize::Pointer => {
                if self.ptr as *const _ != read_head.get() {
                    return Ok(false);
                }
                read_head.set(unsafe {
                    read_head
                        .get()
                        .offset(self.element_count as isize * BYTES_PER_WORD as isize)
                });
                for idx in 0..self.element_count {
                    if !self.get_pointer_element(idx).is_canonical(read_head)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            element_size => {
                if self.ptr != read_head.get() as *const _ {
                    return Ok(false);
                }
                let bit_size =
                    u64::from(self.element_count) * u64::from(data_bits_per_element(element_size));
                let mut word_size = bit_size / BITS_PER_WORD as u64;
                if bit_size % BITS_PER_WORD as u64 != 0 {
                    word_size += 1
                }

                let byte_size = bit_size / BITS_PER_BYTE as u64;
                let mut byte_read_head: *const u8 = read_head.get();
                byte_read_head = unsafe { byte_read_head.offset(byte_size as isize) };
                let read_head_end = unsafe {
                    read_head
                        .get()
                        .offset(word_size as isize * BYTES_PER_WORD as isize)
                };

                let leftover_bits = bit_size % BITS_PER_BYTE as u64;
                if leftover_bits > 0 {
                    let mask: u8 = !((1 << leftover_bits as u8) - 1);
                    let partial_byte = unsafe { *byte_read_head };

                    if partial_byte & mask != 0 {
                        return Ok(false);
                    }
                    byte_read_head = unsafe { byte_read_head.offset(1_isize) };
                }

                while byte_read_head != read_head_end {
                    if unsafe { *byte_read_head } != 0 {
                        return Ok(false);
                    }
                    byte_read_head = unsafe { byte_read_head.offset(1_isize) };
                }

                read_head.set(read_head_end);
                Ok(true)
            }
        }
    }
}

pub struct ListBuilder<'a> {
    arena: &'a mut dyn BuilderArena,
    cap_table: CapTableBuilder,
    ptr: *mut u8,
    segment_id: u32,
    element_count: ElementCount32,
    step: BitCount32,
    struct_data_size: BitCount32,
    struct_pointer_count: WirePointerCount16,
    element_size: ElementSize,
}

impl<'a> ListBuilder<'a> {
    #[inline]
    pub fn new_default(arena: &mut dyn BuilderArena) -> ListBuilder<'_> {
        ListBuilder {
            arena,
            segment_id: 0,
            cap_table: Default::default(),
            ptr: ptr::null_mut(),
            element_count: 0,
            element_size: ElementSize::Void,
            step: 0,
            struct_data_size: 0,
            struct_pointer_count: 0,
        }
    }

    pub fn into_reader(self) -> ListReader<'a> {
        ListReader {
            arena: self.arena.as_reader(),
            segment_id: self.segment_id,
            cap_table: self.cap_table.into_reader(),
            ptr: self.ptr as *const _,
            element_count: self.element_count,
            element_size: self.element_size,
            step: self.step,
            struct_data_size: self.struct_data_size,
            struct_pointer_count: self.struct_pointer_count,
            nesting_limit: 0x7fffffff,
        }
    }

    #[inline]
    pub fn reborrow(&mut self) -> ListBuilder<'_> {
        ListBuilder {
            arena: self.arena,
            ..*self
        }
    }

    pub fn imbue(&mut self, cap_table: CapTableBuilder) {
        self.cap_table = cap_table
    }

    #[inline]
    pub fn len(&self) -> ElementCount32 {
        self.element_count
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn get_struct_element(self, index: ElementCount32) -> StructBuilder<'a> {
        let index_byte = ((u64::from(index) * u64::from(self.step)) / BITS_PER_BYTE as u64) as u32;
        let struct_data = unsafe { self.ptr.offset(index_byte as isize) };
        let struct_pointers =
            unsafe { struct_data.add((self.struct_data_size as usize) / BITS_PER_BYTE) as *mut _ };
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

    pub(crate) fn get_element_size(&self) -> ElementSize {
        self.element_size
    }

    #[inline]
    pub fn get_pointer_element(self, index: ElementCount32) -> PointerBuilder<'a> {
        let offset = (u64::from(index) * u64::from(self.step) / BITS_PER_BYTE as u64) as u32;
        PointerBuilder {
            arena: self.arena,
            segment_id: self.segment_id,
            cap_table: self.cap_table,
            pointer: unsafe { self.ptr.offset(offset as isize) } as *mut _,
        }
    }

    pub(crate) fn into_raw_bytes(&self) -> &'a mut [u8] {
        if self.element_count == 0 {
            // Explictly handle this case to avoid forming a slice to a null pointer,
            // which would be undefined behavior.
            &mut []
        } else {
            let num_bytes = wire_helpers::round_bits_up_to_bytes(
                u64::from(self.step) * u64::from(self.element_count),
            ) as usize;
            unsafe { ::core::slice::from_raw_parts_mut(self.ptr, num_bytes) }
        }
    }
}

/**
  An element that can be stored in a `primitive_list`.
*/
pub trait PrimitiveElement {
    /// Gets the element at position `index`. Bounds checking is *not* performed.
    fn get(list_reader: &ListReader, index: ElementCount32) -> Self;

    /// Gets the element at position `index`. Bounds checking is *not* performed.
    fn get_from_builder(list_builder: &ListBuilder, index: ElementCount32) -> Self;

    /// Sets to element at position `index` to be `value`. Bounds checking is *not* performed.
    fn set(list_builder: &ListBuilder, index: ElementCount32, value: Self);

    /// Returns the size of an individual element.
    fn element_size() -> ElementSize;
}

impl<T: Primitive> PrimitiveElement for T {
    #[inline]
    fn get(list_reader: &ListReader, index: ElementCount32) -> Self {
        let offset = (u64::from(index) * u64::from(list_reader.step) / BITS_PER_BYTE as u64) as u32;
        unsafe {
            let ptr: *const u8 = list_reader.ptr.offset(offset as isize);
            <Self as Primitive>::get(&*(ptr as *const <Self as Primitive>::Raw))
        }
    }

    #[inline]
    fn get_from_builder(list_builder: &ListBuilder, index: ElementCount32) -> Self {
        let offset =
            (u64::from(index) * u64::from(list_builder.step) / BITS_PER_BYTE as u64) as u32;
        unsafe {
            let ptr: *mut <Self as Primitive>::Raw =
                list_builder.ptr.offset(offset as isize) as *mut _;
            <Self as Primitive>::get(&*ptr)
        }
    }

    #[inline]
    fn set(list_builder: &ListBuilder, index: ElementCount32, value: Self) {
        let offset =
            (u64::from(index) * u64::from(list_builder.step) / BITS_PER_BYTE as u64) as u32;
        unsafe {
            let ptr: *mut <Self as Primitive>::Raw =
                list_builder.ptr.offset(offset as isize) as *mut _;
            <Self as Primitive>::set(&mut *ptr, value);
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

impl PrimitiveElement for bool {
    #[inline]
    fn get(list: &ListReader, index: ElementCount32) -> Self {
        let bindex = u64::from(index) * u64::from(list.step);
        unsafe {
            let b: *const u8 = list.ptr.offset((bindex / BITS_PER_BYTE as u64) as isize);
            ((*b) & (1 << (bindex % BITS_PER_BYTE as u64))) != 0
        }
    }
    #[inline]
    fn get_from_builder(list: &ListBuilder, index: ElementCount32) -> Self {
        let bindex = u64::from(index) * u64::from(list.step);
        let b = unsafe { list.ptr.offset((bindex / BITS_PER_BYTE as u64) as isize) };
        unsafe { ((*b) & (1 << (bindex % BITS_PER_BYTE as u64))) != 0 }
    }
    #[inline]
    fn set(list: &ListBuilder, index: ElementCount32, value: Self) {
        let bindex = u64::from(index) * u64::from(list.step);
        let b = unsafe { list.ptr.offset((bindex / BITS_PER_BYTE as u64) as isize) };

        let bitnum = bindex % BITS_PER_BYTE as u64;
        unsafe { (*b) = ((*b) & !(1 << bitnum)) | (u8::from(value) << bitnum) }
    }
    fn element_size() -> ElementSize {
        Bit
    }
}

impl PrimitiveElement for () {
    #[inline]
    fn get(_list: &ListReader, _index: ElementCount32) {}

    #[inline]
    fn get_from_builder(_list: &ListBuilder, _index: ElementCount32) {}

    #[inline]
    fn set(_list: &ListBuilder, _index: ElementCount32, _value: ()) {}

    fn element_size() -> ElementSize {
        Void
    }
}
