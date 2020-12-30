// Smelling Salts
//
// Copyright (c) 2020 Jeron Aldaron Lau
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// https://apache.org/licenses/LICENSE-2.0>, or the Zlib License, <LICENSE-ZLIB
// or http://opensource.org/licenses/Zlib>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::watcher::Watcher;
use std::task::Waker;

/// In the dummy implementation, `RawDevice` corresponds to `()`
pub type RawDevice = ();

/// Represents some device.
#[derive(Debug)]
pub(super) struct Device(RawDevice);

impl Device {
    /// Start checking for events on a new device from a linux file descriptor.
    pub(super) fn new(raw: RawDevice, _events: Watcher) -> Self {
        Device(raw)
    }

    /// Register a waker to wake when the device gets an event.
    pub(super) fn register_waker(&self, _waker: &Waker) {}

    /// Convenience function to get the raw File Descriptor of the Device.
    pub(super) fn raw(&self) -> RawDevice {
        self.0
    }

    /// Stop checking for events on a device from a linux file descriptor.
    pub(super) fn old(&mut self) {}
}
