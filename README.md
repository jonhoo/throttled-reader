# throttled-reader

[![Crates.io](https://img.shields.io/crates/v/throttled-reader.svg)](https://crates.io/crates/throttled-reader)
[![Documentation](https://docs.rs/throttled-reader/badge.svg)](https://docs.rs/throttled-reader/)
[![Build Status](https://travis-ci.org/jonhoo/throttled-reader.svg?branch=master)](https://travis-ci.org/jonhoo/throttled-reader)

This crate provides `ThrottledReader`, a proxy-type for `io::Read` that limits how many times
the underlying reader can be read from. If the read budget is exceeded,
`io::ErrorKind::WouldBlock` is returned instead. This type can be useful to enforce fairness
when reading from many (potentially asynchronous) input streams with highly varying load. If
one stream always has data available, a worker may continue consuming its input forever,
neglecting the other stream.

## Examples

```rust
let mut buf = [0];
let mut stream = ThrottledReader::new(io::empty());

// initially no limit
assert!(stream.read(&mut buf).is_ok());
assert!(stream.read(&mut buf).is_ok());

// set a limit
stream.set_limit(2);
assert!(stream.read(&mut buf).is_ok()); // first is allowed through
assert!(stream.read(&mut buf).is_ok()); // second is also allowed through
// but now the limit is reached, and the underlying stream is no longer accessible
assert_eq!(
    stream.read(&mut buf).unwrap_err().kind(),
    io::ErrorKind::WouldBlock
);

// we can then unthrottle it again after checking other streams
stream.unthrottle();
assert!(stream.read(&mut buf).is_ok());
assert!(stream.read(&mut buf).is_ok());
```
