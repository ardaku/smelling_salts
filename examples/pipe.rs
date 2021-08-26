#![deny(unsafe_code)]

/// Pipe module
mod pipe {
    #![allow(unsafe_code)]

    use flume::Sender;
    use smelling_salts::linux::{Device, Driver, RawDevice, Watcher};
    use std::convert::TryInto;
    use std::future::Future;
    use std::mem::{self, MaybeUninit};
    use std::os::raw;
    use std::pin::Pin;
    use std::sync::Once;
    use std::task::{Context, Poll};

    fn driver() -> &'static Driver {
        static mut DRIVER: MaybeUninit<Driver> = MaybeUninit::uninit();
        static ONCE: Once = Once::new();
        unsafe {
            ONCE.call_once(|| DRIVER = MaybeUninit::new(Driver::new()));
            &*DRIVER.as_ptr()
        }
    }

    // From fcntl.h
    const O_CLOEXEC: raw::c_int = 0o2000000;
    const O_NONBLOCK: raw::c_int = 0o0004000;
    const O_DIRECT: raw::c_int = 0o0040000;

    extern "C" {
        fn pipe2(pipefd: *mut [raw::c_int; 2], flags: raw::c_int) -> RawDevice;
        fn write(fd: RawDevice, buf: *const raw::c_void, count: usize)
            -> isize;
        fn read(fd: RawDevice, buf: *mut raw::c_void, count: usize) -> isize;
        fn close(fd: RawDevice) -> raw::c_int;
    }

    struct PipeDriver(Sender<u32>, RawDevice);

    impl PipeDriver {
        unsafe fn callback(&mut self) -> Option<()> {
            let mut x = MaybeUninit::<u32>::uninit();
            let v = read(self.1, x.as_mut_ptr().cast(), mem::size_of::<u32>());
            if v == mem::size_of::<u32>().try_into().unwrap()
                && self.0.send(x.assume_init()).is_err()
            {
                driver().discard(self.1);
                let _ret = close(self.1);
                assert_eq!(0, _ret);
                return None;
            }
            Some(())
        }
    }

    /// A `PipeReceiver` device future.
    pub struct PipeReceiver(Device<u32>);

    impl Future for PipeReceiver {
        type Output = u32;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
            Pin::new(&mut self.get_mut().0).poll(cx)
        }
    }

    /// A `PipeSender` device.
    pub struct PipeSender(RawDevice);

    impl PipeSender {
        /// Send a 32-bit value over the pipe.
        pub fn send(&self, value: u32) {
            let data = [value];
            let len: usize = unsafe {
                write(self.0, data.as_ptr().cast(), mem::size_of::<u32>())
                    .try_into()
                    .unwrap()
            };
            assert_eq!(len, mem::size_of::<u32>());
        }
    }

    impl Drop for PipeSender {
        fn drop(&mut self) {
            unsafe {
                let _ret = close(self.0);
                assert_eq!(0, _ret);
            }
        }
    }

    /// Create a new `Pipe`.
    pub fn pipe() -> (PipeReceiver, PipeSender) {
        let [fd, sender] = unsafe {
            // Create pipe for communication
            let mut pipe = mem::MaybeUninit::<[raw::c_int; 2]>::uninit();
            let ret =
                pipe2(pipe.as_mut_ptr(), O_CLOEXEC | O_NONBLOCK | O_DIRECT);
            assert!(ret >= 0);
            pipe.assume_init()
        };

        let constructor = |sender| PipeDriver(sender, fd);
        let callback = PipeDriver::callback;
        let watcher = Watcher::new().input();
        (
            PipeReceiver(driver().device(constructor, fd, callback, watcher)),
            PipeSender(sender),
        )
    }
}

use pipe::pipe;

const MAGIC_NUMBER: u32 = 0xDEAD_BEEF;

fn main() {
    pasts::block_on(async {
        let (recver, sender) = pipe();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            sender.send(MAGIC_NUMBER);
        });

        let value = recver.await;
        assert_eq!(value, MAGIC_NUMBER);
    });
}
