# streaming example

Start the server like this:

```
$ cargo run server 127.0.0.1:5000
```

Then start any number of clients like this:

```
cargo run client 127.0.0.1:5000 1000000 64000
```
