# json_config example

This example shows the minimal flow for a JSON-backed config:

- define a simple config schema in [`app_config.capnp`](app_config.capnp)
- stream [`sample-config.json`](sample-config.json) through a serde deserializer
  directly into the generated Cap'n Proto builder
- assert the loaded values through the generated Cap'n Proto reader
- serialize the generated reader back to JSON and print it

Run it like this:

```sh
cargo run
```

Or pass a different JSON file:

```sh
cargo run -- path/to/config.json
```
