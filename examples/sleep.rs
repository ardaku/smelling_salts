#![deny(unsafe_code)]

/// Timer module
mod timer {
    #![allow(unsafe_code)]

    use flume::Sender;
    use smelling_salts::linux::{Device, Driver, RawDevice, Watcher};
    use std::convert::TryInto;
    use std::future::Future;
    use std::mem::{self, MaybeUninit};
    use std::os::raw;
    use std::pin::Pin;
    use std::ptr;
    use std::sync::Once;
    use std::task::{Context, Poll};
    use std::time::Duration;

    fn driver() -> &'static Driver {
        static mut DRIVER: MaybeUninit<Driver> = MaybeUninit::uninit();
        static ONCE: Once = Once::new();
        unsafe {
            ONCE.call_once(|| DRIVER = MaybeUninit::new(Driver::new()));
            &*DRIVER.as_ptr()
        }
    }

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
        fn timerfd_create(clockid: raw::c_int, flags: raw::c_int) -> RawDevice;
        fn timerfd_settime(
            fd: RawDevice,
            flags: raw::c_int,
            new_value: *const ITimerSpec,
            old_value: *mut ITimerSpec,
        ) -> raw::c_int;
        fn read(fd: RawDevice, buf: *mut u64, count: usize) -> isize;
        fn close(fd: RawDevice) -> raw::c_int;
    }

    struct TimerDriver(Sender<usize>, RawDevice);

    impl TimerDriver {
        unsafe fn callback(&mut self) -> Option<()> {
            let mut x = MaybeUninit::<u64>::uninit();
            let v = read(self.1, x.as_mut_ptr(), mem::size_of::<u64>());
            if v == mem::size_of::<u64>().try_into().unwrap()
                && self.0.send(x.assume_init().try_into().unwrap()).is_err()
            {
                driver().discard(self.1);
                let _ret = close(self.1);
                assert_eq!(0, _ret);
                return None;
            }
            Some(())
        }
    }

    /// A `Timer` device future.
    pub struct Timer(Device<usize>);

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
            let constructor = |sender| TimerDriver(sender, fd);
            let callback = TimerDriver::callback;
            let watcher = Watcher::new().input();
            Self(driver().device(constructor, fd, callback, watcher))
        }
    }

    impl Future for Timer {
        type Output = usize;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<usize> {
            Pin::new(&mut self.get_mut().0).poll(cx)
        }
    }
}

// Export the `Timer` future.
use timer::Timer;

fn main() {
    pasts::block_on(async {
        let mut timer = Timer::new(std::time::Duration::from_secs_f32(1.0));
        println!("Sleeping for 1 second 5 times…");
        for i in 0..5 {
            (&mut timer).await;
            println!("Slept {} time(s)…", i + 1);
        }
    });
}
