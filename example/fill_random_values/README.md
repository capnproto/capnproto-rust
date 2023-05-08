# fill_random_values example

This example demonstrates one possible use of runtime type instropection.

The library code in `src/lib.rs` can be used to fill in random data for any Cap'n Proto
type. The annotations in `fill.capnp` can be used to add contraints on
the random generation.

See `fill_addressbook.rs` and `fill_shapes.rs` for example usage.

Here are some example random images output from `fill_shapes.rs`:

![shapes1](shapes1.png)
![shapes2](shapes2.png)
