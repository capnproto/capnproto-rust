/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use common::*;
use endian::*;
use mask::*;
use arena::*;
use blob::*;
use std;

#[repr(u8)]
#[deriving(Eq)]
pub enum FieldSize {
    VOID = 0,
    BIT = 1,
    BYTE = 2,
    TWO_BYTES = 3,
    FOUR_BYTES = 4,
    EIGHT_BYTES = 5,
    POINTER = 6,
    INLINE_COMPOSITE = 7
}

pub fn data_bits_per_element(size : FieldSize) -> BitCount0 {
    match size {
        VOID => 0,
        BIT => 1,
        BYTE => 8,
        TWO_BYTES => 16,
        FOUR_BYTES => 32,
        EIGHT_BYTES => 64,
        POINTER => 0,
        INLINE_COMPOSITE => 0
    }
}

pub fn pointers_per_element(size : FieldSize) -> WirePointerCount {
    match size {
        POINTER => 1,
        _ => 0
    }
}

// Port note: here, this is only valid for T a primitive type. In
// capnproto-c++, it dispatches on the 'kind' of T and can handle
// structs and pointers.
pub fn element_size_for_type<T>() -> FieldSize {
    match bits_per_element::<T>() {
        0 => VOID,
        1 => BIT,
        8 => BYTE,
        16 => TWO_BYTES,
        32 => FOUR_BYTES,
        64 => EIGHT_BYTES,
        b => fail!("don't know how to get field size with {} bits", b)
    }
}

pub enum Kind {
  PRIMITIVE,
  BLOB,
  ENUM,
  STRUCT,
  UNION,
  INTERFACE,
  LIST,
  UNKNOWN
}

// In the future, Rust will have an alignment attribute
// and we won't need the dummy field.
pub struct AlignedData<T> {
    _dummy : u64,
    words : T
}

pub struct StructSize {
    data : WordCount16,
    pointers : WirePointerCount16,
    preferred_list_encoding : FieldSize
}

impl StructSize {
    pub fn total(&self) -> WordCount {
        (self.data as WordCount) + (self.pointers as WordCount) * WORDS_PER_POINTER
    }
}

#[repr(u8)]
#[deriving(Eq)]
pub enum WirePointerKind {
    WP_STRUCT = 0,
    WP_LIST = 1,
    WP_FAR = 2,
    WP_OTHER = 3
}


pub struct WirePointer {
    offset_and_kind : WireValue<u32>,
    upper32bits : u32,
}

pub struct StructRef {
    data_size : WireValue<WordCount16>,
    ptr_count : WireValue<WirePointerCount16>
}

pub struct ListRef {
    element_size_and_count : WireValue<u32>
}

pub struct FarRef {
    segment_id : WireValue<u32>
}

impl StructRef {
    pub fn word_size(&self) -> WordCount {
        self.data_size.get() as WordCount +
            self.ptr_count.get() as WordCount * WORDS_PER_POINTER
    }

    #[inline]
    pub fn set_from_struct_size(&mut self, size : StructSize) {
        self.data_size.set(size.data);
        self.ptr_count.set(size.pointers);
    }

    #[inline]
    pub fn set(&mut self, ds : WordCount16, rc : WirePointerCount16) {
        self.data_size.set(ds);
        self.ptr_count.set(rc);
    }
}

impl ListRef {
    #[inline]
    pub fn element_size(&self) -> FieldSize {
        unsafe {
            std::cast::transmute( (self.element_size_and_count.get() & 7) as u8)
        }
    }

    #[inline]
    pub fn element_count(&self) -> ElementCount {
        (self.element_size_and_count.get() >> 3) as uint
    }

    #[inline]
    pub fn inline_composite_word_count(&self) -> WordCount {
        self.element_count()
    }

    #[inline]
    pub fn set(&mut self, es : FieldSize, ec : ElementCount) {
        assert!(ec < (1 << 29), "Lists are limited to 2**29 elements");
        self.element_size_and_count.set(((ec as u32) << 3 ) | (es as u32));
    }

    #[inline]
    pub fn set_inline_composite(& mut self, wc : WordCount) {
        assert!(wc < (1 << 29), "Inline composite lists are limited to 2 ** 29 words");
        self.element_size_and_count.set((( wc as u32) << 3) | (INLINE_COMPOSITE as u32));
    }
}

impl FarRef {
    #[inline]
    pub fn set(&mut self, si : SegmentId) { self.segment_id.set(si); }
}

impl WirePointer {

    #[inline]
    pub fn kind(&self) -> WirePointerKind {
        unsafe {
            std::cast::transmute((self.offset_and_kind.get() & 3) as u8)
        }
    }

    #[inline]
    pub fn is_capability(&self) -> bool {
        self.offset_and_kind.get() == WP_OTHER as u32
    }

    #[inline]
    pub fn target(&self) -> *Word {
        let thisAddr : *Word = unsafe {std::cast::transmute(&*self) };
        unsafe { thisAddr.offset(1 + ((self.offset_and_kind.get() as int) >> 2)) }
    }

    #[inline]
    pub fn mut_target(&mut self) -> *mut Word {
        let thisAddr : *mut Word = unsafe {std::cast::transmute(&*self) };
        unsafe { thisAddr.offset(1 + ((self.offset_and_kind.get() as int) >> 2)) }
    }

    #[inline]
    pub fn set_kind_and_target<'a>(&mut self, kind : WirePointerKind,
                                   target : *mut Word,
                                   _segmentBuilder : *mut SegmentBuilder<'a>) {
        let thisAddr : int = unsafe {std::cast::transmute(&*self)};
        let targetAddr : int = unsafe {std::cast::transmute(target)};
        self.offset_and_kind.set(
            ((((targetAddr - thisAddr)/BYTES_PER_WORD as int) as i32 - 1) << 2) as u32
                | (kind as u32))
    }

    #[inline]
    pub fn set_kind_with_zero_offset(&mut self, kind : WirePointerKind) {
        self.offset_and_kind.set( kind as u32)
    }

    #[inline]
    pub fn inline_composite_list_element_count(&self) -> ElementCount {
        (self.offset_and_kind.get() >> 2) as ElementCount
    }

    #[inline]
    pub fn set_kind_and_inline_composite_list_element_count(
        &mut self, kind : WirePointerKind, element_count : ElementCount) {
        self.offset_and_kind.set((( element_count as u32 << 2) | (kind as u32)))
    }

    #[inline]
    pub fn far_position_in_segment(&self) -> WordCount {
        (self.offset_and_kind.get() >> 3) as WordCount
    }

    #[inline]
    pub fn is_double_far(&self) -> bool {
        ((self.offset_and_kind.get() >> 2) & 1) != 0
    }

    #[inline]
    pub fn set_far(&mut self, is_double_far : bool, pos : WordCount) {
        self.offset_and_kind.set
            (( pos << 3) as u32 | (is_double_far as u32 << 2) | WP_FAR as u32);
    }

    #[inline]
    pub fn struct_ref(&self) -> StructRef {
        unsafe { std::cast::transmute(self.upper32bits) }
    }

    #[inline]
    pub fn struct_ref_mut<'a>(&'a mut self) -> &'a mut StructRef {
        unsafe { std::cast::transmute(& self.upper32bits) }
    }

    #[inline]
    pub fn list_ref(&self) -> ListRef {
        unsafe { std::cast::transmute(self.upper32bits) }
    }

    #[inline]
    pub fn list_ref_mut<'a>(&'a self) -> &'a mut ListRef {
        unsafe { std::cast::transmute(& self.upper32bits) }
    }

    #[inline]
    pub fn far_ref(&self) -> FarRef {
        unsafe { std::cast::transmute(self.upper32bits) }
    }

    #[inline]
    pub fn far_ref_mut<'a>(&'a mut self) -> &'a mut FarRef {
        unsafe { std::cast::transmute(& self.upper32bits) }
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        (self.offset_and_kind.get() == 0) & (self.upper32bits == 0)
    }
}

struct SegmentAnd<'a, T> {
    segment : *mut SegmentBuilder<'a>,
    value : T
}

mod WireHelpers {
    use std;
    use common::*;
    use layout::*;
    use arena::*;
    use blob::*;

    #[inline]
    pub fn round_bytes_up_to_words(bytes : ByteCount) -> WordCount {
        //# This code assumes 64-bit words.
        (bytes + 7) / BYTES_PER_WORD
    }

    //# The maximum object size is 4GB - 1 byte. If measured in bits,
    //# this would overflow a 32-bit counter, so we need to accept
    //# BitCount64. However, 32 bits is enough for the returned
    //# ByteCounts and WordCounts.
    #[inline]
    pub fn round_bits_up_to_words(bits : BitCount64) -> WordCount {
        //# This code assumes 64-bit words.
        ((bits + 63) / (BITS_PER_WORD as u64)) as WordCount
    }

    #[allow(dead_code)]
    #[inline]
    pub fn round_bits_up_to_bytes(bits : BitCount64) -> ByteCount {
        ((bits + 7) / (BITS_PER_BYTE as u64)) as ByteCount
    }

