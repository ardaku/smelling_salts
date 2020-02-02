//! An OS thread for waking other threads based on file descriptor events.

use std::os::raw;
use std::mem;
use std::task;
use std::ptr;
use std::thread;
use std::sync::atomic;

type Ptr = (Option<task::Waker>, atomic::AtomicBool);

const EPOLLIN: u32 = 0x001;
const EPOLLOUT: u32 = 0x004;

const EPOLL_CTL_ADD: i32 = 1;
const EPOLL_CTL_DEL: i32 = 2;
const EPOLL_CTL_MOD: i32 = 3;

const O_CLOEXEC: raw::c_int = 0x0008_0000;

#[repr(C)]
#[derive(Copy, Clone)]
union EpollData {
   ptr: *mut raw::c_void,
   fd: raw::c_int,
   uint32: u32,
   uint64: u64,
}

#[repr(packed, C)]
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

/// 
static FREED: atomic::AtomicBool = atomic::AtomicBool::new(false);

// Convert a C error (negative on error) into a result.
fn error(err: raw::c_int) -> Result<(), raw::c_int> {
    if err < 0 {
        Err(err)
    } else {
        Ok(())
    }
}

// Free the Epoll instance
fn free(epoll_fd: raw::c_int) {
    println!("==FREE==");

    // close() should never fail.
    let ret = unsafe {
        close(epoll_fd)
    };
    error(ret).unwrap();
}

