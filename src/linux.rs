// Copyright © 2020-2022 The Smelling Salts Contributors.
//
// Licensed under any of:
//  - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
//  - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
//  - MIT License (https://mit-license.org/)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).
//!
//! Linux Smelling Salts API.
//!
//! ```rust,no_run
#![doc = include_str!("../examples/sleep.rs")]
//! ```

#![allow(unsafe_code)]

use pasts::prelude::*;

use std::fmt::{Debug, Formatter};
use std::mem::MaybeUninit;
use std::os::raw::{c_int, c_void};
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::task::Waker;

pub use crate::watcher::Watcher;

#[repr(C)]
union EpollData {
    ptr: *mut c_void,
    fd: c_int,
    uint32: u32,
    uint64: u64,
}

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
    data: EpollData,
}

extern "C" {
    fn epoll_create1(flags: c_int) -> RawFd;
    fn epoll_wait(
        epfd: RawFd,
        events: *mut EpollEvent,
        maxevents: c_int,
        timeout: c_int,
    ) -> c_int;
    fn epoll_ctl(
        epfd: RawFd,
        op: c_int,
        fd: RawFd,
        event: *mut EpollEvent,
    ) -> c_int;
    fn close(fd: RawFd) -> c_int;
}

static START: Once = Once::new();
static mut SLEEPER: MaybeUninit<Sleeper> = MaybeUninit::uninit();

struct Sleeper {
    epoll_fd: RawFd,
}

impl Sleeper {
    fn register(
        &self,
        fd: RawFd,
        events: Watcher,
        device: *mut DeviceInternal,
    ) {
        let data = EpollData { ptr: device.cast() };
        let events = events.0;
        let mut event = EpollEvent { events, data };
        let ret = unsafe { epoll_ctl(self.epoll_fd, 1, fd, &mut event) };
        assert_eq!(ret, 0);
    }

    fn deregister(&self, fd: RawFd) {
        let mut _ev = MaybeUninit::<EpollEvent>::zeroed();
        let ret = unsafe { epoll_ctl(self.epoll_fd, 2, fd, _ev.as_mut_ptr()) };
        assert_eq!(ret, 0);
    }
}

/// Get sleeper.
fn sleeper() -> &'static Sleeper {
    START.call_once(|| unsafe {
        let epoll_fd = epoll_create1(0);
        SLEEPER = MaybeUninit::new(Sleeper { epoll_fd });
        start_thread(epoll_fd);
    });
    unsafe { SLEEPER.assume_init_ref() }
}

struct DeviceInternal {
    // Ready flag for this device.  When true, client owns waker - false server
    // owns waker.
    ready: AtomicBool,
    // Whether the file descriptor should be closed.
    close: bool,
    // Read-only file descriptor associated with this device.
    fd: RawFd,
    // Optional waker
    waker: Option<Waker>,
}

/// [`Notifier`] for asynchronous device events.
pub struct Device(Pin<Box<DeviceInternal>>);

impl Debug for Device {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Device")
    }
}

impl Device {
    /// Create a new device event notifier.
    ///
    ///  - `fd`: The linux file descriptor to register
    ///  - `events`: Which events to watch for
    ///  - `close`: Set to true to close the file descriptor upon drop.
    ///
    /// # Panics
    /// If `fd` cannot be used with epoll, is invalid, already registered, or
    /// out of memory.
    pub fn new(fd: RawFd, events: Watcher, close: bool) -> Self {
        let ready = AtomicBool::new(true);
        let waker = None;
        let device = DeviceInternal {
            ready,
            fd,
            waker,
            close,
        };
        let mut device = Box::pin(device);
        let pointer = Pin::into_inner(device.as_mut());
        sleeper().register(fd, events, pointer);
        Self(device)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        sleeper().deregister(self.0.as_ref().fd);
        if self.0.as_ref().close {
            assert_eq!(0, unsafe { close(self.0.as_ref().fd) });
        }
    }
}

impl Notifier for Device {
    type Event = ();

    fn poll_next(mut self: Pin<&mut Self>, exec: &mut Exec<'_>) -> Poll<()> {
        if self.0.ready.load(Ordering::Acquire) {
            self.0.waker = Some(exec.waker().clone());
            self.0.ready.store(false, Ordering::Release);
            Ready(())
        } else {
            Pending
        }
    }
}

/// Start the Smelling Salts thread.
unsafe fn start_thread(epoll_fd: RawFd) {
    std::thread::spawn(move || loop {
        let mut event = MaybeUninit::<EpollEvent>::uninit();
        // Wait for event, if failed, try again.
        if epoll_wait(epoll_fd, event.as_mut_ptr(), 1, -1) != 1 {
            continue;
        }
        // Since event has succeeded we can assume it's initialized.
        let pointer: *mut DeviceInternal =
            (*event.as_mut_ptr()).data.ptr.cast();
        // Spinlock until ready.
        while (*pointer).ready.load(Ordering::Acquire) {}
        // Release the lock & wake the future
        let maybe_waker = (*pointer).waker.take();
        (*pointer).ready.store(true, Ordering::Release);
        if let Some(w) = maybe_waker {
            w.wake();
        }
    });
}
