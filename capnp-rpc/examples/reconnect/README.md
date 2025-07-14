# Reconnect Example

This demontrates how auto_reconnect is used.

To run, in two separate terminals, do:

```
$ cargo run server 127.0.0.1:4000
```

and

```
$ cargo run client 127.0.0.1:4000
```

The client should now output:

```
Connected (manual=false)
results = 123
We were told to crash!
Connected (manual=false)
results = 124
We were told to crash!
Connected (manual=true)
results = 125
Shutting down
```