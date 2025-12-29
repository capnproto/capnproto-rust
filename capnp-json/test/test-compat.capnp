@0xbfc0c6eeae19d18a;

using Json = import "/capnp/compat/json.capnp";

struct TestJsonTypesCompat {
  voidField      @0  : Void;
  boolField      @1  : Bool;
  int8Field      @2  : Int8;
  int16Field     @3  : Int16;
  int32Field     @4  : Int32;
  int64Field     @5  : Int64;
  uInt8Field     @6  : UInt8;
  uInt16Field    @7  : UInt16;
  uInt32Field    @8  : UInt32;
  uInt64Field    @9  : UInt64;
  float32Field   @10 : Float32;
  float64Field   @11 : Float64;
  textField      @12 : Text;
  dataField      @13 : Data;
  base64Field    @14 : Data $Json.base64;
  hexField       @15 : Data $Json.hex;
  structField    @16 : TestJsonTypesCompat;
  enumField      @17 : TestEnum;

  voidList      @18 : List(Void);
  boolList      @19 : List(Bool);
  int8List      @20 : List(Int8);
  int16List     @21 : List(Int16);
  int32List     @22 : List(Int32);
  int64List     @23 : List(Int64);
  uInt8List     @24 : List(UInt8);
  uInt16List    @25 : List(UInt16);
  uInt32List    @26 : List(UInt32);
  uInt64List    @27 : List(UInt64);
  float32List   @28 : List(Float32);
  float64List   @29 : List(Float64);
  textList      @30 : List(Text);
  dataList      @31 : List(Data);
  structList    @32 : List(TestJsonTypesCompat);
  enumList      @33 : List(TestEnum);
}

enum TestEnum {
  foo @0;
  bar @1;
  baz @2;
  qux @3;
  quux @4;
  corge @5;
  grault @6;
  garply @7;
}

