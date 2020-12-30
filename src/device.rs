// Copyright Jeron Aldaron Lau 2020.
// Distributed under either the Apache License, Version 2.0
//    (See accompanying file LICENSE_APACHE_2_0.txt or copy at
//          https://apache.org/licenses/LICENSE-2.0),
// or the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE_BOOST_1_0.txt or copy at
//          https://www.boost.org/LICENSE_1_0.txt)
// at your option. This file may not be copied, modified, or distributed except
// according to those terms.

use crate::{watcher::Watcher, RawDevice};
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

    /// Returns true if this device hasn't been waked up.
    pub fn should_yield(&self) -> bool {
        self.0.should_yield()
    }
}
