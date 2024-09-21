# streaming example

Start the server like this:

```
$ cargo run server 127.0.0.1:5000
```

Then start any number of clients like this:

```
cargo run client 127.0.0.1:5000 1000000 64000
```

The `1000000` argument above means the message
stream will have a total of 1000000 bytes.
The `64000` argument means that the flow control window
will be set to 64000 bytes.
