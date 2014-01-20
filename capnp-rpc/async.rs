/*
 * Copyright (c) 2014, David Renshaw (dwrenshaw@gmail.com)
 *
 * See the LICENSE file in the capnproto-rust root directory.
 */

use extra;
use std::result::Result;

pub struct Promise<T, E> {
    future : extra::future::Future<Result<T, E>>
}

