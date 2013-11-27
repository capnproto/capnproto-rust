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

pub fn dataBitsPerElement(size : FieldSize) -> BitCount0 {
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

pub fn pointersPerElement(size : FieldSize) -> WirePointerCount {
    match size {
        POINTER => 1,
        _ => 0
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

pub struct StructSize {
    data : WordCount16,
    pointers : WirePointerCount16,
    preferredListEncoding : FieldSize
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
    WP_CAPABILITY = 3
}


pub struct WirePointer {
    offsetAndKind : WireValue<u32>,
    upper32Bits : u32,
}

pub struct StructRef {
    dataSize : WireValue<WordCount16>,
    ptrCount : WireValue<WirePointerCount16>
}

pub struct ListRef {
    elementSizeAndCount : WireValue<u32>
}

pub struct FarRef {
    segmentId : WireValue<u32>
}

impl StructRef {
    pub fn wordSize(&self) -> WordCount {
        self.dataSize.get() as WordCount +
            self.ptrCount.get() as WordCount * WORDS_PER_POINTER
    }

    #[inline]
    pub fn set(&mut self, size : StructSize) {
        self.dataSize.set(size.data);
        self.ptrCount.set(size.pointers);
    }
}

impl ListRef {
    #[inline]
    pub fn elementSize(&self) -> FieldSize {
        unsafe {
            std::cast::transmute( (self.elementSizeAndCount.get() & 7) as u8)
        }
    }

    #[inline]
    pub fn elementCount(&self) -> ElementCount {
        (self.elementSizeAndCount.get() >> 3) as uint
    }

    #[inline]
    pub fn inlineCompositeWordCount(&self) -> WordCount {
        self.elementCount()
    }

    #[inline]
    pub fn set(&mut self, es : FieldSize, ec : ElementCount) {
        assert!(ec < (1 << 29), "Lists are limited to 2**29 elements");
        self.elementSizeAndCount.set(((ec as u32) << 3 ) | (es as u32));
    }

    #[inline]
    pub fn setInlineComposite(& mut self, wc : WordCount) {
        assert!(wc < (1 << 29), "Inline composite lists are limited to 2 ** 29 words");
        self.elementSizeAndCount.set((( wc as u32) << 3) | (INLINE_COMPOSITE as u32));
    }

}

impl WirePointer {

    #[inline]
    pub fn kind(&self) -> WirePointerKind {
        unsafe {
            std::cast::transmute((self.offsetAndKind.get() & 3) as u8)
        }
    }

    #[inline]
    pub fn target(&self) -> *Word {
        let thisAddr : *Word = unsafe {std::cast::transmute(&*self) };
        unsafe { thisAddr.offset(1 + ((self.offsetAndKind.get() as int) >> 2)) }
    }

    #[inline]
    pub fn mut_target(&mut self) -> *mut Word {
        let thisAddr : *mut Word = unsafe {std::cast::transmute(&*self) };
        unsafe { thisAddr.offset(1 + ((self.offsetAndKind.get() as int) >> 2)) }
    }

    #[inline]
    pub fn setKindAndTarget(&mut self, kind : WirePointerKind,
                            target : *mut Word, _segmentBuilder : *mut SegmentBuilder) {
        let thisAddr : int = unsafe {std::cast::transmute(&*self)};
        let targetAddr : int = unsafe {std::cast::transmute(target)};
        self.offsetAndKind.set(
            ((((targetAddr - thisAddr)/BYTES_PER_WORD as int) as i32 - 1) << 2) as u32
                | (kind as u32))
    }

    #[inline]
    pub fn setKindWithZeroOffset(&mut self, kind : WirePointerKind) {
        self.offsetAndKind.set( kind as u32)
    }

    #[inline]
    pub fn inlineCompositeListElementCount(&self) -> ElementCount {
        (self.offsetAndKind.get() >> 2) as ElementCount
    }

    #[inline]
    pub fn setKindAndInlineCompositeListElementCount(
        &mut self, kind : WirePointerKind, elementCount : ElementCount) {
        self.offsetAndKind.set((( elementCount as u32 << 2) | (kind as u32)))
    }


    #[inline]
    pub fn farPositionInSegment(&self) -> WordCount {
        (self.offsetAndKind.get() >> 3) as WordCount
    }

    #[inline]
    pub fn isDoubleFar(&self) -> bool {
        ((self.offsetAndKind.get() >> 2) & 1) != 0
    }

    #[inline]
    pub fn setFar(&mut self, isDoubleFar : bool, pos : WordCount) {
        self.offsetAndKind.set
            (( pos << 3) as u32 | (isDoubleFar as u32 << 2) | WP_FAR as u32);
    }

    #[inline]
    pub fn structRef(&self) -> StructRef {
        unsafe { std::cast::transmute(self.upper32Bits) }
    }

    #[inline]
    pub fn structRefMut<'a>(&'a mut self) -> &'a mut StructRef {
        unsafe { std::cast::transmute(& self.upper32Bits) }
    }

    #[inline]
    pub fn listRef(&self) -> ListRef {
        unsafe { std::cast::transmute(self.upper32Bits) }
    }

    #[inline]
    pub fn listRefMut<'a>(&'a self) -> &'a mut ListRef {
        unsafe { std::cast::transmute(& self.upper32Bits) }
    }

    #[inline]
    pub fn farRef(&self) -> FarRef {
        unsafe { std::cast::transmute(self.upper32Bits) }
    }

    #[inline]
    pub fn farRefMut<'a>(&'a mut self) -> &'a mut FarRef {
        unsafe { std::cast::transmute(& self.upper32Bits) }
    }

    #[inline]
    pub fn isNull(&self) -> bool {
        (self.offsetAndKind.get() == 0) & (self.upper32Bits == 0)
    }
}

