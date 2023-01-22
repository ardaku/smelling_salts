use std::{
    fs::File,
    io::{Read, Write},
    mem::{self, MaybeUninit},
    os::{
        fd::{FromRawFd, OwnedFd, RawFd},
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
}

/// A `PipeReceiver` device future.
pub struct PipeReceiver(Device);

impl Notifier for PipeReceiver {
    type Event = u32;

    fn poll_next(mut self: Pin<&mut Self>, task: &mut Task<'_>) -> Poll<u32> {
        while let Ready(()) = Pin::new(&mut self.0).poll_next(task) {
            let mut bytes = [0; mem::size_of::<u32>()];
            let Err(e) = self.0.read_exact(&mut bytes) else {
                return Ready(u32::from_ne_bytes(bytes));
            };

            dbg!(e);
        }

        Pending
    }
}

/// A `PipeSender` device.
pub struct PipeSender(File);

impl PipeSender {
    /// Send a 32-bit value over the pipe.
    pub fn send(&mut self, value: u32) {
        self.0
            .write_all(&value.to_ne_bytes())
            .expect("Failed to send");
    }
}

/// Create a new `Pipe`.
pub fn pipe() -> (PipeReceiver, PipeSender) {
    let (fd, sender) = unsafe {
        // Create pipe for communication
        let mut pipe = MaybeUninit::<[raw::c_int; 2]>::uninit();
        let ret = pipe2(pipe.as_mut_ptr(), O_CLOEXEC | O_NONBLOCK | O_DIRECT);
        assert!(ret >= 0);
        let [fd, sender] = pipe.assume_init();
        assert_ne!(fd, -1);
        assert_ne!(sender, -1);
        (OwnedFd::from_raw_fd(fd), OwnedFd::from_raw_fd(sender))
    };

    let device = Device::builder().input().watch(fd);

    (PipeReceiver(device), PipeSender(sender.into()))
}

// Usage

const MAGIC_NUMBER: u32 = 0xDEAD_BEEF;

#[async_main]
async fn main(_spawner: impl Spawn) {
    let (mut recver, mut sender) = pipe();

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        sender.send(MAGIC_NUMBER);
    });

    let value = recver.next().await;
    assert_eq!(value, MAGIC_NUMBER);
}
