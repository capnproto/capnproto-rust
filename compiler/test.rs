/*
 * Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

#[link(name = "test", vers = "alpha", author = "dwrensha")];

#[crate_type = "bin"];

extern mod capnprust;

use capnprust::*;

pub mod test_capnp;

#[test]
fn testPrimList () {
    use capnprust::message::*;
    use test_capnp::*;

    // Make the first segment small to force allocation of a second segment.
    let message = MessageBuilder::new(50,
                                      SUGGESTED_ALLOCATION_STRATEGY);

    let testPrimList = message.initRoot::<TestPrimList::Builder>();

    let uint64List = testPrimList.initUint64List(100);

    for i in range(0, uint64List.size()) {
        uint64List.set(i, i as u64);
    }

    do testPrimList.asReader |testPrimListReader| {
        let uint64List = testPrimListReader.getUint64List();
        for i in range(0, uint64List.size()) {
            assert!(uint64List.get(i) == i as u64);
        }
    }
}


fn main () {

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
    inner.setFloat64Field(1.1234567);

    inner.setBoolFieldB(true);

    bigStruct.setBoolField(true);

    let outStream = @std::io::stdout() as @serialize::OutputStream;
    serialize::writeMessage(outStream, message)

}