    #[inline]
    pub unsafe fn bounds_check<'a>(segment : *SegmentReader<'a>,
                                  start : *Word, end : *Word) -> bool {
        //# If segment is null, this is an unchecked message, so we don't do bounds checks.
        return segment.is_null() || (*segment).contains_interval(start, end);
    }

    #[inline]
    pub unsafe fn allocate<'a>(reff : &mut *mut WirePointer,
                               segment : &mut *mut SegmentBuilder<'a>,
                               amount : WordCount, kind : WirePointerKind) -> *mut Word {
        let is_null = (**reff).is_null();
        if (!is_null) {
            zero_object(*segment, *reff)
        }
        match (**segment).allocate(amount) {
            None => {

                //# Need to allocate in a new segment. We'll need to
                //# allocate an extra pointer worth of space to act as
                //# the landing pad for a far pointer.

                let amountPlusRef = amount + POINTER_SIZE_IN_WORDS;
                *segment = (*(**segment).get_arena()).allocate(amountPlusRef);
                let ptr : *mut Word = (**segment).allocate(amountPlusRef).unwrap();

                //# Set up the original pointer to be a far pointer to
                //# the new segment.
                (**reff).set_far(false, (**segment).get_word_offset_to(ptr));
                (**reff).far_ref_mut().segment_id.set((**segment).id);

                //# Initialize the landing pad to indicate that the
                //# data immediately follows the pad.
                *reff = std::cast::transmute(ptr);

                let ptr1 = ptr.offset(POINTER_SIZE_IN_WORDS as int);
                (**reff).set_kind_and_target(kind, ptr1, *segment);
                return ptr1;
            }
            Some(ptr) => {
                (**reff).set_kind_and_target(kind, ptr, *segment);
                return ptr;
            }
        }
    }

    #[inline]
    pub unsafe fn follow_builder_fars<'a>(reff : &mut * mut WirePointer,
                                          ref_target : *mut Word,
                                          segment : &mut *mut SegmentBuilder<'a>) -> *mut Word {
        //# If `ref` is a far pointer, follow it. On return, `ref` will
        //# have been updated to point at a WirePointer that contains
        //# the type information about the target object, and a pointer
        //# to the object contents is returned. The caller must NOT use
        //# `ref->target()` as this may or may not actually return a
        //# valid pointer. `segment` is also updated to point at the
        //# segment which actually contains the object.
        //#
        //# If `ref` is not a far pointer, this simply returns
        //# `refTarget`. Usually, `refTarget` should be the same as
        //# `ref->target()`, but may not be in cases where `ref` is
        //# only a tag.

        if ((**reff).kind() == WP_FAR) {
            *segment = (*(**segment).get_arena()).get_segment((**reff).far_ref().segment_id.get());
            let pad : *mut WirePointer =
                std::cast::transmute((**segment).get_ptr_unchecked((**reff).far_position_in_segment()));
            if !(**reff).is_double_far() {
                *reff = pad;
                return (*pad).mut_target();
            }

            //# Landing pad is another far pointer. It is followed by a
            //# tag describing the pointed-to object.
            *reff = pad.offset(1);
            *segment = (*(**segment).get_arena()).get_segment((*pad).far_ref().segment_id.get());
            return (**segment).get_ptr_unchecked((*pad).far_position_in_segment());
        } else {
            ref_target
        }
    }

    #[inline]
    pub unsafe fn follow_fars<'a>(reff: &mut *WirePointer,
                                 refTarget: *Word,
                                 segment : &mut *SegmentReader<'a>) -> *Word {

        //# If the segment is null, this is an unchecked message,
        //# so there are no FAR pointers.
        if !(*segment).is_null() && (**reff).kind() == WP_FAR {
            *segment =
                (**segment).arena.try_get_segment((**reff).far_ref().segment_id.get());

            let ptr : *Word = (**segment).get_start_ptr().offset(
                (**reff).far_position_in_segment() as int);

            let padWords : int = if ((**reff).is_double_far()) { 2 } else { 1 };
            assert!(bounds_check(*segment, ptr, ptr.offset(padWords)));

            let pad : *WirePointer = std::cast::transmute(ptr);

            if (!(**reff).is_double_far() ) {
                *reff = pad;
                return (*pad).target();
            } else {
                //# Landing pad is another far pointer. It is
                //# followed by a tag describing the pointed-to
                //# object.

                *reff = pad.offset(1);

                *segment =
                    (**segment).arena.try_get_segment((*pad).far_ref().segment_id.get());

                return (**segment).get_start_ptr().offset((*pad).far_position_in_segment() as int);
            }
        } else {
            return refTarget;
        }
    }

    pub unsafe fn zero_object<'a>(mut segment : *mut SegmentBuilder<'a>, reff : *mut WirePointer) {
        //# Zero out the pointed-to object. Use when the pointer is
        //# about to be overwritten making the target object no longer
        //# reachable.

        match (*reff).kind() {
            WP_STRUCT | WP_LIST | WP_OTHER => {
                zero_object_helper(segment,
                                 reff, (*reff).mut_target())
            }
            WP_FAR => {
                segment = (*(*segment).get_arena()).get_segment((*reff).far_ref().segment_id.get());
                let pad : *mut WirePointer =
                    std::cast::transmute((*segment).get_ptr_unchecked((*reff).far_position_in_segment()));

                if ((*reff).is_double_far()) {
                    segment = (*(*segment).get_arena()).get_segment((*pad).far_ref().segment_id.get());

                    zero_object_helper(segment,
                                     pad.offset(1),
                                     (*segment).get_ptr_unchecked((*pad).far_position_in_segment()));

                    std::ptr::set_memory(pad, 0u8, 2);

                } else {
                    zero_object(segment, pad);
                    std::ptr::set_memory(pad, 0u8, 1);
                }
            }
        }
    }

    pub unsafe fn zero_object_helper<'a>(segment : *mut SegmentBuilder<'a>,
                                         tag : *mut WirePointer,
                                         ptr: *mut Word) {
        match (*tag).kind() {
            WP_OTHER => { fail!("Don't know how to handle OTHER") }
            WP_STRUCT => {
                let pointerSection : *mut WirePointer =
                    std::cast::transmute(
                    ptr.offset((*tag).struct_ref().data_size.get() as int));

                let count = (*tag).struct_ref().ptr_count.get() as int;
                for i in range::<int>(0, count) {
                    zero_object(segment, pointerSection.offset(i));
                }
                std::ptr::set_memory(ptr, 0u8, (*tag).struct_ref().word_size());
            }
            WP_LIST => {
                match (*tag).list_ref().element_size() {
                    VOID =>  { }
                    BIT | BYTE | TWO_BYTES | FOUR_BYTES | EIGHT_BYTES => {
                        std::ptr::set_memory(
                            ptr, 0u8,
                            round_bits_up_to_words((
                                    (*tag).list_ref().element_count() *
                                        data_bits_per_element(
                                        (*tag).list_ref().element_size())) as u64))
                    }
                    POINTER => {
                        let count = (*tag).list_ref().element_count() as uint;
                        for i in range::<int>(0, count as int) {
                            zero_object(segment,
                                       std::cast::transmute(ptr.offset(i)))
                        }
                        std::ptr::set_memory(ptr, 0u8, count);
                    }
                    INLINE_COMPOSITE => {
                        let elementTag : *mut WirePointer = std::cast::transmute(ptr);

                        assert!((*elementTag).kind() == WP_STRUCT,
                                "Don't know how to handle non-STRUCT inline composite");

                        let data_size = (*elementTag).struct_ref().data_size.get();
                        let pointer_count = (*elementTag).struct_ref().ptr_count.get();
                        let mut pos : *mut Word = ptr.offset(1);
                        let count = (*elementTag).inline_composite_list_element_count();
                        for _ in range(0, count) {
                            pos = pos.offset(data_size as int);
                            for _ in range(0, pointer_count as uint) {
                                zero_object(
                                    segment,
                                    std::cast::transmute::<*mut Word, *mut WirePointer>(pos));
                                pos = pos.offset(1);
                            }
                        }
                        std::ptr::set_memory(ptr, 0u8,
                                             (*elementTag).struct_ref().word_size() * count + 1);
                    }
                }
            }
            WP_FAR => { fail!("Unexpected FAR pointer") }
        }
    }

    #[inline]
    pub unsafe fn zero_pointer_and_fars<'a>(segment : *mut SegmentBuilder<'a>, reff : *mut WirePointer) {
        //# Zero out the pointer itself and, if it is a far pointer,
        //# zero the landing pad as well, but do not zero the object
        //# body. Used when upgrading.

        if ((*reff).kind() == WP_FAR) {
            let pad = (*(*(*segment).get_arena()).get_segment((*reff).far_ref().segment_id.get()))
                .get_ptr_unchecked((*reff).far_position_in_segment());
            let num_elements = if (*reff).is_double_far() { 2 } else { 1 };
            std::ptr::zero_memory(pad, num_elements);
        }
        std::ptr::zero_memory(reff, 1);
    }

    pub unsafe fn total_size(mut segment : *SegmentReader, mut reff : *WirePointer, mut nesting_limit : int)
                             -> MessageSize {
        let mut result = MessageSize { word_count : 0, cap_count : 0};

        if (*reff).is_null() { return result };
        nesting_limit -= 1;

        let ptr = follow_fars(&mut reff, (*reff).target(), &mut segment);

        match ((*reff).kind()) {
            WP_STRUCT => {
                assert!(bounds_check(segment, ptr, ptr.offset((*reff).struct_ref().word_size() as int)),
                        "Message contains out-of-bounds struct pointer.");
                result.word_count += (*reff).struct_ref().word_size() as u64;

                let pointer_section : *WirePointer =
                    std::cast::transmute(ptr.offset((*reff).struct_ref().data_size.get() as int));
                let count : int = (*reff).struct_ref().ptr_count.get() as int;
                for i in range(0, count) {
                    result.plus_eq(total_size(segment, pointer_section.offset(i), nesting_limit));
                }
            }
            WP_LIST => {
                match (*reff).list_ref().element_size() {
                    VOID => {}
                    BIT | BYTE | TWO_BYTES | FOUR_BYTES | EIGHT_BYTES => {
                        let total_words = round_bits_up_to_words(
                            (*reff).list_ref().element_count() as u64 *
                                data_bits_per_element((*reff).list_ref().element_size()) as u64);
                        assert!(bounds_check(segment, ptr, ptr.offset(total_words as int)),
                                "Message contains out-of-bounds list pointer.");
                        result.word_count += total_words as u64;
                    }
                    POINTER => {
                        let count = (*reff).list_ref().element_count();
                        assert!(bounds_check(segment, ptr, ptr.offset((count * WORDS_PER_POINTER) as int)),
                                "Message contains out-of-bounds list pointer.");

                        result.word_count += count as u64 * WORDS_PER_POINTER as u64;

                        for i in range(0, count as int) {
                            result.plus_eq(
                                total_size(segment, std::cast::transmute::<*Word,*WirePointer>(ptr).offset(i),
                                           nesting_limit));
                        }
                    }
                    INLINE_COMPOSITE => {
                        let word_count = (*reff).list_ref().inline_composite_word_count();
                        assert!(bounds_check(segment, ptr,
                                             ptr.offset(word_count as int + POINTER_SIZE_IN_WORDS as int)),
                                "Message contains out-of-bounds list pointer.");

                        result.word_count += word_count as u64 + POINTER_SIZE_IN_WORDS as u64;

                        let element_tag : *WirePointer = std::cast::transmute(ptr);
                        let count = (*element_tag).inline_composite_list_element_count();

                        assert!((*element_tag).kind() == WP_STRUCT,
                                "Don't know how to handle non-STRUCT inline composite.");

                        assert!((*element_tag).struct_ref().word_size() * count <= word_count,
                                "INLINE_COMPOSITE list's elements overrun its word count");

                        let data_size = (*element_tag).struct_ref().data_size.get();
                        let pointer_count = (*element_tag).struct_ref().ptr_count.get();

                        let mut pos : *Word = ptr.offset(POINTER_SIZE_IN_WORDS as int);
                        for _ in range(0, count) {
                            pos = pos.offset(data_size as int);

                            for _ in range(0, pointer_count) {
                                result.plus_eq(
                                    total_size(segment, std::cast::transmute::<*Word,*WirePointer>(pos),
                                               nesting_limit));
                                pos = pos.offset(POINTER_SIZE_IN_WORDS as int);
                            }
                        }
                    }
                }
            }
            WP_FAR => {
                fail!("Unexpedted FAR pointer.");
            }
            WP_OTHER => {
                if ((*reff).is_capability()) {
                    result.cap_count += 1;
                } else {
                    fail!("Unknown pointer type.");
                }
            }
        }

        result
    }

    pub unsafe fn transfer_pointer<'a>(dst_segment : *mut SegmentBuilder<'a>, dst : *mut WirePointer,
                                       src_segment : *mut SegmentBuilder<'a>, src : *mut WirePointer) {
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

        if ((*src).is_null()) {
            std::ptr::zero_memory(dst, 1);
        } else if (*src).kind() == WP_FAR {
            std::ptr::copy_nonoverlapping_memory(dst, src, 1);
        } else {
            transfer_pointer_split(dst_segment, dst, src_segment, src, (*src).mut_target());
        }
    }

    pub unsafe fn transfer_pointer_split<'a>(dst_segment : *mut SegmentBuilder<'a>, dst : *mut WirePointer,
                                             src_segment : *mut SegmentBuilder<'a>, src_tag : *mut WirePointer,
                                             src_ptr : *mut Word) {
        // Like the other transfer_pointer, but splits src into a tag and a
        // target. Particularly useful for OrphanBuilder.

        if (dst_segment == src_segment) {
            //# Same segment, so create a direct pointer.
            (*dst).set_kind_and_target((*src_tag).kind(), src_ptr, dst_segment);

            //# We can just copy the upper 32 bits. (Use memcpy() to complt with aliasing rules.)
            // (?)
            std::ptr::copy_nonoverlapping_memory(std::ptr::to_mut_unsafe_ptr(&mut (*dst).upper32bits),
                                                 std::ptr::to_unsafe_ptr(&(*src_tag).upper32bits),
                                                 1);
        } else {
            //# Need to create a far pointer. Try to allocate it in the
            //# same segment as the source, so that it doesn't need to
            //# be a double-far.

            match (*src_segment).allocate(1) {
                None => {
                    //# Darn, need a double-far.
                    fail!("unimplemented");
                }
                Some(landing_pad_word) => {
                    //# Simple landing pad is just a pointer.
                    let landing_pad : *mut WirePointer = std::cast::transmute(landing_pad_word);
                    (*landing_pad).set_kind_and_target((*src_tag).kind(), src_ptr, src_segment);
                    std::ptr::copy_nonoverlapping_memory(
                        std::ptr::to_mut_unsafe_ptr(&mut (*landing_pad).upper32bits),
                        std::ptr::to_unsafe_ptr(& (*src_tag).upper32bits), 1);

                    (*dst).set_far(false, (*src_segment).get_word_offset_to(landing_pad_word));
                    (*dst).far_ref().set((*src_segment).get_segment_id());
                }
            }
        }
    }

    #[inline]
    pub unsafe fn init_struct_pointer<'a>(mut reff : *mut WirePointer,
                                          mut segmentBuilder : *mut SegmentBuilder<'a>,
                                          size : StructSize) -> StructBuilder<'a> {
        let ptr : *mut Word = allocate(&mut reff, &mut segmentBuilder, size.total(), WP_STRUCT);
        (*reff).struct_ref_mut().set_from_struct_size(size);

        StructBuilder {
            segment : segmentBuilder,
            data : std::cast::transmute(ptr),
            pointers : std::cast::transmute(
                    ptr.offset((size.data as uint) as int)),
            data_size : size.data as WordCount32 * (BITS_PER_WORD as BitCount32),
            pointer_count : size.pointers,
            bit0offset : 0
        }
    }

    #[inline]
    pub unsafe fn get_writable_struct_pointer<'a>(mut reff : *mut WirePointer,
                                                  mut segment : *mut SegmentBuilder<'a>,
                                                  size : StructSize,
                                                  default_value : *Word) -> StructBuilder<'a> {
        if((*reff).is_null()) {
            if default_value.is_null() ||
                (*std::cast::transmute::<*Word,*WirePointer>(default_value)).is_null() {

                return init_struct_pointer(reff, segment, size);
            }
            fail!("TODO")
        }

        let ref_target = (*reff).mut_target();
        let mut old_ref = reff;
        let mut old_segment = segment;
        let old_ptr = follow_builder_fars(&mut old_ref, ref_target, &mut old_segment);
        assert!((*old_ref).kind() == WP_STRUCT,
                "Message contains non-struct pointer where struct pointer was expected.");

        let old_data_size = (*old_ref).struct_ref().data_size.get();
        let old_pointer_count = (*old_ref).struct_ref().ptr_count.get();
        let old_pointer_section : *mut WirePointer = std::cast::transmute(old_ptr.offset(old_data_size as int));

        if (old_data_size < size.data || old_pointer_count < size.pointers) {
            //# The space allocated for this struct is too small.
            //# Unlike with readers, we can't just run with it and do
            //# bounds checks at access time, because how would we
            //# handle writes? Instead, we have to copy the struct to a
            //# new space now.

            let new_data_size = std::cmp::max(old_data_size, size.data);
            let new_pointer_count = std::cmp::max(old_pointer_count, size.pointers);
            let total_size = new_data_size as WordCount + new_pointer_count as WordCount * WORDS_PER_POINTER;

            //# Don't let allocate() zero out the object just yet.
            zero_pointer_and_fars(segment, reff);

            let ptr = allocate(&mut reff, &mut segment, total_size, WP_STRUCT);
            (*reff).struct_ref().set(new_data_size, new_pointer_count);

            //# Copy data section.
            // Note: copy_nonoverlapping memory's third argument is an element count, not a byte count.
            std::ptr::copy_nonoverlapping_memory(ptr, old_ptr,
                                                 old_data_size as uint);


            //# Copy pointer section.
            let new_pointer_section : *mut WirePointer =
                std::cast::transmute(ptr.offset(new_data_size as int));
            for i in range::<int>(0, old_pointer_count as int) {
                transfer_pointer(segment, new_pointer_section.offset(i),
                                 old_segment, old_pointer_section.offset(i));
            }

            std::ptr::zero_memory(old_ptr, old_data_size as uint + old_pointer_count as uint);

            StructBuilder {
                segment : segment,
                data : std::cast::transmute(ptr),
                pointers : new_pointer_section,
                data_size : new_data_size as u32 * BITS_PER_WORD as u32,
                pointer_count : new_pointer_count,
                bit0offset : 0
            }

        } else {
            StructBuilder {
                segment : old_segment,
                data : std::cast::transmute(old_ptr),
                pointers : old_pointer_section,
                data_size : old_data_size as u32 * BITS_PER_WORD as u32,
                pointer_count : old_pointer_count,
                bit0offset : 0
            }
        }
    }

    #[inline]
    pub unsafe fn init_list_pointer<'a>(mut reff : *mut WirePointer,
                                        mut segmentBuilder : *mut SegmentBuilder<'a>,
                                        element_count : ElementCount,
                                        element_size : FieldSize) -> ListBuilder<'a> {
        match element_size {
            INLINE_COMPOSITE => {
                fail!("Should have called initStructListPointer() instead")
            }
            _ => { }
        }

        let data_size : BitCount0 = data_bits_per_element(element_size);
        let pointer_count = pointers_per_element(element_size);
        let step = (data_size + pointer_count * BITS_PER_POINTER);
        let wordCount = round_bits_up_to_words(element_count as ElementCount64 * (step as u64));
        let ptr = allocate(&mut reff, &mut segmentBuilder, wordCount, WP_LIST);

        (*reff).list_ref_mut().set(element_size, element_count);

        ListBuilder {
            segment : segmentBuilder,
            ptr : std::cast::transmute(ptr),
            step : step,
            element_count : element_count,
            struct_data_size : data_size as u32,
            struct_pointer_count : pointer_count as u16
        }
    }

    #[inline]
    pub unsafe fn init_struct_list_pointer<'a>(mut reff : *mut WirePointer,
                                               mut segmentBuilder : *mut SegmentBuilder<'a>,
                                               element_count : ElementCount,
                                               element_size : StructSize) -> ListBuilder<'a> {
        match element_size.preferred_list_encoding {
            INLINE_COMPOSITE => { }
            otherEncoding => {
                return init_list_pointer(reff, segmentBuilder, element_count, otherEncoding);
            }
        }

        let wordsPerElement = element_size.total();

        //# Allocate the list, prefixed by a single WirePointer.
        let wordCount : WordCount = element_count * wordsPerElement;
        let ptr : *mut WirePointer =
            std::cast::transmute(allocate(&mut reff, &mut segmentBuilder,
                                          POINTER_SIZE_IN_WORDS + wordCount, WP_LIST));

        //# Initialize the pointer.
        (*reff).list_ref_mut().set_inline_composite(wordCount);
        (*ptr).set_kind_and_inline_composite_list_element_count(WP_STRUCT, element_count);
        (*ptr).struct_ref_mut().set_from_struct_size(element_size);

        let ptr1 = ptr.offset(POINTER_SIZE_IN_WORDS as int);

        ListBuilder {
            segment : segmentBuilder,
            ptr : std::cast::transmute(ptr1),
            step : wordsPerElement * BITS_PER_WORD,
            element_count : element_count,
            struct_data_size : element_size.data as u32 * (BITS_PER_WORD as u32),
            struct_pointer_count : element_size.pointers
        }
    }

    #[inline]
    pub unsafe fn get_writable_list_pointer<'a>(orig_ref : *mut WirePointer,
                                                orig_segment : *mut SegmentBuilder<'a>,
                                                element_size : FieldSize,
                                                default_value : *Word) -> ListBuilder<'a> {
        assert!(element_size != INLINE_COMPOSITE,
                "Use get_struct_list_{element,field}() for structs");

        if((*orig_ref).is_null()) {
            if default_value.is_null() ||
                (*std::cast::transmute::<*Word,*WirePointer>(default_value)).is_null() {

                return ListBuilder::new_default();
            }
            fail!("TODO")
        }

        let orig_ref_target = (*orig_ref).mut_target();

        //# We must verify that the pointer has the right size. Unlike
        //# in getWritableStructListReference(), we never need to
        //# "upgrade" the data, because this method is called only for
        //# non-struct lists, and there is no allowed upgrade path *to*
        //# a non-struct list, only *from* them.

        let mut reff = orig_ref;
        let mut segment = orig_segment;
        let mut ptr = follow_builder_fars(&mut reff, orig_ref_target, &mut segment);

        assert!((*reff).kind() == WP_LIST,
                "Called get_list_{field,element}() but existing pointer is not a list");

        let old_size = (*reff).list_ref().element_size();

        if (old_size == INLINE_COMPOSITE) {
            //# The existing element size is INLINE_COMPOSITE, which
            //# means that it is at least two words, which makes it
            //# bigger than the expected element size. Since fields can
            //# only grow when upgraded, the existing data must have
            //# been written with a newer version of the protocol. We
            //# therefore never need to upgrade the data in this case,
            //# but we do need to validate that it is a valid upgrade
            //# from what we expected.

            //# Read the tag to get the actual element count.
            let tag : *WirePointer = std::cast::transmute(ptr);
            assert!((*tag).kind() == WP_STRUCT,
                    "INLINE_COMPOSITE list with non-STRUCT elements not supported.");
            ptr = ptr.offset(POINTER_SIZE_IN_WORDS as int);

            let data_size = (*tag).struct_ref().data_size.get();
            let pointer_count = (*tag).struct_ref().ptr_count.get();

            match element_size {
                VOID => {} //# Anything is a valid upgrad from Void.
                BIT | BYTE | TWO_BYTES | FOUR_BYTES | EIGHT_BYTES => {
                    assert!(data_size >= 1,
                            "Existing list value is incompatible with expected type.");
                }
                POINTER => {
                    assert!(pointer_count >= 1,
                            "Existing list value is incompatible with expected type.");
                    //# Adjust the pointer to point at the reference segment.
                    ptr = ptr.offset(data_size as int);
                }
                INLINE_COMPOSITE => {
                    unreachable!()
                }
            }

            //# OK, looks valid.

            ListBuilder {
                segment : segment,
                ptr : std::cast::transmute(ptr),
                element_count : (*tag).inline_composite_list_element_count(),
                step : (*tag).struct_ref().word_size() * BITS_PER_WORD,
                struct_data_size : data_size as u32 * BITS_PER_WORD as u32,
                struct_pointer_count : pointer_count
            }
        } else {
            let data_size = data_bits_per_element(old_size);
            let pointer_count = pointers_per_element(old_size);

            assert!(data_size >= data_bits_per_element(element_size),
                    "Existing list value is incompatible with expected type.");
            assert!(pointer_count >= pointers_per_element(element_size),
                    "Existing list value is incompatible with expected type.");

            let step = data_size + pointer_count * BITS_PER_POINTER;

            ListBuilder {
                segment : segment,
                ptr : std::cast::transmute(ptr),
                step : step,
                element_count : (*reff).list_ref().element_count(),
                struct_data_size : data_size as u32,
                struct_pointer_count : pointer_count as u16
            }
        }
    }

    #[inline]
    pub unsafe fn get_writable_struct_list_pointer<'a>(_origRefIndex : *mut WirePointer,
                                                       _origSegment : *mut SegmentBuilder<'a>,
                                                       _element_size : StructSize,
                                                       _defaultValue : *Word) -> ListBuilder<'a> {
        fail!("unimplemented")
    }

    #[inline]
    pub unsafe fn init_text_pointer<'a>(_reff : *mut WirePointer,
                                        _segment : *mut SegmentBuilder<'a>,
                                        _size : ByteCount) -> super::SegmentAnd<Text::Builder<'a>> {
        fail!("unimplemented")
    }

    #[inline]
    pub unsafe fn set_text_pointer<'a>(mut reff : *mut WirePointer,
                                       mut segmentBuilder : *mut SegmentBuilder<'a>,
                                       value : &str) {

        // initTextPointer is rolled in here

        let bytes : &[u8] = value.as_bytes();

        //# The byte list must include a NUL terminator
        let byteSize = bytes.len() + 1;

        let ptr =
            allocate(&mut reff, &mut segmentBuilder, round_bytes_up_to_words(byteSize), WP_LIST);

        (*reff).list_ref_mut().set(BYTE, byteSize);
        let dst : *mut u8 = std::cast::transmute(ptr);
        let src : *u8 = bytes.unsafe_ref(0);
        std::ptr::copy_nonoverlapping_memory(dst, src, bytes.len());

        // null terminate
        std::ptr::zero_memory(dst.offset(bytes.len() as int), 1);
    }

    #[inline]
    pub unsafe fn init_data_pointer<'a>(_reff : *mut WirePointer,
                                        _segment : *mut SegmentBuilder<'a>,
                                        _size : ByteCount) -> super::SegmentAnd<Data::Builder<'a>> {
        fail!("unimplemented")
    }

    #[inline]
    pub unsafe fn set_data_pointer<'a>(mut reff : *mut WirePointer,
                                       mut segmentBuilder : *mut SegmentBuilder<'a>,
                                       value : &[u8]) {

        // initDataPointer is rolled in here

        let ptr =
            allocate(&mut reff, &mut segmentBuilder, round_bytes_up_to_words(value.len()), WP_LIST);

        (*reff).list_ref_mut().set(BYTE, value.len());
        let dst : *mut u8 = std::cast::transmute(ptr);
        let src : *u8 = value.unsafe_ref(0);
        std::ptr::copy_nonoverlapping_memory(dst, src, value.len());
    }

    #[inline]
    pub unsafe fn get_writable_text_pointer<'a>(_refIndex : *mut WirePointer,
                                                _segment : *mut SegmentBuilder<'a>,
                                                _default_value : *Word,
                                                _default_size : ByteCount) -> Text::Builder<'a> {
        fail!("unimplemented");
    }

    #[inline]
    pub unsafe fn get_writable_data_pointer<'a>(_refIndex : *mut WirePointer,
                                                _segment : *mut SegmentBuilder<'a>,
                                                _default_value : *Word,
                                                _default_size : ByteCount) -> Data::Builder<'a> {
        fail!("unimplemented");
    }

    pub unsafe fn set_struct_pointer<'a>(mut segment : *mut SegmentBuilder<'a>,
                                         mut reff : *mut WirePointer,
                                         value : StructReader) -> super::SegmentAnd<'a, *mut Word> {
        let data_size : WordCount = round_bits_up_to_words(value.data_size as u64);
        let total_size : WordCount = data_size + value.pointer_count as uint * WORDS_PER_POINTER;

        let ptr = allocate(&mut reff, &mut segment, total_size, WP_STRUCT);
        (*reff).struct_ref().set(data_size as u16, value.pointer_count);

        if (value.data_size == 1) {
            *std::cast::transmute::<*mut Word, *mut u8>(ptr) = value.get_bool_field(0) as u8
        } else {
            std::ptr::copy_nonoverlapping_memory::<Word, *Word>(ptr, std::cast::transmute(value.data),
                                                                value.data_size as uint / BITS_PER_WORD);
        }

        let pointer_section : *mut WirePointer = std::cast::transmute(ptr.offset(data_size as int));
        for i in range(0, value.pointer_count as int) {
            copy_pointer(segment, pointer_section.offset(i), value.segment, value.pointers.offset(i),
                         value.nesting_limit);
        }

        super::SegmentAnd { segment : segment, value : ptr }
    }

    pub unsafe fn set_list_pointer<'a>(mut segment : *mut SegmentBuilder<'a>,
                                       mut reff : *mut WirePointer,
                                       value : ListReader) -> super::SegmentAnd<'a, *mut Word> {
        let total_size = round_bits_up_to_words((value.element_count * value.step) as u64);

        if (value.step <= BITS_PER_WORD) {
            //# List of non-structs.
            let ptr = allocate(&mut reff, &mut segment, total_size, WP_LIST);

            if (value.struct_pointer_count == 1) {
                //# List of pointers.
                (*reff).list_ref().set(POINTER, value.element_count);
                for i in range(0, value.element_count as int) {
                    copy_pointer(segment, std::cast::transmute::<*mut Word,*mut WirePointer>(ptr).offset(i),
                                 value.segment, std::cast::transmute::<*u8,*WirePointer>(value.ptr).offset(i),
                                 value.nesting_limit);
                }
            } else {
                //# List of data.
                let element_size = match (value.step) {
                    0 => VOID,
                    1 => BIT,
                    8 => BYTE,
                    16 => TWO_BYTES,
                    32 => FOUR_BYTES,
                    64 => EIGHT_BYTES,
                    _ => { fail!("invalid list step size: {}", value.step) }
                };

                (*reff).list_ref().set(element_size, value.element_count);
                std::ptr::copy_memory(ptr, std::cast::transmute::<*u8,*Word>(value.ptr), total_size);
            }

            super::SegmentAnd { segment : segment, value : ptr }
        } else {
            //# List of structs.
            let ptr = allocate(&mut reff, &mut segment, total_size + POINTER_SIZE_IN_WORDS, WP_LIST);
            (*reff).list_ref().set_inline_composite(total_size);

            let data_size = round_bits_up_to_words(value.struct_data_size as u64);
            let pointer_count = value.struct_pointer_count;

            let tag : *mut WirePointer = std::cast::transmute(ptr);
            (*tag).set_kind_and_inline_composite_list_element_count(WP_STRUCT, value.element_count);
            (*tag).struct_ref().set(data_size as u16, pointer_count);
            let mut dst = ptr.offset(POINTER_SIZE_IN_WORDS as int);

            let mut src : *Word = std::cast::transmute(value.ptr);
            for _ in range(0, value.element_count) {
                std::ptr::copy_nonoverlapping_memory(dst, src,
                                                     value.struct_data_size as uint / BITS_PER_WORD);
                dst = dst.offset(data_size as int);
                src = src.offset(data_size as int);

                for _ in range(0, pointer_count) {
                    copy_pointer(segment, std::cast::transmute(dst),
                                 value.segment, std::cast::transmute(src), value.nesting_limit);
                    dst = dst.offset(POINTER_SIZE_IN_WORDS as int);
                    src = src.offset(POINTER_SIZE_IN_WORDS as int);
                }
            }
            super::SegmentAnd { segment : segment, value : ptr }
        }
    }

    pub unsafe fn copy_pointer<'a>(dst_segment : *mut SegmentBuilder<'a>, dst : *mut WirePointer,
                                   mut src_segment : *SegmentReader, mut src : *WirePointer,
                                   nesting_limit : int) -> super::SegmentAnd<'a, *mut Word> {
        let src_target = (*src).target();

        if (*src).is_null() {
            std::ptr::zero_memory(dst, 1);
            return super::SegmentAnd { segment : dst_segment, value : std::ptr::mut_null() };
        }

        let mut ptr = follow_fars(&mut src, src_target, &mut src_segment);
        // TODO what if ptr is null?

        match (*src).kind() {
            WP_STRUCT => {
                assert!(nesting_limit > 0,
                        "Message is too deeply-nested or contains cycles.  See ReadOptions.");
                assert!(bounds_check(src_segment, ptr, ptr.offset((*src).struct_ref().word_size() as int)),
                        "Message contains out-of-bounds struct pointer.");
                set_struct_pointer(
                    dst_segment, dst,
                    StructReader {
                        segment : src_segment,
                        data : std::cast::transmute(ptr),
                        pointers : std::cast::transmute(ptr.offset((*src).struct_ref().data_size.get() as int)),
                        data_size : (*src).struct_ref().data_size.get() as u32 * BITS_PER_WORD as u32,
                        pointer_count : (*src).struct_ref().ptr_count.get(),
                        bit0offset : 0,
                        nesting_limit : nesting_limit - 1 })

            }
            WP_LIST => {
                let element_size = (*src).list_ref().element_size();
                assert!(nesting_limit > 0,
                        "Message is too deeply-nested or contains cycles. See ReadOptions.");

                if (element_size == INLINE_COMPOSITE) {
                    let word_count = (*src).list_ref().inline_composite_word_count();
                    let tag : *WirePointer = std::cast::transmute(ptr);
                    ptr = ptr.offset(POINTER_SIZE_IN_WORDS as int);

                    assert!(bounds_check(src_segment, ptr.offset(-1), ptr.offset(word_count as int)),
                            "Message contains out-of-bounds list pointer.");

                    assert!((*tag).kind() == WP_STRUCT,
                            "INLINE_COMPOSITE lists of non-STRUCT type are not supported.");

                    let element_count = (*tag).inline_composite_list_element_count();
                    let words_per_element = (*tag).struct_ref().word_size();

                    assert!(words_per_element * element_count <= word_count,
                            "INLINE_COMPOSITE list's elements overrun its word count.");
                    set_list_pointer(
                        dst_segment, dst,
                        ListReader {
                            segment : src_segment,
                            ptr : std::cast::transmute(ptr),
                            element_count : element_count,
                            step : words_per_element * BITS_PER_WORD,
                            struct_data_size : (*tag).struct_ref().data_size.get() as u32 * BITS_PER_WORD as u32,
                            struct_pointer_count : (*tag).struct_ref().ptr_count.get(),
                            nesting_limit : nesting_limit - 1
                        })
                } else {
                    let data_size = data_bits_per_element(element_size);
                    let pointer_count = pointers_per_element(element_size);
                    let step = data_size + pointer_count * BITS_PER_POINTER;
                    let element_count = (*src).list_ref().element_count();
                    let word_count = round_bits_up_to_words(element_count as u64 * step as u64);

                    assert!(bounds_check(src_segment, ptr, ptr.offset(word_count as int)),
                            "Message contains out-of-bounds list pointer.");

                    set_list_pointer(
                        dst_segment, dst,
                        ListReader {
                            segment : src_segment,
                            ptr : std::cast::transmute(ptr),
                            element_count : element_count,
                            step : step,
                            struct_data_size : data_size as u32,
                            struct_pointer_count : pointer_count as u16,
                            nesting_limit : nesting_limit - 1
                        })
                }
            }
            WP_FAR => {
                fail!("Far pointer should have been handled above");
            }
            WP_OTHER => {
                assert!((*src).is_capability(), "Unknown pointer type.");
                fail!("unimplemented");
            }
        }
    }

    #[inline]
    pub unsafe fn read_struct_pointer<'a>(mut segment: *SegmentReader<'a>,
                                        mut reff : *WirePointer,
                                        defaultValue : *Word,
                                        nesting_limit : int) -> StructReader<'a> {

        if ((*reff).is_null()) {
            if (defaultValue.is_null() ||
                (*std::cast::transmute::<*Word,*WirePointer>(defaultValue)).is_null()) {
                    return StructReader::new_default();
            }

            //segment = std::ptr::null();
            //reff = std::cast::transmute::<*Word,*WirePointer>(defaultValue);
            fail!("default struct values unimplemented");
        }

        let refTarget : *Word = (*reff).target();

        assert!(nesting_limit > 0, "Message is too deeply-nested or contains cycles.");

        let ptr = follow_fars(&mut reff, refTarget, &mut segment);

        let data_size_words = (*reff).struct_ref().data_size.get();

        assert!(bounds_check(segment, ptr,
                            ptr.offset((*reff).struct_ref().word_size() as int)),
                "Message contains out-of-bounds struct pointer.");

        StructReader {segment : segment,
                      data : std::cast::transmute(ptr),
                      pointers : std::cast::transmute(ptr.offset(data_size_words as int)),
                      data_size : data_size_words as u32 * BITS_PER_WORD as BitCount32,
                      pointer_count : (*reff).struct_ref().ptr_count.get(),
                      bit0offset : 0,
                      nesting_limit : nesting_limit - 1 }
     }

    #[inline]
    pub unsafe fn read_list_pointer<'a>(mut segment: *SegmentReader<'a>,
                                      mut reff : *WirePointer,
                                      defaultValue : *Word,
                                      expectedElementSize : FieldSize,
                                      nesting_limit : int ) -> ListReader<'a> {

        if ((*reff).is_null()) {
            if defaultValue.is_null() ||
                (*std::cast::transmute::<*Word,*WirePointer>(defaultValue)).is_null() {
                return ListReader::new_default();
            }
            fail!("list default values unimplemented");
        }

        let refTarget : *Word = (*reff).target();

        if (nesting_limit <= 0) {
           fail!("nesting limit exceeded");
        }

        let mut ptr : *Word = follow_fars(&mut reff, refTarget, &mut segment);

        assert!((*reff).kind() == WP_LIST,
                "Message contains non-list pointer where list pointer was expected {:?}", reff);

        let list_ref = (*reff).list_ref();

        match list_ref.element_size() {
            INLINE_COMPOSITE => {
                let wordCount = list_ref.inline_composite_word_count();

                let tag: *WirePointer = std::cast::transmute(ptr);

                ptr = ptr.offset(1);

                assert!(bounds_check(segment, ptr.offset(-1),
                                    ptr.offset(wordCount as int)));

                assert!((*tag).kind() == WP_STRUCT,
                        "INLINE_COMPOSITE lists of non-STRUCT type are not supported");

                let size = (*tag).inline_composite_list_element_count();
                let struct_ref = (*tag).struct_ref();
                let wordsPerElement = struct_ref.word_size();

                assert!(size * wordsPerElement <= wordCount,
                       "INLINE_COMPOSITE list's elements overrun its word count");

                //# If a struct list was not expected, then presumably
                //# a non-struct list was upgraded to a struct list.
                //# We need to manipulate the pointer to point at the
                //# first field of the struct. Together with the
                //# "stepBits", this will allow the struct list to be
                //# accessed as if it were a primitive list without
                //# branching.

                //# Check whether the size is compatible.
                match expectedElementSize {
                    VOID => {}
                    BIT => fail!("Expected a bit list, but got a list of structs"),
                    BYTE | TWO_BYTES | FOUR_BYTES | EIGHT_BYTES => {
                        assert!(struct_ref.data_size.get() > 0,
                               "Expected a primitive list, but got a list of pointer-only structs")
                    }
                    POINTER => {
                        ptr = ptr.offset(struct_ref.data_size.get() as int);
                        assert!(struct_ref.ptr_count.get() > 0,
                               "Expected a pointer list, but got a list of data-only structs")
                    }
                    INLINE_COMPOSITE => {}
                }

                ListReader {
                    segment : segment,
                    ptr : std::cast::transmute(ptr),
                    element_count : size,
                    step : wordsPerElement * BITS_PER_WORD,
                    struct_data_size : struct_ref.data_size.get() as u32 * (BITS_PER_WORD as u32),
                    struct_pointer_count : struct_ref.ptr_count.get() as u16,
                    nesting_limit : nesting_limit - 1
                }
            }
            _ => {

                //# This is a primitive or pointer list, but all such
                //# lists can also be interpreted as struct lists. We
                //# need to compute the data size and pointer count for
                //# such structs.
                let data_size = data_bits_per_element(list_ref.element_size());
                let pointer_count = pointers_per_element(list_ref.element_size());
                let step = data_size + pointer_count * BITS_PER_POINTER;

                assert!(
                    bounds_check(
                        segment, ptr,
                        ptr.offset(
                            round_bits_up_to_words(
                                (list_ref.element_count() * step) as u64) as int)));

                //# Verify that the elements are at least as large as
                //# the expected type. Note that if we expected
                //# INLINE_COMPOSITE, the expected sizes here will be
                //# zero, because bounds checking will be performed at
                //# field access time. So this check here is for the
                //# case where we expected a list of some primitive or
                //# pointer type.

                let expectedDataBitsPerElement =
                        data_bits_per_element(expectedElementSize);
                let expectedPointersPerElement =
                    pointers_per_element(expectedElementSize);

                assert!(expectedDataBitsPerElement <= data_size);
                assert!(expectedPointersPerElement <= pointer_count)

                ListReader {
                    segment : segment,
                    ptr : std::cast::transmute(ptr),
                    element_count : list_ref.element_count(),
                    step : step,
                    struct_data_size : data_size as u32,
                    struct_pointer_count : pointer_count as u16,
                    nesting_limit : nesting_limit - 1
                }
            }
        }
    }


    #[inline]
    pub unsafe fn read_text_pointer<'a>(mut segment : *SegmentReader<'a>,
                                      mut reff : *WirePointer,
                                      default_value : *Word,
                                      default_size : ByteCount
                                      ) -> Text::Reader<'a> {
        if (reff.is_null() || (*reff).is_null()) {
            return Text::new_reader(std::cast::transmute(default_value), default_size);
        }

        let refTarget = (*reff).target();

        let ptr : *Word = follow_fars(&mut reff, refTarget, &mut segment);

        let list_ref = (*reff).list_ref();

        let size : uint = list_ref.element_count();

        assert!((*reff).kind() == WP_LIST,
                "Message contains non-list pointer where text was expected");

        assert!(list_ref.element_size() == BYTE);

        assert!(bounds_check(segment, ptr,
                            ptr.offset(round_bytes_up_to_words(size) as int)));

        assert!(size > 0, "Message contains text that is not NUL-terminated");

        let str_ptr = std::cast::transmute::<*Word,*u8>(ptr);

        assert!((*str_ptr.offset((size - 1) as int)) == 0u8,
                "Message contains text that is not NUL-terminated");

        Text::new_reader(str_ptr, size-1)
    }

    #[inline]
    pub unsafe fn read_data_pointer<'a>(mut segment : *SegmentReader<'a>,
                                        mut reff : *WirePointer,
                                        default_value : *Word,
                                        default_size : ByteCount
                                        ) -> Data::Reader<'a> {
        if (reff.is_null() || (*reff).is_null()) {
            return Data::new_reader(std::cast::transmute(default_value), default_size);
        }

        let refTarget = (*reff).target();

        let ptr : *Word = follow_fars(&mut reff, refTarget, &mut segment);

        let list_ref = (*reff).list_ref();

        let size : uint = list_ref.element_count();

        assert!((*reff).kind() == WP_LIST,
                "Message contains non-list pointer where text was expected");

        assert!(list_ref.element_size() == BYTE,
                "Message contains list pointer of non-bytes where data was expected");

        assert!(bounds_check(segment, ptr,
                             ptr.offset(round_bytes_up_to_words(size) as int)),
                "Message contains out-of-bounds data pointer.");

        Data::new_reader(std::cast::transmute(ptr), size)
    }


}

