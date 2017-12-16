// Usually 'cargo test' tries to build everything in the examples directory.
// That doesn't work for our examples because they have their own Cargo.toml and build.rs files.
// We can override the default behavior of 'cargo test' by supplying a dummy file, like this,
// and pointing to it in our Cargo.toml's [[example]] section.

fn main() {}
