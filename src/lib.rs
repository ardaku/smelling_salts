// Copyright Jeron Aldaron Lau 2020.
// Distributed under either the Apache License, Version 2.0
//    (See accompanying file LICENSE_APACHE_2_0.txt or copy at
//          https://apache.org/licenses/LICENSE-2.0),
// or the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE_BOOST_1_0.txt or copy at
//          https://www.boost.org/LICENSE_1_0.txt)
// at your option. This file may not be copied, modified, or distributed except
// according to those terms.
//!
//!
//! ## Getting Started
//! Add the following to your `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! smelling_salts = "0.2"
//! pasts = "0.7"
//! ```
//!
//! ### Example
//! ```rust,no_run
//! use smelling_salts::{Device, Watcher};
//!
//! use std::convert::TryInto;
//! use std::future::Future;
//! use std::mem;
//! use std::os::raw;
//! use std::pin::Pin;
//! use std::task::{Context, Poll};
//!
//! #[allow(non_camel_case_types)]
//! type c_ssize = isize; // True for most unix
//! #[allow(non_camel_case_types)]
//! type c_size = usize; // True for most unix
//!
//! const MAGIC_NUMBER: u32 = 0xDEAD_BEEF;
//!
//! // From fcntl.h
//! const O_CLOEXEC: raw::c_int = 0o2000000;
//! const O_NONBLOCK: raw::c_int = 0o0004000;
//! const O_DIRECT: raw::c_int = 0o0040000;
//!
//! extern "C" {
//!     fn pipe2(pipefd: *mut [raw::c_int; 2], flags: raw::c_int) -> raw::c_int;
//!     fn write(fd: raw::c_int, buf: *const raw::c_void, count: c_size) -> c_ssize;
//!     fn read(fd: raw::c_int, buf: *mut raw::c_void, count: c_size) -> c_ssize;
//!     fn close(fd: raw::c_int) -> raw::c_int;
//! }
//!
//! // Convert a C error (negative on error) into a result.
//! fn error(err: raw::c_int) -> Result<(), raw::c_int> {
//!     if err < 0 {
//!         Err(err)
//!     } else {
//!         Ok(())
//!     }
//! }
//!
//! fn fd_close(fd: raw::c_int) {
//!     // close() should never fail.
//!     let ret = unsafe { close(fd) };
//!     error(ret).unwrap();
//! }
//!
//! // Create the sender and receiver for a pipe.
//! fn new_pipe() -> (raw::c_int, raw::c_int) {
//!     let [recver, sender] = unsafe {
//!         // Create pipe for communication
//!         let mut pipe = mem::MaybeUninit::<[raw::c_int; 2]>::uninit();
//!         error(pipe2(pipe.as_mut_ptr(), O_CLOEXEC | O_NONBLOCK | O_DIRECT)).unwrap();
//!         pipe.assume_init()
//!     };
//!
//!     (sender, recver)
//! }
//!
//! fn write_u32(fd: raw::c_int, data: u32) {
//!     let data = [data];
//!     let len: usize = unsafe {
//!         write(fd, data.as_ptr().cast(), mem::size_of::<u32>())
//!             .try_into()
//!             .unwrap()
//!     };
//!     assert_eq!(len, mem::size_of::<u32>());
//! }
//!
//! fn read_u32(fd: raw::c_int) -> Option<u32> {
//!     let ret = unsafe {
//!         let mut buffer = mem::MaybeUninit::<u32>::uninit();
//!         let len: usize = read(fd, buffer.as_mut_ptr().cast(), mem::size_of::<u32>())
//!             .try_into()
//!             .unwrap_or(0);
//!         if len == 0 {
//!             return None;
//!         }
//!         assert_eq!(len, mem::size_of::<u32>());
//!         buffer.assume_init()
//!     };
//!     Some(ret)
//! }
//!
//! pub struct PipeReceiver(Device);
//!
//! impl PipeReceiver {
//!     pub fn new(fd: raw::c_int) -> Self {
//!         PipeReceiver(Device::new(fd, Watcher::new().input()))
//!     }
//! }
//!
//! impl Drop for PipeReceiver {
//!     fn drop(&mut self) {
//!         // Deregister FD, then delete (must be in this order).
//!         self.0.old();
//!         fd_close(self.0.raw());
//!     }
//! }
//!
//! pub struct PipeFuture<'a>(&'a PipeReceiver);
//!
//! impl<'a> PipeFuture<'a> {
//!     pub fn new(recver: &'a PipeReceiver) -> Self {
//!         PipeFuture(recver)
//!     }
//! }
//!
//! impl<'a> Future for PipeFuture<'a> {
//!     type Output = u32;
//!
//!     fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
//!         if let Some(output) = read_u32((self.0).0.raw()) {
//!             Poll::Ready(output)
//!         } else {
//!             (self.0).0.register_waker(cx.waker());
//!             Poll::Pending
//!         }
//!     }
//! }
//!
//! async fn async_main() {
//!     let (sender, recver) = new_pipe();
//!     let device = PipeReceiver::new(recver);
//!
//!     std::thread::spawn(move || {
//!         std::thread::sleep(std::time::Duration::from_millis(1000));
//!         write_u32(sender, MAGIC_NUMBER);
//!         fd_close(sender);
//!     });
//!
//!     let output = PipeFuture::new(&device).await;
//!     assert_eq!(output, MAGIC_NUMBER);
//! }
//!
//! fn main() {
//!     pasts::block_on(async_main());
//! }

#![cfg_attr(feature = "docs-rs", feature(external_doc))]
#![cfg_attr(feature = "docs-rs", doc(include = "../README.md"))]
#![doc = ""]
#![doc(
    html_logo_url = "https://libcala.github.io/logo.svg",
    html_favicon_url = "https://libcala.github.io/icon.svg",
    html_root_url = "https://docs.rs/smelling_salts"
)]
#![deny(unsafe_code)]
#![warn(
    anonymous_parameters,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_qualifications,
    variant_size_differences
)]

mod device;
mod watcher;

#[cfg_attr(target_arch = "wasm32", path = "ffi/web.rs")]
#[cfg_attr(
    not(target_arch = "wasm32"),
    cfg_attr(target_os = "linux", path = "ffi/linux.rs"),
    cfg_attr(target_os = "android", path = "ffi/android.rs"),
    cfg_attr(target_os = "macos", path = "ffi/macos.rs"),
    cfg_attr(target_os = "ios", path = "ffi/ios.rs"),
    cfg_attr(target_os = "windows", path = "ffi/windows.rs"),
    cfg_attr(
        any(
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "bitrig",
            target_os = "openbsd",
            target_os = "netbsd"
        ),
        path = "ffi/bsd.rs"
    ),
    cfg_attr(target_os = "fuchsia", path = "ffi/fuchsia.rs"),
    cfg_attr(target_os = "redox", path = "ffi/redox.rs"),
    cfg_attr(target_os = "none", path = "ffi/none.rs"),
    cfg_attr(target_os = "dummy", path = "ffi/dummy.rs")
)]
#[allow(unsafe_code)]
mod ffi;

pub use device::Device;
pub use ffi::RawDevice;
pub use watcher::Watcher;
