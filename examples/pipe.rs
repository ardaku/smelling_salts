use std::{
    convert::TryInto,
    mem::{self, MaybeUninit},
    os::{raw, unix::io::RawFd},
};

use pasts::prelude::*;
use smelling_salts::epoll::Device;

// From fcntl.h
const O_CLOEXEC: raw::c_int = 0o2000000;
const O_NONBLOCK: raw::c_int = 0o0004000;
const O_DIRECT: raw::c_int = 0o0040000;

extern "C" {
    fn pipe2(pipefd: *mut [RawFd; 2], flags: raw::c_int) -> RawFd;
    fn write(fd: RawFd, buf: *const raw::c_void, count: usize) -> isize;
    fn read(fd: RawFd, buf: *mut raw::c_void, count: usize) -> isize;
    fn close(fd: RawFd) -> raw::c_int;
}

/// A `PipeReceiver` device future.
pub struct PipeReceiver(Option<Device>);

impl Notifier for PipeReceiver {
    type Event = u32;

    fn poll_next(mut self: Pin<&mut Self>, exec: &mut Exec<'_>) -> Poll<u32> {
        let device = self.0.as_mut().unwrap();

        if Pin::new(&mut *device).poll_next(exec).is_pending() {
            return Pending;
        }

        unsafe {
            let mut x = MaybeUninit::<u32>::uninit();
            if read(device.fd(), x.as_mut_ptr().cast(), mem::size_of::<u32>())
                == mem::size_of::<u32>().try_into().unwrap()
            {
                Ready(x.assume_init())
            } else {
                Pending
            }
        }
    }
}

impl Drop for PipeReceiver {
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
            let ret = close(self.0);
            assert_eq!(0, ret);
        }
    }
}

/// Create a new `Pipe`.
pub fn pipe() -> (PipeReceiver, PipeSender) {
    let [fd, sender] = unsafe {
        // Create pipe for communication
        let mut pipe = mem::MaybeUninit::<[raw::c_int; 2]>::uninit();
        let ret = pipe2(pipe.as_mut_ptr(), O_CLOEXEC | O_NONBLOCK | O_DIRECT);
        assert!(ret >= 0);
        pipe.assume_init()
    };

    let device = Some(Device::builder().input().watch(fd));

    (PipeReceiver(device), PipeSender(sender))
}

// Usage

const MAGIC_NUMBER: u32 = 0xDEAD_BEEF;

fn main() {
    pasts::Executor::default().spawn(async {
        let (mut recver, sender) = pipe();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            sender.send(MAGIC_NUMBER);
        });

        let value = recver.next().await;
        assert_eq!(value, MAGIC_NUMBER);
    });
}
