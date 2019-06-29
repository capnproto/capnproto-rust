@0xc90daeac68e62b2a;

struct TestStructInner {
        innerU8 @0: UInt8;
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
    # myList @11: List(TestStructInner);
}
