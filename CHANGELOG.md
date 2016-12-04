### v0.7.5
- Fix bug that prevented compilation of interfaces with generic superclasses.
- More robust error handling.

### v0.7.4
- Deprecate `compile()` and `compile_with_src_prefixes()` in favor of `CompilerCommand`.

### v0.7.3
- `capnpc -orust ./foo/bar/baz.capnp` now correctly writes to `./foo/bar/baz_capnp.rs` rather than
  just `./baz_capnp.rs`. If you were depending on the old behavior you can use the `--src-prefix`
  flag for finer-grained control of the output location.

### v0.7.2
- Nicer formatting for floating point literals.

### v0.7.1
- Fix bug that prevented pipelining on an AnyPointer field.
