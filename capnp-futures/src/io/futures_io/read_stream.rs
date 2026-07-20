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

use capnp::{message, Error};
use futures_core::stream::Stream;
use futures_io::AsyncRead;

use crate::io::futures_io::Compat;

/// An incoming sequence of messages.
#[must_use = "streams do nothing unless polled"]
pub struct ReadStream<'a, R>
where
    R: AsyncRead + Unpin,
{
    inner: crate::io::read_stream::ReadStream<'a, Compat<R>>,
}

impl<R> Unpin for ReadStream<'_, R> where R: AsyncRead + Unpin {}

impl<'a, R> ReadStream<'a, R>
where
    R: AsyncRead + Unpin + 'a,
{
    pub fn new(reader: R, options: message::ReaderOptions) -> Self {
        ReadStream {
            inner: crate::io::read_stream::ReadStream::new(Compat::new(reader), options),
        }
    }
}

impl<'a, R> Stream for ReadStream<'a, R>
where
    R: AsyncRead + Unpin + 'a,
{
    type Item = Result<message::Reader<capnp::serialize::OwnedSegments>, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}
