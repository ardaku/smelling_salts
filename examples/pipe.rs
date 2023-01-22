use std::{
    convert::TryInto,
    mem::{self, MaybeUninit},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd, RawFd},
        raw,
    },
};

use async_main::async_main;
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
}

/// A `PipeReceiver` device future.
pub struct PipeReceiver(Device);

impl Notifier for PipeReceiver {
    type Event = u32;

    fn poll_next(mut self: Pin<&mut Self>, task: &mut Task<'_>) -> Poll<u32> {
        let device = &mut self.0;

        if Pin::new(&mut *device).poll_next(task).is_pending() {
            return Pending;
        }

        unsafe {
            let mut x = MaybeUninit::<u32>::uninit();
            if read(
                device.as_raw_fd(),
                x.as_mut_ptr().cast(),
                mem::size_of::<u32>(),
            ) == mem::size_of::<u32>().try_into().unwrap()
            {
                Ready(x.assume_init())
            } else {
                Pending
            }
        }
    }
}

/// A `PipeSender` device.
pub struct PipeSender(OwnedFd);

impl PipeSender {
    /// Send a 32-bit value over the pipe.
    pub fn send(&self, value: u32) {
        let data = [value];
        let len: usize = unsafe {
            write(
                self.0.as_raw_fd(),
                data.as_ptr().cast(),
                mem::size_of::<u32>(),
            )
            .try_into()
            .unwrap()
        };
        assert_eq!(len, mem::size_of::<u32>());
    }
}

/// Create a new `Pipe`.
pub fn pipe() -> (PipeReceiver, PipeSender) {
    let (fd, sender) = unsafe {
        // Create pipe for communication
        let mut pipe = mem::MaybeUninit::<[raw::c_int; 2]>::uninit();
        let ret = pipe2(pipe.as_mut_ptr(), O_CLOEXEC | O_NONBLOCK | O_DIRECT);
        assert!(ret >= 0);
        let [fd, sender] = pipe.assume_init();
        assert_ne!(fd, -1);
        assert_ne!(sender, -1);
        (OwnedFd::from_raw_fd(fd), OwnedFd::from_raw_fd(sender))
    };

    let device = Device::builder().input().watch(fd);

    (PipeReceiver(device), PipeSender(sender))
}

// Usage

const MAGIC_NUMBER: u32 = 0xDEAD_BEEF;

#[async_main]
async fn main(_spawner: impl Spawn) {
    let (mut recver, sender) = pipe();

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        sender.send(MAGIC_NUMBER);
    });

    let value = recver.next().await;
    assert_eq!(value, MAGIC_NUMBER);
}
