### v0.8.2
- Prevent a double-borrow that could happen in rare situations with ForkedPromise.

### v0.8.1
- Fix a possible deadlock.

### v0.8.0
- Drop GJ dependency in favor of futures-rs.
- Fix a bug that could in rare cases cause Disembargo messages to fail with a
  "does not point back to sender" error.

### v0.7.4
- Eliminate some calls to unwrap(), in favor of saner error handling.
- Eliminate dependency on capnp/c++.capnp.

### v0.7.3
- Directly include rpc.capnp and rpc-twoparty.capnp to make the build more robust.

### v0.7.2
- Fix "unimplemented" panic that could happen on certain broken capabilities.

### v0.7.1
- Fix bug where piplining on a method that returned a null capability could cause a panic.
