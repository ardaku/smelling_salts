// Copyright Jeron Aldaron Lau 2020.
// Distributed under either the Apache License, Version 2.0
//    (See accompanying file LICENSE_APACHE_2_0.txt or copy at
//          https://apache.org/licenses/LICENSE-2.0),
// or the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE_BOOST_1_0.txt or copy at
//          https://www.boost.org/LICENSE_1_0.txt)
// at your option. This file may not be copied, modified, or distributed except
// according to those terms.

use crate::watcher::{Watcher, EPOLLIN};

use std::convert::TryInto;
use std::mem;
use std::mem::MaybeUninit;
use std::os::raw;
use std::os::unix::io::RawFd;
use std::sync::Once;
use std::sync::{Condvar, Mutex};
use std::task::Waker;
use std::thread;

/// On Linux, `RawDevice` corresponds to [RawFd](std::os::unix::io::RawFd)
pub type RawDevice = RawFd;

const EPOLL_CTL_ADD: i32 = 1;
const EPOLL_CTL_DEL: i32 = 2;

const O_CLOEXEC: raw::c_int = 0x0008_0000;

#[repr(C)]
#[derive(Copy, Clone)]
union EpollData {
    ptr: *mut raw::c_void,
    fd: RawFd,
    uint32: u32,
    uint64: u64,
}

#[repr(packed, C)]
#[derive(Copy, Clone)]
struct EpollEvent {
    events: u32,     /* Epoll events */
    data: EpollData, /* User data variable */
}

#[allow(non_camel_case_types)]
type c_ssize = isize; // True for most unix
#[allow(non_camel_case_types)]
type c_size = usize; // True for most unix

extern "C" {
    fn epoll_create1(flags: raw::c_int) -> raw::c_int;
    // fn close(fd: RawFd) -> raw::c_int;
    fn epoll_ctl(
        epfd: RawFd,
        op: raw::c_int,
        fd: RawFd,
        event: *mut EpollEvent,
    ) -> raw::c_int;
    fn epoll_wait(
        epfd: RawFd,
        events: *mut EpollEvent,
        maxevents: raw::c_int,
        timeout: raw::c_int,
    ) -> raw::c_int;
    fn pipe2(pipefd: *mut [RawFd; 2], flags: raw::c_int) -> raw::c_int;
    fn write(fd: RawFd, buf: *const raw::c_void, count: c_size) -> c_ssize;
    fn read(fd: RawFd, buf: *mut raw::c_void, count: c_size) -> c_ssize;
}

// Used to initialize the hardware thread.
static INIT: Once = Once::new();
static mut SHARED_CONTEXT: SharedContext = SharedContext::new();

#[derive(Debug, PartialEq, Clone)]
struct DeviceID(u64);

/// A message sent from a thread to the hardware thread.
#[derive(Debug)]
enum Message {
    /// Add device (ID, FD).
    Device(DeviceID, RawFd, Watcher),
    /// There's a new waker for a device.
    Waker(DeviceID, Waker),
    /// Disconnect a device.
    Disconnect(RawFd, *const (Mutex<bool>, Condvar)),
}

