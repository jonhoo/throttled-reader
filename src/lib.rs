//! This crate provides `ThrottledReader`, a proxy-type for `io::Read` that limits how many times
//! the underlying reader can be read from. If the read budget is exceeded,
//! `io::ErrorKind::WouldBlock` is returned instead. This type can be useful to enforce fairness
//! when reading from many (potentially asynchronous) input streams with highly varying load. If
//! one stream always has data available, a worker may continue consuming its input forever,
//! neglecting the other stream.
//!
//! # Examples
//!
//! ```rust
//! # use std::io;
//! # use std::io::prelude::*;
//! # use throttled_reader::ThrottledReader;
//! let mut buf = [0];
//! let mut stream = ThrottledReader::new(io::empty());
//!
//! // initially no limit
//! assert!(stream.read(&mut buf).is_ok());
//! assert!(stream.read(&mut buf).is_ok());
//!
//! // set a limit
//! stream.set_limit(2);
//! assert!(stream.read(&mut buf).is_ok()); // first is allowed through
//! assert!(stream.read(&mut buf).is_ok()); // second is also allowed through
//! // but now the limit is reached, and the underlying stream is no longer accessible
//! assert_eq!(
//!     stream.read(&mut buf).unwrap_err().kind(),
//!     io::ErrorKind::WouldBlock
//! );
//!
//! // we can then unthrottle it again after checking other streams
//! stream.unthrottle();
//! assert!(stream.read(&mut buf).is_ok());
//! assert!(stream.read(&mut buf).is_ok());
//! ```
#![deny(missing_docs)]
use std::io;

/// `ThrottleReader` proxies an `io::Read`, but enforces a budget on how many `read` calls can be
/// made to the underlying reader.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ThrottledReader<R> {
    reader: R,
    read_budget: Option<usize>,
}

impl<R> ThrottledReader<R> {
    /// Construct a new throttler that wraps the given reader.
    ///
    /// The new `ThrottledReader` initially has no limit.
    pub fn new(reader: R) -> Self {
        ThrottledReader {
            reader,
            read_budget: None,
        }
    }

    /// Set the number of `read` calls allowed to the underlying reader.
    pub fn set_limit(&mut self, limit: usize) {
        self.read_budget = Some(limit);
    }

    /// Remove the limit on how many `read` calls can be issued to the underlying reader.
    pub fn unthrottle(&mut self) {
        self.read_budget = None;
    }

    /// Check how many more `read` calls may be issued to the underlying reader.
    ///
    /// Returns `None` if the reader is not currently throttled.
    pub fn remaining(&self) -> Option<usize> {
        self.read_budget
    }

    /// Extract the underlying reader.
    pub fn into_inner(self) -> R {
        self.reader
    }
}

impl<R> io::Read for ThrottledReader<R>
where
    R: io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.read_budget.map(|r| r.checked_sub(1)) {
            None => {
                // no limit
                self.reader.read(buf)
            }
            Some(None) => {
                // past limit
                Err(io::Error::new(io::ErrorKind::WouldBlock, "read throttled"))
            }
            Some(Some(remaining)) => {
                // above limit
                self.read_budget = Some(remaining);
                self.reader.read(buf)
            }
        }
    }
}

impl<R> From<R> for ThrottledReader<R> {
    fn from(reader: R) -> Self {
        ThrottledReader::new(reader)
    }
}

impl<R> Default for ThrottledReader<R>
where
    R: Default,
{
    fn default() -> Self {
        ThrottledReader {
            reader: R::default(),
            read_budget: None,
        }
    }
}

use std::ops::{Deref, DerefMut};
impl<R> Deref for ThrottledReader<R> {
    type Target = R;
    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}

impl<R> DerefMut for ThrottledReader<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::prelude::*;

    #[test]
    fn it_works() {
        let mut s = ThrottledReader::new(io::empty());
        // initially no limit
        assert_eq!(s.read(&mut [0]).unwrap(), 0);
        assert_eq!(s.read(&mut [0]).unwrap(), 0);
        assert_eq!(s.read(&mut [0]).unwrap(), 0);

        // set a limit
        s.set_limit(2);
        assert_eq!(s.read(&mut [0]).unwrap(), 0); // first is allowed through
        assert_eq!(s.remaining(), Some(1));
        assert_eq!(s.read(&mut [0]).unwrap(), 0); // second is allowed through
        assert_eq!(s.remaining(), Some(0));
        assert_eq!(
            s.read(&mut [0]).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        ); // third is *not* allowed
        assert_eq!(s.remaining(), Some(0));
        assert_eq!(
            s.read(&mut [0]).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        ); // obviously neither is fourth
        assert_eq!(s.remaining(), Some(0));

        // unthrottle again
        s.unthrottle();
        assert_eq!(s.read(&mut [0]).unwrap(), 0);
        assert_eq!(s.read(&mut [0]).unwrap(), 0);
        assert_eq!(s.read(&mut [0]).unwrap(), 0);
    }
}
