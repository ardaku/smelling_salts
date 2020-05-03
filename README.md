# Smelling Salts

#### Start a thread to wake an async executor when the OS's I/O event notifier gathers that the hardware is ready.

[![Build Status](https://api.travis-ci.org/AldaronLau/smelling_salts.svg?branch=master)](https://travis-ci.org/AldaronLau/smelling_salts)
[![Docs](https://docs.rs/smelling_salts/badge.svg)](https://docs.rs/smelling_salts)
[![crates.io](https://img.shields.io/crates/v/smelling_salts.svg)](https://crates.io/crates/smelling_salts)

If you're writing a Rust library to handle hardware asynchronously, you should
use this crate.  This library automatically wakes futures by registering a waker
with a device that you construct with a file descriptor.

### Currently Supported Platforms
- Linux (epoll)

### Planned Platforms
- Windows
- MacOS
- BSD
- Various Bare Metal?
- Others?

## Table of Contents
- [Getting Started](#getting-started)
   - [Example](#example)
   - [API](#api)
   - [Features](#features)
- [Upgrade](#upgrade)
- [License](#license)
   - [Contribution](#contribution)

## Getting Started
Add the following to your `Cargo.toml`.

```toml
[dependencies]
smelling_salts = "0.1"
# Only include pasts for applications, don't use in libraries.
pasts = "0.1"
```

### Example
```rust,no_run
use pasts;
use smelling_salts::{Device, Watcher};

use std::convert::TryInto;
use std::future::Future;
use std::mem;
use std::os::raw;
use std::pin::Pin;
use std::task::{Context, Poll};

#[allow(non_camel_case_types)]
type c_ssize = isize; // True for most unix
#[allow(non_camel_case_types)]
type c_size = usize; // True for most unix

const MAGIC_NUMBER: u32 = 0xDEAD_BEEF;

// From fcntl.h
const O_CLOEXEC: raw::c_int = 0o2000000;
const O_NONBLOCK: raw::c_int = 0o0004000;
const O_DIRECT: raw::c_int = 0o0040000;

extern "C" {
    fn pipe2(pipefd: *mut [raw::c_int; 2], flags: raw::c_int) -> raw::c_int;
    fn write(fd: raw::c_int, buf: *const raw::c_void, count: c_size) -> c_ssize;
    fn read(fd: raw::c_int, buf: *mut raw::c_void, count: c_size) -> c_ssize;
    fn close(fd: raw::c_int) -> raw::c_int;
}

// Convert a C error (negative on error) into a result.
fn error(err: raw::c_int) -> Result<(), raw::c_int> {
    if err < 0 {
        Err(err)
    } else {
        Ok(())
    }
}

fn fd_close(fd: raw::c_int) {
    // close() should never fail.
    let ret = unsafe { close(fd) };
    error(ret).unwrap();
}

// Create the sender and receiver for a pipe.
fn new_pipe() -> (raw::c_int, raw::c_int) {
    let [recver, sender] = unsafe {
        // Create pipe for communication
        let mut pipe = mem::MaybeUninit::<[raw::c_int; 2]>::uninit();
        error(pipe2(pipe.as_mut_ptr(), O_CLOEXEC | O_NONBLOCK | O_DIRECT)).unwrap();
        pipe.assume_init()
    };

    (sender, recver)
}

fn write_u32(fd: raw::c_int, data: u32) {
    let data = [data];
    let len: usize = unsafe {
        write(fd, data.as_ptr().cast(), mem::size_of::<u32>())
            .try_into()
            .unwrap()
    };
    assert_eq!(len, mem::size_of::<u32>());
}

fn read_u32(fd: raw::c_int) -> Option<u32> {
    let ret = unsafe {
        let mut buffer = mem::MaybeUninit::<u32>::uninit();
        let len: usize = read(fd, buffer.as_mut_ptr().cast(), mem::size_of::<u32>())
            .try_into()
            .unwrap_or(0);
        if len == 0 {
            return None;
        }
        assert_eq!(len, mem::size_of::<u32>());
        buffer.assume_init()
    };
    Some(ret)
}

pub struct PipeReceiver(Device);

impl PipeReceiver {
    pub fn new(fd: raw::c_int) -> Self {
        PipeReceiver(Device::new(fd, Watcher::new().input()))
    }
}

impl Drop for PipeReceiver {
    fn drop(&mut self) {
        // Deregister FD, then delete (must be in this order).
        self.0.old();
        fd_close(self.0.fd());
    }
}

pub struct PipeFuture<'a>(&'a PipeReceiver);

impl<'a> PipeFuture<'a> {
    pub fn new(recver: &'a PipeReceiver) -> Self {
        PipeFuture(recver)
    }
}

impl<'a> Future for PipeFuture<'a> {
    type Output = u32;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if let Some(output) = read_u32((self.0).0.fd()) {
            Poll::Ready(output)
        } else {
            let waker = cx.waker();
            (self.0).0.register_waker(waker.clone());
            Poll::Pending
        }
    }
}

async fn async_main() {
    let (sender, recver) = new_pipe();
    let device = PipeReceiver::new(recver);

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        write_u32(sender, MAGIC_NUMBER);
        fd_close(sender);
    });

    let output = PipeFuture::new(&device).await;
    assert_eq!(output, MAGIC_NUMBER);
}

fn main() {
    <pasts::ThreadInterrupt as pasts::Interrupt>::block_on(async_main());
}
```

### API
API documentation can be found on [docs.rs](https://docs.rs/smelling_salts).

### Features
There are no optional features.

## Upgrade
You can use the
[changelog](https://github.com/AldaronLau/smelling_salts/blob/master/CHANGELOG.md)
to facilitate upgrading this crate as a dependency.

## License
Licensed under either of
 - Apache License, Version 2.0,
   ([LICENSE-APACHE](https://github.com/AldaronLau/smelling_salts/blob/master/LICENSE-APACHE) or
   [https://www.apache.org/licenses/LICENSE-2.0](https://www.apache.org/licenses/LICENSE-2.0))
 - Zlib License,
   ([LICENSE-ZLIB](https://github.com/AldaronLau/smelling_salts/blob/master/LICENSE-ZLIB) or
   [https://opensource.org/licenses/Zlib](https://opensource.org/licenses/Zlib))

at your option.

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

Contributors are always welcome (thank you for being interested!), whether it
be a bug report, bug fix, feature request, feature implementation or whatever.
Don't be shy about getting involved.  I always make time to fix bugs, so usually
a patched version of the library will be out a few days after a report.
Features requests will not complete as fast.  If you have any questions, design
critques, or want me to find you something to work on based on your skill level,
you can email me at [jeronlau@plopgrizzly.com](mailto:jeronlau@plopgrizzly.com).
Otherwise,
[here's a link to the issues on GitHub](https://github.com/AldaronLau/smelling_salts/issues).
Before contributing, check out the
[contribution guidelines](https://github.com/AldaronLau/smelling_salts/blob/master/CONTRIBUTING.md),
and, as always, make sure to follow the
[code of conduct](https://github.com/AldaronLau/smelling_salts/blob/master/CODE_OF_CONDUCT.md).