static zero : u64 = 0;
fn zero_pointer() -> *WirePointer { unsafe {std::cast::transmute(std::ptr::to_unsafe_ptr(&zero))}}

pub struct PointerReader<'a> {
    segment : *SegmentReader<'a>,
    pointer : *WirePointer,
    nesting_limit : int
}

impl <'a> PointerReader<'a> {
    pub fn new_default<'b>() -> PointerReader<'b> {
        PointerReader { segment : std::ptr::null(),
                        pointer : std::ptr::null(),
                        nesting_limit : 0x7fffffff }
    }

    pub fn get_root<'b>(segment : *SegmentReader<'b>, location : *Word,
                        nesting_limit : int) -> PointerReader<'b> {
        unsafe {
            assert!(WireHelpers::bounds_check(segment, location,
                                              location.offset(POINTER_SIZE_IN_WORDS as int)),
                    "Root location out of bounds.");

            PointerReader { segment : segment,
                            pointer : std::cast::transmute(location),
                            nesting_limit : nesting_limit }
        }
    }

    pub fn get_root_unchecked<'b>(location : *Word) -> PointerReader<'b> {
        PointerReader { segment : std::ptr::null(),
                        pointer : unsafe { std::cast::transmute(location) },
                        nesting_limit : 0x7fffffff }
    }

    pub fn is_null(&self) -> bool {
        self.pointer.is_null() || unsafe { (*self.pointer).is_null() }
    }

    pub fn get_struct(&self, default_value: *Word) -> StructReader<'a> {
        let reff : *WirePointer = if self.pointer.is_null() { zero_pointer() } else { self.pointer };
        unsafe {
            WireHelpers::read_struct_pointer(self.segment, reff,
                                             default_value, self.nesting_limit)
        }
    }

    pub fn get_list(&self, expected_element_size : FieldSize, default_value : *Word) -> ListReader<'a> {
        let reff = if self.pointer.is_null() { zero_pointer() } else { self.pointer };
        unsafe {
            WireHelpers::read_list_pointer(self.segment,
                                           reff,
                                           default_value,
                                           expected_element_size, self.nesting_limit)
        }
    }

    pub fn get_text(&self, default_value : *Word, default_size : ByteCount) -> Text::Reader<'a> {
        unsafe {
            WireHelpers::read_text_pointer(self.segment, self.pointer, default_value, default_size)
        }
    }

    pub fn get_data(&self, default_value : *Word, default_size : ByteCount) -> Data::Reader<'a> {
        unsafe {
            WireHelpers::read_data_pointer(self.segment, self.pointer, default_value, default_size)
        }
    }

    pub fn total_size(&self) -> MessageSize {
        unsafe {
            WireHelpers::total_size(self.segment, self.pointer, self.nesting_limit)
        }
    }
}

