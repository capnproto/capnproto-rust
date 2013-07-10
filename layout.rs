use common::*;
use endian::*;
use arena::*;
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

pub enum WirePointerKind {
    WP_STRUCT = 0,
    WP_LIST = 1,
    WP_FAR = 2,
    WP_RESERVED_3 = 3
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

    #[inline(always)]
    pub fn set(&mut self, size : StructSize) {
        self.dataSize.set(size.data);
        self.ptrCount.set(size.pointers);
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

    #[inline(always)]
    pub fn inlineCompositeWordCount(&self) -> WordCount {
        self.elementCount()
    }

    #[inline(always)]
    pub fn set(&mut self, es : FieldSize, ec : ElementCount) {
        assert!(ec < (1 << 29), "Lists are limited to 2**29 elements");
        self.elementSizeAndCount.set(((ec as u32) << 3 ) | (es as u32));
    }

    #[inline(always)]
    pub fn setInlineComposite(& mut self, wc : WordCount) {
        assert!(wc < (1 << 29), "Inline composite lists are limited to 2 ** 29 words");
        self.elementSizeAndCount.set((( wc as u32) << 3) | (INLINE_COMPOSITE as u32));
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
    pub fn getMut<'a>(segment : &'a mut [u8], index : WordCount) -> &'a mut WirePointer {
        unsafe {
                std::cast::transmute(segment.unsafe_ref(index * BYTES_PER_WORD))
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
    pub fn setKindAndTarget(&mut self, kind : WirePointerKind,
                            target : WordCount, thisOffset : WordCount) {
        self.offsetAndKind.set(((target as i32 - thisOffset as i32 - 1) << 2) as u32
                               | (kind as u32))
    }

    #[inline(always)]
    pub fn setKindWithZeroOffset(&mut self, kind : WirePointerKind) {
        self.offsetAndKind.set( kind as u32)
    }

    #[inline(always)]
    pub fn inlineCompositeListElementCount(&self) -> ElementCount {
        (self.offsetAndKind.get() >> 2) as ElementCount
    }

    #[inline(always)]
    pub fn setKindAndInlineCompositeListElementCount(
        &mut self, kind : WirePointerKind, elementCount : ElementCount) {
        self.offsetAndKind.set((( elementCount as u32 << 2) | (kind as u32)))
    }


    #[inline(always)]
    pub fn farPositionInSegment(&self) -> WordCount {
        (self.offsetAndKind.get() >> 3) as WordCount
    }

    #[inline(always)]
    pub fn isDoubleFar(&self) -> bool {
        ((self.offsetAndKind.get() >> 2) & 1) as bool
    }

    #[inline(always)]
    pub fn setFar(&mut self, isDoubleFar : bool, pos : WordCount) {
        self.offsetAndKind.set
            (( pos << 3) as u32 | (isDoubleFar as u32 << 2) | WP_FAR as u32);
    }

    #[inline(always)]
    pub fn structRef(&self) -> StructRef {
        unsafe { std::cast::transmute(self.upper32Bits) }
    }

    #[inline(always)]
    pub fn structRefMut<'a>(&'a mut self) -> &'a mut StructRef {
        unsafe { std::cast::transmute(& self.upper32Bits) }
    }

    #[inline(always)]
    pub fn listRef(&self) -> ListRef {
        unsafe { std::cast::transmute(self.upper32Bits) }
    }

    #[inline(always)]
    pub fn listRefMut<'a>(&'a self) -> &'a mut ListRef {
        unsafe { std::cast::transmute(& self.upper32Bits) }
    }

    #[inline(always)]
    pub fn farRef(&self) -> FarRef {
        unsafe { std::cast::transmute(self.upper32Bits) }
    }

    #[inline(always)]
    pub fn farRefMut<'a>(&'a mut self) -> &'a mut FarRef {
        unsafe { std::cast::transmute(& self.upper32Bits) }
    }

    #[inline(always)]
    pub fn isNull(&self) -> bool {
        (self.offsetAndKind.get() == 0) & (self.upper32Bits == 0)
    }

}


mod WireHelpers {
    use std;
    use common::*;
    use layout::*;
    use arena::*;


    #[inline(always)]
    pub fn roundBytesUpToWords(bytes : ByteCount) -> WordCount {
        // This code assumes 64-bit words.
        (bytes + 7) / BYTES_PER_WORD
    }

    // The maximum object size is 4GB - 1 byte. If measured in bits,
    // this would overflow a 32-bit counter, so we need to accept
    // BitCount64. However, 32 bits is enough for the returned
    // ByteCounts and WordCounts.

