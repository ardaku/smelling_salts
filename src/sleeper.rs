// Copyright © 2020-2021 The Smelling Salts Contributors.
//
// Licensed under any of:
// - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
// - MIT License (https://mit-license.org/)
// - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).

#![allow(unsafe_code)]

use crate::Watcher;
use std::convert::TryFrom;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::os::raw::c_int;

/// On Linux, `RawDevice` corresponds to [RawFd](std::os::unix::io::RawFd)
pub type RawDevice = std::os::unix::io::RawFd;

// Epoll event structure.
#[cfg_attr(
    any(
        all(
            target_arch = "x86",
            not(target_env = "musl"),
            not(target_os = "android")
        ),
        target_arch = "x86_64"
    ),
    repr(packed)
)]
#[repr(C)]
struct EpollEvent {
    events: u32,
    data: u64,
}

// Import the epoll API.
extern "C" {
    fn epoll_create1(flags: c_int) -> RawDevice;
    fn epoll_wait(
        epfd: RawDevice,
        events: *mut EpollEvent,
        maxevents: c_int,
        timeout: c_int,
    ) -> c_int;
    fn epoll_ctl(
        epfd: RawDevice,
        op: c_int,
        fd: RawDevice,
        event: *mut EpollEvent,
    ) -> c_int;
    fn close(epfd: RawDevice) -> c_int;
}

/// Put the thread to sleep.  Smelling Salts dependants should start a
/// separate thread to put the `Sleeper` on.  The `Sleeper` should then call
/// epoll on Linux, kqueue on BSD and MacOS, and IOCP on Windows.
///
/// Currently, only supports Linux (epoll).
#[derive(Debug)]
pub struct Sleeper {
    sleeper: RawDevice,
    // Make sure `Sleeper` doesn't implement `Send` or `Sync`.  This is
    // necessary, because if it did, `Device` would be unsound.
    _phantom_data: PhantomData<*mut ()>,
}

impl Drop for Sleeper {
    fn drop(&mut self) {
        let ret = unsafe { close(self.sleeper) };
        assert_eq!(0, ret);
    }
}

impl Default for Sleeper {
    fn default() -> Self {
        let _phantom_data = PhantomData;
        let sleeper = unsafe { epoll_create1(0) };
        Self {
            sleeper,
            _phantom_data,
        }
    }
}

impl Sleeper {
    /// Create a new sleeper.
    pub fn new() -> Sleeper {
        Self::default()
    }

    /// Watch a device for asynchronous events.
    pub fn watch<T: Reactor>(&self, raw: T, events: Watcher) -> Device {
        let raw: Box<dyn Reactor> = Box::new(raw);
        let raw = Box::into_raw(Box::new(raw));
        let sleeper = self.sleeper;
        let data = u64::try_from(raw as usize).unwrap();
        let events = events.0;

        // Add device to watch list on sleeper.
        let mut event = EpollEvent { events, data };
        let ret = unsafe { epoll_ctl(sleeper, 1, (*raw).raw(), &mut event) };
        assert_eq!(ret, 0);

        Device { raw, sleeper }
    }

    /// Get this sleeper as it's own raw device.
    pub fn raw(&self) -> RawDevice {
        self.sleeper
    }

    /// Go into infinitely blocking sleep/wake loop.
    pub fn sleep(&self) {
        let sleeper = self.sleeper;
        let mut event = MaybeUninit::<EpollEvent>::zeroed();

        // Wait for a successful event.
        loop {
            // Wait for event, if failed, try again.
            if unsafe { epoll_wait(sleeper, event.as_mut_ptr(), 1, -1) } != 1 {
                continue;
            }
            // Since event has succeeded we can assume it's initialized.
            let data = unsafe { (*event.as_mut_ptr()).data };
            // Convert data into mutable reference to `T`.
            let data = usize::try_from(data).unwrap();
            let data = data as *mut &mut dyn Reactor;
            unsafe { (*data).react() };
        }
    }
}

/// A hardware device (usually backed by a file descriptor or similar).
#[derive(Debug)]
pub struct Device {
    sleeper: RawDevice,
    raw: *mut Box<dyn Reactor>,
}

impl Device {
    /// Get this sleeper as it's own raw device.
    pub fn raw(&self) -> RawDevice {
        unsafe { (&*self.raw).raw() }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        // Unregister the file descriptor or similar
        unsafe {
            let mut _ev = MaybeUninit::<EpollEvent>::zeroed();
            let ret = epoll_ctl(self.sleeper, 2, self.raw(), _ev.as_mut_ptr());
            assert_eq!(ret, 0);
        }

        // Free the file descriptor or similar
        Reactor::drop(unsafe { &mut **self.raw });
    }
}

/// Asynchronous device state.
pub trait Reactor: 'static {
    /// Get the file descriptor or similar that was used to create the
    /// [`Device`](crate::Device).
    fn raw(&self) -> RawDevice;

    /// Callback for when the [`Device`](crate::Device) is woken.
    ///
    /// You should send a message on an async channel if data is ready in this
    /// callback to wake the application.  This works well with the
    /// [`flume`](https://docs.rs/flume) crate, but you may use other channel
    /// implementations if you wish.
    fn react(&mut self);

    /// Callback for when the [`Device`](crate::Device) is dropped.
    ///
    /// You should call `free()` or a similar function in this callback.
    fn drop(&mut self);
}
