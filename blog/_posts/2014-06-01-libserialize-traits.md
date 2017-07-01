---
layout: post
title: the libserialize traits
author: dwrensha
---

A number of people have asked me whether
Cap'n Proto might be able to hook into
the `Encodable` and `Decodable`
traits of Rust's `libserialize`.
My current answer is
"perhaps, but it probably wouldn't buy us much."

#### libserialize

The purpose of `Encodable` and `Decodable`
is to provide a convenient way
to make existing Rust data types
mobile.
For example, you might have a Rust data type `Foo`,

```
struct Foo {
  a : u64,
  b : String,
}
```

and you might encounter a need to
send values of type `Foo` between processes.
Using `libserialize`, you can
add a `deriving` annotation, like this:

```
#[deriving(Encodable, Decodable)]
struct Foo {
  a : u64,
  b : String,
}
```

which automatically gives `Foo`
the methods
`encode` and `decode`,
allowing translation to and from
JSON, EBML, or any other encoding
that implements the `Encoder` and `Decoder` traits.


In the case of JSON,
this approach has a secondary use case.
For structs, arrays, and primitives,
the mapping between Rust and JSON
is canonical and simple enough
that you can in fact use
`libserialize`'s JSON codec
for communication with externally
defined interfaces,
as when you're constructing the JSON body of
an HTTP request to some server that you don't control.




#### Cap'n Proto

The typical mode of use of Cap'n Proto
follows a different pattern.
We start by defining the types that we
need to be mobile.
For the above example, we would have a
[schema file](http://kentonv.github.io/capnproto/language.html)
containing this definition:

```
struct Foo {
  a @0 : UInt64;
  b @1 : Text;
}
```

We could then use that schema to generate
code in any of the [supported languages](https://kentonv.github.io/capnproto/otherlang.html).
For Rust, this would give us
types named
`Foo::Reader` and `Foo::Builder`
with accessor methods
providing
access to the `a` and `b` fields.
You can think of these readers and builders
as fancy pointers into a byte array
representing an *already serialized* `Foo`.
Cap'n Proto lets us access and modify
these bytes in a way that's nearly
as convenient as accessing and modifying
Rust-native structs.

The chief advantages of Cap'n Proto,
including its high performance and the small size of its generated code,
are only possible because all operations on data are directly backed by
byte arrays in this way.


#### conclusion?

Suppose you've already defined
some Rust data types,
you now want them to be mobile,
and you also want to use Cap'n Proto.
What options are available to you?

You could move the data type definitions into a schema file
and replace all uses in the Rust code
with the generated reader and builder types.
If feasible, this is the way to go,
as it gives you all the benefits that
Cap'n Proto was designed for, including
[backwards compatibility](http://kentonv.github.io/capnproto/language.html#evolving_your_protocol).


It might, however, be too awkward to use the Cap'n Proto
readers and builders everywhere.
An alternative on the opposite side of the
spectrum
would be to
mimic the behavior of the JSON codec.
You could implement `Encoder` and `Decoder`
for a Cap'n Proto schema describing Rust values, as outlined below.

```
struct RustValue {
  union {
    struct  @0 : Struct;
    variant @1 : Variant;
    array   @2 : List(RustValue);
    uint8   @3 : UInt8;
    uint16  @4 : UInt16;
    uint32  @5 : UInt32;
    uint64  @6 : UInt64;
    ...
  }
}

struct Struct {
  fields @0 : List(Field);
}

struct Field {
  name  @0 : Text;
  value @1 : RustValue;
}

struct Variant {
   name  @0 : Text;
   args  @1 : List(RustValue);
}
```

Note that this may or may not actually be
more efficient than the JSON version.


Another option might be to
move the data type definitions into a schema file
but keep the Rust type definitions as well,
and to implement some code generation
for translating between them,
perhaps through the `Encoder` and `Decoder` traits or something similar.
This would preserve some of the advantages
of both approaches, but would likely add considerable complexity.