    #[inline(always)]
    pub fn roundBitsUpToWords(bits : BitCount64) -> WordCount {
        // This code assumes 64-bit words.
        ((bits + 63) / (BITS_PER_WORD as u64)) as WordCount
    }

    #[inline(always)]
    pub fn roundBitsUpToBytes(bits : BitCount64) -> ByteCount {
        ((bits + 7) / (BITS_PER_BYTE as u64)) as ByteCount
    }

    #[inline(always)]
    pub fn allocate(refIndex : WordCount,
                    segment : @mut SegmentBuilder,
                    amount : WordCount, kind : WirePointerKind) -> WordCount {
        // TODO zeroObject?

        match segment.allocate(amount) {
            None => {
                // Need to allocate in a new segment. We'll need to
                // allocate an extra pointer worth of space to act as
                // the landing pad for a far pointer.

                let amountPlusRef = amount + POINTER_SIZE_IN_WORDS;
                let segment = segment.messageBuilder.getSegmentWithAvailable(amountPlusRef);
                let ptr : WordCount = segment.allocate(amountPlusRef).unwrap();

                {
                    let reff = WirePointer::getMut(segment.segment, refIndex);
                    reff.setFar(false, ptr);
                    reff.farRefMut().segmentId.set(segment.id);
                }

                return ptr + POINTER_SIZE_IN_WORDS;
            }
            Some(ptr) => {
                let reff = WirePointer::getMut(segment.segment, refIndex);
                reff.setKindAndTarget(kind, ptr, refIndex);
                return ptr;
            }
        }
    }

    #[inline(always)]
    pub fn followFars<'a>(refIndex: WordCount,
                          segment : SegmentReader<'a>)
        -> (WordCount, WirePointer, SegmentReader<'a>) {
        let reff = WirePointer::get(segment.segment, refIndex);

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
            _ => { (reff.target(refIndex), reff, segment )  }
        }
    }


    #[inline(always)]
    pub fn initStructPointer(refIndex : WordCount,
                             segment : @mut SegmentBuilder,
                             size : StructSize) -> StructBuilder {

        let ptr : WordCount = allocate(refIndex, segment, size.total(), WP_STRUCT);
        WirePointer::getMut(segment.segment, refIndex).structRefMut().set(size);
        StructBuilder {
            segment : segment,
            data : ptr * BYTES_PER_WORD,
            pointers : (ptr + size.data as WordCount),
            dataSize : size.data as WordCount32 * (BITS_PER_WORD as BitCount32),
            pointerCount : size.pointers,
            bit0Offset : 0
        }
    }

    #[inline(always)]
    pub fn initListPointer(refIndex : WordCount,
                           segment : @mut SegmentBuilder,
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
        let ptr = allocate(refIndex, segment, wordCount, WP_LIST);

        WirePointer::getMut(segment.segment, refIndex).listRefMut().set(elementSize, elementCount);

        ListBuilder {
            segment : segment,
            ptr : ptr * BYTES_PER_WORD,
            step : step,
            elementCount : elementCount,
            structDataSize : dataSize as u32,
            structPointerCount : pointerCount as u16
        }
    }

    #[inline(always)]
    pub fn initStructListPointer(refIndex : WordCount,
                                 segment : @mut SegmentBuilder,
                                 elementCount : ElementCount,
                                 elementSize : StructSize) -> ListBuilder {
        match elementSize.preferredListEncoding {
            INLINE_COMPOSITE => { }
            otherEncoding => {
                return initListPointer(refIndex, segment, elementCount, otherEncoding);
            }
        }

        let wordsPerElement = elementSize.total();

        // Allocate the list, prefixed by a single WirePointer.
        let wordCount : WordCount = elementCount * wordsPerElement;
        let ptr : WordCount = allocate(refIndex, segment,
                                       POINTER_SIZE_IN_WORDS + wordCount, WP_LIST);

        // Initalize the pointer.
        WirePointer::getMut(segment.segment, refIndex).listRefMut().setInlineComposite(wordCount);

        // Initialize the list tag.
        WirePointer::getMut(segment.segment, ptr).setKindAndInlineCompositeListElementCount(
            WP_STRUCT, elementCount);
        WirePointer::getMut(segment.segment, ptr).structRefMut().set(elementSize);

        let ptr1 = ptr + POINTER_SIZE_IN_WORDS;

        ListBuilder {
            segment : segment,
            ptr : ptr1 * BYTES_PER_WORD,
            step : wordsPerElement * BITS_PER_WORD,
            elementCount : elementCount,
            structDataSize : elementSize.data as u32 * (BITS_PER_WORD as u32),
            structPointerCount : elementSize.pointers
        }
    }


    #[inline(always)]
    pub fn setTextPointer(refIndex : WirePointerCount, segment : @mut SegmentBuilder,
                          value : &str) {

        // initTextPointer is rolled in here

        let bytes : &[u8] = value.as_bytes();

        // The byte list must include a NUL terminator
        let byteSize = bytes.len() + 1;

        let ptr : WordCount = allocate(refIndex, segment, roundBytesUpToWords(byteSize), WP_LIST);

        WirePointer::getMut(segment.segment, refIndex).listRefMut().set(BYTE, byteSize);

        unsafe {
            let dst : *mut u8 = segment.segment.unsafe_mut_ref(ptr * BYTES_PER_WORD);
            let src : *u8 = bytes.unsafe_ref(0);
            std::ptr::copy_memory(dst, src, bytes.len());
        }

        // null terminate
        segment.segment[ptr * BYTES_PER_WORD + bytes.len()] = 0;
    }

    #[inline(always)]
    pub fn readStructPointer<'a>(segment: SegmentReader<'a>,
                                 oRefIndex : Option<WirePointerCount>,
                                 defaultValue : Option<&'a [u8]>,
                                 nestingLimit : int) -> StructReader<'a> {

        let (refIndex, segment) =
            if (oRefIndex == None ||
                WirePointer::get(segment.segment, oRefIndex.unwrap()).isNull()) {

                match defaultValue {
                    // A default struct value is always stored in its own
                    // static buffer.

                    Some (wp) if (! WirePointer::get(wp, 0).isNull()) => {
                        (0, SegmentReader {messageReader : segment.messageReader,
                                           segment : wp })
                    }
                    _ => {
                        return StructReader::newDefault(segment);
                    }
                }
        } else {
            (oRefIndex.unwrap(), segment)
        };

       if (nestingLimit <= 0) {
           fail!("nesting limit exceeded");
        }

        let (ptr, reff, segment) = followFars(refIndex, segment);

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
                               refIndex : WordCount,
                               defaultValue : uint,
                               expectedElementSize : FieldSize,
                               nestingLimit : int ) -> ListReader<'a> {
       if (nestingLimit <= 0) {
           fail!("nesting limit exceeded");
        }

        let (ptr1, reff, segment) = followFars(refIndex, segment);
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
                               refIndex : WordCount,
                               defaultValue : uint,
                               defaultSize : ByteCount
                              ) -> &'a str {

        let (ptr, reff, segment) = followFars(refIndex, segment);

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

