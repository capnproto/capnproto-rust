### v0.7.3
- `capnpc -orust ./foo/bar/baz.capnp` now correctly writes to `./foo/bar/baz_capnp.rs` rather than
  just `./baz_capnp.rs`. If you were depending on the old behavior you can use the `--src-prefix`
  flag for finer-grained control of the output location.

### v0.7.2
- Nicer formatting for floating point literals.

### v0.7.1
- Fix bug that prevented pipelining on an AnyPointer field.
