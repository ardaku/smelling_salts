// Smelling Salts
// Copyright © 2020-2021 Jeron Aldaron Lau.
//
// Licensed under any of:
// - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
// - MIT License (https://mit-license.org/)
// - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).

use crate::{watcher::Watcher, RawDevice};
use std::task::{Context, Poll};

/// Represents some device.
#[derive(Debug)]
pub struct Device(Box<dyn crate::raw::Device>);

impl Device {
    /// Start checking for events on a new device from a linux file descriptor.
    #[inline(always)]
    pub fn new(fd: RawDevice, events: Watcher) -> Self {
        crate::raw::GLOBAL.with(|g| Device(g.device(fd, events.0)))
    }

    /// Register a waker to wake when the device gets an event.
    #[inline(always)]
    pub fn sleep<T>(&mut self, context: &Context<'_>) -> Poll<T> {
        self.0.sleep(context);
        Poll::Pending
    }

    /// Convenience function to get the raw File Descriptor of the Device.
    #[inline(always)]
    pub fn raw(&self) -> RawDevice {
        self.0.raw()
    }

    /// Stop checking for events on a device from a linux file descriptor.
    #[inline(always)]
    pub fn stop(&mut self) -> RawDevice {
        self.0.free()
    }

    /// Returns true if this device hasn't been waked up.
    #[inline(always)]
    pub fn pending(&self) -> bool {
        self.0.pending()
    }
}
