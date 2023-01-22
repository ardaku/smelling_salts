// Copyright © 2020-2023 The Smelling Salts Contributors.
//
// Licensed under any of:
//  - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
//  - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
//  - MIT License (https://mit-license.org/)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).

use whisk::Channel;

use crate::{kind::DeviceKind, Interface, Platform, Watch};

impl Interface for Platform {
    const WATCH_INPUT: u32 = 0b01;
    const WATCH_OUTPUT: u32 = 0b10;

    fn watch(_kind: &DeviceKind, _watch: Watch) -> Channel {
        Channel::new()
    }

    fn unwatch(_kind: &DeviceKind) {}
}
