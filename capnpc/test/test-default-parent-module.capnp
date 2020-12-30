@0xd5753aeadc144c23;

using TestOverride = import "test-default-parent-module-override.capnp";

struct Foo {
   s @0 :Text;
   b @1 :TestOverride.Baz;
}

struct Bar {
   f @0 :Foo;
}
