// Smelling Salts
//
// Copyright (c) 2020 Jeron Aldaron Lau
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// https://apache.org/licenses/LICENSE-2.0>, or the Zlib License, <LICENSE-ZLIB
// or http://opensource.org/licenses/Zlib>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::{
    watcher::Watcher,
    RawDevice,
};
use std::task::Waker;

/// Represents some device.
#[derive(Debug)]
pub struct Device(crate::ffi::Device);

impl Device {
    /// Start checking for events on a new device from a linux file descriptor.
    pub fn new(fd: RawDevice, events: Watcher) -> Self {
        Device(crate::ffi::Device::new(fd, events))
    }

    /// Register a waker to wake when the device gets an event.
    pub fn register_waker(&self, waker: &Waker) {
        self.0.register_waker(waker);
    }

    /// Convenience function to get the raw File Descriptor of the Device.
    pub fn raw(&self) -> RawDevice {
        self.0.raw()
    }

    /// Stop checking for events on a device from a linux file descriptor.
    #[allow(clippy::mutex_atomic)]
    pub fn old(&mut self) {
        self.0.old()
    }
}