pub struct PointerBuilder<'a> {
    segment : *mut SegmentBuilder<'a>,
    pointer : *mut WirePointer
}

impl <'a> PointerBuilder<'a> {

    #[inline]
    pub fn get_root(segment : *mut SegmentBuilder<'a>, location : *mut Word) -> PointerBuilder<'a> {
        PointerBuilder {segment : segment, pointer : unsafe { std::cast::transmute(location) }}
    }

    pub fn is_null(&self) -> bool {
        unsafe { (*self.pointer).is_null() }
    }


    pub fn get_struct(&self, size : StructSize, default_value : *Word) -> StructBuilder<'a> {
        unsafe {
            WireHelpers::get_writable_struct_pointer(
                self.pointer,
                self.segment,
                size,
                default_value)
        }
    }

    pub fn get_list(&self, element_size : FieldSize, default_value : *Word) -> ListBuilder<'a> {
        unsafe {
            WireHelpers::get_writable_list_pointer(
                self.pointer, self.segment, element_size, default_value)
        }
    }

    pub fn get_struct_list(&self, element_size : StructSize, default_value : *Word) -> ListBuilder<'a> {
        unsafe {
            WireHelpers::get_writable_struct_list_pointer(
                self.pointer, self.segment, element_size, default_value)
        }
    }

    pub fn get_text(&self, default_value : *Word, default_size : ByteCount) -> Text::Builder<'a> {
        unsafe {
            WireHelpers::get_writable_text_pointer(
                self.pointer, self.segment, default_value, default_size)
        }
    }

    pub fn get_data(&self, default_value : *Word, default_size : ByteCount) -> Data::Builder<'a> {
        unsafe {
            WireHelpers::get_writable_data_pointer(
                self.pointer, self.segment, default_value, default_size)
        }
    }

    pub fn init_struct(&self, size : StructSize) -> StructBuilder<'a> {
        unsafe {
            WireHelpers::init_struct_pointer(self.pointer, self.segment, size)
        }
    }

    pub fn init_list(&self, element_size : FieldSize, element_count : ElementCount) -> ListBuilder<'a> {
        unsafe {
            WireHelpers::init_list_pointer(
                self.pointer, self.segment, element_count, element_size)
        }
    }

    pub fn init_struct_list(&self, element_count : ElementCount, element_size : StructSize)
                            -> ListBuilder<'a> {
        unsafe {
            WireHelpers::init_struct_list_pointer(
                self.pointer, self.segment, element_count, element_size)
        }
    }

    pub fn init_text(&self, size : ByteCount) -> Text::Builder<'a> {
        unsafe {
            WireHelpers::init_text_pointer(self.pointer, self.segment, size).value
        }
    }

    pub fn init_data(&self, size : ByteCount) -> Data::Builder<'a> {
        unsafe {
            WireHelpers::init_data_pointer(self.pointer, self.segment, size).value
        }
    }

    pub fn set_struct(&self, value : &StructReader<'a>) {
        unsafe {
            WireHelpers::set_struct_pointer(self.segment, self.pointer, *value);
        }
    }

    pub fn set_list(&self, value : &ListReader<'a>) {
        unsafe {
            WireHelpers::set_list_pointer(self.segment, self.pointer, *value);
        }
    }

    pub fn set_text(&self, value : &str) {
        unsafe {
            WireHelpers::set_text_pointer(self.pointer, self.segment, value)
        }
    }

    pub fn set_data(&self, value : &[u8]) {
        unsafe {
            WireHelpers::set_data_pointer(self.pointer, self.segment, value)
        }
    }


    pub fn clear(&self) {
        unsafe {
            WireHelpers::zero_object(self.segment, self.pointer);
            std::ptr::zero_memory(self.pointer, 1);
        }
    }
}

