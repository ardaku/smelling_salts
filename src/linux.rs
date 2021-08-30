// Copyright © 2020-2021 The Smelling Salts Contributors.
//
// Licensed under any of:
// - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
// - MIT License (https://mit-license.org/)
// - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).
//
//! The Linux version of the Smelling Salts API
//!
//! # Timer Example
//! ```rust no_run
//! #![deny(unsafe_code)]
//! 
//! /// Timer module
//! mod timer {
//!     #![allow(unsafe_code)]
//! 
//!     use flume::Sender;
//!     use smelling_salts::linux::{Device, Driver, RawDevice, Watcher};
//!     use std::convert::TryInto;
//!     use std::future::Future;
//!     use std::mem::{self, MaybeUninit};
//!     use std::os::raw;
//!     use std::pin::Pin;
//!     use std::ptr;
//!     use std::sync::Once;
//!     use std::task::{Context, Poll};
//!     use std::time::Duration;
//! 
//!     fn driver() -> &'static Driver {
//!         static mut DRIVER: MaybeUninit<Driver> = MaybeUninit::uninit();
//!         static ONCE: Once = Once::new();
//!         unsafe {
//!             ONCE.call_once(|| DRIVER = MaybeUninit::new(Driver::new()));
//!             &*DRIVER.as_ptr()
//!         }
//!     }
//! 
//!     #[repr(C)]
//!     struct TimeSpec {
//!         sec: isize,
//!         nsec: raw::c_long,
//!     }
//! 
//!     #[repr(C)]
//!     struct ITimerSpec {
//!         interval: TimeSpec,
//!         value: TimeSpec,
//!     }
//! 
//!     extern "C" {
//!         fn timerfd_create(clockid: raw::c_int, flags: raw::c_int) -> RawDevice;
//!         fn timerfd_settime(
//!             fd: RawDevice,
//!             flags: raw::c_int,
//!             new_value: *const ITimerSpec,
//!             old_value: *mut ITimerSpec,
//!         ) -> raw::c_int;
//!         fn read(fd: RawDevice, buf: *mut u64, count: usize) -> isize;
//!         fn close(fd: RawDevice) -> raw::c_int;
//!     }
//! 
//!     struct TimerDriver(Sender<usize>, RawDevice);
//! 
//!     impl TimerDriver {
//!         unsafe fn callback(&mut self) -> Option<()> {
//!             let mut x = MaybeUninit::<u64>::uninit();
//!             let v = read(self.1, x.as_mut_ptr(), mem::size_of::<u64>());
//!             if v == mem::size_of::<u64>().try_into().unwrap()
//!                 && self.0.send(x.assume_init().try_into().unwrap()).is_err()
//!             {
//!                 driver().discard(self.1);
//!                 let _ret = close(self.1);
//!                 assert_eq!(0, _ret);
//!                 std::mem::drop(std::ptr::read(self));
//!                 return None;
//!             }
//!             Some(())
//!         }
//!     }
//! 
//!     /// A `Timer` device future.
//!     pub struct Timer(Device<usize>);
//! 
//!     impl Timer {
//!         /// Create a new `Timer`.
//!         pub fn new(dur: Duration) -> Self {
//!             // Create Monotonic (1), Non-Blocking (2048) Timer
//!             let fd = unsafe { timerfd_create(1, 2048) };
//!             let sec = dur.as_secs() as _;
//!             let nsec = dur.subsec_nanos() as _;
//!             let its = ITimerSpec {
//!                 interval: TimeSpec { sec, nsec },
//!                 value: TimeSpec { sec, nsec },
//!             };
//!             let _ret = unsafe { timerfd_settime(fd, 0, &its, ptr::null_mut()) };
//!             assert_eq!(0, _ret);
//!             let constructor = |sender| TimerDriver(sender, fd);
//!             let callback = TimerDriver::callback;
//!             let watcher = Watcher::new().input();
//!             Self(driver().device(constructor, fd, callback, watcher))
//!         }
//!     }
//! 
//!     impl Future for Timer {
//!         type Output = usize;
//!         fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<usize> {
//!             Pin::new(&mut self.get_mut().0).poll(cx)
//!         }
//!     }
//! }
//! 
//! // Export the `Timer` future.
//! use timer::Timer;
//! 
//! fn main() {
//!     pasts::block_on(async {
//!         let mut timer = Timer::new(std::time::Duration::from_secs_f32(1.0));
//!         println!("Sleeping for 1 second 5 times…");
//!         for i in 0..5 {
//!             (&mut timer).await;
//!             println!("Slept {} time(s)…", i + 1);
//!         }
//!     });
//! }
//! ```

use std::fmt::{Debug, Error, Formatter};
use std::future::Future;
use std::mem::MaybeUninit;
use std::os::raw::{c_int, c_void};
use std::pin::Pin;
use std::task::{Context, Poll};

