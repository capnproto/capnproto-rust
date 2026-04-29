@0xd8c625bf1f3f8cf4;

interface ByteStream {
  write @0 (bytes :Data) -> stream;
  end @1 ();
}

interface Transfer {
  wait @0 () -> (sha256 :Data);
}

interface Sender {
  send @0 (stream :ByteStream, size :UInt64, chunkSize :UInt32) -> (transfer :Transfer);
}
