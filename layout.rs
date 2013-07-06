use common::*;
use endian::*;
use arena::SegmentReader;
use std;


pub enum FieldSize {
// is there a way to force this to fit within a single byte?
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

pub struct StructReader<'self> {
    segment : SegmentReader<'self>,
    data : ByteCount,
    pointers : WordCount,
    dataSize : BitCount0,
    pointerCount : WirePointerCount16,
    bit0Offset : BitCount8,
    nestingLimit : int
}

pub struct ListReader<'self> {
    segment : SegmentReader<'self>,
    ptr : ByteCount,
    elementCount : ElementCount,
    step : BitCount0,
    structDataSize : BitCount32,
    structPointerCount : WirePointerCount16,
    nestingLimit : int
}


pub enum WirePointerKind {
    WP_STRUCT = 0,
    WP_LIST = 1,
    WP_FAR = 2,
    WP_RESERVED_3 = 3
}


pub struct WirePointer {
    offsetAndKind : WireValue<u32>,
    upper32Bits : WireValue<u32>,
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
}

impl ListRef {
    #[inline(always)]
    pub fn elementSize(&self) -> FieldSize {
        unsafe {
            std::cast::transmute( (self.elementSizeAndCount.get() & 7) as u64)
        }
    }

    #[inline(always)]
    pub fn elementCount(&self) -> ElementCount {
        (self.elementSizeAndCount.get() >> 3) as uint
    }

    #[inline(alwyas)]
    pub fn inlineCompositeWordCount(&self) -> WordCount {
        self.elementCount()
    }

}

impl WirePointer {

    #[inline(always)]
    pub fn get(segment : &[u8], index : WordCount) -> WirePointer {
        unsafe {
            let p : *WirePointer =
                std::cast::transmute(segment.unsafe_ref(index * BYTES_PER_WORD));
            *p
        }
    }

    #[inline(always)]
    pub fn kind(&self) -> WirePointerKind {
        unsafe {
            std::cast::transmute((self.offsetAndKind.get() & 3) as u64)
        }
    }

    #[inline(always)]
    pub fn target(&self, thisOffset : WordCount) -> WordCount {
        (thisOffset as i32 + (1 + ((self.offsetAndKind.get() as i32) >> 2))) as WordCount
    }

    #[inline(always)]
    pub fn inlineCompositeListElementCount(&self) -> ElementCount {
        (self.offsetAndKind.get() >> 2) as ElementCount
    }

    #[inline(always)]
    pub fn isDoubleFar(&self) -> bool {
        ((self.offsetAndKind.get() >> 2) & 1) as bool
    }

    #[inline(always)]
    pub fn farPositionInSegment(&self) -> WordCount {
        (self.offsetAndKind.get() >> 3) as WordCount
    }

    #[inline(always)]
    pub fn structRef(&self) -> StructRef {
        unsafe { std::cast::transmute(self.upper32Bits) }
    }

    #[inline(always)]
    pub fn listRef(&self) -> ListRef {
        unsafe { std::cast::transmute(self.upper32Bits) }
    }

    #[inline(always)]
    pub fn farRef(&self) -> FarRef {
        unsafe { std::cast::transmute(self.upper32Bits) }
    }

}


mod WireHelpers {
    use std;
    use common::*;
    use layout::*;
    use arena::*;

    #[inline(always)]
    pub fn followFars<'a>(location: WordCount,
                          reff : WirePointer,
                          segment : SegmentReader<'a>)
        -> (WordCount, WirePointer, SegmentReader<'a>) {
        match reff.kind() {
            WP_FAR => {
                let segment =
                    segment.messageReader.getSegmentReader(reff.farRef().segmentId.get());

                let ptr : WordCount = reff.farPositionInSegment();
                let padWords = if (reff.isDoubleFar()) { 2 } else { 1 };

                // TODO better bounds check?
                assert!( ptr + padWords < segment.segment.len() );

                let pad = WirePointer::get(segment.segment, ptr);

                if (reff.isDoubleFar() ) {

                    return (pad.target(ptr), pad, segment);

                } else {
                    // Landing pad is another far pointer. It is
                    // followed by a tag describing the pointed-to
                    // object.

                    let reff = WirePointer::get(segment.segment, ptr + 1);

                    let segment =
                        segment.messageReader.getSegmentReader(pad.farRef().segmentId.get());

                    return (pad.farPositionInSegment(), reff, segment);
                }
            }
            _ => { (reff.target(location), reff, segment )  }
        }
    }


