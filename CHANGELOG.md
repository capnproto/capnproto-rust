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
