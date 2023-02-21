use std::{
    io::Read,
    mem,
    os::{
        fd::{FromRawFd, RawFd},
        raw,
    },
    ptr,
    time::Duration,
};

use async_main::{async_main, Spawn};
use pasts::prelude::*;
use smelling_salts::{Device, OsDevice, Watch};

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
}

/// A `Timer` device future.
pub struct Timer(Device);

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

        assert_eq!(_ret, 0);
        assert_ne!(fd, -1);

        let fd = unsafe { OsDevice::from_raw_fd(fd) };

        Self(Device::new(fd, Watch::INPUT))
    }
}

impl Notify for Timer {
    type Event = usize;

    fn poll_next(mut self: Pin<&mut Self>, task: &mut Task<'_>) -> Poll<usize> {
        while let Ready(()) = Pin::new(&mut self.0).poll_next(task) {
            let mut bytes = [0; mem::size_of::<u64>()];
            let Err(e) = self.0.read_exact(&mut bytes) else {
                let count = u64::from_ne_bytes(bytes);

                return Ready(count.try_into().unwrap_or(usize::MAX));
            };

            dbg!(e);
        }

        Pending
    }
}

// Usage

#[async_main]
async fn main(_spawner: impl Spawn) {
    let mut timer = Timer::new(Duration::from_secs_f32(1.0));

    println!("Sleeping for 1 second 5 times…");

    for i in 1..=5 {
        let n = timer.next().await;

        println!("Slept {i} time(s)! [{n}]");
    }
}
