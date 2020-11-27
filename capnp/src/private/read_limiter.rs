// Copyright (c) 2013-2015 Sandstorm Development Group, Inc. and contributors
// Licensed under the MIT License:
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

#[cfg(feature = "sync_reader")]
pub use sync::ReadLimiter;

#[cfg(feature = "sync_reader")]
mod sync {
    use crate::{Error, Result};
    use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

    pub struct ReadLimiter {
        pub limit: AtomicUsize,
        pub limit_reached: AtomicBool,
    }

    impl ReadLimiter {
        pub fn new(limit: u64) -> ReadLimiter {
            if limit > core::usize::MAX as u64 {
                panic!("traversal_limit_in_words cannot be bigger than core::usize::MAX")
            }

            ReadLimiter {
                limit: AtomicUsize::new(limit as usize),
                limit_reached: AtomicBool::new(false),
            }
        }

        #[inline]
        pub fn can_read(&self, amount: usize) -> Result<()> {
            let limit_reached = self.limit_reached.load(Ordering::Relaxed);
            if limit_reached {
                return Err(Error::failed(format!("read limit exceeded")));
            }

            let prev_limit = self.limit.fetch_sub(amount, Ordering::Relaxed);
            if prev_limit == amount {
                self.limit_reached.store(true, Ordering::Relaxed);
            } else if prev_limit < amount {
                self.limit_reached.store(true, Ordering::Relaxed);
                return Err(Error::failed(format!("read limit exceeded")));
            }

            Ok(())
        }
    }
}

#[cfg(not(feature = "sync_reader"))]
pub use unsync::ReadLimiter;

#[cfg(not(feature = "sync_reader"))]
mod unsync {
    use crate::{Error, Result};
    use core::cell::Cell;

    pub struct ReadLimiter {
        pub limit: Cell<u64>,
    }

    impl ReadLimiter {
        pub fn new(limit: u64) -> ReadLimiter {
            ReadLimiter {
                limit: Cell::new(limit),
            }
        }

        #[inline]
        pub fn can_read(&self, amount: usize) -> Result<()> {
            let amount = amount as u64;
            let current = self.limit.get();
            if amount > current {
                Err(Error::failed(format!("read limit exceeded")))
            } else {
                self.limit.set(current - amount);
                Ok(())
            }
        }
    }
}
