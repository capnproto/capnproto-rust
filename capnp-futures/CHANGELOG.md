## v0.25.0
- Follow v0.25.0 release of other capnp crates.

## v0.24.0
- Follow v0.24.0 release of other capnp crates.

## v0.23.1
- Fix capnp dependency to be capnp-v0.23.0 rather than capnp-v0.23.0-alpha.

## v0.23.0
- Follow v0.23.0 release of other capnp crates.

## v0.22.0
- Follow v0.22.0 release of other capnp crates.

## v0.21.0
- Follow v0.21.0 release of other capnp crates.

## v0.20.1
- Remove dependency on futures crate in favor of lighter-weight dependencies
  on futures-channel and futures-util.

## v0.20.0
- write_queue objects should be Send now, when appropriate.
- Follow v0.20.0 release of other capnp crates.

## v0.19.1
- Fix bug in `write_queue::len()`.

## v0.19.0
- Follow v0.19.0 release of other capnp crates.

## v0.18.2
- Fix overflow bug in read_message that could potentially lead to denial of service
  attacks on 32-bit targets.

## v0.18.1
- Fix two bugs in serialize_packed::PackedRead where a premature end-of-file
  could trigger an infinite loop.

## v0.18.0
- Follow v0.18.0 release of other capnp crates.

## v0.17.0
- Follow v0.17.0 release of other capnp crates.

## v0.16.0
- Follow v0.16.0 release of other capnp crates.

## v0.15.1
- Fill in unimplemented len() method of write_queue::Sender.
- Add is_empty() method to write_queue::Sender.
- Apply a bunch of formatting and style fixes that should have no observable effects.

## v0.15.0
- Follow v0.15.0 release of other capnp crates.

## v0.14.2
- Add serialize_packed module.

## v0.14.1
- Include LICENSE in published crate.

## v0.14.0
- Make `read_message()` return an error on EOF, to match the behavior of `capnp::serialize::read_message()`.

## v0.13.2
- Rename `read_message()` to `try_read_message()`, for consistency with `capnp::serialize::try_read_message()`.

## v0.13.1
- Remove unneeded dependency on 'executor' feature of the future crate.

## v0.13.0
- Remove some requirements for 'static lifetimes.

## v0.12.0
- Use new capnp::serialize::SegmentLengthsBuilder API.

## v0.11.0
- Remove serialize::Transport.
- Switch to std::future::Future.
- Bump minimum supported rustc version to 1.39.0.

## v0.10.1
- Remove dependency on byteorder crate, in favor of from_le_bytes() and to_le_bytes().

## v0.10.0
- Update to 2018 edition.
- Update minimum required rustc version to 1.35.

## v0.9.1
- Call flush() after writing each message, to allow usage with a std::io::BufWriter wrapper.

## v0.9.0
- No changes -- just a version bump to match the rest of the capnp crates.

## v0.1.1
- Add `serialize::Transport`.
- Update byteorder dependency.

## v0.1.0
- Add `WriteQueue`.

## v0.0.2
- Add `ReadStream`.

## v0.0.1
- Code pulled in from https://github.com/dwrensha/capnproto-rust/pull/66.