// This function checks for hardware events using epoll_wait (blocking I/O) in
// a loop.
fn hardware_thread(recver: RawFd) {
    // Wakers
    let mut wakers: Vec<Option<Waker>> = vec![None];

    // Create a new epoll instance.
    let epoll_fd = unsafe { epoll_create1(O_CLOEXEC) };
    error(epoll_fd).unwrap();

    // Add receiver to epoll.
    unsafe {
        error(epoll_ctl(
            epoll_fd,
            EPOLL_CTL_ADD,
            recver,
            &mut EpollEvent {
                events: EPOLLIN,               /* available for read operations */
                data: EpollData { uint64: 0 }, // Use reserved ID, 0 for pipe
            },
        ))
        .unwrap();
    }

    // An infinite loop that goes until the program exits.
    loop {
        // Wait
        let mut ev = MaybeUninit::<EpollEvent>::uninit();

        // Wait for something to happen.
        if unsafe {
            epoll_wait(
                epoll_fd,
                ev.as_mut_ptr(),
                1,  /* Get one event at a time */
                -1, /* wait indefinitely */
            )
        } < 0
        {
            // Ignore error
            continue;
        }
        let index = DeviceID(unsafe { ev.assume_init().data.uint64 });

        // Check epoll event for which hardware (or recv).
        if index.0 == 0 {
            // Pipe
            let mut buffer = mem::MaybeUninit::<Message>::uninit();
            let len = unsafe {
                read(
                    recver,
                    buffer.as_mut_ptr().cast(),
                    mem::size_of::<Message>(),
                )
            };
            let message = unsafe { buffer.assume_init() };
            assert_eq!(len as usize, mem::size_of::<Message>());
            match message {
                Message::Device(device_id, device_fd, events) => {
                    let index: usize = device_id.0.try_into().unwrap();
                    // Resize wakers Vec
                    wakers.resize(wakers.len().max(index), None);
                    // Register into epoll
                    unsafe {
                        error(epoll_ctl(
                            epoll_fd,
                            EPOLL_CTL_ADD,
                            device_fd,
                            &mut EpollEvent {
                                events: events.0,
                                data: EpollData {
                                    uint64: device_id.0,
                                },
                            },
                        ))
                        .unwrap();
                    }
                }
                Message::Waker(device_id, waker) => {
                    let index: usize = device_id.0.try_into().unwrap();
                    // Place waker into wakers Vec
                    wakers[index - 1] = Some(waker);
                }
                Message::Disconnect(device_fd, pair) => unsafe {
                    // Unregister from epoll
                    error(epoll_ctl(
                        epoll_fd,
                        EPOLL_CTL_DEL,
                        device_fd,
                        &mut EpollEvent {
                            /* ignored, can't be null, though */
                            events: 0,
                            data: EpollData { uint64: 0 },
                        },
                    ))
                    .unwrap();
                    // Let the device thread know we're done.
                    let (lock, cvar) = &*pair;
                    let mut started = lock.lock().unwrap();
                    *started = true;
                    cvar.notify_one();
                },
            }
            continue;
        } else {
            context().lock().unwrap().ready.push(index.clone());
        }

        // File descriptor (device wake)
        let id: usize = index.0.try_into().unwrap();
        if let Some(waker) = wakers[id - 1].take() {
            waker.wake();
        }
    }
}

// Convert a C error (negative on error) into a result.
fn error(err: raw::c_int) -> Result<(), raw::c_int> {
    if err < 0 {
        extern "C" {
            fn __errno_location() -> *mut raw::c_int;
        }
        Err(unsafe { *__errno_location() })
    } else {
        Ok(())
    }
}

fn context() -> &'static mut Mutex<Context> {
    unsafe { &mut *SHARED_CONTEXT.0.as_mut_ptr() }
}

struct Context {
    // Variables for figuring out the next id
    next: DeviceID,
    garbage: Vec<DeviceID>,
    // Send side of the pipe.
    sender: RawFd,
    // List of ready device IDs.
    ready: Vec<DeviceID>,
}

impl Context {
    // Initialize context.
    fn new(sender: RawFd) -> Self {
        Context {
            next: DeviceID(1),
            garbage: Vec::new(),
            sender,
            ready: Vec::new(),
        }
    }

    // Create an ID
    fn create_id(&mut self) -> DeviceID {
        if let Some(id) = self.garbage.pop() {
            id
        } else {
            let ret = DeviceID(self.next.0);
            self.next.0 += 1;
            ret
        }
    }

    // Delete an ID, so it can be re-used.
    fn delete_id(&mut self, device_id: DeviceID) {
        if device_id.0 == self.next.0 - 1 {
            self.next.0 -= 1;
        } else {
            self.garbage.push(device_id);
        }
    }
}

