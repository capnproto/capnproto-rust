#[cfg(feature = "build-capnp")]
fn main() {
    cmake::build("capnproto");
}

#[cfg(not(feature = "build-capnp"))]
fn main() {
    if !which::which("capnp").is_ok() {
        panic!(
            "capnp executable not found. install it with your package manager or enable the \
            \"build-capnp\" feature to build it from source"
        );
    }
}
