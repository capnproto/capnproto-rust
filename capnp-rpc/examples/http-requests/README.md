# Cap'n Proto HTTP example

This is an implementation of a server
that allows HTTP requests to be made through Cap'n Proto interfaces.

Start the server like this:

```
$ cargo run server 127.0.0.1:4000
```

Then start any number of clients like this:

```
$ cargo run client 127.0.0.1:4000
```