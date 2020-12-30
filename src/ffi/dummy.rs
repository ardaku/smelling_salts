// Copyright Jeron Aldaron Lau 2020.
// Distributed under either the Apache License, Version 2.0
//    (See accompanying file LICENSE_APACHE_2_0.txt or copy at
//          https://apache.org/licenses/LICENSE-2.0),
// or the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE_BOOST_1_0.txt or copy at
//          https://www.boost.org/LICENSE_1_0.txt)
// at your option. This file may not be copied, modified, or distributed except
// according to those terms.

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

    /// Check if should yield to executor.
    pub(super) fn should_yield(&self) -> bool {
        true
    }
}
