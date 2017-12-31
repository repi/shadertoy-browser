# Rusty Shadertoy Browser

[![Build Status](https://travis-ci.com/repi/shadertoy-browser.svg?token=8SzG1tHkq3FpBRftmohU&branch=master)](https://travis-ci.com/repi/shadertoy-browser) ![MIT/Apache](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)

Small [Shadertoy](http://shadertoy.com) browser & viewer for Mac built in [Rust](https://www.rust-lang.org).

This application uses the [Shadertoy REST API](http://shadertoy.com/api) to search for Shadertoys and then downloads them locally and converts them using [`shaderc-rs`](https://crates.io/crates/shaderc) and [`spirv-cross`](https://crates.io/crates/spirv_cross) to be natively rendered on Mac using [`metal-rs`](https://crates.io/crates/metal-rs).

The API queries are done through the [`shadertoy`](https://crates.io/crates/shadertoy) crate, which can be found in  `src/shadertoy`


## Usage

TODO

## Building

First make sure you have [Rust installed](https://www.rust-lang.org/en-US/install.html) installed.
Then building is easy:

```
$ git clone https://github.com/repi/shadertoy-browser
$ cd shadertoy-browser
$ cargo build --release
$ ./target/release/shadertoy-browser
```

## License

Licensed under either of the following, at your option:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

### Contribution

Contributions are welcome! Please note that your contributions are assumed to be dual-licensed under Apache-2.0/MIT.
