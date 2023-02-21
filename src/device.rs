// Copyright © 2020-2023 The Smelling Salts Contributors.
//
// Licensed under any of:
//  - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
//  - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
//  - MIT License (https://mit-license.org/)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).

use pasts::prelude::*;
use whisk::Channel;

use crate::{kind::DeviceKind, OsDevice, Platform, Watch};

/// An owned device handle that's being watched for wake events
///
/// Dropping the device will remove it from the watchlist.
#[derive(Debug)]
pub struct Device {
    pub(crate) channel: Channel,
    pub(crate) kind: DeviceKind,
}

impl Device {
    /// Create a device that watches events coming from an [`OsDevice`].
    // It's ok to be unreachable here for mock impl
    #[allow(unreachable_code, unused_variables)]
    pub fn new(fd: impl Into<OsDevice>, watch: Watch) -> Self {
        let kind = fd.into().into();
        let channel = Platform::watch(&kind, watch);

        Self { channel, kind }
    }
}

impl Notify for Device {
    type Event = ();

    fn poll_next(mut self: Pin<&mut Self>, task: &mut Task<'_>) -> Poll {
        Pin::new(&mut self.channel).poll_next(task)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        Platform::unwatch(&self.kind);
    }
}
