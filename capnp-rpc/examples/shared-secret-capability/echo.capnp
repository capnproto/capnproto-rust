@0xbce2b6edc3ea24c9;

interface Echo {
  # simple capnp interface for testing
  echo @0 (message :Text) -> (responseMessage :Text);
}