pub trait FromStructReader<'a> {
    fn from_struct_reader(reader : StructReader<'a>) -> Self;
}

pub struct StructReader<'a> {
    segment : *SegmentReader<'a>,
    data : *u8,
    pointers : *WirePointer,
    data_size : BitCount32,
    pointer_count : WirePointerCount16,
    bit0offset : BitCount8,
    nesting_limit : int
}

impl <'a> StructReader<'a>  {

    pub fn new_default() -> StructReader {
        StructReader { segment : std::ptr::null(),
                       data : std::ptr::null(),
                       pointers : std::ptr::null(), data_size : 0, pointer_count : 0,
                       bit0offset : 0, nesting_limit : 0x7fffffff}
    }

    pub fn get_data_section_size(&self) -> BitCount32 { self.data_size }

    pub fn get_pointer_section_size(&self) -> WirePointerCount16 { self.pointer_count }

    pub fn get_data_section_as_blob(&self) -> uint { fail!("unimplemented") }

    #[inline]
    pub fn get_data_field<T:Clone + std::num::Zero>(&self, offset : ElementCount) -> T {
        // We need to check the offset because the struct may have
        // been created with an old version of the protocol that did
        // not contain the field.
        if ((offset + 1) * bits_per_element::<T>() <= self.data_size as uint) {
            unsafe {
                let dwv : *WireValue<T> = std::cast::transmute(self.data);
                (*dwv.offset(offset as int)).get()
            }
        } else {
            return std::num::Zero::zero()
        }
    }


