// Smelling Salts
// Copyright © 2020-2021 Jeron Aldaron Lau.
//
// Licensed under any of:
// - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
// - MIT License (https://mit-license.org/)
// - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).

use crate::watcher::Watcher;

use std::mem::MaybeUninit;
use std::os::raw;
use std::os::unix::io::RawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::task::Waker;
use std::thread;

/// On Linux, `RawDevice` corresponds to [RawFd](std::os::unix::io::RawFd)
pub type RawDevice = RawFd;

const EPOLL_CTL_ADD: i32 = 1;
const EPOLL_CTL_DEL: i32 = 2;

const O_CLOEXEC: raw::c_int = 0x0008_0000;

#[repr(C)]
#[derive(Copy, Clone)]
union EpollData {
    ptr: *mut raw::c_void,
    fd: RawFd,
    uint32: u32,
    uint64: u64,
}

#[repr(packed, C)]
#[derive(Copy, Clone)]
struct EpollEvent {
    events: u32,     /* Epoll events */
    data: EpollData, /* User data variable */
}

extern "C" {
    fn epoll_create1(flags: raw::c_int) -> raw::c_int;
    // fn close(fd: RawFd) -> raw::c_int;
    fn epoll_ctl(
        epfd: RawFd,
        op: raw::c_int,
        fd: RawFd,
        event: *mut EpollEvent,
    ) -> raw::c_int;
    fn epoll_wait(
        epfd: RawFd,
        events: *mut EpollEvent,
        maxevents: raw::c_int,
        timeout: raw::c_int,
    ) -> raw::c_int;
}

// Convert a C error (negative on error) into a result.
fn error(err: raw::c_int) -> Result<(), raw::c_int> {
    if err < 0 {
        extern "C" {
            fn __errno_location() -> *mut raw::c_int;
        }
        Err(unsafe { *__errno_location() })
    } else {
        Ok(())
    }
}

// Initializer for shared context.
static ONCE: Once = Once::new();
// Shared context.
static mut SHARED: MaybeUninit<&'static SharedCx> = MaybeUninit::uninit();

type ProtectCx = (Vec<(RawDevice, Option<Waker>, Arc<AtomicBool>)>, Vec<usize>);

// Devices and their associated wakers.
#[derive(Debug)]
struct SharedCx {
    // The epoll descriptor.
    poll: raw::c_int,
    // File descriptor, waker and ready flag for each device.  Plus garbage.
    devs: Mutex<ProtectCx>,
}

impl SharedCx {
    /// Lazily initialize and connect to the shared context.
    fn new() -> &'static Self {
        ONCE.call_once(|| unsafe {
            // Create a new epoll instance.
            let epoll_fd = epoll_create1(O_CLOEXEC);
            error(epoll_fd).unwrap();

            // Set shared context.
            SHARED = MaybeUninit::new(Box::leak(Box::new(SharedCx {
                poll: epoll_fd,
                devs: Mutex::new((Vec::new(), Vec::new())),
            })));
            // Start background waker thread.
            thread::spawn(|| hardware_thread(SHARED.assume_init()));
        });
        unsafe { SHARED.assume_init() }
    }

    /// Add a device to listen to.  Returns index.
    fn add(
        &self,
        dev_fd: RawDevice,
        watcher: Watcher,
        ready: Arc<AtomicBool>,
    ) -> usize {
        let mut cx = self.devs.lock().unwrap();
        // Allocate spot.
        let state = (dev_fd, None, ready);
        let index = if let Some(index) = cx.1.pop() {
            cx.0[index] = state;
            index
        } else {
            let index = cx.0.len();
            cx.0.push(state);
            index
        };
        // Register with epoll
        unsafe {
            error(epoll_ctl(
                self.poll,
                EPOLL_CTL_ADD,
                dev_fd,
                &mut EpollEvent {
                    events: watcher.0,
                    data: EpollData {
                        uint64: index as u64,
                    },
                },
            ))
        }
        .unwrap();
        // Return index
        index
    }

    /// Register waker.  Returns true if should wake immediately.
    fn reg(&self, index: usize, waker: Waker) -> bool {
        let mut cx = self.devs.lock().unwrap();
        if cx.0[index].2.load(Ordering::SeqCst) {
            return true;
        }
        cx.0[index].1 = Some(waker);
        false
    }

    /// Stop listening to a device.
    fn del(&self, index: usize) {
        let mut cx = self.devs.lock().unwrap();
        // Add to trash.
        cx.1.push(index);
        // Unregister from epoll, ignore any error (mostly ENOENT, already
        // deleted).
        let _ = unsafe {
            error(epoll_ctl(
                self.poll,
                EPOLL_CTL_DEL,
                cx.0[index].0,
                &mut EpollEvent {
                    /* ignored, can't be null, though */
                    events: 0,
                    data: EpollData { uint64: 0 },
                },
            ))
        };
    }
}

// This function checks for hardware events using epoll_wait (blocking I/O) in
// a loop.
fn hardware_thread(shared: &SharedCx) {
    // An infinite loop that goes until the program exits.
    loop {
        // Wait for a device to be ready.
        let mut ev = MaybeUninit::<EpollEvent>::uninit();
        if unsafe {
            epoll_wait(
                shared.poll,
                ev.as_mut_ptr(),
                1,  /* Get one event at a time */
                -1, /* wait indefinitely */
            )
        } < 0
        {
            // Ignore error
            continue;
        }
        // Get the index of the device.
        let index = unsafe { ev.assume_init().data.uint64 } as usize;
        //
        let mut cx = shared.devs.lock().unwrap();

        // Mark as ready and wake.
        cx.0[index].2.store(true, Ordering::SeqCst);
        if let Some(waker) = cx.0[index].1.take() {
            waker.wake();
        }
    }
}

/// Represents some device.
#[derive(Debug)]
pub(super) struct Device {
    // Shared context.
    shared: &'static SharedCx,
    // File descriptor.
    index: usize,
    // Check if it's ready.
    ready: Arc<AtomicBool>,
    // Raw File descriptor (FIXME: Remove?)
    raw: RawFd,
}

impl Device {
    /// Start checking for events on a new device from a linux file descriptor.
    pub(super) fn new(raw: RawFd, events: Watcher) -> Self {
        // Start background thread if not running, and get state.
        let shared = SharedCx::new();
        // Default to ready
        let ready = Arc::new(AtomicBool::new(true));
        // Start listening for events on the file descriptor.
        let index = shared.add(raw, events, ready.clone());
        // Return the device
        Device {
            shared,
            index,
            raw,
            ready,
        }
    }

    /// Register a waker to wake when the device gets an event.
    pub(super) fn register_waker(&self, waker: &Waker) {
        if self.shared.reg(self.index, waker.clone()) {
            waker.wake_by_ref();
        }
    }

    /// Convenience function to get the raw File Descriptor of the Device.
    pub(super) fn raw(&self) -> RawFd {
        self.raw
    }

    /// Stop checking for events on a device from a linux file descriptor.
    pub(super) fn old(&mut self) {
        self.shared.del(self.index);
    }

    /// Check if should yield to executor.
    pub(super) fn should_yield(&self) -> bool {
        !self.ready.load(Ordering::SeqCst)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.old();
    }
}
