//! An OS thread for waking other threads based on file descriptor events.

use std::os::raw;
use std::mem;
use std::task;
use std::ptr;
use std::thread;

type Ptr = Option<task::Waker>;

const EPOLLIN: u32 = 0x001;
const EPOLLOUT: u32 = 0x004;

const EPOLL_CTL_ADD: i32 = 1;
const EPOLL_CTL_DEL: i32 = 2;
const EPOLL_CTL_MOD: i32 = 3;

#[repr(C)]
#[derive(Copy, Clone)]
union EpollData {
   ptr: *mut raw::c_void,
   fd: raw::c_int,
   uint32: u32,
   uint64: u64,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct EpollEvent {
    events: u32,        /* Epoll events */
    data: EpollData,    /* User data variable */
}

#[allow(non_camel_case_types)]
type c_ssize = isize; // True for most unix
#[allow(non_camel_case_types)]
type c_size = usize; // True for most unix

extern "C" {
    fn epoll_create1(flags: raw::c_int) -> raw::c_int;
    fn close(fd: raw::c_int) -> raw::c_int;
    fn epoll_ctl(epfd: raw::c_int, op: raw::c_int, fd: raw::c_int,
        event: *mut EpollEvent) -> raw::c_int;
    fn epoll_wait(epfd: raw::c_int, events: *mut EpollEvent,
        maxevents: raw::c_int, timeout: raw::c_int) -> raw::c_int;
    fn pipe2(pipefd: *mut [raw::c_int; 2], flags: raw::c_int) -> raw::c_int;
    fn write(fd: raw::c_int, buf: *const raw::c_void, count: c_size) -> c_ssize;
    fn read(fd: raw::c_int, buf: *mut raw::c_void, count: c_size) -> c_ssize;
}

/// File descriptor for the epoll instance.
static mut EPOLL_FD: mem::MaybeUninit<raw::c_int> = mem::MaybeUninit::uninit();

/// File descriptor for the write side of the pipe.
static mut WRITE_FD: mem::MaybeUninit<raw::c_int> = mem::MaybeUninit::uninit();

/// Whether or not static mutable globals have been initialized.
///
/// Once they have been initialized, they don't change, so reading them is safe.
static INIT: std::sync::Once = std::sync::Once::new();

// Convert a C error (negative on error) into a result.
fn error(err: raw::c_int) -> Result<(), ()> {
    if err < 0 {
        Err(())
    } else {
        Ok(())
    }
}

// Free the Epoll instance
fn free(epoll_fd: raw::c_int) {
    // close() should never fail.
    let ret = unsafe {
        close(epoll_fd)
    };
    error(ret).unwrap();
}

/// Start the Epoll Thread, if not already started, and return it.
fn start() {
    unsafe {
        // Create a new epoll instance.
        let epoll_fd = epoll_create1(0 /* no flags */);
        error(epoll_fd).unwrap();
        EPOLL_FD = mem::MaybeUninit::new(epoll_fd);
        // Open a pipe.
        let mut pipe = mem::MaybeUninit::<[raw::c_int; 2]>::uninit();
        error(pipe2(pipe.as_mut_ptr(), 0 /* no flags */)).unwrap();
        let [read_fd, write_fd] = pipe.assume_init();
        WRITE_FD = mem::MaybeUninit::new(write_fd);
        
        // Spawn and detach the thread.
        thread::spawn(move || {
            // Add read pipe to epoll instance.
            let mut pipe_listener = Listener::init(epoll_fd, read_fd, false);
            // Run until pipe creates an interrupt.
            loop {
                let mut ev = mem::MaybeUninit::<EpollEvent>::uninit();
                // Wait for something to happen.
                if epoll_wait(epoll_fd, ev.as_mut_ptr(), 1 /* Get one event at a time */, -1 /* wait indefinitely */) < 0 {
                    // Ignore error
                    continue;
                }
                let ptr = ev.assume_init().data.ptr.cast::<Ptr>();
                dbg!(ptr);
                if ptr.is_null() {
                    let mut buffer = mem::MaybeUninit::<[u8; 1]>::uninit();
                    let len = read(read_fd, buffer.as_mut_ptr().cast(), 1);
                    let ret = buffer.assume_init()[0];
                    assert_eq!(ret, 0);
                    assert_eq!(len, 1);
                    break;
                }
                let mut data = Box::from_raw(ptr);
                // Wake waiting thread if it's waiting.
                if let Some(waker) = (*data).take() {
                    waker.wake();
                }
                let _ = Box::into_raw(data);
            }
            // Free up resources
            pipe_listener.free();
            free(epoll_fd);
            // Don't try to free a null box.
            mem::forget(pipe_listener);
        });
    }
}

/// Safely get `(epoll_fd, write_fd)`
#[inline(always)]
fn get_fds() -> (raw::c_int, raw::c_int) {
    // Make sure that epoll thread has already started.  This way, we know that
    // the file descriptors have been initialized.
    INIT.call_once(start);
    // This access is safe without a mutex because the file descriptors don't
    // change after initialization.
    unsafe {
        (EPOLL_FD.assume_init(), WRITE_FD.assume_init())
    }
}

/// A listener on a file descriptor.
pub struct Listener(raw::c_int, Box<EpollEvent>, Box<EpollEvent>);

impl Listener {
    /// Create a new Listener (start listening on file descriptor).
    pub fn new(fd: raw::c_int) -> Listener {
        // Get the epoll file descriptor.
        let (epoll_fd, _) = get_fds();
        // Create the listener
        unsafe {
            Self::init(epoll_fd, fd, true)
        }
    }

