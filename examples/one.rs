use waker_thread::Listener;

fn main() {
    let listener = Listener::new(1 /* STDIN */);

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        listener.exit();
    });

    std::thread::sleep(std::time::Duration::from_millis(2000));
}
