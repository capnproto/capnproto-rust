# Copyright (c) 2013-2014 Sandstorm Development Group, Inc. and contributors
# Licensed under the MIT License:
#
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
#
# The above copyright notice and this permission notice shall be included in
# all copies or substantial portions of the Software.
#
# THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
# IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
# FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
# AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
# LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
# OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
# THE SOFTWARE.
#

@0x99d187209d25cee7;

struct TestPrimList {
    uint8List  @0 : List(UInt8);
    int8List   @1 : List(Int8);
    uint16List @2 : List(UInt16);
    int16List  @3 : List(Int16);
    uint32List @4 : List(UInt32);
    int32List  @5 : List(Int32);
    uint64List @6 : List(UInt64);
    int64List @7 : List(Int64);
    float32List @8 : List(Float32);
    boolList @9 : List(Bool);
    voidList @10 : List(Void);
}

struct TestStructList {
   structList @0 : List(TestPrimList);
}

struct TestBlob {
   textField @0 : Text;
   dataField @1 : Data;
}

struct TestBigStruct {
  voidField      @0  : Void;
  boolField      @1  : Bool;
  int8Field      @2  : Int8;
  int16Field     @3  : Int16;
  int32Field     @4  : Int32;
  int64Field     @5  : Int64;
  uint8Field     @6  : UInt8;
  uint16Field    @7  : UInt16;
  uint32Field    @8  : UInt32;
  uint64Field    @9  : UInt64;
  float32Field   @10 : Float32;
  float64Field   @11 : Float64;

  structField @12 : Inner;
  anotherStructField @13 : Inner;

  struct Inner {
    uint32Field    @0  : UInt32;
    uint64Field    @1  : UInt64;
    float32Field   @2 : Float32;
    float64Field   @3 : Float64;
    boolFieldA     @4  : Bool;
    boolFieldB     @5  : Bool;
    boolFieldC     @6  : Bool;
    boolFieldD     @7  : Bool;
  }
}

enum AnEnum {
     foo @0;
     bar @1;
     baz @2;
     qux @3;
}

struct TestComplexList {
   enumList @0 : List(AnEnum);
   textList @1 : List(Text);
   dataList @2 : List(Data);
   primListList @3 : List(List(Int32));
   primListListList @4 : List(List(List(Int16)));
   enumListList @5 : List(List(AnEnum));
   textListList @6 : List(List(Text));
   dataListList @7 : List(List(Data));
   structListList @8 : List(List(TestBigStruct));
}

struct TestDefaults {
   voidField     @0  :Void      = void;
   boolField     @1  :Bool      = true;
   int8Field     @2  :Int8      = -123;
   int16Field    @3  :Int16     = -12345;
   int32Field    @4  :Int32     = -12345678;
   int64Field    @5  :Int64     = -123456789012345;
   uint8Field    @6  :UInt8     = 234;
   uint16Field   @7  :UInt16    = 45678;
   uint32Field   @8  :UInt32    = 3456789012;
   uint64Field   @9  :UInt64    = 12345678901234567890;
   float32Field  @10 :Float32   = 1234.5;
   float64Field  @11 :Float64   = -123e45;
}

struct TestEmptyStruct {

}

struct TestAnyPointer {
   anyPointerField @0 :AnyPointer;
}

struct TestUnion {
   union0 :union {
     u0f0s0  @0 :Void;
     u0f0s1  @1 :Bool;
     u0f0s8  @2 :Int8;
     u0f0s16 @3 :Int16;
     u0f0s32 @4 :Int32;
     u0f0s64 @5 :Int64;
     u0f0sp  @6 :Text;
   }
}

struct TestGroups {
  groups :union {
    foo :group {
      corge @0 :Int32;
      grault @2 :Int64;
      garply @8 :Text;
    }
    bar :group {
      corge @3 :Int32;
      grault @4 :Text;
      garply @5 :Int64;
    }
    baz :group {
      corge @1 :Int32;
      grault @6 :Text;
      garply @7 :Text;
      quz @9 : Float64;
    }
  }
}

struct TestOldVersion {
  # A subset of TestNewVersion.
  old1 @0 :Int64;
  old2 @1 :Text;
  old3 @2 :TestOldVersion;
}

struct TestNewVersion {
  # A superset of TestOldVersion.
  old1 @0 :Int64;
  old2 @1 :Text;
  old3 @2 :TestNewVersion;
  new1 @3 :Int64 = 987;
  new2 @4 :Text = "baz";
  new3 @5 :TestDefaults;
}

struct TestConstants {
   const voidConst     :Void = void;
   const boolConst     :Bool = true;
   const int8Const     :Int8 = -123;
   const int16Const    :Int16 = -12345;
   const int32Const    :Int32 = -12345678;
   const int64Const    :Int64 = -123456789012345;
   const uint8Const    :UInt8 = 234;
   const uint16Const   :UInt16 = 45678;
   const uint32Const   :UInt32 = 3456789012;
   const uint64Const   :UInt64 = 12345678901234567890;
   const float32Const  :Float32 = 1234.5;
   const float64Const  :Float64 = -123e45;
}

const globalInt :UInt32 = 12345;

interface TestInterface {
   foo @0 (i :UInt32, j :Bool) -> (x : Text);
   bar @1 () -> ();
   baz @2 (s : TestBigStruct);
   bazz @3 (s : TestBigStruct) -> (r : TestBigStruct);
}


interface TestExtends extends(TestInterface) {
   qux @0 ();
   corge @1 TestBigStruct -> ();
   grault @2 () -> TestBigStruct;
}

struct TestKeywords {
  struct As {}
  struct Box {}
  struct Break {}
  struct Continue {}
  struct Crate {}
  struct Else {}
  struct Enum {}
  struct Extern {}
  # ...
  struct Struct{}
  struct Super{}
  struct True{}
  struct Trait{}
  struct Type{}
  struct Unsafe{}
  struct Use{}
  struct While{}
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

struct TestAllTypes {
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
  structField    @14 : TestAllTypes;
  enumField      @15 : TestEnum;
  interfaceField @16 : Void;  # TODO

  voidList      @17 : List(Void);
  boolList      @18 : List(Bool);
  int8List      @19 : List(Int8);
  int16List     @20 : List(Int16);
  int32List     @21 : List(Int32);
  int64List     @22 : List(Int64);
  uInt8List     @23 : List(UInt8);
  uInt16List    @24 : List(UInt16);
  uInt32List    @25 : List(UInt32);
  uInt64List    @26 : List(UInt64);
  float32List   @27 : List(Float32);
  float64List   @28 : List(Float64);
  textList      @29 : List(Text);
  dataList      @30 : List(Data);
  structList    @31 : List(TestAllTypes);
  enumList      @32 : List(TestEnum);
  interfaceList @33 : List(Void);  # TODO
}
