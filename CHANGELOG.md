## v0.8.5
- Eliminate possible void-list-amplification in total_size().

## v0.8.4
- Eliminate panics in total_size() and set_root().
- Eliminate possible void-list-amplification in zero_object_helper().

## v0.8.3
- Prevent integer overflow possible with very long struct lists on 32-bit systems.
- Fix bug where the capnp_word!() macro was not exported for big endian targets.

## v0.8.2
- Shave some bytes off the representation of StructReader and friends.
- Fix some potential integer overflows.

## v0.8.1
- Redesign segment arenas to require less unsafe code.

## v0.8.0
- Replace optional GJ dependency with futures-rs.
- Remove `ResultsDoneHook` hack.
- No breaking changes for non-RPC users.

## v0.7.5
- Implement DoubleEndedIter for ListIter.
- Implement From<std::str::Utf8Error> for ::capnp::Error.
- Address some new linter warnings.

## v0.7.4
- Fix rare case where `serialize_packed::read()` could fail on valid input.

### v0.7.3
- Get `message::Builder::get_root_as_reader()` to work on empty messages.

### v0.7.2
- Implement `From<std::string::FromUtf8Error>` for `capnp::Error`
- More and better iterators.
