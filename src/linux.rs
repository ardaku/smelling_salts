//! Linux Smelling Salts API.
//!
//! ```rust no_run
//! #![deny(unsafe_code)]
//!
//! /// Timer module
//! mod timer {
//!     #![allow(unsafe_code)]
//!
//!     use smelling_salts::linux::{Device, Watcher};
//!     use std::convert::TryInto;
//!     use std::future::Future;
//!     use std::mem::{self, MaybeUninit};
//!     use std::os::raw;
//!     use std::os::unix::io::RawFd;
//!     use std::pin::Pin;
//!     use std::ptr;
//!     use std::task::{Context, Poll};
//!     use std::time::Duration;
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
//!         fn timerfd_create(clockid: raw::c_int, flags: raw::c_int) -> RawFd;
//!         fn timerfd_settime(
//!             fd: RawFd,
//!             flags: raw::c_int,
//!             new_value: *const ITimerSpec,
//!             old_value: *mut ITimerSpec,
//!         ) -> raw::c_int;
//!         fn read(fd: RawFd, buf: *mut u64, count: usize) -> isize;
//!     }
//!
//!     /// A `Timer` device future.
//!     pub struct Timer(Device, RawFd);
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
//!             let watcher = Watcher::new().input();
//!             Self(Device::new(fd, watcher, true), fd)
//!         }
//!     }
//!
//!     impl Future for Timer {
//!         type Output = usize;
//!         fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<usize> {
//!             let fd = self.1;
//!             let mut this = self.get_mut();
//!             let ret = Pin::new(&mut this.0).poll(cx).map(|()| unsafe {
//!                 let mut x = MaybeUninit::<u64>::uninit();
//!                 let v = read(fd, x.as_mut_ptr(), mem::size_of::<u64>());
//!                 if v == mem::size_of::<u64>().try_into().unwrap() {
//!                     x.assume_init().try_into().unwrap()
//!                 } else {
//!                     0
//!                 }
//!             });
//!             if ret == Poll::Ready(0) {
//!                 Pin::new(&mut this).poll(cx)
//!             } else {
//!                 ret
//!             }
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

#![allow(unsafe_code)]

use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::mem::MaybeUninit;
use std::os::raw::{c_int, c_void};
use std::os::unix::io::RawFd;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::task::{Context, Poll, Waker};

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

/// Asynchronous device future.  Becomes ready when awoken.
pub struct Device(Pin<Box<DeviceInternal>>);

impl Debug for Device {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Device")
    }
}

impl Device {
    /// Create a new asynchronous `Device` future.
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

impl Future for Device {
    type Output = ();

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        if self.0.ready.load(Ordering::Acquire) {
            self.0.waker = Some(cx.waker().clone());
            self.0.ready.store(false, Ordering::Release);
            Poll::Ready(())
        } else {
            Poll::Pending
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
