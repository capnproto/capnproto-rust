---
layout: post
title: custom mutable reference types
author: dwrensha
---

Rust's *mutable references* provide exclusive access to writable locations in memory.
If we have `x : &'a mut T`,
then we know that the referred-to `T`
cannot be read or modified except through dereferencing `x`.
In other words, `x` can have no aliases.
This guarantee is
crucial for memory safety,
as it implies that
any mutations we apply through
`x` have no risk of invalidating references held
elsewhere in our program.


The `Builder` types
of [capnproto-rust](https://github.com/dwrensha/capnproto-rust)
also need to provide an exclusivity guarantee.
Recall that if `Foo` is a struct defined in a Cap'n Proto schema,
then a `foo::Builder<'a>`
provides access to a writable location
in arena-allocated memory that contains
a `Foo` in [Cap'n Proto format](https://kentonv.github.io/capnproto/encoding.html).
To protect access to that memory, a `foo::Builder<'a>` ought to behave
as if it were a `&'a mut Foo`,
even though the `Foo` type
cannot directly exist in Rust
(because Cap'n Proto struct layout
differs from Rust struct layout).

So the question arises: how do we define custom mutable references?

As we'll see, the easy part is ensuring exclusive access,
which can be achieved simply by not
implementing the `Copy` trait for `foo::Builder<'a>`.
The tricky part is making it ergonomic to reuse a reference,
something that built-in mutable references achieve through
special *automatic reborrowing* semantics. Our custom references
can use similar semantics, but they need to be slightly more explicit about it.

Okay, let's get concrete. Suppose that `Foo` is defined in a Cap'n Proto schema like this:

```
struct Foo {
  x @0 : Float32;
  blob @1 : Data;
}
```

When we call `capnp compile -orust foo.capnp`, we get generated code
containing the following definitions:

```
mod foo {
  pub struct Builder<'a> {...}

  impl <'a> Builder<'a> {
    pub fn get_x(self) -> f32 {...}
    pub fn set_x(&mut self, value : f32) {...}
    pub fn get_blob(self) -> ::capnp::data::Builder<'a> {...}
    pub fn set_blob(&mut self, value : ::capnp::data::Reader) {...}
    pub fn init_blob(self, length : u32) -> ::capnp::data::Builder<'a> {...}
    ...
  }
  ...
}
```

You see here the usual accessor methods that allow us to
read and modify a `Foo`.
Note that the `get_` and `init_` methods take a by-value `self`
parameter.
This ensures that at most one `::capnp::data::Builder` referring to the `blob` field
can be obtained.
For example, if we call `foo.init_blob()` then we cannot later call `foo.get_blob()`,
because `foo` *moves into* the first call
and cannot be used again.
As the `::capnp::data::Builder<'a>` type is in fact just a typedef for `&'a mut [u8]`,
it should be extra clear here why exclusivity is important to maintain.


One thing we might do with these accessors is
initialize the `Foo` and return a reference to its interior,
as does this function:

```
fn init_and_return_slice<'a>(foo : foo::Builder<'a>) -> &'a mut [u8] {
    foo.init_blob(100).slice_mut(5, 10)
}
```






But what if we want to call this function and
then afterwards call `set_x()`?
We might write something like this:

```
fn do_some_things_wrong<'a>(mut foo : foo::Builder<'a>) {
   {
     let slice = init_and_return_slice(foo);
     slice[0] = 42;
   }
   foo.set_x(1.23);
}
```
but if we try to compile this function, we get the following typecheck error:

```
main.rs:19:9: 19:12 error: use of moved value: `foo`
main.rs:19         foo.set_x(1.23);
                   ^~~
```

The same pass-by-move semantics that were essential to preventing
aliasing have now become a problem.
We would like to be
able to borrow `foo` for just the inner block,
and then reuse it for the final line.
If `foo` were a built-in mutable reference, such a *reborrow*
would take place automatically, and everything would just work.
Fortunately, we can make do with our custom mutable reference
if we use the following following function,
which is also included in the generated code:

```
mod foo {
  ...
  impl <'a> Builder <'a> {
    pub fn borrow<'b>(&'b mut self) -> Builder<'b> { ... }
  }
  ...
}
```

Using this, we can write our function as follows, and it successfully typechecks.

```
fn do_some_things_right<'a>(mut foo : foo::Builder<'a>) {
    {
        let slice = init_and_return_slice(foo.borrow());
        slice[0] = 42
    }
    foo.set_x(1.23);
}
```

So it appears that the main inconviences of using our custom mutable references
compared to built-in mutable references
is that we need to add some calls to `.borrow()` and maybe add some `mut`'s to some bindings.
In fact, it seems to me that it would be possible for Rust to support
a built-in `Reborrow` trait that could eliminate even these
inconveniences.


Finally, in case you're wondering why we prefer by-move `self` over `&mut self`
in our generated accessor methods, suppose that we also define this type in our schema:

```
struct Bar {
  oneFoo @0 : Foo;
}
```


Using by-move `self` allows us to return references deep in the interior of a `Bar`, like this:

```
fn init_field_and_return_slice<'a>(bar : bar::Builder<'a>) -> &'a mut [u8] {
    bar.init_one_foo().init_blob(100).slice_mut(5, 10)
}
```

If `init_blob()` instead took a `&mut self` parameter, this function would fail to typecheck
because the `foo::Builder` returned by `bar.init_one_foo()` does not live long enough.

