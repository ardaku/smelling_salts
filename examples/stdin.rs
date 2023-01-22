use std::{
    io::{self, BufRead, BufReader},
    os::fd::AsFd,
};

use async_main::async_main;
use pasts::prelude::*;
use smelling_salts::{Device, Watch};

/// A `Stdin` device future.
pub struct Stdin(BufReader<Device>);

impl Default for Stdin {
    fn default() -> Self {
        Self::new()
    }
}

impl Stdin {
    /// Create a new `Stdin` device handle.
    pub fn new() -> Self {
        let owned_fd = io::stdin().as_fd().try_clone_to_owned().unwrap();
        let device = Device::new(owned_fd, Watch::INPUT);

        Self(BufReader::new(device))
    }
}

impl Notifier for Stdin {
    type Event = String;

    fn poll_next(
        mut self: Pin<&mut Self>,
        task: &mut Task<'_>,
    ) -> Poll<String> {
        while let Ready(()) = Pin::new(self.0.get_mut()).poll_next(task) {
            let mut string = String::new();
            let Err(e) = self.0.read_line(&mut string) else {
                string.pop();
                return Ready(string);
            };

            dbg!(e);
        }

        Pending
    }
}

#[async_main]
async fn main(_spawner: impl Spawn) {
    let mut stdin = Stdin::new();

    loop {
        let you_said = stdin.next().await;

        println!("You said: \"{you_said}\"");
    }
}
