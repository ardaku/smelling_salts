// Copyright © 2020-2022 The Smelling Salts Contributors.
//
// Licensed under any of:
//  - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
//  - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
//  - MIT License (https://mit-license.org/)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).
//! # Epoll Smelling Salts API
//!
//! ```rust,no_run
#![doc = include_str!("../examples/sleep.rs")]
//! ```

use std::{
    mem::{self, MaybeUninit},
    os::{
        raw::{c_int, c_void},
        unix::io::RawFd,
    },
    sync::{Arc, Once},
};

use pasts::prelude::*;
use whisk::{Channel, Queue};

type Poll<T = ()> = pasts::prelude::Poll<T>;

const EPOLLIN: u32 = 0x0001;
const EPOLLOUT: u32 = 0x0004;
const EPOLLET: u32 = 1 << 31;

/// Device handle
///
/// Dropping the device will remove it from the watchlist.
#[derive(Debug)]
pub struct Device {
    channel: Channel,
    fd: RawFd,
}

impl Device {
    /// Get a builder for the [`Device`]
    pub fn builder() -> DeviceBuilder {
        DeviceBuilder { events: EPOLLET }
    }

    /// Get the file descriptor for this device
    pub fn fd(&self) -> RawFd {
        self.fd
    }
}

impl Notifier for Device {
    type Event = ();

    fn poll_next(mut self: Pin<&mut Self>, exec: &mut Exec<'_>) -> Poll {
        Pin::new(&mut self.channel).poll_next(exec)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let epoll_fd = state().epoll_fd;
        let mut _ev = MaybeUninit::<EpollEvent>::zeroed();
        let ret = unsafe { epoll_ctl(epoll_fd, 2, self.fd, _ev.as_mut_ptr()) };
        assert_eq!(ret, 0);
    }
}

/// Builder for [`Device`].
#[derive(Debug)]
#[allow(missing_copy_implementations)]
pub struct DeviceBuilder {
    events: u32,
}

impl DeviceBuilder {
    /// Watch for specific events
    pub fn events(mut self, which: u32) -> Self {
        self.events |= which;
        self
    }

    /// Watch for input
    pub fn input(self) -> Self {
        self.events(EPOLLIN)
    }

    /// Watch for output
    pub fn output(self) -> Self {
        self.events(EPOLLOUT)
    }

    /// Finish building the [`Device`]
    pub fn watch(self, fd: RawFd) -> Device {
        let state = state();
        let channel = Channel::new();
        let queue: Arc<Queue> = channel.clone().into();
        let ptr: *mut _ = unsafe { mem::transmute(queue) };
        let data = EpollData { ptr };
        let events = self.events;
        let mut event = EpollEvent { events, data };
        let ret = unsafe { epoll_ctl(state.epoll_fd, 1, fd, &mut event) };
        assert_eq!(ret, 0);

        Device { channel, fd }
    }
}

#[repr(C)]
union EpollData {
    ptr: *mut c_void,
    fd: c_int,
    uint32: u32,
    uint64: u64,
}

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
}

struct State {
    epoll_fd: RawFd,
}

static START: Once = Once::new();
static mut STATE: MaybeUninit<State> = MaybeUninit::uninit();

fn state() -> &'static State {
    START.call_once(|| unsafe {
        let epoll_fd = epoll_create1(0);
        STATE = MaybeUninit::new(State { epoll_fd });
        std::thread::spawn(move || {
            pasts::Executor::default().spawn(epoll(STATE.assume_init_ref()))
        });
    });
    unsafe { STATE.assume_init_ref() }
}

// The main epoll thread
async fn epoll(state: &'static State) {
    unsafe {
        loop {
            // Wait for events
            let mut event = MaybeUninit::uninit();
            if epoll_wait(state.epoll_fd, event.as_mut_ptr(), 1, -1) != 1 {
                // If failed, try again
                continue;
            }

            // Send wake notification
            let pointer = (*event.as_mut_ptr()).data.ptr;
            let queue: Arc<Queue> = mem::transmute(pointer);
            queue.send(()).await;
            mem::forget(queue);
        }
    }
}