    #[inline]
    pub fn get_bool_field(&self, offset : ElementCount) -> bool {
        let mut boffset : BitCount32 = offset as BitCount32;
        if (boffset < self.data_size) {
            if (offset == 0) {
                boffset = self.bit0offset as BitCount32;
            }
            unsafe {
                let b : *u8 = self.data.offset((boffset as uint / BITS_PER_BYTE) as int);
                ((*b) & (1 << (boffset % BITS_PER_BYTE as u32 ))) != 0
            }
        } else {
            false
        }
    }

    #[inline]
    pub fn get_data_field_mask<T:Clone + std::num::Zero + Mask>(&self,
                                                                offset : ElementCount,
                                                                mask : T) -> T {
        Mask::mask(self.get_data_field(offset), mask)
    }

    #[inline]
    pub fn get_bool_field_mask(&self,
                               offset : ElementCount,
                               mask : bool) -> bool {
       self.get_bool_field(offset) ^ mask
    }

    #[inline]
    pub fn get_pointer_field(&self, ptr_index : WirePointerCount) -> PointerReader<'a> {
        if (ptr_index < self.pointer_count as WirePointerCount) {
            PointerReader {
                segment : self.segment,
                pointer : unsafe { self.pointers.offset(ptr_index as int) },
                nesting_limit : self.nesting_limit
            }
        } else {
            PointerReader::new_default()
        }
    }

    pub fn total_size(&self) -> MessageSize {
        let mut result = MessageSize {
            word_count : WireHelpers::round_bits_up_to_words(self.data_size as u64) as u64 +
                self.pointer_count as u64 * WORDS_PER_POINTER as u64,
            cap_count : 0 };

        for i in range(0, self.pointer_count as int) {
            unsafe {
                result.plus_eq(WireHelpers::total_size(self.segment, self.pointers.offset(i),
                                                       self.nesting_limit));
            }
        }

        // TODO when we have read limiting: segment->unread()

        result
    }
}

