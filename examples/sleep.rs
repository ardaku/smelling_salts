#![deny(unsafe_code)]

/// Timer module
mod timer {
    #![allow(unsafe_code)]

    use pasts::prelude::*;
    use smelling_salts::linux::{Device, Watcher};
    use std::convert::TryInto;
    use std::mem::{self, MaybeUninit};
    use std::os::raw;
    use std::os::unix::io::RawFd;
    use std::ptr;
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
        fn timerfd_create(clockid: raw::c_int, flags: raw::c_int) -> RawFd;
        fn timerfd_settime(
            fd: RawFd,
            flags: raw::c_int,
            new_value: *const ITimerSpec,
            old_value: *mut ITimerSpec,
        ) -> raw::c_int;
        fn read(fd: RawFd, buf: *mut u64, count: usize) -> isize;
    }

    /// A `Timer` device future.
    pub struct Timer(Device, RawFd);

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
            let watcher = Watcher::new().input();
            Self(Device::new(fd, watcher, true), fd)
        }
    }

    impl Notifier for Timer {
        type Event = usize;

        fn poll_next(self: Pin<&mut Self>, exec: &mut Exec<'_>) -> Poll<usize> {
            let fd = self.1;
            let mut this = self.get_mut();
            let ret = Pin::new(&mut this.0).poll_next(exec).map(|()| unsafe {
                let mut x = MaybeUninit::<u64>::uninit();
                let v = read(fd, x.as_mut_ptr(), mem::size_of::<u64>());
                if v == mem::size_of::<u64>().try_into().unwrap() {
                    x.assume_init().try_into().unwrap()
                } else {
                    0
                }
            });
            if ret == Poll::Ready(0) {
                Pin::new(&mut this).poll_next(exec)
            } else {
                ret
            }
        }
    }
}

use self::timer::Timer;
use pasts::prelude::*;

fn main() {
    pasts::Executor::default().spawn(Box::pin(async {
        let mut timer = Timer::new(std::time::Duration::from_secs_f32(1.0));
        println!("Sleeping for 1 second 5 times…");
        for i in 0..5 {
            timer.next().await;
            println!("Slept {} time(s)!", i + 1);
        }
    }));
}
