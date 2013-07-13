use std;
use common::*;
use endian::*;
use message::*;
use serialize::*;

pub struct PackedOutputStream {
    inner : @OutputStream
}


impl OutputStream for PackedOutputStream {
    pub fn write(@self, buf : &[u8]) {
        // TODO
        self.inner.write(buf)
    }
}

pub fn writePackedMessage(outputStream : @OutputStream,
                          message : &MessageBuilder) {
    let packedOutputStream = @PackedOutputStream {inner : outputStream} as @OutputStream;
    writeMessage(packedOutputStream, message);
}