@0x9fedc87e438cde81;

interface ByteStream {
   write @0 (bytes :Data) -> stream;
   # Writes a chunk.

   end @1 ();
   # Ends the stream.
}

interface Receiver {
   writeStream @0 () -> (stream :ByteStream, sha256 :Data);
   # Uses set_pipeline() to set up `stream` immediately.
   # Actually returns when `end()` is called on that stream.
   # `sha256` is the SHA256 checksum of the received data.
}
