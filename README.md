# Smelling Salts

#### Start a thread to wake an async executor when the OS's I/O event notifier gathers that the hardware is ready.

[![Build Status](https://api.travis-ci.org/AldaronLau/smelling_salts.svg?branch=master)](https://travis-ci.org/AldaronLau/smelling_salts)
[![Docs](https://docs.rs/smelling_salts/badge.svg)][0]
[![crates.io](https://img.shields.io/crates/v/smelling_salts.svg)](https://crates.io/crates/smelling_salts)

If you're writing a Rust library to handle hardware asynchronously, you should
use this crate.  This library automatically wakes futures by registering a waker
with a device that you construct with a file descriptor.

### Currently Supported Platforms
- Linux (epoll)

### Planned Platforms
- Windows
- MacOS
- BSD
- Various Bare Metal?
- Others?

## Table of Contents
- [Getting Started](#getting-started)
   - [Example](#example)
   - [API](#api)
   - [Features](#features)
- [Upgrade](#upgrade)
- [License](#license)
   - [Contribution](#contribution)

### API
API documentation can be found on [docs.rs][0].

### Features
There are no optional features.

## Upgrade
You can use the [changelog][3] to facilitate upgrading this crate as a
dependency.

## License
Licensed under either of
 - Apache License, Version 2.0 ([LICENSE_APACHE_2_0.txt][7]
   or [https://www.apache.org/licenses/LICENSE-2.0][8])
 - Boost License, Version 1.0 ([LICENSE_BOOST_1_0.txt][9]
   or [https://www.boost.org/LICENSE_1_0.txt][10])

at your option.

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

Anyone is more than welcome to contribute!  Don't be shy about getting involved,
whether with a question, idea, bug report, bug fix, feature request, feature
implementation, or other enhancement.  Other projects have strict contributing
guidelines, but this project accepts any and all formats for pull requests and
issues.  For ongoing code contributions, if you wish to ensure your code is
used, open a draft PR so that I know not to write the same code.  If a feature
needs to be bumped in importance, I may merge an unfinished draft PR into it's
own branch and finish it (after a week's deadline for the person who openned
it).  Contributors will always be notified in this situation, and given a choice
to merge early.

All pull request contributors will have their username added in the contributors
section of the release notes of the next version after the merge, with a message
thanking them.  I always make time to fix bugs, so usually a patched version of
the library will be out a few days after a report.  Features requests will not
complete as fast.  If you have any questions, design critques, or want me to
find you something to work on based on your skill level, you can email me at
[jeronlau@plopgrizzly.com](mailto:jeronlau@plopgrizzly.com).  Otherwise,
[here's a link to the issues on GitHub][12], and, as always, make sure to read
and follow the [Code of Conduct][11].

[0]: https://docs.rs/smelling_salts
[1]: https://crates.io/crates/smelling_salts
[2]: https://github.com/AldaronLau/smelling_salts/actions?query=workflow%3Atests
[3]: https://github.com/AldaronLau/smelling_salts/blob/main/CHANGELOG.md
[4]: https://github.com/AldaronLau/smelling_salts/blob/main/README.md
[5]: https://github.com/AldaronLau/smelling_salts
[6]: https://aldaronlau.com/
[7]: https://github.com/AldaronLau/smelling_salts/blob/main/LICENSE-APACHE
[8]: https://www.apache.org/licenses/LICENSE-2.0
[9]: https://github.com/AldaronLau/smelling_salts/blob/main/LICENSE_BOOST_1_0.txt
[10]: https://www.boost.org/LICENSE_1_0.txt
[11]: https://github.com/AldaronLau/smelling_salts/blob/main/CODE_OF_CONDUCT.md
[12]: https://github.com/AldaronLau/smelling_salts/issues
