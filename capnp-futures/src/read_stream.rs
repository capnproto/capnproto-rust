// Copyright (c) 2016 Sandstorm Development Group, Inc. and contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use std::pin::Pin;
use std::task::{Context, Poll};
use futures::future::Future;
use futures::stream::Stream;
use futures::{AsyncRead};

use capnp::{Error, message};

#[must_use = "streams do nothing unless polled"]
pub struct ReadStream<R> where R: AsyncRead + Unpin {
    options: message::ReaderOptions,
    read: Pin<Box<crate::serialize::Read<R>>>,
}

impl <R> Unpin for ReadStream<R> where R: AsyncRead + Unpin {}

impl <R> ReadStream<R> where R: AsyncRead + Unpin {
    pub fn new(reader: R, options: message::ReaderOptions) -> ReadStream<R> {
        ReadStream {
            read: Box::pin(crate::serialize::read_message(reader, options)),
            options: options,
        }
    }
}

impl <R> Stream for ReadStream<R> where R: AsyncRead + Unpin {
    type Item = Result<message::Reader<crate::serialize::OwnedSegments>, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let (r, m) = match Future::poll(Pin::new(&mut self.read), cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e))),
            Poll::Ready(Ok(x)) => x,
        };
        self.read = Box::pin(crate::serialize::read_message(r, self.options));
        match m {
            Some(message) => Poll::Ready(Some(Ok(message))),
            None => Poll::Ready(None),
        }
    }
}