    unsafe fn init(epoll_fd: raw::c_int, fd: raw::c_int, is: bool) -> Listener {
        // Build two EpollEvent structures to switch between.
        let events = EPOLLIN | EPOLLOUT;
        let ptr = if is {
            let data: *mut Ptr = Box::into_raw(Box::new(None));
            data.cast()
        } else {
            ptr::null_mut()
        };
        let ptr_clone = if is {
            let data: *mut Ptr = Box::into_raw(Box::new(None));
            data.cast()
        } else {
            ptr::null_mut()
        };
        let event = Box::new(EpollEvent { events, data: EpollData { ptr } });
        let mut event_clone = event.clone();
        event_clone.data = EpollData { ptr: ptr_clone };
        let event = Box::into_raw(event);

        // This C FFI call is safe because, according to the epoll
        // documentation, adding is safe while another thread is waiting on
        // epoll, the mutable reference to EpollEvent isn't used after the call,
        // and the box lifetime is handled properly by this struct.
        // Shouldn't fail
        error(epoll_ctl(epoll_fd, EPOLL_CTL_ADD, fd, event)).unwrap();

        dbg!(event);
        // Re-construct box, so that it can be free'd at drop.
        let event = Box::from_raw(event);
        // Construct the listener.
        Listener(fd, event, event_clone)
    }

    /// Attach a waker to this Listener.  Do this before checking for new data.
    pub fn wake_on_event(&mut self, waker: task::Waker) {
        // Get the epoll file descriptor.
        let (epoll_fd, _) = get_fds();
        // Move waker into new box.
        unsafe {
            (*(*self.2).data.ptr.cast::<Ptr>()) = Some(waker);
        }
        // This C FFI call is safe because, according to the epoll
        // documentation, modifying is safe while another thread is waiting on
        // epoll, and the mutable reference to EpollEvent isn't used after the
        // call.
        unsafe {
            // Copy box, into_raw won't run constructor twice.
            let data: *mut EpollEvent = Box::into_raw(mem::transmute_copy(&self.2));
            dbg!(data);
            // Shouldn't fail
            error(epoll_ctl(epoll_fd, EPOLL_CTL_MOD, self.0, data)).unwrap();
        };
        // Swap the two boxes to avoid memory bugs.
        mem::swap(&mut self.2, &mut self.1);
    }

    /// Exit the thread.
    pub fn exit(&self) {
        // Get the epoll file descriptor.
        let (_, write_fd) = get_fds();
        // Tell the loop to stop waiting, so that it actually exits.
        if unsafe { write(write_fd, [0u8].as_ptr().cast(), 1) } != 1 {
            panic!("Writing to the pipe failed, should never happen!");
        }
    }

    unsafe fn free(&mut self) {
        // FIXME
        /*// Get the epoll file descriptor.
        let (epoll_fd, _) = get_fds();
        // This C FFI call is safe because, according to the epoll
        // documentation, deleting is safe while another thread is waiting on
        // epoll, and the mutable reference to EpollEvent isn't used after the
        // call.  It's also necessary to guarentee the Box<Ptr> isn't used after
        // free.
        // Shouldn't fail - EpollEvent is unused, but can't be null
        error(epoll_ctl(epoll_fd, EPOLL_CTL_DEL, self.0, &mut EpollEvent {
            events: EPOLLIN | EPOLLOUT,
            data: EpollData { ptr: ptr::null_mut() },
        })).unwrap();*/
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        unsafe {
            self.free();
        }
    }
}
