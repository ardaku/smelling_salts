// Smelling Salts
// Copyright © 2020-2021 Jeron Aldaron Lau.
//
// Licensed under any of:
// - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
// - MIT License (https://mit-license.org/)
// - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).
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
//! use std::future::Future;
//! use std::mem::MaybeUninit;
//! use std::os::raw;
//! use std::pin::Pin;
//! use std::task::{Context, Poll};
//! use std::time::Duration;
//! 
//! #[repr(C)]
//! struct TimeSpec {
//!     sec: isize,
//!     nsec: raw::c_long,
//! }
//! 
//! #[repr(C)]
//! struct ITimerSpec {
//!     interval: TimeSpec,
//!     value: TimeSpec,
//! }
//! 
//! extern "C" {
//!     fn timerfd_create(clockid: raw::c_int, flags: raw::c_int) -> raw::c_int;
//!     fn timerfd_settime(
//!         fd: raw::c_int,
//!         flags: raw::c_int,
//!         new_value: *const ITimerSpec,
//!         old_value: *mut ITimerSpec,
//!     ) -> raw::c_int;
//!     fn read(fd: raw::c_int, buf: *mut u64, count: usize) -> isize;
//!     fn close(fd: raw::c_int) -> raw::c_int;
//!     fn __errno_location() -> *mut raw::c_int;
//! }
//! 
//! struct Sleep(Device, u64);
//! 
//! impl Sleep {
//!     fn new(dur: Duration) -> Self {
//!         let sec = dur.as_secs() as _;
//!         let nsec = dur.subsec_nanos() as _;
//! 
//!         let timerfd = unsafe {
//!             timerfd_create(1 /*Monotonic*/, 2048 /*Nonblock*/)
//!         };
//!         let x = unsafe {
//!             timerfd_settime(
//!                 timerfd,
//!                 0,
//!                 &ITimerSpec {
//!                     interval: TimeSpec { sec, nsec },
//!                     value: TimeSpec { sec, nsec },
//!                 },
//!                 std::ptr::null_mut(),
//!             )
//!         };
//!         assert_eq!(0, x);
//! 
//!         Sleep(Device::new(timerfd, Watcher::new().input()), 0)
//!     }
//! }
//! 
//! impl Future for Sleep {
//!     type Output = ();
//! 
//!     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
//!         // Queue
//!         if self.1 != 0 {
//!             self.1 -= 1;
//!             return Poll::Ready(());
//!         }
//!         // Early return if a different device woke the executor.
//!         if self.0.pending() {
//!             return self.0.sleep(cx);
//!         }
//!         // 
//!         let mut x = MaybeUninit::<u64>::uninit();
//!         let v = unsafe {
//!             read(self.0.raw(), x.as_mut_ptr(), std::mem::size_of::<u64>())
//!         };
//!         if v > 0 {
//!             self.1 += unsafe { x.assume_init() };
//!             self.poll(cx)
//!         } else {
//!             self.0.sleep(cx)
//!         }
//!     }
//! }
//! 
//! impl Drop for Sleep {
//!     fn drop(&mut self) {
//!         assert_eq!(0, unsafe { close(self.0.stop()) });
//!     }
//! }
//! 
//! fn main() {
//!     pasts::block_on(async {
//!         for _ in 0..5 {
//!             println!("Sleeping for 1 second…");
//!             Sleep::new(Duration::new(1, 0)).await;
//!         }
//!     });
//! }
//! ```

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
mod raw;
mod watcher;

pub use device::Device;
pub use raw::RawDevice;
pub use watcher::Watcher;
