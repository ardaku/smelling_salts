// Copyright © 2020-2023 The Smelling Salts Contributors.
//
// Licensed under any of:
//  - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
//  - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
//  - MIT License (https://mit-license.org/)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).

#[derive(Debug)]
pub(crate) enum DeviceKind {
    #[cfg(any(unix, target_os = "wasi"))]
    OwnedFd(std::os::fd::OwnedFd),
}

/// An owned handle to an unwatched OS device
#[derive(Debug)]
#[repr(transparent)]
pub struct OsDevice(DeviceKind);

#[cfg(any(unix, target_os = "wasi"))]
impl From<std::os::fd::OwnedFd> for OsDevice {
    fn from(fd: std::os::fd::OwnedFd) -> Self {
        Self(DeviceKind::OwnedFd(fd))
    }
}

#[cfg(any(unix, target_os = "wasi"))]
impl std::os::fd::FromRawFd for OsDevice {
    unsafe fn from_raw_fd(fd: std::os::fd::RawFd) -> Self {
        Self(DeviceKind::OwnedFd(std::os::fd::OwnedFd::from_raw_fd(fd)))
    }
}

impl From<OsDevice> for DeviceKind {
    fn from(os_device: OsDevice) -> Self {
        os_device.0
    }
}