/// Start the Epoll Thread, if not already started, and return it.
fn start() {
    println!("==START==");

    unsafe {
        // Create a new epoll instance.
        let epoll_fd = epoll_create1(O_CLOEXEC /* thread-safe */);
        error(epoll_fd).unwrap();
        EPOLL_FD = mem::MaybeUninit::new(epoll_fd);
        // Open a pipe.
        let mut pipe = mem::MaybeUninit::<[raw::c_int; 2]>::uninit();
        error(pipe2(pipe.as_mut_ptr(), 0 /* no flags */)).unwrap();
        let [read_fd, write_fd] = pipe.assume_init();
        WRITE_FD = mem::MaybeUninit::new(write_fd);
        // Add read pipe to epoll instance.
        let mut pipe_listener = Listener::init(epoll_fd, read_fd, false);
        
        // Spawn and detach the thread.
        thread::spawn(move || {
            // Run until pipe creates an interrupt.
            loop {
                let mut ev = mem::MaybeUninit::<EpollEvent>::uninit();
                // Wait for something to happen.
                if epoll_wait(epoll_fd, ev.as_mut_ptr(), 1 /* Get one event at a time */, -1 /* wait indefinitely */) < 0 {
                    // Ignore error
                    continue;
                }
                let ptr: *mut raw::c_void = ev.assume_init().data.ptr;
                if ptr.is_null() {
                    let mut buffer = mem::MaybeUninit::<[u8; 1]>::uninit();
                    let len = read(read_fd, buffer.as_mut_ptr().cast(), 1);
                    let ret = buffer.assume_init()[0];
                    assert_eq!(len, 1);
                    match ret {
                        0 => {
                            println!("EXITING!!");
                            break
                        },
                        1 => {
                            println!("Freeing on event");
                            let mut buffer = mem::MaybeUninit::<[u8; mem::size_of::<usize>()]>::uninit();
                            let len = read(read_fd, buffer.as_mut_ptr().cast(), mem::size_of::<usize>());
                            let ret = buffer.assume_init();
                            assert_eq!(len as usize, mem::size_of::<usize>());
                            let _ = Box::<Ptr>::from_raw(mem::transmute(ret));
                        },
                        _ => unreachable!(),
                    }
                }
                let ptr: *mut Ptr = ptr.cast();
                // Wake waiting thread if it's waiting.
                if let Some(waker) = (*ptr).0.take() {
                    waker.wake();
                }
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
    println!("GET FDS");

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
pub struct Listener {
    fd: raw::c_int, // File descriptor for this
    a: *mut Ptr,
    b: *mut Ptr,
}

unsafe impl Send for Listener {}

impl Listener {
    /// Create a new Listener (start listening on file descriptor).
    pub fn new(fd: raw::c_int) -> Listener {
        println!("NEW Listener");

        // Get the epoll file descriptor.
        let (epoll_fd, _) = get_fds();
        // Create the listener
        unsafe {
            Self::init(epoll_fd, fd, true)
        }
    }

    unsafe fn init(epoll_fd: raw::c_int, fd: raw::c_int, is: bool) -> Listener {
        println!("INIT Listener");

        // Check for input and output
        let events = EPOLLIN | EPOLLOUT;
        // Build two EpollEvent structures to switch between.
        let box_a: *mut Ptr = if is { Box::into_raw(Box::new((None, atomic::AtomicBool::new(false)))) } else { ptr::null_mut() };
        let box_b: *mut Ptr = if is { Box::into_raw(Box::new((None, atomic::AtomicBool::new(false)))) } else { ptr::null_mut() };
        // This C FFI call is safe because, according to the epoll
        // documentation, adding is safe while another thread is waiting on
        // epoll, the mutable reference to EpollEvent isn't used after the call,
        // and the box lifetime is handled properly by this struct.
        // Shouldn't fail
        error(epoll_ctl(epoll_fd, EPOLL_CTL_ADD, fd, &mut EpollEvent {
            events, data: EpollData { ptr: box_a.cast() }
        })).unwrap();
        // Construct the listener.
        Listener { fd, a: box_a, b: box_b }
    }

    /// Attach a waker to this Listener.  Do this before checking for new data.
    pub fn wake_on_event(&mut self, waker: task::Waker) {
        println!("WAKE Listener");

        // Get the epoll file descriptor.
        let (epoll_fd, _) = get_fds();
        // Move waker into new box.
        unsafe { (*self.b).0 = Some(waker); }
        // This C FFI call is safe because, according to the epoll
        // documentation, modifying is safe while another thread is waiting on
        // epoll, and the mutable reference to EpollEvent isn't used after the
        // call.
        unsafe {
            // Transmute copy box, don't run constructor twice.
            let data: *mut EpollEvent = mem::transmute_copy(&self.b);
            // Shouldn't fail
            error(epoll_ctl(epoll_fd, EPOLL_CTL_MOD, self.fd, data)).unwrap();
        };
        // Swap the two boxes so different memory is used next time.
        mem::swap(&mut self.b, &mut self.a);
    }

    /// Exit the thread.
    pub fn exit(&self) {
        FREED.store(true, atomic::Ordering::SeqCst);

        println!("EXIT Listener");

        // Get the epoll file descriptor.
        let (_, write_fd) = get_fds();
        // Tell the loop to stop waiting, so that it actually exits.
        if unsafe { write(write_fd, [0u8].as_ptr().cast(), 1) } != 1 {
            panic!("Writing to the pipe failed, should never happen!");
        }
    }

    unsafe fn free(&mut self) {
        println!("FREE Listener");

        println!("Free");
        // Get the epoll file descriptor.
        let (epoll_fd, _) = get_fds();
        println!("Got FDS");
        // This C FFI call is safe because, according to the epoll
        // documentation, deleting is safe while another thread is waiting on
        // epoll, and the mutable reference to EpollEvent isn't used after the
        // call.  It's also necessary to guarentee the Box<Ptr> isn't used after
        // free.
        // Shouldn't fail - EpollEvent is unused, but can't be null
        error(epoll_ctl(epoll_fd, EPOLL_CTL_DEL, self.fd, &mut EpollEvent {
            events: EPOLLIN | EPOLLOUT,
            data: EpollData { ptr: ptr::null_mut() },
        })).unwrap();
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        println!("DROP Listener");
        // Free box_b
        let _ = unsafe { Box::<Ptr>::from_raw(self.b) };
        // Unregister listener
        unsafe {
            self.free();
        }
        println!("Free boxes");
        // Send message to free box_a, since it might stil be used

        // Get the epoll file descriptor.
        let (_, write_fd) = get_fds();
        //
        let a: [u8; mem::size_of::<usize>()] = unsafe { mem::transmute(self.a) };
        let mut fail = false;
        // Tell the loop to stop waiting, so that it actually exits.
        if unsafe { write(write_fd, [1u8].as_ptr().cast(), 1) } != 1 {
            fail = true;
        }
        if unsafe { write(write_fd, a.as_ptr().cast(), mem::size_of::<usize>()) } != mem::size_of::<usize>() as isize {
            fail = true;
        }
        if fail || FREED.load(atomic::Ordering::SeqCst) {
            println!("Freeing after pipe is alrady closd");
            let _ = unsafe { Box::<Ptr>::from_raw(self.a) };
        }
        println!("DROPPED Listener");
    }
}
