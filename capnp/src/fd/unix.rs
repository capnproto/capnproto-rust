// Copyright (c) 2026 Sandstorm Development Group, Inc. and contributors
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

use std::os::fd::{AsRawFd, RawFd};
pub use std::os::fd::{AsFd, BorrowedFd, OwnedFd};

use crate::private::capability::ClientHook;

#[derive(Default)]
pub struct FdHooks {
    hooks: Vec<Box<dyn ClientHook>>,
    fds: Vec<RawFd>,
}

impl FdHooks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn try_push(
        &mut self,
        hook: Box<dyn ClientHook>,
    ) -> Result<(u8, &dyn ClientHook), Box<dyn ClientHook>> {
        let Ok(index) = self.fds.len().try_into() else {
            return Err(hook);
        };
        if index == u8::MAX {
            return Err(hook);
        }
        let Some(fd) = hook.get_fd() else {
            return Err(hook);
        };
        let fd = fd.as_raw_fd();
        self.hooks.push(hook);
        self.fds.push(fd);
        Ok((index, &*self.hooks[usize::from(index)]))
    }

    pub fn as_fds(&self) -> &[BorrowedFd<'_>] {
        let fds: *const [RawFd] = &raw const *self.fds;
        let fds = fds as *const [BorrowedFd<'_>];
        // SAFETY: `fds` consists of `RawFd` conversions of `BorrowedFd<'_>`
        // return values from `<dyn ClientHook + 'static>::get_fd()`.
        //
        // The signature of `get_fd()` requires its return value to live as
        // long as `self`; each corresponding hook is stored in `self.hooks`,
        // kept alive by our own `self` reference. `BorrowdFd<'_>` is
        // documented to have the same representation as host file descriptors,
        // as represented by `RawFd`.
        unsafe { &*fds }
    }
}
