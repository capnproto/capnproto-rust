@0xdb9f38dcb3dfbb6a;

interface Foo {
    identity @0 (x: UInt32) -> (y: UInt32);
    crash @1 ();
}
