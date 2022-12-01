# async-byte-channel

A simple implementation of an in-memory channel with
a read end that that implements `AsyncRead`
and a write end that implements `AsyncWrite`.

Intended for usage in tests, in order to
avoid depending on heavier-weight I/O libraries.
