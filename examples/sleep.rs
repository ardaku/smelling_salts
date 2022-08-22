use std::{
    convert::TryInto,
    mem::{self, MaybeUninit},
    os::{raw, unix::io::RawFd},
    ptr,
    time::Duration,
};

use pasts::prelude::*;
use smelling_salts::epoll::Device;

#[repr(C)]
struct TimeSpec {
    sec: isize,
    nsec: raw::c_long,
}

#[repr(C)]
struct ITimerSpec {
    interval: TimeSpec,
    value: TimeSpec,
}

extern "C" {
    fn timerfd_create(clockid: raw::c_int, flags: raw::c_int) -> RawFd;
    fn timerfd_settime(
        fd: RawFd,
        flags: raw::c_int,
        new_value: *const ITimerSpec,
        old_value: *mut ITimerSpec,
    ) -> raw::c_int;
    fn read(fd: RawFd, buf: *mut u64, count: usize) -> isize;
    fn close(fd: RawFd) -> raw::c_int;
}

/// A `Timer` device future.
pub struct Timer(Option<Device>);

impl Drop for Timer {
    fn drop(&mut self) {
        // Remove from watchlist
        let device = self.0.take().unwrap();
        let fd = device.fd();
        drop(device);
        // Close file descriptor
        let ret = unsafe { close(fd) };
        assert_eq!(0, ret);
    }
}

impl Timer {
    /// Create a new `Timer`.
    pub fn new(dur: Duration) -> Self {
        // Create Monotonic (1), Non-Blocking (2048) Timer
        let fd = unsafe { timerfd_create(1, 2048) };
        let sec = dur.as_secs() as _;
        let nsec = dur.subsec_nanos() as _;
        let its = ITimerSpec {
            interval: TimeSpec { sec, nsec },
            value: TimeSpec { sec, nsec },
        };
        let _ret = unsafe { timerfd_settime(fd, 0, &its, ptr::null_mut()) };
        assert_eq!(0, _ret);
        Self(Some(Device::builder().input().build(fd)))
    }
}

impl Notifier for Timer {
    type Event = usize;

    fn poll_next(mut self: Pin<&mut Self>, exec: &mut Exec<'_>) -> Poll<usize> {
        let device = self.0.as_mut().unwrap();

        if Pin::new(&mut *device).poll_next(exec).is_pending() {
            return Pending;
        }

        unsafe {
            let mut x = MaybeUninit::<u64>::uninit();
            if read(device.fd(), x.as_mut_ptr(), mem::size_of::<u64>())
                == mem::size_of::<u64>().try_into().unwrap()
            {
                let count = x.assume_init().try_into().unwrap();
                if count == 0 {
                    Pending
                } else {
                    Ready(count)
                }
            } else {
                Pending
            }
        }
    }
}

// Usage

fn main() {
    pasts::Executor::default().spawn(async {
        let mut timer = Timer::new(std::time::Duration::from_secs_f32(1.0));
        println!("Sleeping for 1 second 5 times…");
        for i in 1..=5 {
            timer.next().await;
            println!("Slept {i} time(s)!");
        }
    });
}
