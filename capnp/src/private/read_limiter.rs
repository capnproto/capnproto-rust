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
    use crate::{Error, ErrorKind, Result};
    use core::sync::atomic::{AtomicUsize, Ordering};

    pub struct ReadLimiter {
        limit: AtomicUsize,
        error_on_limit_exceeded: bool,
    }

    impl ReadLimiter {
        pub fn new(limit: Option<usize>) -> Self {
            match limit {
                Some(value) => Self {
                    limit: AtomicUsize::new(value),
                    error_on_limit_exceeded: true,
                },
                None => Self {
                    limit: AtomicUsize::new(usize::MAX),
                    error_on_limit_exceeded: false,
                },
            }
        }

        #[inline]
        pub fn can_read(&self, amount: usize) -> Result<()> {
            // We use separate AtomicUsize::load() and AtomicUsize::store() steps, which may
            // result in undercounting reads if multiple threads are reading at the same.
            // That's okay -- a denial of service attack will eventually hit the limit anyway.
            //
            // We could instead do a single fetch_sub() step, but that seems to be slower.

            let current = self.limit.load(Ordering::Relaxed);
            if amount > current && self.error_on_limit_exceeded {
                return Err(Error::from_kind(ErrorKind::ReadLimitExceeded));
            } else {
                // The common case is current >= amount. Note that we only branch once in that case.
                // If we combined the fields into an Option<AtomicUsize>, we would
                // need to branch twice in the common case.
                self.limit
                    .store(current.wrapping_sub(amount), Ordering::Relaxed);
            }
            Ok(())
        }
    }
}

#[cfg(not(feature = "sync_reader"))]
pub use unsync::ReadLimiter;

#[cfg(not(feature = "sync_reader"))]
mod unsync {
    use crate::{Error, ErrorKind, Result};
    use core::cell::Cell;

    pub struct ReadLimiter {
        limit: Cell<usize>,
        error_on_limit_exceeded: bool,
    }

    impl ReadLimiter {
        pub fn new(limit: Option<usize>) -> Self {
            match limit {
                Some(value) => Self {
                    limit: Cell::new(value),
                    error_on_limit_exceeded: true,
                },
                None => Self {
                    limit: Cell::new(usize::MAX),
                    error_on_limit_exceeded: false,
                },
            }
        }

        #[inline]
        pub fn can_read(&self, amount: usize) -> Result<()> {
            let current = self.limit.get();
            if amount > current && self.error_on_limit_exceeded {
                Err(Error::from_kind(ErrorKind::ReadLimitExceeded))
            } else {
                // The common case is current >= amount. Note that we only branch once in that case.
                // If we combined the fields into an Option<Cell<usize>>, we would
                // need to branch twice in the common case.
                self.limit.set(current.wrapping_sub(amount));
                Ok(())
            }
        }
    }
}