static EMPTY_SEGMENT : [u8,..0] = [];

pub struct StructReader<'self> {
    segment : SegmentReader<'self>,
    data : ByteCount,
    pointers : WordCount,
    dataSize : BitCount0,
    pointerCount : WirePointerCount16,
    bit0Offset : BitCount8,
    nestingLimit : int
}

impl <'self> StructReader<'self>  {

    // TODO Can this be cleaned up? It seems silly that we need the
    // segmentReader argument just to get the messageReader, which
    // will be unused.
    pub fn newDefault<'a>(segmentReader : SegmentReader<'a>) -> StructReader<'a> {
        StructReader { segment : SegmentReader {messageReader : segmentReader.messageReader,
                                                segment : EMPTY_SEGMENT.slice(0,0)},
                      data : 0, pointers : 0, dataSize : 0, pointerCount : 0,
                      bit0Offset : 0, nestingLimit : 0x7fffffff}
    }

    pub fn readRoot<'a>(location : WordCount, segment : SegmentReader<'a>,
                        nestingLimit : int) -> StructReader<'a> {
        //  the pointer to the struct is at segment[location * 8]

        // TODO boundscheck
        WireHelpers::readStructPointer(segment, Some(location), None, nestingLimit)
    }

    pub fn getDataSectionSize(&self) -> BitCount0 { self.dataSize }

    pub fn getPointerSectionSize(&self) -> WirePointerCount16 { self.pointerCount }

    pub fn getDataSectionAsBlob(&self) -> uint { fail!("unimplemented") }

    #[inline(always)]
    pub fn getDataField<T:Copy>(&self, offset : ElementCount) -> T {
        if ((offset + 1) * bitsPerElement::<T>() <= self.dataSize) {
            let totalByteOffset = self.data + bytesPerElement::<T>() * offset;
            WireValue::getFromBuf(self.segment.segment, totalByteOffset).get()
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

    pub fn getStructField(&self, ptrIndex : WirePointerCount, defaultValue : Option<&'self [u8]>)
        -> StructReader<'self> {
        let oRefIndex = if (ptrIndex >= self.pointerCount as WirePointerCount)
            { None }
        else
            { Some(self.pointers + ptrIndex) };
        WireHelpers::readStructPointer(self.segment, oRefIndex,
                                       defaultValue, self.nestingLimit)
    }

    pub fn getListField(&self,
                        ptrIndex : WirePointerCount, expectedElementSize : FieldSize,
                        defaultValue : uint) -> ListReader<'self> {
        let location = self.pointers + ptrIndex;

        WireHelpers::readListPointer(self.segment,
                                     location,
                                     defaultValue,
                                     expectedElementSize, self.nestingLimit)

    }

    pub fn getTextField(&self, ptrIndex : WirePointerCount,
                            defaultValue : uint, defaultSize : ByteCount) -> &'self str {
        let location = self.pointers + ptrIndex;
        WireHelpers::readTextPointer(self.segment, location, defaultValue, defaultSize)
    }

    pub fn totalSize(&self) -> WordCount64 {
        fail!("totalSize is unimplemented");
    }

}