pub use crate::watcher::Watcher;

/// On Linux, `RawDevice` corresponds to [RawFd](std::os::unix::io::RawFd)
pub type RawDevice = std::os::unix::io::RawFd;

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
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
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
}

/// Internal state
struct State {
    /// User state.
    user: *mut (),
    /// Callback
    callback: unsafe fn(&mut ()) -> Option<()>,
}

/// Userspace Driver
///
/// Typically, you'll want to store this in a global static, initialized with
/// [`Once`](std::sync::Once).
#[derive(Debug, Copy, Clone)]
pub struct Driver(RawDevice);

impl Default for Driver {
    fn default() -> Self {
        Self::new()
    }
}

// Epoll is thread-safe.
#[allow(unsafe_code)]
unsafe impl Send for Driver {}
#[allow(unsafe_code)]
unsafe impl Sync for Driver {}

impl Driver {
    /// Create a new userspace `Driver`.
    ///
    /// Typically in Smelling Salts, the driver will start up a new thread or
    /// task running something like epoll, kqueue or iocp.  Currently, Smelling
    /// Salts only supports Linux.  The thread will continue running until the
    /// program exits.
    #[allow(unsafe_code)]
    pub fn new() -> Self {
        let epoll = unsafe { epoll_create1(0) };

        std::thread::spawn(move || loop {
            let mut event = MaybeUninit::<EpollEvent>::zeroed();
            // Wait for event, if failed, try again.
            if unsafe { epoll_wait(epoll, event.as_mut_ptr(), 1, -1) } != 1 {
                continue;
            }
            // Since event has succeeded we can assume it's initialized.
            let data = unsafe { (*event.as_mut_ptr()).data.ptr };
            // Cast data to `State`
            let state: *mut State = data.cast();
            // Run the callback
            unsafe {
                if ((*state).callback)(&mut *(*state).user).is_none() {
                    // Move data out of C allocated memory.
                    let data = std::ptr::read_unaligned(state);
                    free(state.cast());

                    // Free user data.
                    let _user = Box::from_raw(data.user);
                }
            }
        });

        Driver(epoll)
    }

    /// Register a new device from a `RawDevice`.
    ///  - `constructor`: Provide a function to build the state.
    ///  - `raw_device`: The operating system's device type.
    ///  - `callback`: callback for when the device is ready to be read/written.
    ///    - `state`: Userspace driver state.
    ///
    /// When the `Sender` fails with the `Disconnect` error, it is up to the
    /// programmer to free all resources.  You should always call `send()`
    /// instead of `try_send()` to block so that if the application falls behind
    /// you don't build up a huge queue.
    ///
    /// # Safety
    /// If callback returns `None`, it **must** have called
    /// [`Driver::discard()`](crate::linux::Driver::discard) on the file
    /// descriptor or else it's undefined behavior.
    #[allow(unsafe_code)]
    pub fn device<E, S, F>(
        &self,
        constructor: F,
        raw_device: RawDevice,
        callback: unsafe fn(&mut S) -> Option<()>,
        events: Watcher,
    ) -> Device<E>
    where
        E: 'static,
        F: FnOnce(flume::Sender<E>) -> S,
    {
        // Build a double-buffered bounded channel.
        let (sender, receiver) = flume::bounded(2);

        // Allocate state.
        let state = unsafe {
            let state = malloc(std::mem::size_of::<State>()).cast();
            *state = State {
                user: Box::into_raw(Box::new(constructor(sender))).cast(),
                callback: std::mem::transmute(callback),
            };
            state.cast()
        };

        // Add
        let data = EpollData { ptr: state };
        let events = events.0;
        let mut event = EpollEvent { events, data };
        let ret = unsafe { epoll_ctl(self.0, 1, raw_device, &mut event) };
        assert_eq!(ret, 0);

        // Return the `Device`.
        Device(receiver.into_recv_async())
    }

    /// Discard device data (stop listening for events).
    ///
    /// If you call this without returning `Result::Err` from your callback, you
    /// *will* leak memory.
    #[allow(unsafe_code)]
    pub fn discard(&self, device: RawDevice) {
        // Deregister the file descriptor
        let fd = device;
        let mut _ev = MaybeUninit::<EpollEvent>::zeroed();
        let ret = unsafe { epoll_ctl(self.0, 2, fd, _ev.as_mut_ptr()) };
        assert_eq!(ret, 0);
    }
}

/// Hardware device.
pub struct Device<E: 'static>(flume::r#async::RecvFut<'static, E>);

impl<E> Debug for Device<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        write!(f, "Device")
    }
}

impl<E: 'static> Future for Device<E> {
    type Output = E;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Should never disconnect.
        Pin::new(&mut self.get_mut().0).poll(cx).map(|x| x.unwrap())
    }
}