mod WireHelpers {
    use std;
    use common::*;
    use layout::*;
    use arena::*;
    use blob::*;

    #[inline]
    pub fn roundBytesUpToWords(bytes : ByteCount) -> WordCount {
        //# This code assumes 64-bit words.
        (bytes + 7) / BYTES_PER_WORD
    }

    //# The maximum object size is 4GB - 1 byte. If measured in bits,
    //# this would overflow a 32-bit counter, so we need to accept
    //# BitCount64. However, 32 bits is enough for the returned
    //# ByteCounts and WordCounts.
    #[inline]
    pub fn roundBitsUpToWords(bits : BitCount64) -> WordCount {
        //# This code assumes 64-bit words.
        ((bits + 63) / (BITS_PER_WORD as u64)) as WordCount
    }

    #[inline]
    pub fn roundBitsUpToBytes(bits : BitCount64) -> ByteCount {
        ((bits + 7) / (BITS_PER_BYTE as u64)) as ByteCount
    }

    #[inline]
    pub unsafe fn boundsCheck<'a>(segment : *SegmentReader<'a>,
                                  start : *Word, end : *Word) -> bool {
        //# If segment is null, this is an unchecked message, so we don't do bounds checks.
        return segment.is_null() || (*segment).contains_interval(start, end);
    }

    #[inline]
    pub unsafe fn allocate(reff : &mut *mut WirePointer,
                           segment : &mut *mut SegmentBuilder,
                           amount : WordCount, kind : WirePointerKind) -> *mut Word {
        let isNull = (**reff).isNull();
        if (!isNull) {
            zeroObject(*segment, *reff)
        }
        match (**segment).allocate(amount) {
            None => {

                //# Need to allocate in a new segment. We'll need to
                //# allocate an extra pointer worth of space to act as
                //# the landing pad for a far pointer.

                let amountPlusRef = amount + POINTER_SIZE_IN_WORDS;
                *segment = (*(**segment).messageBuilder).get_segment_with_available(amountPlusRef);
                let ptr : *mut Word = (**segment).allocate(amountPlusRef).unwrap();

                //# Set up the original pointer to be a far pointer to
                //# the new segment.
                (**reff).setFar(false, (**segment).get_word_offset_to(ptr));
                (**reff).farRefMut().segmentId.set((**segment).id);

                //# Initialize the landing pad to indicate that the
                //# data immediately follows the pad.
                *reff = std::cast::transmute(ptr);

                let ptr1 = ptr.offset(POINTER_SIZE_IN_WORDS as int);
                (**reff).setKindAndTarget(kind, ptr1, *segment);
                return ptr1;
            }
            Some(ptr) => {
                (**reff).setKindAndTarget(kind, ptr, *segment);
                return ptr;
            }
        }
    }

    #[inline]
    pub unsafe fn followFars<'a>(reff: &mut *WirePointer,
                                 refTarget: *Word,
                                 segment : &mut *SegmentReader<'a>) -> *Word {

        //# If the segment is null, this is an unchecked message,
        //# so there are no FAR pointers.
        if !(*segment).is_null() && (**reff).kind() == WP_FAR {
            *segment =
                (*(**segment).messageReader).get_segment_reader((**reff).farRef().segmentId.get());

            let ptr : *Word = (**segment).get_start_ptr().offset(
                (**reff).farPositionInSegment() as int);

            let padWords : int = if ((**reff).isDoubleFar()) { 2 } else { 1 };
            assert!(boundsCheck(*segment, ptr, ptr.offset(padWords)));

            let pad : *WirePointer = std::cast::transmute(ptr);

            if (!(**reff).isDoubleFar() ) {
                *reff = pad;
                return (*pad).target();
            } else {
                //# Landing pad is another far pointer. It is
                //# followed by a tag describing the pointed-to
                //# object.

                *reff = pad.offset(1);

                *segment =
                    (*(**segment).messageReader).get_segment_reader((*pad).farRef().segmentId.get());

                return (**segment).get_start_ptr().offset((*pad).farPositionInSegment() as int);
            }
        } else {
            return refTarget;
        }
    }

    pub unsafe fn zeroObject(mut segment : *mut SegmentBuilder, reff : *mut WirePointer) {
        //# Zero out the pointed-to object. Use when the pointer is
        //# about to be overwritten making the target object no longer
        //# reachable.

        match (*reff).kind() {
            WP_STRUCT | WP_LIST | WP_CAPABILITY => {
                zeroObjectHelper(segment,
                                 reff, (*reff).mut_target())
            }
            WP_FAR => {
                segment = std::ptr::to_mut_unsafe_ptr(
                    (*(*segment).messageBuilder).segment_builders[(*reff).farRef().segmentId.get()]);
                let pad : *mut WirePointer =
                    std::cast::transmute((*segment).get_ptr_unchecked((*reff).farPositionInSegment()));

                if ((*reff).isDoubleFar()) {
                    segment = std::ptr::to_mut_unsafe_ptr(
                        (*(*segment).messageBuilder).segment_builders[(*pad).farRef().segmentId.get()]);

                    zeroObjectHelper(segment,
                                     pad.offset(1),
                                     (*segment).get_ptr_unchecked((*pad).farPositionInSegment()));

                    std::ptr::set_memory(pad, 0u8, 2);

                } else {
                    zeroObject(segment, pad);
                    std::ptr::set_memory(pad, 0u8, 1);
                }
            }
        }
    }

    pub unsafe fn zeroObjectHelper(segment : *mut SegmentBuilder,
                                   tag : *mut WirePointer,
                                   ptr: *mut Word) {
        match (*tag).kind() {
            WP_CAPABILITY => { fail!("Don't know how to handle CAPABILITY") }
            WP_STRUCT => {
                let pointerSection : *mut WirePointer =
                    std::cast::transmute(
                    ptr.offset((*tag).structRef().dataSize.get() as int));

                let count = (*tag).structRef().ptrCount.get() as int;
                for i in range::<int>(0, count) {
                    zeroObject(segment, pointerSection.offset(i));
                }
                std::ptr::set_memory(ptr, 0u8, (*tag).structRef().wordSize());
            }
            WP_LIST => {
                match (*tag).listRef().elementSize() {
                    VOID =>  { }
                    BIT | BYTE | TWO_BYTES | FOUR_BYTES | EIGHT_BYTES => {
                        std::ptr::set_memory(
                            ptr, 0u8,
                            roundBitsUpToWords((
                                    (*tag).listRef().elementCount() *
                                        dataBitsPerElement(
                                        (*tag).listRef().elementSize())) as u64))
                    }
                    POINTER => {
                        let count = (*tag).listRef().elementCount() as uint;
                        for i in range::<int>(0, count as int) {
                            zeroObject(segment,
                                       std::cast::transmute(ptr.offset(i)))
                        }
                        std::ptr::set_memory(ptr, 0u8, count);
                    }
                    INLINE_COMPOSITE => {
                        let elementTag : *mut WirePointer = std::cast::transmute(ptr);

                        assert!((*elementTag).kind() == WP_STRUCT,
                                "Don't know how to handle non-STRUCT inline composite");

                        let dataSize = (*elementTag).structRef().dataSize.get();
                        let pointerCount = (*elementTag).structRef().ptrCount.get();
                        let mut pos : *mut Word = ptr.offset(1);
                        let count = (*elementTag).inlineCompositeListElementCount();
                        for _ in range(0, count) {
                            pos = pos.offset(dataSize as int);
                            for _ in range(0, pointerCount as uint) {
                                zeroObject(
                                    segment,
                                    std::cast::transmute::<*mut Word, *mut WirePointer>(pos));
                                pos = pos.offset(1);
                            }
                        }
                        std::ptr::set_memory(ptr, 0u8,
                                             (*elementTag).structRef().wordSize() * count + 1);
                    }
                }
            }
            WP_FAR => { fail!("Unexpected FAR pointer") }
        }
    }

    #[inline]
    pub unsafe fn initStructPointer(mut reff : *mut WirePointer,
                             mut segmentBuilder : *mut SegmentBuilder,
                             size : StructSize) -> StructBuilder {
        let ptr : *mut Word = allocate(&mut reff, &mut segmentBuilder, size.total(), WP_STRUCT);
        (*reff).structRefMut().set(size);

        StructBuilder {
            segment : segmentBuilder,
            data : std::cast::transmute(ptr),
            pointers : std::cast::transmute(
                    ptr.offset((size.data as uint) as int)),
            dataSize : size.data as WordCount32 * (BITS_PER_WORD as BitCount32),
            pointerCount : size.pointers,
            bit0Offset : 0
        }
    }

    #[inline]
    pub unsafe fn getWritableStructPointer(_reff : *mut WirePointer,
                                    _segment : *mut SegmentBuilder,
                                    _size : StructSize,
                                    _defaultValue : *Word) -> StructBuilder {
        fail!("unimplemented")
    }

    #[inline]
    pub unsafe fn initListPointer(mut reff : *mut WirePointer,
                           mut segmentBuilder : *mut SegmentBuilder,
                           elementCount : ElementCount,
                           elementSize : FieldSize) -> ListBuilder {
        match elementSize {
            INLINE_COMPOSITE => {
                fail!("Should have called initStructListPointer() instead")
            }
            _ => { }
        }

        let dataSize : BitCount0 = dataBitsPerElement(elementSize);
        let pointerCount = pointersPerElement(elementSize);
        let step = (dataSize + pointerCount * BITS_PER_POINTER);
        let wordCount = roundBitsUpToWords(elementCount as ElementCount64 * (step as u64));
        let ptr = allocate(&mut reff, &mut segmentBuilder, wordCount, WP_LIST);

        (*reff).listRefMut().set(elementSize, elementCount);

        ListBuilder {
            segment : segmentBuilder,
            ptr : std::cast::transmute(ptr),
            step : step,
            elementCount : elementCount,
            structDataSize : dataSize as u32,
            structPointerCount : pointerCount as u16
        }
    }

    #[inline]
    pub unsafe fn initStructListPointer(mut reff : *mut WirePointer,
                                        mut segmentBuilder : *mut SegmentBuilder,
                                        elementCount : ElementCount,
                                        elementSize : StructSize) -> ListBuilder {
        match elementSize.preferredListEncoding {
            INLINE_COMPOSITE => { }
            otherEncoding => {
                return initListPointer(reff, segmentBuilder, elementCount, otherEncoding);
            }
        }

        let wordsPerElement = elementSize.total();

        //# Allocate the list, prefixed by a single WirePointer.
        let wordCount : WordCount = elementCount * wordsPerElement;
        let ptr : *mut WirePointer =
            std::cast::transmute(allocate(&mut reff, &mut segmentBuilder,
                                          POINTER_SIZE_IN_WORDS + wordCount, WP_LIST));

        //# Initalize the pointer.
        (*reff).listRefMut().setInlineComposite(wordCount);
        (*ptr).setKindAndInlineCompositeListElementCount(WP_STRUCT, elementCount);
        (*ptr).structRefMut().set(elementSize);

        let ptr1 = ptr.offset(POINTER_SIZE_IN_WORDS as int);

        ListBuilder {
            segment : segmentBuilder,
            ptr : std::cast::transmute(ptr1),
            step : wordsPerElement * BITS_PER_WORD,
            elementCount : elementCount,
            structDataSize : elementSize.data as u32 * (BITS_PER_WORD as u32),
            structPointerCount : elementSize.pointers
        }
    }

    #[inline]
    pub unsafe fn getWritableListPointer(_origRefIndex : *mut WirePointer,
                                         _origSegment : *mut SegmentBuilder,
                                         _elementSize : FieldSize,
                                         _defaultValue : *Word) -> ListBuilder {
        fail!("unimplemented")
    }

    #[inline]
    pub unsafe fn getWritableStructListPointer(_origRefIndex : *mut WirePointer,
                                               _origSegment : *mut SegmentBuilder,
                                               _elementSize : StructSize,
                                               _defaultValue : *Word) -> ListBuilder {
        fail!("unimplemented")
    }

    #[inline]
    pub unsafe fn setTextPointer(mut reff : *mut WirePointer,
                          mut segmentBuilder : *mut SegmentBuilder,
                          value : &str) {

        // initTextPointer is rolled in here

        let bytes : &[u8] = value.as_bytes();

        //# The byte list must include a NUL terminator
        let byteSize = bytes.len() + 1;

        let ptr =
            allocate(&mut reff, &mut segmentBuilder, roundBytesUpToWords(byteSize), WP_LIST);

        (*reff).listRefMut().set(BYTE, byteSize);
        let dst : *mut u8 = std::cast::transmute(ptr);
        let src : *u8 = bytes.unsafe_ref(0);
        std::ptr::copy_nonoverlapping_memory(dst, src, bytes.len());

        // null terminate
        std::ptr::zero_memory(dst.offset(bytes.len() as int), 1);
    }

    #[inline]
    pub unsafe fn getWritableTextPointer(_refIndex : *mut WirePointer,
                                         _segment : *mut SegmentBuilder,
                                         _defaultValue : &'static str) -> Text::Builder {
        fail!("unimplemented");
    }

    #[inline]
    pub unsafe fn readStructPointer<'a>(mut segment: *SegmentReader<'a>,
                                        mut reff : *WirePointer,
                                        defaultValue : *Word,
                                        nestingLimit : int) -> StructReader<'a> {

        if ((*reff).isNull()) {
            if (defaultValue.is_null() ||
                (*std::cast::transmute::<*Word,*WirePointer>(defaultValue)).isNull()) {
                    return StructReader::newDefault();
            }

            //segment = std::ptr::null();
            //reff = std::cast::transmute::<*Word,*WirePointer>(defaultValue);
            fail!("default struct values unimplemented");
        }

        let refTarget : *Word = (*reff).target();

        assert!(nestingLimit > 0, "Message is too deeply-nested or contains cycles.");

        let ptr = followFars(&mut reff, refTarget, &mut segment);

        let dataSizeWords = (*reff).structRef().dataSize.get();

        assert!(boundsCheck(segment, ptr,
                            ptr.offset((*reff).structRef().wordSize() as int)),
                "Message contained out-of-bounds struct pointer.");

        StructReader {segment : segment,
                      data : std::cast::transmute(ptr),
                      pointers : std::cast::transmute(ptr.offset(dataSizeWords as int)),
                      dataSize : dataSizeWords as u32 * BITS_PER_WORD as BitCount32,
                      pointerCount : (*reff).structRef().ptrCount.get(),
                      bit0Offset : 0,
                      nestingLimit : nestingLimit - 1 }
     }

    #[inline]
    pub unsafe fn readListPointer<'a>(mut segment: *SegmentReader<'a>,
                                      mut reff : *WirePointer,
                                      defaultValue : *Word,
                                      expectedElementSize : FieldSize,
                                      nestingLimit : int ) -> ListReader<'a> {

        if ((*reff).isNull()) {
            if defaultValue.is_null() ||
                (*std::cast::transmute::<*Word,*WirePointer>(defaultValue)).isNull() {
                return ListReader::newDefault();
            }
            fail!("list default values unimplemented");
        }

        let refTarget : *Word = (*reff).target();

        if (nestingLimit <= 0) {
           fail!("nesting limit exceeded");
        }

        let mut ptr : *Word = followFars(&mut reff, refTarget, &mut segment);

        assert!((*reff).kind() == WP_LIST,
                "Message contains non-list pointer where list pointer was expected {:?}", reff);

        let listRef = (*reff).listRef();

        match listRef.elementSize() {
            INLINE_COMPOSITE => {
                let wordCount = listRef.inlineCompositeWordCount();

                let tag: *WirePointer = std::cast::transmute(ptr);

                ptr = ptr.offset(1);

                assert!(boundsCheck(segment, ptr.offset(-1),
                                    ptr.offset(wordCount as int)));

                assert!((*tag).kind() == WP_STRUCT,
                        "INLINE_COMPOSITE lists of non-STRUCT type are not supported");

                let size = (*tag).inlineCompositeListElementCount();
                let structRef = (*tag).structRef();
                let wordsPerElement = structRef.wordSize();

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
                        assert!(structRef.dataSize.get() > 0,
                               "Expected a primitive list, but got a list of pointer-only structs")
                    }
                    POINTER => {
                        ptr = ptr.offset(structRef.dataSize.get() as int);
                        assert!(structRef.ptrCount.get() > 0,
                               "Expected a pointer list, but got a list of data-only structs")
                    }
                    INLINE_COMPOSITE => {}
                }

                ListReader {
                    segment : segment,
                    ptr : std::cast::transmute(ptr),
                    elementCount : size,
                    step : wordsPerElement * BITS_PER_WORD,
                    structDataSize : structRef.dataSize.get() as u32 * (BITS_PER_WORD as u32),
                    structPointerCount : structRef.ptrCount.get() as u16,
                    nestingLimit : nestingLimit - 1
                }
            }
            _ => {

                //# This is a primitive or pointer list, but all such
                //# lists can also be interpreted as struct lists. We
                //# need to compute the data size and pointer count for
                //# such structs.
                let dataSize = dataBitsPerElement(listRef.elementSize());
                let pointerCount = pointersPerElement(listRef.elementSize());
                let step = dataSize + pointerCount * BITS_PER_POINTER;

                assert!(
                    boundsCheck(
                        segment, ptr,
                        ptr.offset(
                            roundBitsUpToWords(
                                (listRef.elementCount() * step) as u64) as int)));

                //# Verify that the elements are at least as large as
                //# the expected type. Note that if we expected
                //# INLINE_COMPOSITE, the expected sizes here will be
                //# zero, because bounds checking will be performed at
                //# field access time. So this check here is for the
                //# case where we expected a list of some primitive or
                //# pointer type.

                let expectedDataBitsPerElement =
                        dataBitsPerElement(expectedElementSize);
                let expectedPointersPerElement =
                    pointersPerElement(expectedElementSize);

                assert!(expectedDataBitsPerElement <= dataSize);
                assert!(expectedPointersPerElement <= pointerCount);

                ListReader {
                    segment : segment,
                    ptr : std::cast::transmute(ptr),
                    elementCount : listRef.elementCount(),
                    step : step,
                    structDataSize : dataSize as u32,
                    structPointerCount : pointerCount as u16,
                    nestingLimit : nestingLimit - 1
                }
            }
        }

    }

    #[inline]
    pub unsafe fn readTextPointer<'a>(mut segment : *SegmentReader<'a>,
                                      mut reff : *WirePointer,
                                      defaultValue : &'a str
                                      //defaultSize : ByteCount
                                      ) -> Text::Reader<'a> {
        if (reff.is_null() || (*reff).isNull()) {
            return defaultValue;
        }

        let refTarget = (*reff).target();

        let ptr : *Word = followFars(&mut reff, refTarget, &mut segment);

        let listRef = (*reff).listRef();

        let size : uint = listRef.elementCount();

        assert!((*reff).kind() == WP_LIST,
                "Message contains non-list pointer where text was expected");

        assert!(listRef.elementSize() == BYTE);

        assert!(boundsCheck(segment, ptr,
                            ptr.offset(roundBytesUpToWords(size) as int)));

        assert!(size > 0, "Message contains text that is not NUL-terminated");

        let strPtr = std::cast::transmute::<*Word,*i8>(ptr);

        assert!((*strPtr.offset((size - 1) as int)) == 0i8,
                "Message contains text that is not NUL-terminated");

        std::str::raw::c_str_to_static_slice(strPtr)
    }
}

