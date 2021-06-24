//! This example is public domain.

use smelling_salts::{Device, Watcher};
use std::future::Future;
use std::mem::MaybeUninit;
use std::os::raw;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

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
    fn timerfd_create(clockid: raw::c_int, flags: raw::c_int) -> raw::c_int;
    fn timerfd_settime(
        fd: raw::c_int,
        flags: raw::c_int,
        new_value: *const ITimerSpec,
        old_value: *mut ITimerSpec,
    ) -> raw::c_int;
    fn read(fd: raw::c_int, buf: *mut u64, count: usize) -> isize;
    fn close(fd: raw::c_int) -> raw::c_int;
    fn __errno_location() -> *mut raw::c_int;
}

struct Sleep(Device, u64);

impl Sleep {
    fn new(dur: Duration) -> Self {
        let sec = dur.as_secs() as _;
        let nsec = dur.subsec_nanos() as _;

        let timerfd = unsafe {
            timerfd_create(1 /*Monotonic*/, 2048 /*Nonblock*/)
        };
        let x = unsafe {
            timerfd_settime(
                timerfd,
                0,
                &ITimerSpec {
                    interval: TimeSpec { sec, nsec },
                    value: TimeSpec { sec, nsec },
                },
                std::ptr::null_mut(),
            )
        };
        assert_eq!(0, x);

        Sleep(Device::new(timerfd, Watcher::new().input()), 0)
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        // Queue
        if self.1 != 0 {
            self.1 -= 1;
            return Poll::Ready(());
        }
        // Early return if a different device woke the executor.
        if self.0.pending() {
            return self.0.sleep(cx);
        }
        //
        let mut x = MaybeUninit::<u64>::uninit();
        let v = unsafe {
            read(self.0.raw(), x.as_mut_ptr(), std::mem::size_of::<u64>())
        };
        if v > 0 {
            self.1 += unsafe { x.assume_init() };
            self.poll(cx)
        } else {
            self.0.sleep(cx)
        }
    }
}

impl Drop for Sleep {
    fn drop(&mut self) {
        assert_eq!(0, unsafe { close(self.0.stop()) });
    }
}

fn main() {
    pasts::block_on(async {
        for _ in 0..5 {
            println!("Sleeping for 1 second…");
            Sleep::new(Duration::new(1, 0)).await;
        }
    });
}
