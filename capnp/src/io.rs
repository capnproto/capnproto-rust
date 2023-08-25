//! Custom I/O traits that roughly mirror `std::io::{Read, BufRead, Write}`.
//! This extra layer of indirection enables support of no-std environments.

use crate::{Error, ErrorKind, Result};

/// A rough approximation of std::io::Read.
pub trait Read {
    /// Attempts to read some bytes into `buf` and returns the number of bytes read.
    /// A return value of Ok(0) means that the end of the stream was reached.
    ///
    /// Unlike with std::io::Read, implementations are expected to handle EINTR
    /// (i.e. std::io::ErrorKind::Interrupted) internally, by looping until either success
    /// is achieved or some other error is hit.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
        while !buf.is_empty() {
            match self.read(buf)? {
                0 => break,
                n => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
            }
        }
        if !buf.is_empty() {
            Err(Error::from_kind(ErrorKind::FailedToFillTheWholeBuffer))
        } else {
            Ok(())
        }
    }
}

/// A rough approximation of std::io::BufRead.
pub trait BufRead: Read {
    fn fill_buf(&mut self) -> Result<&[u8]>;
    fn consume(&mut self, amt: usize);
}

/// A rough approximation of std::io::Write.
pub trait Write {
    fn write_all(&mut self, buf: &[u8]) -> Result<()>;
}

/// Blanket impls for when `std` is enabled.
#[cfg(feature = "std")]
mod std_impls {
    use crate::io::{BufRead, Read, Write};
    use crate::Result;

    impl<R> Read for R
    where
        R: std::io::Read,
    {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            loop {
                match std::io::Read::read(self, buf) {
                    Ok(n) => return Ok(n),
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }

    impl<R> BufRead for R
    where
        R: std::io::BufRead,
    {
        fn fill_buf(&mut self) -> Result<&[u8]> {
            Ok(std::io::BufRead::fill_buf(self)?)
        }
        fn consume(&mut self, amt: usize) {
            std::io::BufRead::consume(self, amt)
        }
    }

    impl<W> Write for W
    where
        W: std::io::Write,
    {
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {
            std::io::Write::write_all(self, buf)?;
            Ok(())
        }
    }
}

/// Blanket impls for when `embedded-io` is enabled and `std` is not.
#[cfg(all(feature = "embedded-io", not(feature = "std")))]
mod embedded_io_impls {
    use crate::io::{BufRead, Read, Write};
    use crate::Result;
    use embedded_io::Error;

    impl<W: embedded_io::Write> Write for W {
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {
            embedded_io::Write::write_all(self, buf).map_err(|e| match e {
                embedded_io::WriteAllError::WriteZero => {
                    crate::Error::from_kind(crate::ErrorKind::Failed)
                }
                embedded_io::WriteAllError::Other(e) => crate::Error::from_kind(e.kind().into()),
            })?;
            Ok(())
        }
    }

    impl<R: embedded_io::Read> Read for R {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            embedded_io::Read::read(self, buf).map_err(|e| crate::Error::from_kind(e.kind().into()))
        }
    }

    impl<R: embedded_io::BufRead + embedded_io::Read> BufRead for R {
        fn fill_buf(&mut self) -> Result<&[u8]> {
            embedded_io::BufRead::fill_buf(self)
                .map_err(|e| crate::Error::from_kind(e.kind().into()))
        }

        fn consume(&mut self, amt: usize) {
            embedded_io::BufRead::consume(self, amt)
        }
    }
}

/// Fallback impls, for when neither `std` nor `embedded-io` is enabled.
#[cfg(not(any(feature = "std", feature = "embedded-io")))]
mod no_std_impls {
    use crate::io::{BufRead, Read, Write};
    use crate::{Error, ErrorKind, Result};

    impl<'a> Write for &'a mut [u8] {
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {
            if buf.len() > self.len() {
                return Err(Error::from_kind(ErrorKind::BufferNotLargeEnough));
            }
            let amt = buf.len();
            let (a, b) = core::mem::take(self).split_at_mut(amt);
            a.copy_from_slice(buf);
            *self = b;
            Ok(())
        }
    }

    #[cfg(feature = "alloc")]
    impl Write for alloc::vec::Vec<u8> {
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {
            self.extend_from_slice(buf);
            Ok(())
        }
    }

    impl<W: ?Sized> Write for &mut W
    where
        W: Write,
    {
        fn write_all(&mut self, buf: &[u8]) -> Result<()> {
            (**self).write_all(buf)
        }
    }

    impl<'a> Read for &'a [u8] {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            let amt = core::cmp::min(buf.len(), self.len());
            let (a, b) = self.split_at(amt);

            buf[..amt].copy_from_slice(a);
            *self = b;
            Ok(amt)
        }
    }

    impl<R: ?Sized> Read for &mut R
    where
        R: Read,
    {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            (**self).read(buf)
        }
    }

    impl<'a> BufRead for &'a [u8] {
        fn fill_buf(&mut self) -> Result<&[u8]> {
            Ok(*self)
        }
        fn consume(&mut self, amt: usize) {
            *self = &self[amt..]
        }
    }

    impl<R: ?Sized> BufRead for &mut R
    where
        R: BufRead,
    {
        fn fill_buf(&mut self) -> Result<&[u8]> {
            (**self).fill_buf()
        }
        fn consume(&mut self, amt: usize) {
            (**self).consume(amt)
        }
    }
}
