# Smelling Salts

#### [Changelog][3] | [Source][4] | [Getting Started][5]

[![tests](https://github.com/AldaronLau/smelling_salts/workflows/tests/badge.svg)][2]
[![docs](https://docs.rs/smelling_salts/badge.svg)][0]
[![crates.io](https://img.shields.io/crates/v/smelling_salts.svg)][1]

Abstraction over OS APIs to handle asynchronous device waking.

## About
If you're writing a Rust library to handle hardware asynchronously, you should
use this crate.  This library automatically wakes futures by registering a waker
with a device that you construct with a file descriptor.

### Currently Supported APIs
 - Epoll (Linux)

### Planned APIs
 - Run loops (MacOS)
 - Kqueue (BSD/MacOS)
 - IOCP (Windows)
 - Various Bare Metal?
 - Others?

## License
Licensed under any of
 - Apache License, Version 2.0, ([LICENSE_APACHE_2_0.txt][7]
   or [https://www.apache.org/licenses/LICENSE-2.0][8])
 - Boost Software License, Version 1.0, ([LICENSE_BOOST_1_0.txt][11]
   or [https://www.boost.org/LICENSE_1_0.txt][12])
 - MIT License, ([LICENSE_MIT.txt][9] or [https://mit-license.org/][10])

at your option.

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
licensed as described above, without any additional terms or conditions.

## Help
If you want help using or contributing to this library, feel free to send me an
email at [aldaronlau@gmail.com][13].

[0]: https://docs.rs/smelling_salts
[1]: https://crates.io/crates/smelling_salts
[2]: https://github.com/AldaronLau/smelling_salts/actions?query=workflow%3Atests
[3]: https://github.com/AldaronLau/smelling_salts/blob/main/CHANGELOG.md
[4]: https://github.com/AldaronLau/smelling_salts/
[5]: https://docs.rs/smelling_salts#getting-started
[6]: https://aldaronlau.com/
[7]: https://github.com/AldaronLau/smelling_salts/blob/main/LICENSE_APACHE_2_0.txt
[8]: https://www.apache.org/licenses/LICENSE-2.0
[9]: https://github.com/AldaronLau/smelling_salts/blob/main/LICENSE_MIT.txt
[10]: https://mit-license.org/
[11]: https://github.com/AldaronLau/smelling_salts/blob/main/LICENSE_BOOST_1_0.txt
[12]: https://www.boost.org/LICENSE_1_0.txt
[13]: mailto:aldaronlau@gmail.com
