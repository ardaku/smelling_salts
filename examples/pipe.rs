#![deny(unsafe_code)]

/// Pipe module
mod pipe {
    #![allow(unsafe_code)]

    use smelling_salts::linux::{Device, Watcher};
    use std::convert::TryInto;
    use std::future::Future;
    use std::mem::{self, MaybeUninit};
    use std::os::raw;
    use std::os::unix::io::RawFd;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    // From fcntl.h
    const O_CLOEXEC: raw::c_int = 0o2000000;
    const O_NONBLOCK: raw::c_int = 0o0004000;
    const O_DIRECT: raw::c_int = 0o0040000;

    extern "C" {
        fn pipe2(pipefd: *mut [RawFd; 2], flags: raw::c_int) -> RawFd;
        fn write(fd: RawFd, buf: *const raw::c_void, count: usize)
            -> isize;
        fn read(fd: RawFd, buf: *mut raw::c_void, count: usize) -> isize;
        fn close(fd: RawFd) -> raw::c_int;
    }

    /// A `PipeReceiver` device future.
    pub struct PipeReceiver(Device, RawFd);

    impl Future for PipeReceiver {
        type Output = u32;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
            let fd = self.1;
            let mut this = self.get_mut();
            let ret = Pin::new(&mut this.0).poll(cx).map(|()| unsafe {
                let mut x = MaybeUninit::<u32>::uninit();
                let v = read(fd, x.as_mut_ptr().cast(), mem::size_of::<u32>());
                if v == mem::size_of::<u32>().try_into().unwrap() {
                    Some(x.assume_init().try_into().unwrap())
                } else {
                    None
                }
            });
            match ret {
                Poll::Ready(None) => Pin::new(&mut this).poll(cx),
                Poll::Ready(Some(x)) => Poll::Ready(x),
                Poll::Pending => Poll::Pending,
            }
        }
    }

    /// A `PipeSender` device.
    pub struct PipeSender(RawFd);

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

        let watcher = Watcher::new().input();
        (
            PipeReceiver(Device::new(fd, watcher, true), fd),
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
