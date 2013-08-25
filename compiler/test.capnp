#
# Copyright (c) 2013, David Renshaw (dwrenshaw@gmail.com)
#
# See the LICENSE file in the capnproto-rust root directory.
#

@0x99d187209d25cee7;

struct TestPrimList {
    uint64List @0 : List(UInt64);
}


struct BigStruct {
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

  structField @12 : Inner;

  struct Inner {
    uInt32Field    @0  : UInt32;
    uInt64Field    @1  : UInt64;
    float32Field   @2 : Float32;
    float64Field   @3 : Float64;
    boolFieldA     @4  : Bool;
    boolFieldB     @5  : Bool;
    boolFieldC     @6  : Bool;
    boolFieldD     @7  : Bool;
  }
}
