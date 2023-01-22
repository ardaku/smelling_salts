// Copyright © 2020-2023 The Smelling Salts Contributors.
//
// Licensed under any of:
//  - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
//  - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
//  - MIT License (https://mit-license.org/)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).

use crate::{Interface, Platform};

/// A bitfield specifying which events to watch for
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct Watch(pub(crate) u32);

impl Watch {
    /// Watch for input events.
    pub const INPUT: Self = Self(Platform::WATCH_INPUT);
    /// Watch for output events.
    pub const OUTPUT: Self = Self(Platform::WATCH_OUTPUT);

    /// Add output to events that are being watched for.
    pub fn output(self) -> Self {
        Self(self.0 | Platform::WATCH_OUTPUT)
    }

    /// Add input to events that are being watched for.
    pub fn input(self) -> Self {
        Self(self.0 | Platform::WATCH_INPUT)
    }

    /// Construct watch from raw bitfield.
    ///
    /// # Safety
    /// Must be a valid bitfield of the platform's event types.
    pub unsafe fn from_raw(watch: u32) -> Self {
        Self(watch)
    }
}
