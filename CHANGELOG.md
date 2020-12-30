# Changelog
All notable changes to `smelling_salts` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://jeronlau.tk/semver/).

## [0.2.0] - 2020-12-29
### Added
 - `Device::should_yield()` for checking if the waker was trying to wake a
   different task.

## [0.1.0] - 2020-05-03
### Added
 - `Device` for registering wakers for file descriptors.
 - `Watcher` for constructing event (input or output) list to watch for.
