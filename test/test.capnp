# Copyright (c) 2013-2016 Sandstorm Development Group, Inc. and contributors
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

@0xa7c73bdce79c15a0;

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
  interfaceField @16 : TestInterface;

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

interface Bootstrap {
  testInterface @0 () -> (cap: TestInterface);
  testExtends @1 () -> (cap: TestExtends);
  testExtends2 @2 () -> (cap: TestExtends2);
  testPipeline @3 () -> (cap: TestPipeline);
  testCallOrder @4 () -> (cap: TestCallOrder);
  testMoreStuff @5 () -> (cap: TestMoreStuff);
}

interface TestInterface {
  foo @0 (i :UInt32, j :Bool) -> (x :Text);
  bar @1 () -> ();
  baz @2 (s: TestAllTypes);
}

interface TestExtends extends(TestInterface) {
  qux @0 ();
  corge @1 TestAllTypes -> ();
  grault @2 () -> TestAllTypes;
}

interface TestExtends2 extends(TestExtends) {}

interface TestPipeline {
  getCap @0 (n: UInt32, inCap :TestInterface) -> (s: Text, outBox :Box);
  getNullCap @1 () -> (cap :TestInterface);
  testPointers @2 (cap :TestInterface, obj :AnyPointer, list :List(TestInterface)) -> ();

  struct Box {
    cap @0 :TestInterface;
  }
}

interface TestCallOrder {
  getCallSequence @0 (expected: UInt32) -> (n: UInt32);
  # First call returns 0, next returns 1, ...
  #
  # The input `expected` is ignored but useful for disambiguating debug logs.
}

interface TestTailCallee {
  struct TailResult {
    i @0 :UInt32;
    t @1 :Text;
    c @2 :TestCallOrder;
  }

  foo @0 (i :Int32, t :Text) -> TailResult;
}

interface TestTailCaller {
  foo @0 (i :Int32, callee :TestTailCallee) -> TestTailCallee.TailResult;
}

interface TestHandle {}

interface TestMoreStuff extends(TestCallOrder) {
  # Catch-all type that contains lots of testing methods.

  callFoo @0 (cap :TestInterface) -> (s: Text);
  # Call `cap.foo()`, check the result, and return "bar".

  callFooWhenResolved @1 (cap :TestInterface) -> (s: Text);
  # Like callFoo but waits for `cap` to resolve first.

  neverReturn @2 (cap :TestInterface) -> (capCopy :TestInterface);
  # Doesn't return.  You should cancel it.

  hold @3 (cap :TestInterface) -> ();
  # Returns immediately but holds on to the capability.

  callHeld @4 () -> (s: Text);
  # Calls the capability previously held using `hold` (and keeps holding it).

  getHeld @5 () -> (cap :TestInterface);
  # Returns the capability previously held using `hold` (and keeps holding it).

  echo @6 (cap :TestCallOrder) -> (cap :TestCallOrder);
  # Just returns the input cap.

  expectCancel @7 (cap :TestInterface) -> ();
  # evalLater()-loops forever, holding `cap`.  Must be canceled.

  methodWithDefaults @8 (a :Text, b :UInt32 = 123, c :Text = "foo") -> (d :Text, e :Text = "bar");

  getHandle @9 () -> (handle :TestHandle);
  # Get a new handle. Tests have an out-of-band way to check the current number of live handles, so
  # this can be used to test garbage collection.

  getNull @10 () -> (nullCap :TestMoreStuff);
  # Always returns a null capability.

  getHandleCount @11 () -> (count: Int64);

  dontHold @12 (cap :TestInterface) -> ();
  # Returns immediately and does not hold on to the capability.

  callEachCapability @13 (caps :List(TestInterface)) -> ();
  # Calls TestInterface::foo(123, true) on each cap.
}
