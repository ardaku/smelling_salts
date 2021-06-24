//! This example is public domain.

use smelling_salts::{Device, Watcher};

use std::convert::TryInto;
use std::future::Future;
use std::mem;
use std::os::raw;
use std::pin::Pin;
use std::task::{Context, Poll};

#[allow(non_camel_case_types)]
type c_ssize = isize; // True for most unix
#[allow(non_camel_case_types)]
type c_size = usize; // True for most unix

const MAGIC_NUMBER: u32 = 0xDEAD_BEEF;

// From fcntl.h
const O_CLOEXEC: raw::c_int = 0o2000000;
const O_NONBLOCK: raw::c_int = 0o0004000;
const O_DIRECT: raw::c_int = 0o0040000;

extern "C" {
    fn pipe2(pipefd: *mut [raw::c_int; 2], flags: raw::c_int) -> raw::c_int;
    fn write(fd: raw::c_int, buf: *const raw::c_void, count: c_size)
        -> c_ssize;
    fn read(fd: raw::c_int, buf: *mut raw::c_void, count: c_size) -> c_ssize;
    fn close(fd: raw::c_int) -> raw::c_int;
}

// Convert a C error (negative on error) into a result.
fn error(err: raw::c_int) -> Result<(), raw::c_int> {
    if err < 0 {
        Err(err)
    } else {
        Ok(())
    }
}

fn fd_close(fd: raw::c_int) {
    // close() should never fail.
    let ret = unsafe { close(fd) };
    error(ret).unwrap();
}

// Create the sender and receiver for a pipe.
fn new_pipe() -> (raw::c_int, raw::c_int) {
    let [recver, sender] = unsafe {
        // Create pipe for communication
        let mut pipe = mem::MaybeUninit::<[raw::c_int; 2]>::uninit();
        error(pipe2(pipe.as_mut_ptr(), O_CLOEXEC | O_NONBLOCK | O_DIRECT))
            .unwrap();
        pipe.assume_init()
    };

    (sender, recver)
}

fn write_u32(fd: raw::c_int, data: u32) {
    let data = [data];
    let len: usize = unsafe {
        write(fd, data.as_ptr().cast(), mem::size_of::<u32>())
            .try_into()
            .unwrap()
    };
    assert_eq!(len, mem::size_of::<u32>());
}

fn read_u32(fd: raw::c_int) -> Option<u32> {
    let ret = unsafe {
        let mut buffer = mem::MaybeUninit::<u32>::uninit();
        let len: usize =
            read(fd, buffer.as_mut_ptr().cast(), mem::size_of::<u32>())
                .try_into()
                .unwrap_or(0);
        if len == 0 {
            return None;
        }
        assert_eq!(len, mem::size_of::<u32>());
        buffer.assume_init()
    };
    Some(ret)
}

pub struct PipeReceiver(Device);

impl PipeReceiver {
    pub fn new(fd: raw::c_int) -> Self {
        PipeReceiver(Device::new(fd, Watcher::new().input()))
    }
}

impl Drop for PipeReceiver {
    fn drop(&mut self) {
        // Deregister FD, then delete (must be in this order).
        fd_close(self.0.stop());
    }
}

pub struct PipeFuture<'a>(&'a mut PipeReceiver);

impl<'a> PipeFuture<'a> {
    pub fn new(recver: &'a mut PipeReceiver) -> Self {
        PipeFuture(recver)
    }
}

impl<'a> Future for PipeFuture<'a> {
    type Output = u32;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if let Some(output) = read_u32((self.0).0.raw()) {
            Poll::Ready(output)
        } else {
            (self.0).0.sleep(cx)
        }
    }
}

async fn async_main() {
    let (sender, recver) = new_pipe();
    let mut device = PipeReceiver::new(recver);

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        write_u32(sender, MAGIC_NUMBER);
        fd_close(sender);
    });

    let output = PipeFuture::new(&mut device).await;
    assert_eq!(output, MAGIC_NUMBER);
}

fn main() {
    pasts::block_on(async_main());
}
