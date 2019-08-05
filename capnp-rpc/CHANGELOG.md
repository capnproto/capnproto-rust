## next

- Export Disconnector struct from capnp_rpc (#140)

## v0.10.0
- Update to Rust 2018.
- Update minimum required rustc version to 1.35.

## v0.9.0
- Remove deprecated items.
- Add ImbuedMessageBuilder to provide functionality that was previously automatically provided
  by capnp::message::Builder.

## v0.8.3
- Add RpcSystem::get_disconnector() method.
- Migrate away from some deprecated futures-rs functionality.

## v0.8.2
- Prevent a double-borrow that could happen in rare situations with ForkedPromise.

## v0.8.1
- Fix a possible deadlock.

## v0.8.0
- Drop GJ dependency in favor of futures-rs.
- Fix a bug that could in rare cases cause Disembargo messages to fail with a
  "does not point back to sender" error.

## v0.7.4
- Eliminate some calls to unwrap(), in favor of saner error handling.
- Eliminate dependency on capnp/c++.capnp.

## v0.7.3
- Directly include rpc.capnp and rpc-twoparty.capnp to make the build more robust.

## v0.7.2
- Fix "unimplemented" panic that could happen on certain broken capabilities.

## v0.7.1
- Fix bug where piplining on a method that returned a null capability could cause a panic.