struct SharedContext(MaybeUninit<Mutex<Context>>);

impl SharedContext {
    const fn new() -> Self {
        SharedContext(MaybeUninit::uninit())
    }

    fn init(&mut self, context: Mutex<Context>) {
        self.0 = MaybeUninit::new(context);
    }
}

/// Represents some device.
#[derive(Debug)]
pub(super) struct Device {
    // File descriptor to be registered with epoll.
    raw: RawFd,
    // Software ID for identifying this device.
    device_id: DeviceID,
    // True if old() deconstructor has already been called.
    old: bool,
}

impl Device {
    /// Start checking for events on a new device from a linux file descriptor.
    pub(super) fn new(raw: RawFd, events: Watcher) -> Self {
        // Start thread if it wasn't running before.
        INIT.call_once(|| {
            // Create pipe for communication
            let mut pipe = MaybeUninit::<[RawFd; 2]>::uninit();
            error(unsafe { pipe2(pipe.as_mut_ptr(), O_CLOEXEC) }).unwrap();
            let [recver, sender] = unsafe { pipe.assume_init() };
            // Initialize shared context.
            unsafe { SHARED_CONTEXT.init(Mutex::new(Context::new(sender))) }
            // Start hardware thread
            let _join = thread::spawn(move || hardware_thread(recver));
        });
        // Get a new device ID
        let mut context = context().lock().unwrap();
        let device_id = context.create_id();
        let write_fd = context.sender;
        let message = [Message::Device(DeviceID(device_id.0), raw, events)];

        // Send message to register this device.
        unsafe {
            if write(
                write_fd,
                message.as_ptr().cast(),
                mem::size_of::<Message>(),
            ) as usize
                != mem::size_of::<Message>()
            {
                panic!("Failed write to pipe");
            }
        }

        // Deconstructor hasn't run yet.
        let old = false;

        // Return the device
        Device {
            raw,
            device_id,
            old,
        }
    }

    /// Register a waker to wake when the device gets an event.
    pub(super) fn register_waker(&self, waker: &Waker) {
        assert_eq!(self.old, false);
        let mut context = context().lock().unwrap();
        let write_fd = context.sender;
        let message =
            [Message::Waker(DeviceID(self.device_id.0), waker.clone())];
        unsafe {
            if write(
                write_fd,
                message.as_ptr().cast(),
                mem::size_of::<Message>(),
            ) as usize
                != mem::size_of::<Message>()
            {
                panic!("Failed write to pipe");
            }
        }
        context.ready.retain(|device| device.0 != self.device_id.0);
    }

    /// Convenience function to get the raw File Descriptor of the Device.
    pub(super) fn raw(&self) -> RawFd {
        self.raw
    }

    /// Stop checking for events on a device from a linux file descriptor.
    #[allow(clippy::mutex_atomic)]
    pub(super) fn old(&mut self) {
        // Make sure that this deconstructor hasn't already run.
        if self.old {
            return;
        }
        self.old = true;

        //
        let mut context = context().lock().unwrap();
        let write_fd = context.sender;
        let pair = Box::pin((Mutex::new(false), Condvar::new()));
        let message = [Message::Disconnect(self.raw, &*pair)];
        // Unregister ID
        unsafe {
            if write(
                write_fd,
                message.as_ptr().cast(),
                mem::size_of::<Message>(),
            ) as usize
                != mem::size_of::<Message>()
            {
                panic!("Failed write to pipe");
            }
        }
        // Free ID to be able to be used again.
        context.delete_id(DeviceID(self.device_id.0));
        // Wait for the deregister to complete.
        let (lock, cvar) = &*pair;
        let mut started = lock.lock().unwrap();
        while !*started {
            started = cvar.wait(started).unwrap();
        }
    }

    /// Check if should yield to executor.
    pub(super) fn should_yield(&self) -> bool {
        let context = context().lock().unwrap();
        !context.ready.contains(&self.device_id)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        self.old();
    }
}
