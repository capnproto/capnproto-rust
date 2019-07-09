@0xc90daeac68e62b2a;

struct TestStructInner {
        innerU8 @0: UInt8;
}

struct TestUnion {
    union {
        variantOne @0: UInt64;
        variantTwo @1: TestStructInner;
        variantThree @2: Void;
    }
}

struct ListUnion {
    union {
        empty @0: Void;
        withList @1: List(TestStructInner);
        withData @2: Data;
        testUnion @3: TestUnion;
    }
}

struct TestStruct {
    myBool @0: Bool;
    myInt8 @1: Int8;
    myInt16 @2: Int16;
    myInt32 @3: Int32;
    myInt64 @4: Int64;
    myUint8 @5: UInt8;
    myUint16 @6: UInt16;
    myUint32 @7: UInt32;
    myUint64 @8: UInt64;
    # my_float32: f32,
    # my_float64: f64,
    myText @9: Text;
    myData @10: Data;
    structInner @11: TestStructInner;
    myPrimitiveList @12: List(UInt16);
    myList @13: List(TestStructInner);
    inlineUnion: union {
            firstVariant @14: UInt64;
            secondVariant @15: TestStructInner;
            thirdVariant @16: Void;
    }
    externalUnion @17: TestUnion;
    listUnion @18: ListUnion;
}

struct FloatStruct {
    myFloat32 @0: Float32;
    myFloat64 @1: Float64;
}