pub struct StructBuilder {
    segment : @ mut SegmentBuilder,
    data : ByteCount,
    pointers : WordCount,
    dataSize : BitCount32,
    pointerCount : WirePointerCount16,
    bit0Offset : BitCount8
}

impl StructBuilder {
    pub fn initRoot(segment : @ mut SegmentBuilder,
                    location : WordCount,
                    size : StructSize) -> StructBuilder {
        WireHelpers::initStructPointer(
            location, segment, size
        )
    }

    #[inline(always)]
    pub fn setDataField<T:Copy>(&self, offset : ElementCount, value : T) {
        let totalByteOffset = self.data + bytesPerElement::<T>() * offset;
        WireValue::getFromBufMut(self.segment.segment, totalByteOffset).set(value);
    }

    pub fn initStructField(&self, ptrIndex : WirePointerCount, size : StructSize)
        -> StructBuilder {
        WireHelpers::initStructPointer(self.pointers + ptrIndex, self.segment, size)
    }

    pub fn initListField(&self, ptrIndex : WirePointerCount,
                         elementSize : FieldSize, elementCount : ElementCount)
        -> ListBuilder {
        WireHelpers::initListPointer(self.pointers + ptrIndex,
                                     self.segment, elementCount, elementSize)
    }

    pub fn initStructListField(&self, ptrIndex : WirePointerCount,
                               elementCount : ElementCount, elementSize : StructSize)
        -> ListBuilder {
        WireHelpers::initStructListPointer(self.pointers + ptrIndex,
                                           self.segment, elementCount, elementSize)
    }

    pub fn setTextField(&self, ptrIndex : WirePointerCount, value : &str) {
        WireHelpers::setTextPointer(self.pointers + ptrIndex, self.segment, value)
    }

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

impl <'self> ListReader<'self> {

    #[inline(always)]
    pub fn size(&self) -> ElementCount { self.elementCount }

    #[inline(always)]
    pub fn getDataElement<T:Copy>(&self, index : ElementCount) -> T {
        let totalByteOffset = self.ptr + index * self.step / BITS_PER_BYTE;

        WireValue::getFromBuf(self.segment.segment,
                              totalByteOffset).get()
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

pub struct ListBuilder {
    segment : @mut SegmentBuilder,
    ptr : ByteCount,
    elementCount : ElementCount,
    step : BitCount0,
    structDataSize : BitCount32,
    structPointerCount : WirePointerCount16
}

impl ListBuilder {

    #[inline(always)]
    pub fn size(&self) -> ElementCount { self.elementCount }

    #[inline(always)]
    pub fn getDataElement<T:Copy>(&self, index : ElementCount) -> T {
        let totalByteOffset = self.ptr + index * self.step / BITS_PER_BYTE;

        WireValue::getFromBuf(self.segment.segment,
                              totalByteOffset).get()
    }

    #[inline(always)]
    pub fn setDataElement<T:Copy>(&self, index : ElementCount, value : T) {
        let totalByteOffset = self.ptr + index * self.step / BITS_PER_BYTE;

        WireValue::getFromBufMut(self.segment.segment,
                                 totalByteOffset).set(value)
    }

    pub fn getStructElement(&self, index : ElementCount) -> StructBuilder {
        let indexBit = index * self.step;
        let structData = self.ptr + indexBit / BITS_PER_BYTE;
        let structPointers = (structData + (self.structDataSize as uint) / BITS_PER_BYTE);
        StructBuilder {
            segment : self.segment,
            data : structData,
            pointers : structPointers / BYTES_PER_WORD,
            dataSize : self.structDataSize,
            pointerCount : self.structPointerCount,
            bit0Offset : (indexBit % BITS_PER_BYTE) as u8
        }
    }

}