// Copyright © 2020-2023 The Smelling Salts Contributors.
//
// Licensed under any of:
//  - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
//  - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
//  - MIT License (https://mit-license.org/)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).
//
//! Abstraction over OS APIs to handle asynchronous device waking.
//!
//! ## Getting Started
//! Most devices are represented as file descriptors on unix-like operating
//! systems (MacOS also has run loops).  On Windows, devices are usually sockets
//! or handles.  WebAssembly running in the browser doesn't have a equivalent.
//! To get around these device backend differences, Smelling Salts exposed an
//! [`OsDevice`] type which has [`From`] conversions implemented for the
//! platform types.
//!
//! An [`OsDevice`] by itself isn't that useful, though.  When you have a handle
//! to a device, you want to asynchronously watch it for events.  For this, you
//! construct a [`Device`], which implements [`Notifier`](pasts::Notifier).
//! But, general devices aren't that useful either, so you'll need to wrap it
//! in your own custom type.  Usually, you will filter out some of the events,
//! so you'll need to implement [`Notifier`](pasts::Notifier).
//!
//! Here's a simple example implementing a [`Notifier`](pasts::Notifier) for
//! stdin line reading:
//!
//! ```rust,no_run
#![cfg_attr(target_os = "linux", doc = include_str!("../examples/stdin.rs"))]
//! ```

#![doc(
    html_logo_url = "https://libcala.github.io/logo.svg",
    html_favicon_url = "https://libcala.github.io/icon.svg",
    html_root_url = "https://docs.rs/smelling_salts"
)]
#![warn(
    anonymous_parameters,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    nonstandard_style,
    rust_2018_idioms,
    single_use_lifetimes,
    trivial_casts,
    trivial_numeric_casts,
    unreachable_pub,
    unused_extern_crates,
    unused_qualifications,
    variant_size_differences
)]

mod device;
mod kind;
mod watch;

#[cfg_attr(target_os = "linux", path = "epoll.rs")]
#[cfg_attr(not(target_os = "linux"), path = "mock.rs")]
mod platform;

pub use self::{device::Device, kind::OsDevice, watch::Watch};

trait Interface {
    const WATCH_INPUT: u32;
    const WATCH_OUTPUT: u32;

    /// Watch a [`DeviceKind`]
    fn watch(kind: &kind::DeviceKind, watch: Watch) -> whisk::Channel;

    /// Unwatch a [`DeviceKind`]
    fn unwatch(kind: &kind::DeviceKind);
}

struct Platform;

impl Platform {
    fn watch(kind: &kind::DeviceKind, watch: Watch) -> whisk::Channel {
        <Platform as Interface>::watch(kind, watch)
    }

    fn unwatch(kind: &kind::DeviceKind) {
        <Platform as Interface>::unwatch(kind);
    }
}