pub trait HasStructSize {
    fn struct_size(unused_self : Option<Self>) -> StructSize;
}

pub trait FromStructBuilder<'a> {
    fn from_struct_builder(structBuilder : StructBuilder<'a>) -> Self;
}

pub struct StructBuilder<'a> {
    segment : *mut SegmentBuilder<'a>,
    data : *mut u8,
    pointers : *mut WirePointer,
    data_size : BitCount32,
    pointer_count : WirePointerCount16,
    bit0offset : BitCount8
}

impl <'a> StructBuilder<'a> {
    pub fn as_reader<T>(&self, f : |StructReader| -> T) -> T {
        unsafe {
            let segmentReader = &(*self.segment).reader;
            f ( StructReader {
                    segment : std::ptr::to_unsafe_ptr(segmentReader),
                    data : std::cast::transmute(self.data),
                    pointers : std::cast::transmute(self.pointers),
                    data_size : self.data_size,
                    pointer_count : self.pointer_count,
                    bit0offset : self.bit0offset,
                    nesting_limit : 0x7fffffff
                })
        }
    }

    #[inline]
    pub fn set_data_field<T:Clone>(&self, offset : ElementCount, value : T) {
        unsafe {
            let ptr : *mut WireValue<T> = std::cast::transmute(self.data);
            (*ptr.offset(offset as int)).set(value)
        }
    }

