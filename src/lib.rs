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
//! Each module is enabled with a feature by the same name.  The module is not
//! included if the target platform doesn't support it.
//!
//! These are abstractions over OS-defined APIs for waking futures.  Each module
//! contains:
//!
//!  - `Device`: A handle to the device
//!  - `DeviceBuilder`: API-specific builder for `Device`

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

#[cfg(all(target_os = "linux", feature = "epoll"))]
pub mod epoll;
