// Smelling Salts
// Copyright © 2020-2021 Jeron Aldaron Lau.
//
// Licensed under any of:
// - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
// - MIT License (https://mit-license.org/)
// - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).

#![allow(unsafe_code)]

use std::mem::MaybeUninit;
use std::sync::Once;
use std::task::Context;

pub use ffi::RawDevice;

#[cfg_attr(
    any(target_arch = "wasm32", target_arch = "asmjs"),
    cfg_attr(target_os = "wasi", path = "raw/wasi.rs"),
    cfg_attr(target_os = "aldro", path = "raw/aldro.rs"),
    cfg_attr(
        any(target_os = "unknown", target_os = "emscripten"),
        path = "raw/dom.rs"
    )
)]
#[cfg_attr(
    not(any(target_arch = "wasm32", target_arch = "asmjs")),
    cfg_attr(target_os = "linux", path = "raw/linux.rs"),
    cfg_attr(target_os = "android", path = "raw/android.rs"),
    cfg_attr(target_os = "macos", path = "raw/macos.rs"),
    cfg_attr(target_os = "ios", path = "raw/ios.rs"),
    cfg_attr(target_os = "windows", path = "raw/windows.rs"),
    cfg_attr(target_os = "fuchsia", path = "raw/fuchsia.rs"),
    cfg_attr(target_os = "redox", path = "raw/redox.rs"),
    cfg_attr(
        any(
            target_os = "freebsd",
            target_os = "dragonfly",
            target_os = "bitrig",
            target_os = "openbsd",
            target_os = "netbsd"
        ),
        path = "raw/bsd.rs",
    )
)]
mod ffi;

pub(crate) trait Global {
    /// Create a new `Device`.
    fn device(&self, fd: RawDevice, events: u32) -> Box<dyn Device>;
}

pub(crate) trait Device: std::fmt::Debug + Send + Sync {
    /// Return `true` if this wasn't the device that woke, reset.
    fn pending(&self) -> bool;
    /// Reset the `Waker`.
    fn sleep(&mut self, cx: &Context<'_>);
    /// Get the raw device descriptor.
    fn raw(&self) -> RawDevice;
    /// Stop listening on this device (automatic on Drop).
    fn free(&mut self) -> RawDevice;
}

/// Global state for when the system implementation can fail.
struct FakeGlobal;

impl Global for FakeGlobal {
    fn device(&self, fd: RawDevice, _events: u32) -> Box<dyn Device> {
        Box::new(FakeDevice(fd))
    }
}

#[derive(Debug)]
struct FakeDevice(RawDevice);

impl Device for FakeDevice {
    fn pending(&self) -> bool {
        true
    }
    fn sleep(&mut self, _cx: &Context<'_>) {}
    fn raw(&self) -> RawDevice {
        self.0
    }
    fn free(&mut self) -> RawDevice {
        self.raw()
    }
}

static START: Once = Once::new();
static mut GLOBAL: MaybeUninit<Box<dyn Global>> = MaybeUninit::uninit();

pub(crate) fn global() -> &'static dyn Global {
    START.call_once(|| unsafe {
        std::ptr::write(GLOBAL.as_mut_ptr(), ffi::global());
    });
    unsafe { &*(*GLOBAL.as_ptr()) }
}
