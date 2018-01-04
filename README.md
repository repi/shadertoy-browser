# Rusty Shadertoy Browser

[![Crate](https://img.shields.io/crates/v/shadertoy-browser.svg)](https://crates.io/crates/shadertoy-browser)
[![Build Status](https://travis-ci.com/repi/shadertoy-browser.svg?token=8SzG1tHkq3FpBRftmohU&branch=master)](https://travis-ci.com/repi/shadertoy-browser)
![MIT/Apache](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)

Small [Shadertoy](http://shadertoy.com) browser & viewer for Mac built in [Rust](https://www.rust-lang.org).

This application uses the [Shadertoy REST API](http://shadertoy.com/api) to search for Shadertoys and then downloads them locally and converts them using [`shaderc-rs`](https://crates.io/crates/shaderc) and [`spirv-cross`](https://crates.io/crates/spirv_cross) to be natively rendered on Mac using [`metal-rs`](https://crates.io/crates/metal-rs).

The API queries are done through the [`shadertoy`](https://crates.io/crates/shadertoy) crate, which is also part of this repository in [`src/shadertoy`](src/shadertoy)

![Render](https://raw.githubusercontent.com/repi/shadertoy-browser/master/ScreenshotRender.jpg?token=ABNEZJ60PVG0ncu_xnMImD4OMbP0Wc1vks5aVspLwA%3D%3D)
![Output](https://raw.githubusercontent.com/repi/shadertoy-browser/master/ScreenshotOutput.jpg?token=ABNEZOX6bEUtIh8T5W82SeuXlcWBcNUNks5aVsrcwA%3D%3D)

## Building

First make sure you have [Rust installed](https://www.rust-lang.org/en-US/install.html) installed.
Then building & running is easy:

```sh
$ git clone https://github.com/repi/shadertoy-browser
$ cd shadertoy-browser
$ cargo build --release

# this will download and view all shadertoys with "car" in the name
$ cargo run --release -- -s car 
```

## Usage

Keys:

- `LEFT` and `RIGHT` - switch between shadertoys.
- `SPACE` - toggle grid view mode
- `ENTER` - open shadertoy.com for current shader

If the screen is red that indicates the shader wasn't able to be built.

Optional command-line settings:

```text
USAGE:
    shadertoy-browser [FLAGS] [OPTIONS]

FLAGS:
    -b, --buildall    Build all shaders upfront. This is useful to stress test compilation, esp. together with
                      --headless
        --help        Prints help information
    -h, --headless    Don't render, only download shadertoys
    -V, --version     Prints version information

OPTIONS:
    -k, --apikey <key>          Set shadertoy API key to use. Create your key on https://www.shadertoy.com/myapps
                                [default: BtHtWD]
    -f, --filter <filter>...    Inclusion filters [values: VR, SoundOutput, SoundInput, Webcam, MultiPass, MusicStream]
    -o, --order <order>         Sort order [default: Popular]  [values: Name, Love, Popular, Newest, Hot]
    -s, --search <string>       Search string to filter which shadertoys to get
    -t, --threads <threads>     How many threads to use for downloading & processing shaders. 0 = disables threading, -1
                                = use all logical processors [default: -1]
```

To use the Rust shadertoy API directly in another app or library, check out the [`shadertoy`](https://crates.io/crates/shadertoy) crate, [docs](http://docs.rs/shadertoy) and [README](src/shadertoy/README.MD).

## License

Licensed under either of the following, at your option:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

### Contribution

Contributions are welcome! Please note that your contributions are assumed to be dual-licensed under Apache-2.0/MIT.
