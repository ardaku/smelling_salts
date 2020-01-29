use waker_thread::Listener;

fn main() {
    let _listener = Listener::new(1 /* STDIN */);

    std::thread::sleep(std::time::Duration::from_millis(1000));

    
}
