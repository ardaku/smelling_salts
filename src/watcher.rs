// Copyright Jeron Aldaron Lau 2020.
// Distributed under either the Apache License, Version 2.0
//    (See accompanying file LICENSE_APACHE_2_0.txt or copy at
//          https://apache.org/licenses/LICENSE-2.0),
// or the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE_BOOST_1_0.txt or copy at
//          https://www.boost.org/LICENSE_1_0.txt)
// at your option. This file may not be copied, modified, or distributed except
// according to those terms.

pub(crate) const EPOLLIN: u32 = 0x0001;
const EPOLLOUT: u32 = 0x0004;
const EPOLLET: u32 = 1 << 31;

/// Which events to watch for to trigger a wake-up.
#[derive(Debug, Copy, Clone)]
pub struct Watcher(pub(crate) u32);

impl Watcher {
    /// Create empty Watcher (requesting nothing)
    pub fn new() -> Watcher {
        Watcher(EPOLLET)
    }

    /// Create Watcher from raw bitmask
    ///
    /// # Safety
    /// This function requires the correct usage of the bitflags from the epoll
    /// C API.
    #[allow(unsafe_code)]
    pub unsafe fn from_raw(raw: u32) -> Watcher {
        Watcher(EPOLLET | raw)
    }

    /// Watch for input from device.
    pub fn input(mut self) -> Self {
        self.0 |= EPOLLIN;
        self
    }

    /// Watch for device to be ready for output.
    pub fn output(mut self) -> Self {
        self.0 |= EPOLLOUT;
        self
    }
}

impl Default for Watcher {
    fn default() -> Self {
        Self::new()
    }
}
