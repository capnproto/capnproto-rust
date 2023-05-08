---
layout: post
title: run-time reflection
author: dwrensha
---



Version 0.17 of
[capnproto-rust](https://github.com/capnproto/capnproto-rust)
is now available!
It introduces support for dynamically typed values,
enabling a kind of run-time reflection.
The new functionality lets you write
ordinary Rust code to
iterate through the fields of any Cap'n Proto
[struct](https://capnproto.org/language.html#structs),
with access to  names, types, and
[annotations](https://capnproto.org/language.html#annotations).
Previously, you would have needed to use code generation or procedural
macros to achieve such things.

This blog post will show two particular applications of reflection:
debug printing and generating random structured data.
The former has until now been sorely missing in capnproto-rust,
and the latter can yield (pretty?) pictures like this:

<img width="350" src="{{site.baseurl}}/assets/shapes-000.svg"
 alt = "colorful randomly generated shapes" />

The post will also look at some details
of how the implementation of reflection works.

## Debug Printing

A major motivation for adding reflection to capnproto-rust
is to implement the [`Debug`](https://doc.rust-lang.org/core/fmt/trait.Debug.html)
trait for all Cap'n Proto structs.
Because the data for such structs lives behind
a layer of indirection
(i.e. [synthetic references]({{site.baseurl}}/2014/12/27/custom-mutable-references.html)),
merely adding a `#[derive(Debug)]` annotation to the type declarations
would not be sufficient to print any useful information.
Instead, we need to add custom `Debug` implementations
that read the underlying data.

One way to achieve that would be to generate,
for each struct, separate implementation logic
which iterates through the struct fields and prints their values in sequence.
That's the approach proposed by [@as-com](https://github.com/as-com)
in pull request [#390](https://github.com/capnproto/capnproto-rust/pull/390).

However, now that we have reflection, we can avoid generating so much single-purpose
code. Instead, our `Debug` implementation for each struct immediately
delegates to a shared implementation that knows how to deal with
*any* Cap'n Proto struct type. See
[stringify.rs](https://github.com/capnproto/capnproto-rust/blob/f7c86befe11b27f33c2a45957d402abff2b9e347/capnp/src/stringify.rs)
if you are curious about what the code looks like.

Using reflection in this way does have a (small)
run-time cost, as it requires
more branching than the static approach
implemented in [#390](https://github.com/capnproto/capnproto-rust/pull/390).
However, because it improves maintainability and
reduces code bloat (and therefore compile times!),
the cost seems worth paying.

To see the new `Debug` functionality in action,
suppose that we have the following schema for an address book:

```
struct Person {
  id @0 :UInt32;
  name @1 :Text;
  email @2 :Text;
  phones @3 :List(PhoneNumber);

  struct PhoneNumber {
    number @0 :Text;
    type @1 :Type;

    enum Type {
      mobile @0;
      home @1;
      work @2;
    }
  }

  employment :union {
    unemployed @4 :Void;
    employer @5 :Text;
    school @6 :Text;
    selfEmployed @7 :Void;
  }
}

struct AddressBook {
  people @0 :List(Person);
}
```

If `address_book` is a value of this type,
then we can print it with
```rust
println!("{:?}", address_book)
```
and we get:

```
(people = [(id = 123, name = "Alice", email = "alice@example.com", phones = [(number = "555-1212", type = mobile)], employment = (school = "MIT")), (id = 456, name = "Bob", email = "bob@example.com", phones = [(number = "555-4567", type = home), (number = "555-7654", type = work)], employment = (unemployed = ()))])
```

The format here is the standard [capnproto text format](https://github.com/capnproto/capnproto/blob/b2afb7f8fe393466a38e2fd2ad98482c34aafcee/c%2B%2B/src/capnp/serialize-text.h#L34-L40).
We can make it more readable via the ["alternate"](https://doc.rust-lang.org/std/fmt/struct.Formatter.html#method.alternate) flag.
If we print it with
```rust
println!("{:#?}", address_book)
```
then we get:
```
(
  people = [
    (
      id = 123,
      name = "Alice",
      email = "alice@example.com",
      phones = [
        (
          number = "555-1212",
          type = mobile
        )
      ],
      employment = (
        school = "MIT"
      )
    ),
    (
      id = 456,
      name = "Bob",
      email = "bob@example.com",
      phones = [
        (
          number = "555-4567",
          type = home
        ),
        (
          number = "555-7654",
          type = work
        )
      ],
      employment = (
        unemployed = ()
      )
    )
  ]
)
```

## Filling in Random Values

Reflection also makes it easy to generate random values of any
Cap'n Proto type, as might be useful
in various kinds of testing.
The new directory
[fill_random_values](https://github.com/capnproto/capnproto-rust/tree/master/example/fill_random_values)
contains some example code
illustrating this idea.


### Random Addressbook

If we take the address book schema discussed above
and plug it into `fill_random_values`,
the output looks like this:


```
(
  people = [
    (
      id = 640675312,
      name = "i",
      email = "npcvojhliloc",
      phones = [
        (
          number = "y",
          type = mobile
        ),
        (
          number = "mfqhbgmtgmbkyslpw",
          type = work
        ),
        (
          number = "",
          type = home
        ),
        (
          number = "yi",
          type = work
        ),
        (
          number = "vgcqfrhqlparbptuwu",
          type = home
        ),
        (
          number = "qkhyxjplpufjlxknp",
          type = mobile
        ),
        (
          number = "oyenjhvaikluhpoedkj",
          type = work
        ),
        (
          number = "y",
          type = work
        )
      ],
      employment = (
        unemployed = ()
      )
    ),
    (
      id = 3188155808,
      name = "mpe",
      email = "vgqcfacrnhqrqxe",
      phones = [],
      employment = (
        employer = "aobikqcv"
      )
    ),
    ...
  ]
)
```

That's definitely some random gibberish!

To make the output more "realistic",
we can constrain the values
of the fields using some annotations from `fill.capnp`.
We might mark up the schema like this:

```
using Fill = import "fill.capnp";
using Corpora = import "corpora.capnp";

struct Person {
  id @0 :UInt32;
  name @1 :Text $Fill.SelectFrom(List(Text)).choices(Corpora.scientists);
  email @2 :Text $Fill.SelectFrom(List(Text)).choices(Corpora.emails);
  phones @3 :List(PhoneNumber) $Fill.lengthRange((max = 3));

  struct PhoneNumber {
    number @0 :Text $Fill.phoneNumber;
    type @1 :Type;

    enum Type {
      mobile @0;
      home @1;
      work @2;
    }
  }

  employment :union {
    unemployed @4 :Void;
    employer @5 :Text $Fill.SelectFrom(List(Text)).choices(Corpora.corporations);
    school @6 :Text $Fill.SelectFrom(List(Text)).choices(Corpora.schools);
    selfEmployed @7 :Void;
  }
}

struct AddressBook {
  people @0 :List(Person) $Fill.lengthRange((max = 5));
}
```

Annotations are signified by the dollar sign `$`. For example,
the line
```
  phones @3 :List(PhoneNumber) $Fill.lengthRange((max = 3));
```
tells the `fill_random_values` library
that the value in the `phones` field should have length at most 3,
and the line
```
  name @1 :Text $Fill.SelectFrom(List(Text)).choices(Corpora.scientists);
```
indicates that the value for the `name`
field should be chosen the list of choices given by the `Corpora.scientists` constant,
which is defined in the schema file `corpora.capnp`.

Here is some example output after these constraints are applied:

```
(
  people = [
    (
      id = 2252764513,
      name = "Willard Gibbs",
      email = "c@example.com",
      phones = [
        (
          number = "985-555-1858",
          type = mobile
        ),
        (
          number = "558-555-1461",
          type = work
        ),
        (
          number = "585-555-1163",
          type = home
        )
      ],
      employment = (
        school = "Penn State"
      )
    ),
    (
      id = 2222057070,
      name = "Joseph Priestley",
      email = "carol@example.com",
      phones = [
        (
          number = "149-555-1350",
          type = work
        ),
        (
          number = "685-555-1721",
          type = home
        ),
        (
          number = "818-555-1428",
          type = home
        )
      ],
      employment = (
        employer = "LKQ Corporation"
      )
    )
  ]
)

```

That looks better!
You can imagine that maybe the code you are testing
does some basic validation on its inputs,
and that values generated in this way are more likely to pass the validation,
and therefore can achieve better code coverage.

### Random Shapes

To illustrate some of the other capabilities of `fill_random_values`,
here is a schema describing a recursive geometric grammar,
with `fill_random_values` annotations already added:

```
using Fill = import "fill.capnp";

struct Color {
  red   @0 : UInt8;
  green @1 : UInt8;
  blue  @2 : UInt8;
}

struct Point {
  # A point in normalized coordinates. (0,0) is the upper-left of
  # the current subcanvas, and (1,1) is the lower-right of the
  # current subcanvas.

  x @0 : Float64 $Fill.float64Range((min = 0.0, max = 1.0));
  y @1 : Float64 $Fill.float64Range((min = 0.0, max = 1.0));
}

struct Line {
  start @0 : Point;
  end   @1 : Point;

  thickness @2 : Float64 $Fill.float64Range((min = 0.01, max = 0.95));
  # Stroke width as a percent of the current subcanvas's diagonal length.

  color @3 : Color;
}

struct Circle {
  center @0 : Point;
  # The center of the circle.

  radius @1 : Float64 $Fill.float64Range((min = 0.01, max = 0.25));
  # The radius of the circle, as a proportion of the current
  # subcanvas's diagonal length.

  fillColor @2 : Color;
}

struct Subcanvas {
  # A canvas contained in a larger canvas.

  center @0 : Point;
  width @1 : Float64 $Fill.float64Range((min = 0.0, max = 1.0));
  height @2 : Float64 $Fill.float64Range((min = 0.0, max = 1.0));
  canvas @3 : Canvas;
}

struct Canvas {
  # A canvas containing some geometric elements.

  backgroundColor @0 : Color;
  lines @1 : List(Line) $Fill.lengthRange((max = 5));
  circles @2 : List(Circle) $Fill.lengthRange((max = 5));
  subcanvases @3 : List(Subcanvas) $Fill.lengthRange((max = 3));
}
```

We can pass this to `fill_random_values`
and then render the output as SVGs, yielding:

|  |  |  |
| <img width="195" src="{{site.baseurl}}/assets/shapes-001.svg"/> | <img width="195" src="{{site.baseurl}}/assets/shapes-002.svg"/> | <img width="195" src="{{site.baseurl}}/assets/shapes-003.svg"/> |
| <img width="195" src="{{site.baseurl}}/assets/shapes-004.svg"/> | <img width="195" src="{{site.baseurl}}/assets/shapes-005.svg"/> | <img width="195" src="{{site.baseurl}}/assets/shapes-006.svg"/> |
| <img width="195" src="{{site.baseurl}}/assets/shapes-007.svg"/> | <img width="195" src="{{site.baseurl}}/assets/shapes-008.svg"/> | <img width="195" src="{{site.baseurl}}/assets/shapes-009.svg"/> |

I think these are fun! But the colors are maybe a bit too wild for my taste.
What if we fixed a color palette?
To that end, we define a constant:

```
const palette: List(Color) = [
 (red = 0x22, green = 0xd7, blue = 0xb5),
 (red = 0x11, green = 0xb1, blue = 0x86),
 (red = 0x7c, green = 0xa4, blue = 0xf5),
 (red = 0xe7, green = 0x60, blue = 0x1d),
 (red = 0x25, green = 0x23, blue = 0x25),
 (red = 0x89, green = 0x74, blue = 0x59),
];
```
and then we annotate the color fields like this:
```
  color @3 : Color $Fill.SelectFrom(List(Color)).choices(.palette);
```

Now the output, with constrained colors, looks like:

|  |  |  |
| <img width="195" src="{{site.baseurl}}/assets/shapes-palette-001.svg"/> | <img width="195" src="{{site.baseurl}}/assets/shapes-palette-002.svg"/> | <img width="195" src="{{site.baseurl}}/assets/shapes-palette-003.svg"/> |
| <img width="195" src="{{site.baseurl}}/assets/shapes-palette-004.svg"/> | <img width="195" src="{{site.baseurl}}/assets/shapes-palette-005.svg"/> | <img width="195" src="{{site.baseurl}}/assets/shapes-palette-006.svg"/> |
| <img width="195" src="{{site.baseurl}}/assets/shapes-palette-007.svg"/> | <img width="195" src="{{site.baseurl}}/assets/shapes-palette-008.svg"/> | <img width="195" src="{{site.baseurl}}/assets/shapes-palette-009.svg"/> |

Now that's starting to look like something I might frame and hang on a wall.

## How It Works

The implementation of reflection
in capnproto-rust largely follows
the original implementation of reflection
in [capnproto-c++](https://github.com/capnproto/capnproto).
The main idea is to
stash type descriptions in the generated code.

The schema compiler plugin (capnpc-rust or capnpc-c++)
receives its input as a
[`CodeGeneratorRequest`](https://github.com/capnproto/capnproto/blob/b2afb7f8fe393466a38e2fd2ad98482c34aafcee/c%2B%2B/src/capnp/schema.capnp#L498-L542)
Cap'n Proto message,
containing a list of
[`Node`](https://github.com/capnproto/capnproto/blob/b2afb7f8fe393466a38e2fd2ad98482c34aafcee/c%2B%2B/src/capnp/schema.capnp#L30-L199)
values that describe all of the user-declared types.
To support reflection, we save these `Node` values
as static constants, and we make them available (in Rust) via
an `Introspect` trait:

```rust
pub trait Introspect {
    fn introspect() -> Type;
}
```

This `Type` type represents a recursive data structure that can describe
any [Cap'n Proto type](https://capnproto.org/language.html#language-reference).
Usually, recursive data structures require some form of heap allocation
to avoid having infinite size.
In this case, however, we achieve the necessary indirection
by holding static references to the `Node` values in the generated code.
This allows the entire reflection system to work
without needing any heap allocation.

(Note that this setup implies that reflection is only possible on types
that are known at compile time.
The C++ implementation does offer further support for registering new types at
run time, but adding such support in Rust would require a significant amount of
additional effort.)

One tricky of all this is
the fact that Cap'n Proto has
[generic types](https://capnproto.org/language.html#generic-types).
That is, structs can have type parameters.
We need to be able to retrieve information about such structs
*after applying type substitution* for those parameters.

The C++ implementation
has a [scary comment](https://github.com/capnproto/capnproto/blob/b2afb7f8fe393466a38e2fd2ad98482c34aafcee/c%2B%2B/src/capnp/raw-schema.h#L40-L42)
about how it solves this problem:

```c++
// Note that while we generate one `RawSchema` per type, we generate a
// `RawBrandedSchema` for every _instance_ of a generic type -- or, at
// least, every instance that is actually used. For generated-code types,
// we use template magic to initialize these.
```

Rust most assuredly does not have "template magic",
so it's not immediately clear how to solve the equivalent problem in Rust.

Fortunately,
while Rust does not support type-parameterized static variables,
it does support type-parameterized functions,
and we can push type resolution logic into a
*function* generated for every Cap'n Proto struct type:

```rust
pub fn get_field_types<T1, T2, ...>(field_index: u16) -> introspect::Type {
...
}
```

When you retrieve a field of a dynamic struct in capnproto-rust,
the implemention will call the underlying `get_field_types()` method
to retrieve the type of the field.
It will then return to you a `dynamic_value::Reader` tagged with that type.


## Ideas for Future Projects

Reflection has a wide range of possible applications,
including:

  * Automatic conversion between Cap'n Proto data and
various self-describing formats such JSON and XML.
  * Structure-aware fuzz testing and fault injection.
  * Database adapters.
  * Analysis tools for structured logs.
  * Conversion between Cap'n Proto data and native Rust structs, perhaps using some of the existing or future reflection features described in this
[recent Shepherd's Oasis blog post](https://soasis.org/posts/a-mirror-for-rust-a-plan-for-generic-compile-time-introspection-in-rust/).

I'm excited to see what users come up with!
