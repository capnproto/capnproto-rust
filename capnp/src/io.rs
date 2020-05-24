//! Custom I/O traits that roughly mirror `std::io::{Read, BufRead, Write}`.
//! This extra layer of indirection enables support of no-std environments.

use crate::Result;

pub trait Read {
    fn read_exact(&mut self, buf: &mut [u8]) -> Result<()>;
}

pub trait BufRead : Read {
    fn fill_buf(&mut self) -> Result<&[u8]>;
    fn consume(&mut self, amt: usize);
}

pub trait Write {
    fn write_all(&mut self, buf: &[u8]) -> Result<()>;
}

#[cfg(feature="std")]
mod std_impls {
    use crate::{Result};
    use crate::io::{Read, BufRead, Write};

    impl <R> Read for R where R: std::io::Read {
        fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
            std::io::Read::read_exact(self, buf)?;
            Ok(())
        }
    }

    impl <R> BufRead for R where R: std::io::BufRead {
        fn fill_buf(&mut self) -> Result<&[u8]> {
            Ok(std::io::BufRead::fill_buf(self)?)
        }
        fn consume(&mut self, amt: usize) {
            std::io::BufRead::consume(self, amt)
        }
    }

    impl <W> Write for W where W: std::io::Write {
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {
            std::io::Write::write_all(self, buf)?;
            Ok(())
        }
    }
}

#[cfg(not(feature="std"))]
mod no_std_impls {
    use alloc::string::ToString;
    use crate::{Error, Result};
    use crate::io::{Read, BufRead, Write};

    impl <'a> Write for &'a mut [u8] {
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {
            if buf.len() > self.len() {
                return Err(Error::failed("buffer is not large enough".to_string()));
            }
            let amt = buf.len();
            let (a, b) = core::mem::replace(self, &mut []).split_at_mut(amt);
            a.copy_from_slice(buf);
            *self = b;
            Ok(())
        }
    }

    impl Write for alloc::vec::Vec<u8> {
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {
            self.extend_from_slice(buf);
            Ok(())
        }
    }

    impl <W: ?Sized> Write for &mut W where W: Write {
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {
            (**self).write_all(buf)
        }
    }

    impl <'a> Read for &'a [u8] {
        fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
            if buf.len() > self.len() {
                return Err(Error::failed("buffer is not large enough".to_string()));
            }
            let (a, b) = self.split_at(buf.len());
            buf.copy_from_slice(a);
            *self = b;
            Ok(())
        }
    }

    impl <R: ?Sized> Read for &mut R where R: Read {
        fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
            (**self).read_exact(buf)
        }
    }

    impl <'a> BufRead for &'a [u8] {
        fn fill_buf(&mut self) -> Result<&[u8]> {
            Ok(*self)
        }
        fn consume(&mut self, amt: usize) {
            *self = &self[amt..]
        }
    }

    impl <R: ?Sized> BufRead for &mut R where R: BufRead {
        fn fill_buf(&mut self) -> Result<&[u8]> {
            (**self).fill_buf()
        }
        fn consume(&mut self, amt: usize) {
            (**self).consume(amt)
        }
    }
}
