---
layout: post
title: lifetime variables and safety
author: dwrensha
---

Like its C++ counterpart,
capnproto-rust relies heavily
on raw pointer manipulation
to achieve good performance.
In fact,
the translation
from C++ to Rust
for the core low level
data structures
is quite direct,
as you can see by comparing
layout.c++ and layout.rs.

One important  difference is that
the Rust language, in addition to
requiring that we explicitly
label the pointer manipulations as "unsafe",
also provides us
with better facilities
for building
a safe external interface
on top of them.

To illustrate, let's look at some
ways one might misuse the C++ interface.

Here's a program that builds two
`Person` structs, each a part of
its own `AddressBook` message.

```
#include "addressbook.capnp.h"
#include <capnp/message.h>
#include <iostream>

using addressbook::Person;
using addressbook::AddressBook;

Person::Builder returnPersonBuilder(int id) {
  ::capnp::MallocMessageBuilder message;

  auto addressBook = message.initRoot<AddressBook>();
  auto people = addressBook.initPeople(1);

  people[0].setId(id);
  people[0].setName("Alice");

  return people[0];
}

int main(int argc, char* argv[]) {
  auto person1 = returnPersonBuilder(123);
  auto person2 = returnPersonBuilder(456);
  std::cout << person1.getId() << "\n";
  return 0;
}

```

You might expect the program to print
"123", but it actually prints "456".
The problem is that the `Person::Builder` returned
by the `returnPersonBuilder()` function
is unsafe to use because it
outlives its `MessageBuilder`.

Here is a snippet showing a related problem.

```
{
  ::capnp::MallocMessageBuilder message;

  auto addressBook = message.initRoot<AddressBook>();
  auto people = addressBook.initPeople(1);

  auto alice = people[0];
  alice.setId(123);

  auto person = message.initRoot<Person>();

  std::cout << alice.getId() << "\n";
}
```
You might expect this code to print "123", but
it actually prints "0" because `alice`
is no longer valid after `message` has
been initialized a second time.

Both of these errors could be statically
detected and prevented in Rust.
The key is to arrange that the
`MessageBuilder::initRoot()` function
*borrow* a reference to the message that invokes it,
and to keep track of the *lifetime* of that borrow.
The Rust typechecker will then be able to detect
if the message is borrowed again
or if some sub-builder of it---whose type will
be annotated with the lifetime---outlives the
lifetime.

To make this concrete, in Rust
the signature of `MessageBuilder::initRoot` could look something like this:

```
pub fn initRoot<'a, T : FromStructBuilder<'a>>(&'a mut self) -> T;

```

where `FromStructBuilder` is the trait

```
pub trait FromStructBuilder<'a> {
    fn fromStructBuilder(structBuilder : StructBuilder<'a>) -> Self;
}
```
and `StructBuilder` is a low-level type
which should not be exposed to the safe user interface.
Here `'a` is the lifetime variable that tracks the borrow
of the message builder.
The generated code for `AddressBook` and `Person` will then
contain implementations for
the `FromStructBuilder` trait:

```
impl <'a> FromStructBuilder<'a> for AddressBook::Builder<'a> { ... }
impl <'a> FromStructBuilder<'a> for Person::Builder<'a> { ... }
```

Unfortunately, there's one hitch: the Rust compiler does not yet
quite support this kind of interplay between lifetime
variables and traits.
This is why I have been so eagerly watching
Rust issues [5121](https://github.com/mozilla/rust/issues/5121),
[7331](https://github.com/mozilla/rust/issues/7331), and
[10391](https://github.com/mozilla/rust/issues/10391).

In the meantime, capnproto-rust
does partially enforce the kind of lifetime safety
described above, but only for message readers, not message builders,
and only using a somewhat roundabout strategy that makes it
awkward to support the more complex
Cap'n Proto types like lists of lists.

