/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[link(name = "test", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnprust;

//use capnprust::*;

pub mod test_capnp;

#[test]
fn testPrimList () {
    use capnprust::message::*;
    use test_capnp::*;

    // Make the first segment small to force allocation of a second segment.
    let message = MessageBuilder::new(50,
                                      SUGGESTED_ALLOCATION_STRATEGY);

    let testPrimList = message.initRoot::<TestPrimList::Builder>();

    let uint8List = testPrimList.initUint8List(100);

    for i in range(0, uint8List.size()) {
        uint8List.set(i, i as u8);
    }

    let uint64List = testPrimList.initUint64List(20);

    for i in range(0, uint64List.size()) {
        uint64List.set(i, i as u64);
    }

    let boolList = testPrimList.initBoolList(65);

    boolList.set(0, true);
    boolList.set(1, true);
    boolList.set(2, true);
    boolList.set(3, true);
    boolList.set(5, true);
    boolList.set(8, true);
    boolList.set(13, true);
    boolList.set(64, true);

    assert!(boolList.get(0));
    assert!(!boolList.get(4));
    assert!(!boolList.get(63));
    assert!(boolList.get(64));


    let voidList = testPrimList.initVoidList(1025);
    voidList.set(257, ());

    do testPrimList.asReader |testPrimListReader| {
        let uint8List = testPrimListReader.getUint8List();
        for i in range(0, uint8List.size()) {
            assert!(uint8List.get(i) == i as u8);
        }
        let uint64List = testPrimListReader.getUint64List();
        for i in range(0, uint64List.size()) {
            assert!(uint64List.get(i) == i as u64);
        }

        let boolList = testPrimListReader.getBoolList();
        assert!(boolList.get(0));
        assert!(boolList.get(1));
        assert!(boolList.get(2));
        assert!(boolList.get(3));
        assert!(!boolList.get(4));
        assert!(boolList.get(5));
        assert!(!boolList.get(6));
        assert!(!boolList.get(7));
        assert!(boolList.get(8));
        assert!(!boolList.get(9));
        assert!(!boolList.get(10));
        assert!(!boolList.get(11));
        assert!(!boolList.get(12));
        assert!(boolList.get(13));
        assert!(!boolList.get(63));
        assert!(boolList.get(64));


        assert!(testPrimListReader.getVoidList().size() == 1025);
    }
}

#[test]
fn testBigStruct() {

    use capnprust::message::*;
    use test_capnp::*;

    // Make the first segment small to force allocation of a second segment.
    let message = MessageBuilder::new(5,
                                      SUGGESTED_ALLOCATION_STRATEGY);

    let bigStruct = message.initRoot::<BigStruct::Builder>();

    bigStruct.setBoolField(false);
    bigStruct.setInt8Field(-128);
    bigStruct.setInt16Field(0);
    bigStruct.setInt32Field(1009);

    let inner = bigStruct.initStructField();
    inner.setFloat64Field(0.1234567);

    inner.setBoolFieldB(true);

    bigStruct.setBoolField(true);

    do bigStruct.asReader |bigStructReader| {
        assert!(bigStructReader.getInt8Field() == -128);
        assert!(bigStructReader.getInt32Field() == 1009);

        let innerReader = bigStructReader.getStructField();
        assert!(!innerReader.getBoolFieldA());
        assert!(innerReader.getBoolFieldB());
        assert!(innerReader.getFloat64Field() == 0.1234567);
    }

}

fn main () {

}
