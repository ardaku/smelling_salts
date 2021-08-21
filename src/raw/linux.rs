// Smelling Salts
// Copyright © 2020-2021 Jeron Aldaron Lau.
//
// Licensed under any of:
// - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
// - MIT License (https://mit-license.org/)
// - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).

use std::mem::MaybeUninit;
use std::os::raw;
use std::os::unix::io::RawFd;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::task::{Context, Wake, Waker};

/// On Linux, `RawDevice` corresponds to [RawFd](std::os::unix::io::RawFd)
pub type RawDevice = RawFd;

#[cfg_attr(
    any(
        all(
            target_arch = "x86",
            not(target_env = "musl"),
            not(target_os = "android")
        ),
        target_arch = "x86_64"
    ),
    repr(packed)
)]
#[repr(C)]
struct EpollEvent {
    events: u32,
    data: u64,
}

extern "C" {
    fn epoll_create1(flags: raw::c_int) -> RawFd;
    fn epoll_wait(
        epfd: RawFd,
        events: *mut EpollEvent,
        maxevents: raw::c_int,
        timeout: raw::c_int,
    ) -> raw::c_int;
    fn epoll_ctl(
        epfd: RawFd,
        op: raw::c_int,
        fd: RawFd,
        event: *mut EpollEvent,
    ) -> raw::c_int;
}

#[inline(always)]
fn thread_loop(epfd: RawFd) {
    // Uninitialized events
    let mut event = MaybeUninit::<EpollEvent>::zeroed();

    // Wait for successful event(s).
    let n = unsafe { epoll_wait(epfd, event.as_mut_ptr(), 1, -1) };
    if n != 1 {
        return;
    }

    // Set pending to false, and wake the waker.
    let event = unsafe { event.assume_init() };
    let dw = event.data as usize as *mut DevWaker;

    unsafe {
        (*dw).pending.store(false, Ordering::SeqCst);
        (*dw).waker.wake_by_ref();
    }
}

#[derive(Debug)]
struct DevWaker {
    pending: AtomicBool,
    waker: Waker,
}

#[derive(Debug)]
struct Device {
    /// File descriptor for epoll.
    epollfd: RawFd,
    /// File descriptor for this device.
    fd: RawFd,
    /// Watcher info.
    events: u32,
    /// Waker
    waker: [Box<DevWaker>; 2],
}

impl Drop for Device {
    #[inline(always)]
    fn drop(&mut self) {
        super::Device::free(self);
    }
}

impl super::Device for Device {
    #[inline(always)]
    fn pending(&self) -> bool {
        self.waker[0].pending.load(Ordering::SeqCst)
    }

    #[inline(always)]
    fn free(&mut self) -> RawFd {
        let fd = self.fd;
        if fd != -1 {
            let mut event = MaybeUninit::<EpollEvent>::uninit();
            unsafe { epoll_ctl(self.epollfd, 2, fd, event.as_mut_ptr()) };
            self.fd = -1;
        }
        fd
    }

    #[inline(always)]
    fn raw(&self) -> RawFd {
        self.fd
    }

    #[inline(always)]
    fn sleep(&mut self, cx: &Context<'_>) {
        // Check if they are not the same
        if !self.waker[0].waker.will_wake(cx.waker()) {
            // Clone waker.
            self.waker[1].waker = cx.waker().clone();
            // Get new pointer.
            let ptr: *mut DevWaker = &mut *self.waker[1];
            // Modify device to use new waker.
            let mut event = EpollEvent {
                events: self.events,
                data: ptr as usize as _,
            };
            unsafe { epoll_ctl(self.epollfd, 3, self.fd, &mut event) };
            // Swap waker buffers.
            self.waker.swap(0, 1);
        }
        // Set pending to true.
        self.waker[0].pending.store(true, Ordering::SeqCst);
    }
}

struct Global {
    epollfd: RawFd,
}

impl super::Global for Global {
    #[inline(always)]
    fn device(&self, fd: RawFd, events: u32) -> Box<dyn super::Device> {
        let epollfd = self.epollfd;
        let mut waker = [Box::new(fake_waker()), Box::new(fake_waker())];
        let ptr: *mut DevWaker = &mut *waker[0];
        let mut event = EpollEvent {
            events,
            data: ptr as usize as _,
        };
        unsafe { epoll_ctl(epollfd, 1, fd, &mut event) };
        Box::new(Device {
            epollfd,
            fd,
            events,
            waker,
        })
    }
}

#[inline(always)]
fn fake_waker() -> DevWaker {
    struct FakeWaker;

    impl Wake for FakeWaker {
        #[inline(always)]
        fn wake(self: Arc<Self>) {}
        #[inline(always)]
        fn wake_by_ref(self: &Arc<Self>) {}
    }

    let waker = Arc::new(FakeWaker).into();
    let pending = AtomicBool::new(true);

    DevWaker { pending, waker }
}

#[inline(always)]
pub(super) fn global() -> Box<dyn super::Global> {
    // Create epoll state.
    let epollfd = unsafe { epoll_create1(0) };
    if epollfd == -1 {
        panic!("Failed to create epoll instance");
    }

    // Spawn a separate thread.
    std::thread::spawn(move || loop {
        thread_loop(epollfd)
    });

    // Return global state.
    Box::new(Global { epollfd })
}
