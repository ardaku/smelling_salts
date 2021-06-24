// Smelling Salts
// Copyright © 2020-2021 Jeron Aldaron Lau.
//
// Licensed under any of:
// - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
// - MIT License (https://mit-license.org/)
// - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).

use std::task::Context;

#[cfg_attr(target_arch = "wasm32", path = "raw/web.rs")]
#[cfg_attr(
    not(target_arch = "wasm32"),
    cfg_attr(target_os = "linux", path = "raw/linux.rs"),
    cfg_attr(target_os = "android", path = "raw/android.rs"),
    cfg_attr(target_os = "macos", path = "raw/macos.rs"),
    cfg_attr(target_os = "ios", path = "raw/ios.rs"),
    cfg_attr(target_os = "windows", path = "raw/windows.rs"),
    cfg_attr(
        any(
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "bitrig",
            target_os = "openbsd",
            target_os = "netbsd"
        ),
        path = "raw/bsd.rs",
    ),
    cfg_attr(target_os = "fuchsia", path = "raw/fuchsia.rs"),
    cfg_attr(target_os = "redox", path = "raw/redox.rs"),
    cfg_attr(target_os = "dive", path = "raw/dive.rs")
)]
#[allow(unsafe_code)]
mod ffi;

pub(crate) trait Global {
    /// Create a new `Device`.
    fn device(&self, fd: RawDevice, events: u32) -> Box<dyn Device>;
}

pub(crate) trait Device: std::fmt::Debug {
    /// Return `true` if this wasn't the device that woke, reset.
    fn pending(&self) -> bool;
    /// Reset the `Waker`.
    fn sleep(&mut self, cx: &Context<'_>);
    /// Get the raw device descriptor.
    fn raw(&self) -> RawDevice;
    /// Stop listening on this device (automatic on Drop).
    fn free(&mut self) -> RawDevice;
}

thread_local! {
    pub(crate) static GLOBAL: Box<dyn Global> = ffi::global();
}

pub use ffi::RawDevice;