    #[inline(always)]
    pub fn readStructPointer<'a>(segment: SegmentReader<'a>,
                                 location : WordCount,
                                 reff : WirePointer,
                                 defaultValue : uint,
                                 nestingLimit : int) -> StructReader<'a> {

       if (nestingLimit <= 0) {
           fail!("nesting limit exceeded");
        }

        let (ptr, reff, segment)  = followFars(location, reff, segment);

        let dataSizeWords = reff.structRef().dataSize.get();

        StructReader {segment : segment,
                      data : ptr * BYTES_PER_WORD,
                      pointers : ptr + (dataSizeWords as WordCount),
                      dataSize : dataSizeWords as BitCount0 * BITS_PER_WORD,
                      pointerCount : reff.structRef().ptrCount.get(),
                      bit0Offset : 0,
                      nestingLimit : nestingLimit - 1 }

     }

    #[inline(always)]
    pub fn readListPointer<'a>(segment: SegmentReader<'a>,
                               location : WordCount,
                               reff : WirePointer,
                               defaultValue : uint,
                               expectedElementSize : FieldSize,
                               nestingLimit : int ) -> ListReader<'a> {
       if (nestingLimit <= 0) {
           fail!("nesting limit exceeded");
        }

        let (ptr1, reff, segment) = followFars(location, reff, segment);
        let mut ptr = ptr1;

        match reff.kind() {
            WP_LIST => { }
            _ => { fail!("Message contains non-list pointer where list pointer was expected") }
        }

        let listRef = reff.listRef();

        match listRef.elementSize() {
            INLINE_COMPOSITE => {
                let wordCount = listRef.inlineCompositeWordCount();

                let tag = WirePointer::get(segment.segment, ptr);

                ptr += POINTER_SIZE_IN_WORDS;

                // TODO bounds check

                match tag.kind() {
                    WP_STRUCT => {}
                    _ => fail!("INLINE_COMPOSITE lists of non-STRUCT type are not supported")
                }

                let size = tag.inlineCompositeListElementCount();
                let structRef = tag.structRef();
                let wordsPerElement = structRef.wordSize();

                assert!(size * wordsPerElement <= wordCount,
                       "INLINE_COMPOSITE list's elements overrun its word count");

                // If a struct list was not expected, then presumably
                // a non-struct list was upgraded to a struct list. We
                // need to manipulate the pointer to point at the
                // first field of the struct. Together with the
                // "stepBits", this will allow the struct list to be
                // accessed as if it were a primitive list without
                // branching.

                // Check whether the size is compatible.
                match expectedElementSize {
                    VOID => {}
                    BIT => fail!("Expected a bit list, but got a list of structs"),
                    BYTE | TWO_BYTES | FOUR_BYTES | EIGHT_BYTES => {
                        assert!(structRef.dataSize.get() > 0,
                               "Expected a primitive list, but got a list of pointer-only structs")
                    }
                    POINTER => {
                        ptr += structRef.dataSize.get() as WordCount;
                        assert!(structRef.ptrCount.get() > 0,
                               "Expected a pointer list, but got a list of data-only structs")
                    }
                    INLINE_COMPOSITE => {}
                }

                ListReader {
                    segment : segment,
                    ptr : ptr * BYTES_PER_WORD,
                    elementCount : size,
                    step : wordsPerElement * BITS_PER_WORD,
                    structDataSize : structRef.dataSize.get() as u32 * (BITS_PER_WORD as u32),
                    structPointerCount : structRef.ptrCount.get() as u16,
                    nestingLimit : nestingLimit - 1
                }
            }
            _ => {

                // This is a primitive or pointer list, but all such
                // lists can also be interpreted as struct lists. We
                // need to compute the data size and pointer count for
                // such structs.
                let dataSize = dataBitsPerElement(listRef.elementSize());
                let pointerCount = pointersPerElement(listRef.elementSize());
                let step = dataSize + pointerCount * BITS_PER_POINTER;

                // TODO bounds check


                // Verify that the elements are at least as large as
                // the expected type. Note that if we expected
                // INLINE_COMPOSITE, the expected sizes here will be
                // zero, because bounds checking will be performed at
                // field access time. So this check here is for the
                // case where we expected a list of some primitive or
                // pointer type.

                let expectedDataBitsPerElement =
                    dataBitsPerElement(expectedElementSize);
                let expectedPointersPerElement =
                    pointersPerElement(expectedElementSize);

                assert!(expectedDataBitsPerElement <= dataSize);
                assert!(expectedPointersPerElement <= pointerCount);

                ListReader {
                    segment : segment,
                    ptr : ptr * BYTES_PER_WORD,
                    elementCount : listRef.elementCount(),
                    step : step,
                    structDataSize : dataSize as u32,
                    structPointerCount : pointerCount as u16,
                    nestingLimit : nestingLimit - 1
                }
            }
        }

    }

    #[inline(always)]
    pub fn readTextPointer<'a>(segment : SegmentReader<'a>,
                               location : WordCount,
                               reff : WirePointer,
                               defaultValue : uint,
                               defaultSize : ByteCount
                              ) -> &'a str {

        let (ptr, reff, segment) = followFars(location, reff, segment);

        let listRef = reff.listRef();

        let size : uint = listRef.elementCount();

        match reff.kind() {
            WP_LIST => { }
            _ => { fail!("Message contains non-list pointer where text was expected") }
        };

        // TODO size assertion, bounds check

        assert!(size > 0, "Message contains text that is not NUL-terminated");

        let startByte = ptr * BYTES_PER_WORD;
        let slice = segment.segment.slice(startByte, startByte + size);

        assert!(slice[size-1] == 0, "Message contains text that is not NUL-terminated");

        std::str::from_bytes_slice(slice)
    }
}