static EMPTY_SEGMENT : [Word,..0] = [];

pub struct StructReader<'a> {
    segment : *SegmentReader<'a>,
    data : *u8,
    pointers : *WirePointer,
    dataSize : BitCount32,
    pointerCount : WirePointerCount16,
    bit0Offset : BitCount8,
    nestingLimit : int
}

impl <'a> StructReader<'a>  {

    pub fn newDefault() -> StructReader {
        StructReader { segment : std::ptr::null(),
                       data : std::ptr::null(),
                       pointers : std::ptr::null(), dataSize : 0, pointerCount : 0,
                       bit0Offset : 0, nestingLimit : 0x7fffffff}
    }

    pub fn readRoot<'b>(location : WordCount, segment : *SegmentReader<'b>,
                        nestingLimit : int) -> StructReader<'b> {
        //  the pointer to the struct is at segment[location]
        unsafe {
            // TODO bounds check
            let reff : *WirePointer =
                std::cast::transmute((*segment).segment.unsafe_ref(location));

            WireHelpers::readStructPointer(segment, reff, std::ptr::null(), nestingLimit)
        }
    }

    pub fn getDataSectionSize(&self) -> BitCount32 { self.dataSize }

    pub fn getPointerSectionSize(&self) -> WirePointerCount16 { self.pointerCount }

    pub fn getDataSectionAsBlob(&self) -> uint { fail!("unimplemented") }

    #[inline]
    pub fn getDataField<T:Clone + std::num::Zero>(&self, offset : ElementCount) -> T {
        // We need to check the offset because the struct may have
        // been created with an old version of the protocol that did
        // not contain the field.
        if ((offset + 1) * bitsPerElement::<T>() <= self.dataSize as uint) {
            unsafe {
                let dwv : *WireValue<T> = std::cast::transmute(self.data);
                (*dwv.offset(offset as int)).get()
            }
        } else {
            return std::num::Zero::zero()
        }
    }


    #[inline]
    pub fn getBoolField(&self, offset : ElementCount) -> bool {
        let mut boffset : BitCount32 = offset as BitCount32;
        if (boffset < self.dataSize) {
            if (offset == 0) {
                boffset = self.bit0Offset as BitCount32;
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
    pub fn getDataFieldMask<T:Clone + std::num::Zero + Mask>(&self,
                                                            offset : ElementCount,
                                                            mask : T) -> T {
        Mask::mask(self.getDataField(offset), mask)
    }

    #[inline]
    pub fn getBoolFieldMask(&self,
                            offset : ElementCount,
                            mask : bool) -> bool {
       self.getBoolField(offset) ^ mask
    }


    pub fn getStructField(&self, ptrIndex : WirePointerCount, _defaultValue : Option<&'a [u8]>)
        -> StructReader<'a> {
        let reff : *WirePointer = if (ptrIndex >= self.pointerCount as WirePointerCount)
            { std::ptr::null() }
        else
            { unsafe { self.pointers.offset(ptrIndex as int)} };

        unsafe {
            WireHelpers::readStructPointer(self.segment, reff,
                                           std::ptr::null(), self.nestingLimit)
        }
    }

    pub fn getListField(&self,
                        ptrIndex : WirePointerCount, expectedElementSize : FieldSize,
                        _defaultValue : Option<&'a [u8]>) -> ListReader<'a> {
        let reff : *WirePointer =
            if (ptrIndex >= self.pointerCount as WirePointerCount)
            { std::ptr::null() }
            else { unsafe{ self.pointers.offset(ptrIndex as int )} };

        unsafe {
            WireHelpers::readListPointer(self.segment,
                                         reff,
                                         std::ptr::null(),
                                         expectedElementSize, self.nestingLimit)
        }
    }

    pub fn getTextField(&self, ptrIndex : WirePointerCount,
                        defaultValue : &'a str) -> Text::Reader<'a> {
        let reff : *WirePointer =
            if (ptrIndex >= self.pointerCount as WirePointerCount) {
                std::ptr::null()
            } else {
                unsafe{self.pointers.offset(ptrIndex as int)}
            };
        unsafe {
            WireHelpers::readTextPointer(self.segment, reff, defaultValue)
        }
    }

    pub fn totalSize(&self) -> WordCount64 {
        fail!("totalSize is unimplemented");
    }

}

pub trait HasStructSize {
    fn structSize(unused_self : Option<Self>) -> StructSize;
}

pub trait FromStructBuilder {
    fn fromStructBuilder(structBuilder : StructBuilder) -> Self;
}

pub struct StructBuilder {
    segment : *mut SegmentBuilder,
    data : *mut u8,
    pointers : *mut WirePointer,
    dataSize : BitCount32,
    pointerCount : WirePointerCount16,
    bit0Offset : BitCount8
}

impl StructBuilder {
    pub fn as_reader<T>(&self, f : |StructReader| -> T) -> T {
        unsafe {
            (*self.segment).as_reader( |segmentReader| {
                f ( StructReader {
                        segment : std::ptr::to_unsafe_ptr(segmentReader),
                        data : std::cast::transmute(self.data),
                        pointers : std::cast::transmute(self.pointers),
                        dataSize : self.dataSize,
                        pointerCount : self.pointerCount,
                        bit0Offset : self.bit0Offset,
                        nestingLimit : 0x7fffffff
                    })
            })
        }
    }

    pub fn initRoot(segment : *mut SegmentBuilder,
                    location : *mut WirePointer,
                    size : StructSize) -> StructBuilder {
        unsafe {
            WireHelpers::initStructPointer(location, segment, size)
        }
    }

    #[inline]
    pub fn setDataField<T:Clone>(&self, offset : ElementCount, value : T) {
        unsafe {
            let ptr : *mut WireValue<T> = std::cast::transmute(self.data);
            (*ptr.offset(offset as int)).set(value)
        }
    }

    #[inline]
    pub fn getDataField<T:Clone>(&self, offset : ElementCount) -> T {
        unsafe {
            let ptr : *mut WireValue<T> = std::cast::transmute(self.data);
            (*ptr.offset(offset as int)).get()
        }
    }

    #[inline]
    pub fn setBoolField(&self, offset : ElementCount, value : bool) {
        //# This branch should be compiled out whenever this is
        //# inlined with a constant offset.
        let boffset : BitCount0 = if (offset == 0) { self.bit0Offset as uint } else { offset };
        let b = unsafe { self.data.offset((boffset / BITS_PER_BYTE) as int)};
        let bitnum = boffset % BITS_PER_BYTE;
        unsafe { (*b) = (( (*b) & !(1 << bitnum)) | (value as u8 << bitnum)) }
    }

    #[inline]
    pub fn getBoolField(&self, offset : ElementCount) -> bool {
        let boffset : BitCount0 =
            if (offset == 0) {self.bit0Offset as BitCount0} else {offset};
        let b = unsafe { self.data.offset((boffset / BITS_PER_BYTE) as int) };
        unsafe { ((*b) & (1 << (boffset % BITS_PER_BYTE ))) != 0 }
    }

    //# Initializes the struct field at the given index in the pointer
    //# section. If it is already initialized, the previous value is
    //# discarded or overwritten. The struct is initialized to the type's
    //# default state (all-zero). Use getStructField() if you want the
    //# struct to be initialized as a copy of the field's default value
    //# (which may have non-null pointers).
    pub fn initStructField(&self, ptrIndex : WirePointerCount, size : StructSize)
        -> StructBuilder {
        unsafe {
            WireHelpers::initStructPointer(self.pointers.offset(ptrIndex as int),
                                           self.segment, size)
        }
    }

    //# Gets the struct field at the given index in the pointer
    //# section. If the field is not already initialized, it is
    //# initialized as a deep copy of the given default value (a flat
    //# message), or to the empty state if defaultValue is nullptr.
    pub fn getStructField(&self, ptrIndex : WirePointerCount, size : StructSize,
                          _defaultValue : Option<()>) -> StructBuilder {
        unsafe {
            WireHelpers::getWritableStructPointer(
                self.pointers.offset(ptrIndex as int),
                self.segment,
                size,
                std::ptr::null())
        }
    }

    //# Allocates a new list of the given size for the field at the given
    //# index in the pointer segment, and return a pointer to it. All
    //# elements are initialized to zero.
    pub fn initListField(&self, ptrIndex : WirePointerCount,
                         elementSize : FieldSize, elementCount : ElementCount)
        -> ListBuilder {
        unsafe {
            WireHelpers::initListPointer(
                self.pointers.offset(ptrIndex as int),
                self.segment, elementCount, elementSize)
        }
    }

    //# Gets the already-allocated list field for the given pointer
    //# index, ensuring that the list is suitable for storing
    //# non-struct elements of the given size. If the list is not
    //# already allocated, it is allocated as a deep copy of the given
    //# default value (a flat message). If the default value is null,
    //# an empty list is used.
    pub fn getListField(&self, ptrIndex : WirePointerCount,
                        elementSize : FieldSize, _defaultValue : Option<()>) -> ListBuilder {
        unsafe {
            WireHelpers::getWritableListPointer(
                self.pointers.offset(ptrIndex as int),
                self.segment, elementSize, std::ptr::null())
        }
    }

    //# Allocates a new list of the given size for the field at the
    //# given index in the pointer segment, and return a pointer to it.
    //# Each element is initialized to its empty state.
    pub fn initStructListField(&self, ptrIndex : WirePointerCount,
                               elementCount : ElementCount, elementSize : StructSize)
        -> ListBuilder {
        unsafe { WireHelpers::initStructListPointer(
                self.pointers.offset(ptrIndex as int),
                self.segment, elementCount, elementSize)
        }
    }

    //# Gets the already-allocated list field for the given pointer
    //# index, ensuring that the list is suitable for storing struct
    //# elements of the given size. If the list is not already
    //# allocated, it is allocated as a deep copy of the given default
    //# value (a flat message). If the default value is null, an empty
    //# list is used.
    pub fn getStructListField(&self, ptrIndex : WirePointerCount,
                              elementSize : StructSize,
                              _defaultValue : Option<()>) -> ListBuilder {
        unsafe {
            WireHelpers::getWritableStructListPointer(
                self.pointers.offset(ptrIndex as int),
                self.segment, elementSize,
                std::ptr::null())
        }
    }

    pub fn setTextField(&self, ptrIndex : WirePointerCount, value : &str) {
        unsafe {
            WireHelpers::setTextPointer(
                self.pointers.offset(ptrIndex as int),
                self.segment, value)
        }
    }


    pub fn getTextField(&self, ptrIndex : WirePointerCount,
                        defaultValue : &'static str) -> Text::Builder {
        unsafe {
            WireHelpers::getWritableTextPointer(
                self.pointers.offset(ptrIndex as int),
                self.segment, defaultValue)
        }
    }

}

pub struct ListReader<'a> {
    segment : *SegmentReader<'a>,
    ptr : *u8,
    elementCount : ElementCount,
    step : BitCount0,
    structDataSize : BitCount32,
    structPointerCount : WirePointerCount16,
    nestingLimit : int
}

impl <'a> ListReader<'a> {

    pub fn newDefault() -> ListReader {
        ListReader { segment : std::ptr::null(),
                    ptr : std::ptr::null(), elementCount : 0, step: 0, structDataSize : 0,
                    structPointerCount : 0, nestingLimit : 0x7fffffff}
    }

    #[inline]
    pub fn size(&self) -> ElementCount { self.elementCount }

    pub fn getStructElement(&self, index : ElementCount) -> StructReader<'a> {
        assert!(self.nestingLimit > 0,
                "Message is too deeply-nested or contains cycles");
        let indexBit : BitCount64 = index as ElementCount64 * (self.step as BitCount64);

        let structData : *u8 = unsafe {
            self.ptr.offset((indexBit as uint / BITS_PER_BYTE) as int) };

        let structPointers : *WirePointer = unsafe {
                std::cast::transmute(
                    structData.offset((self.structDataSize as uint / BITS_PER_BYTE) as int))
        };

/*
        assert!(self.structPointerCount == 0 ||
                structPointers % BYTES_PER_POINTER == 0,
                "Pointer section of struct list element not aligned"
               );
*/
        StructReader {
            segment : self.segment,
            data : structData,
            pointers : structPointers,
            dataSize : self.structDataSize as BitCount32,
            pointerCount : self.structPointerCount,
            bit0Offset : (indexBit % (BITS_PER_BYTE as u64)) as u8,
            nestingLimit : self.nestingLimit - 1
        }
    }

    pub fn getListElement(&self, _index : ElementCount, _expectedElementSize : FieldSize)
        -> ListReader<'a> {
        fail!("unimplemented")
    }
}


pub struct ListBuilder {
    segment : *mut SegmentBuilder,
    ptr : *mut u8,
    elementCount : ElementCount,
    step : BitCount0,
    structDataSize : BitCount32,
    structPointerCount : WirePointerCount16
}

impl ListBuilder {

    #[inline]
    pub fn size(&self) -> ElementCount { self.elementCount }

    pub fn getStructElement(&self, index : ElementCount) -> StructBuilder {
        let indexBit = index * self.step;
        let structData = unsafe{ self.ptr.offset((indexBit / BITS_PER_BYTE) as int)};
        let structPointers = unsafe {
            std::cast::transmute(
                structData.offset(((self.structDataSize as uint) / BITS_PER_BYTE) as int))
        };
        StructBuilder {
            segment : self.segment,
            data : structData,
            pointers : structPointers,
            dataSize : self.structDataSize,
            pointerCount : self.structPointerCount,
            bit0Offset : (indexBit % BITS_PER_BYTE) as u8
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

