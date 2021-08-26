# Changelog
All notable changes to `smelling_salts` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://jeronlau.tk/semver/).

## [0.5.1] - 2021-08-26
### Fixed
 - Example in the documentation not compiling

## [0.5.0] - 2021-08-26
### Added
 - `Driver` struct.
 - Implementation of `Future` for `Device`

### Changed
 - Moved everything into the `linux` module.
 - Smelling salts now depends on the `flume` crate as it's only dependency

### Removed
 - All methods on `Device`

## [0.4.0] - 2021-07-05
### Added
 - Implementation for `Send` + `Sync` on `Device`

### Changed
 - Now always only starts one thread.

## [0.3.0] - 2021-06-24
### Changed
 - Rename `Device::register_waker()` to `Device::sleep()`, which now
   additionally returns `Poll::Pending` and now requires a mutable reference.
 - Rename `Device::old()` to `Device::stop()`, which now additionally returns
   the `RawDevice` that was stopped, or -1 if already stopped.
 - Rename `Device::should_yield()` to `Device::pending()`

### Fixed
 - Undefined behavior on some architectures (specifically raspberry pi)

## [0.2.4] - 2021-02-14
### Fixed
 - Libraries built on smelling\_salts using 100% of CPU unnecessarily

## [0.2.3] - 2021-02-06
### Fixed
 - Not compiling for Android

## [0.2.2] - 2021-01-16
### Fixed
 - Stalling issue (futures no longer waked after non-deterministic event).

## [0.2.1] - 2020-12-30
### Fixed
 - Not compiling dummy implementation.

## [0.2.0] - 2020-12-29
### Added
 - `Device::should_yield()` for checking if the waker was trying to wake a
   different task.

## [0.1.0] - 2020-05-03
### Added
 - `Device` for registering wakers for file descriptors.
 - `Watcher` for constructing event (input or output) list to watch for.
