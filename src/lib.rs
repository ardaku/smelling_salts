// Smelling Salts
// Copyright © 2020-2021 Jeron Aldaron Lau.
//
// Licensed under any of:
// - Apache License, Version 2.0 (https://www.apache.org/licenses/LICENSE-2.0)
// - MIT License (https://mit-license.org/)
// - Boost Software License, Version 1.0 (https://www.boost.org/LICENSE_1_0.txt)
// At your choosing (See accompanying files LICENSE_APACHE_2_0.txt,
// LICENSE_MIT.txt and LICENSE_BOOST_1_0.txt).
//!
//!
//! ## Getting Started
//! Add the following to your `Cargo.toml`.
//!
//! ```toml
//! #################
//! # For Libraries #
//! #################
//!
//! [dependencies.smelling_salts]
//! version = "0.5"
//!
//! [dependencies.flume]
//! version = "0.10"
//! default-features = false
//! features = ["async"]
//!
//! ####################
//! # For Applications #
//! ####################
//!
//! [dependencies.pasts]
//! version = "0.8"
//! ```

#![doc(
    html_logo_url = "https://libcala.github.io/logo.svg",
    html_favicon_url = "https://libcala.github.io/icon.svg",
    html_root_url = "https://docs.rs/smelling_salts"
)]
#![deny(unsafe_code)]
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

// #[cfg(target_os = "linux")]
// pub mod linux;
#[cfg(target_os = "linux")]
mod watcher;

#[cfg(target_os = "linux")]
pub mod linux;
