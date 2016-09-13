## v0.7.4
- Fix rare case where serialize_packed::read() could fail on valid input.

### v0.7.3
- Get `message::Builder::get_root_as_reader()` to work on empty messages.

### v0.7.2
- Implement `From<std::string::FromUtf8Error>` for `capnp::Error`
- More and better iterators.
