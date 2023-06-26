# capnp-import

Rust library for fetching the official Cap-n-Proto compiler (capnp) for a particular operating system, compiling files and aggregating their output into the place specified by macro.

`capnp-import` builds a set of paths to files or folders using the capnp tool, which it downloads or builds if it is missing, and aggregates the resulting import files into a helper include file. Usage:

``` toml
# Inside Cargo.toml
[dependencies]
capnp-import = "0.2"
```
```rust
// Inside main.rs
    capnp_import::capnp_import!("schema/*.capnp");
```

A release archive for the given version for the current operating system will be downloaded and the binary will be extracted into the target directory. If a particular version was already downloaded and is present in the target directory, it will be reused. If no binary is available, a source release will be downloaded and a build will be attempted. If this isn't supported, the tool will try to use an existing capnp installation on the machine.