    #[inline]
    pub fn set_data_field_mask<T:Clone + std::num::Zero + Mask>(&self,
                                                                offset : ElementCount,
                                                                value : T,
                                                                mask : T) {
        self.set_data_field(offset, Mask::mask(value, mask));
    }

    #[inline]
    pub fn get_data_field<T:Clone>(&self, offset : ElementCount) -> T {
        unsafe {
            let ptr : *mut WireValue<T> = std::cast::transmute(self.data);
            (*ptr.offset(offset as int)).get()
        }
    }

    #[inline]
    pub fn get_data_field_mask<T:Clone + std::num::Zero + Mask>(&self,
                                                                offset : ElementCount,
                                                                mask : T) -> T {
        Mask::mask(self.get_data_field(offset), mask)
    }


    #[inline]
    pub fn set_bool_field(&self, offset : ElementCount, value : bool) {
        //# This branch should be compiled out whenever this is
        //# inlined with a constant offset.
        let boffset : BitCount0 = if (offset == 0) { self.bit0offset as uint } else { offset };
        let b = unsafe { self.data.offset((boffset / BITS_PER_BYTE) as int)};
        let bitnum = boffset % BITS_PER_BYTE;
        unsafe { (*b) = (( (*b) & !(1 << bitnum)) | (value as u8 << bitnum)) }
    }

    #[inline]
    pub fn set_bool_field_mask(&self,
                               offset : ElementCount,
                               value : bool,
                               mask : bool) {
       self.set_bool_field(offset , value ^ mask);
    }

    #[inline]
    pub fn get_bool_field(&self, offset : ElementCount) -> bool {
        let boffset : BitCount0 =
            if (offset == 0) {self.bit0offset as BitCount0} else {offset};
        let b = unsafe { self.data.offset((boffset / BITS_PER_BYTE) as int) };
        unsafe { ((*b) & (1 << (boffset % BITS_PER_BYTE ))) != 0 }
    }

    #[inline]
    pub fn get_bool_field_mask(&self,
                               offset : ElementCount,
                               mask : bool) -> bool {
       self.get_bool_field(offset) ^ mask
    }


    #[inline]
    pub fn get_pointer_field(&self, ptr_index : WirePointerCount) -> PointerBuilder<'a> {
        PointerBuilder {
                segment : self.segment,
                pointer : unsafe { self.pointers.offset(ptr_index as int) }
            }
    }

}

pub struct ListReader<'a> {
    segment : *SegmentReader<'a>,
    ptr : *u8,
    element_count : ElementCount,
    step : BitCount0,
    struct_data_size : BitCount32,
    struct_pointer_count : WirePointerCount16,
    nesting_limit : int
}

impl <'a> ListReader<'a> {

    pub fn new_default() -> ListReader {
        ListReader { segment : std::ptr::null(),
                    ptr : std::ptr::null(), element_count : 0, step: 0, struct_data_size : 0,
                    struct_pointer_count : 0, nesting_limit : 0x7fffffff}
    }

    #[inline]
    pub fn size(&self) -> ElementCount { self.element_count }

    pub fn get_struct_element(&self, index : ElementCount) -> StructReader<'a> {
        assert!(self.nesting_limit > 0,
                "Message is too deeply-nested or contains cycles");
        let indexBit : BitCount64 = index as ElementCount64 * (self.step as BitCount64);

        let structData : *u8 = unsafe {
            self.ptr.offset((indexBit as uint / BITS_PER_BYTE) as int) };

        let structPointers : *WirePointer = unsafe {
                std::cast::transmute(
                    structData.offset((self.struct_data_size as uint / BITS_PER_BYTE) as int))
        };

/*
        assert!(self.struct_pointer_count == 0 ||
                structPointers % BYTES_PER_POINTER == 0,
                "Pointer section of struct list element not aligned"
               );
*/
        StructReader {
            segment : self.segment,
            data : structData,
            pointers : structPointers,
            data_size : self.struct_data_size as BitCount32,
            pointer_count : self.struct_pointer_count,
            bit0offset : (indexBit % (BITS_PER_BYTE as u64)) as u8,
            nesting_limit : self.nesting_limit - 1
        }
    }

    #[inline]
    pub fn get_pointer_element(&self, index : ElementCount) -> PointerReader<'a> {
        PointerReader {
            segment : self.segment,
            pointer : unsafe {
                std::cast::transmute(self.ptr.offset((index * self.step / BITS_PER_BYTE) as int))
            },
            nesting_limit : self.nesting_limit
        }
    }
}


pub struct ListBuilder<'a> {
    segment : *mut SegmentBuilder<'a>,
    ptr : *mut u8,
    element_count : ElementCount,
    step : BitCount0,
    struct_data_size : BitCount32,
    struct_pointer_count : WirePointerCount16
}

impl <'a> ListBuilder<'a> {

    #[inline]
    pub fn new_default<'a>() -> ListBuilder<'a> {
        ListBuilder {
            segment : std::ptr::mut_null(), ptr : std::ptr::mut_null(), element_count : 0,
            step : 0, struct_data_size : 0, struct_pointer_count : 0
        }
    }

    #[inline]
    pub fn size(&self) -> ElementCount { self.element_count }

    pub fn get_struct_element(&self, index : ElementCount) -> StructBuilder<'a> {
        let indexBit = index * self.step;
        let structData = unsafe{ self.ptr.offset((indexBit / BITS_PER_BYTE) as int)};
        let structPointers = unsafe {
            std::cast::transmute(
                structData.offset(((self.struct_data_size as uint) / BITS_PER_BYTE) as int))
        };
        StructBuilder {
            segment : self.segment,
            data : structData,
            pointers : structPointers,
            data_size : self.struct_data_size,
            pointer_count : self.struct_pointer_count,
            bit0offset : (indexBit % BITS_PER_BYTE) as u8
        }
    }

    #[inline]
    pub fn get_pointer_element(&self, index : ElementCount) -> PointerBuilder<'a> {
        PointerBuilder {
            segment : self.segment,
            pointer : unsafe {
                std::cast::transmute(self.ptr.offset((index * self.step / BITS_PER_BYTE) as int))
            }
        }
    }
}


pub trait PrimitiveElement : Clone {
    #[inline]
    fn get(listReader : &ListReader, index : ElementCount) -> Self {
        unsafe {
            let ptr : *u8 =
                listReader.ptr.offset(
                                 (index * listReader.step / BITS_PER_BYTE) as int);
            (*std::cast::transmute::<*u8,*WireValue<Self>>(ptr)).get()
        }
    }

    #[inline]
    fn get_from_builder(listBuilder : &ListBuilder, index : ElementCount) -> Self {
        unsafe {
            let ptr : *mut WireValue<Self> =
                std::cast::transmute(
                listBuilder.ptr.offset(
                                     (index * listBuilder.step / BITS_PER_BYTE) as int));
            (*ptr).get()
        }
    }

    #[inline]
    fn set(listBuilder : &ListBuilder, index : ElementCount, value: Self) {
        unsafe {
            let ptr : *mut WireValue<Self> =
                std::cast::transmute(
                listBuilder.ptr.offset(
                    (index * listBuilder.step / BITS_PER_BYTE) as int));
            (*ptr).set(value);
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
    fn get(list : &ListReader, index : ElementCount) -> bool {
        //# Ignore stepBytes for bit lists because bit lists cannot be
        //# upgraded to struct lists.
        let bindex : BitCount0 = index * list.step;
        unsafe {
            let b : *u8 = list.ptr.offset((bindex / BITS_PER_BYTE) as int);
            ((*b) & (1 << (bindex % BITS_PER_BYTE))) != 0
        }
    }
    #[inline]
    fn get_from_builder(list : &ListBuilder, index : ElementCount) -> bool {
        //# Ignore stepBytes for bit lists because bit lists cannot be
        //# upgraded to struct lists.
        let bindex : BitCount0 = index * list.step;
        let b = unsafe { list.ptr.offset((bindex / BITS_PER_BYTE) as int) };
        unsafe { ((*b) & (1 << (bindex % BITS_PER_BYTE ))) != 0 }
    }
    #[inline]
    fn set(list : &ListBuilder, index : ElementCount, value : bool) {
        //# Ignore stepBytes for bit lists because bit lists cannot be
        //# upgraded to struct lists.
        let bindex : BitCount0 = index;
        let b = unsafe { list.ptr.offset((bindex / BITS_PER_BYTE) as int) };

        let bitnum = bindex % BITS_PER_BYTE;
        unsafe { (*b) = (( (*b) & !(1 << bitnum)) | (value as u8 << bitnum)) }
    }
}

impl PrimitiveElement for () {
    #[inline]
    fn get(_list : &ListReader, _index : ElementCount) -> () { () }

    #[inline]
    fn get_from_builder(_list : &ListBuilder, _index : ElementCount) -> () { () }

    #[inline]
    fn set(_list : &ListBuilder, _index : ElementCount, _value : ()) { }
}