impl <'self> StructReader<'self>  {
    pub fn readRoot<'a>(location : WordCount, segment : SegmentReader<'a>,
                        nestingLimit : int) -> StructReader<'a> {
        //  the pointer to the struct is at segment[location * 8]

        // TODO boundscheck
        WireHelpers::readStructPointer(segment, location,
                                       WirePointer::get(segment.segment, location),
                                       0, nestingLimit)

    }

    pub fn getDataSectionSize(&self) -> BitCount0 { self.dataSize }

    pub fn getPointerSectionSize(&self) -> WirePointerCount16 { self.pointerCount }

    pub fn getDataSectionAsBlob(&self) -> uint { fail!("unimplemented") }

    #[inline(always)]
    pub fn getDataField<T:Copy>(&self, offset : ElementCount) -> T {
        if ((offset + 1) * bitsPerElement::<T>() <= self.dataSize) {
            let totalByteOffset = self.data + bytesPerElement::<T>() * offset;
            unsafe {
                let wvp : *WireValue<T> =
                    std::cast::transmute(self.segment.segment.unsafe_ref(totalByteOffset));
                (*wvp).get()
            }
        } else {
            fail!("getDataField")
        }
    }

    #[inline(always)]
    pub fn getDataFieldBool(&self, offset : ElementCount) -> bool {
        let mut boffset : BitCount0 = offset;
        if (boffset < self.dataSize) {
            if (offset == 0) {
                boffset = self.bit0Offset as BitCount0;
            }
            let b : u8 = self.segment.segment[self.data + boffset / BITS_PER_BYTE];

            (b & (1 << (boffset % BITS_PER_BYTE ))) != 0

        } else {
            fail!("getDataFieldBool")
        }
    }

    pub fn getStructField(&self, ptrIndex : WirePointerCount, defaultValue : uint)
        -> StructReader<'self> {
        let location = self.pointers + ptrIndex;
        let reff = WirePointer::get(self.segment.segment, location);
        WireHelpers::readStructPointer(self.segment, location, reff,
                                       defaultValue, self.nestingLimit)
    }

    pub fn getListField(&self,
                        ptrIndex : WirePointerCount, expectedElementSize : FieldSize,
                        defaultValue : uint) -> ListReader<'self> {
        let location = self.pointers + ptrIndex;

        let reff = WirePointer::get(self.segment.segment, location);

        WireHelpers::readListPointer(self.segment,
                                     location,
                                     reff,
                                     defaultValue,
                                     expectedElementSize, self.nestingLimit)

    }

    pub fn getTextField(&self, ptrIndex : WirePointerCount,
                            defaultValue : uint, defaultSize : ByteCount) -> &'self str {
        let location = self.pointers + ptrIndex;
        let reff = WirePointer::get(self.segment.segment, location);
        WireHelpers::readTextPointer(self.segment, location, reff, defaultValue, defaultSize)
    }

    pub fn totalSize(&self) -> WordCount64 {
        fail!("totalSize is unimplemented");
    }

}

impl <'self> ListReader<'self> {

    #[inline(always)]
    pub fn size(&self) -> ElementCount { self.elementCount }

    #[inline(always)]
    pub fn getDataElement<T:Copy>(&self, index : ElementCount) -> T {
        let totalByteOffset = self.ptr + index * self.step / BITS_PER_BYTE;

        unsafe {
            let wvp : *WireValue<T> =
                std::cast::transmute(self.segment.segment.unsafe_ref(totalByteOffset));
            (*wvp).get()
        }
    }

    pub fn getStructElement(&self, index : ElementCount) -> StructReader<'self> {
        assert!(self.nestingLimit > 0,
                "Message is too deeply-nested or contains cycles");
        let indexBit : BitCount64 = index as ElementCount64 * (self.step as BitCount64);
        let structData : ByteCount = self.ptr + (indexBit as uint / BITS_PER_BYTE);
        let structPointers : ByteCount =
            structData + (self.structDataSize as BitCount0 / BITS_PER_BYTE);

        assert!(self.structPointerCount == 0 ||
                structPointers % BYTES_PER_POINTER == 0,
                "Pointer section of struct list element not aligned"
               );

        StructReader {
            segment : self.segment,
            data : structData,
            pointers : structPointers / BYTES_PER_WORD,
            dataSize : self.structDataSize as BitCount0,
            pointerCount : self.structPointerCount,
            bit0Offset : indexBit % (BITS_PER_BYTE as u64)  as u8,
            nestingLimit : self.nestingLimit - 1
        }
    }
}


