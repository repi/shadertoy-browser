# Rusty Shadertoy Browser

[![Crate](https://img.shields.io/crates/v/shadertoy-browser.svg)](https://crates.io/crates/shadertoy-browser)
[![Build Status](https://travis-ci.org/repi/shadertoy-browser.svg?branch=master)](https://travis-ci.org/repi/shadertoy-browser)
[![Build Status](https://ci.appveyor.com/api/projects/status/ayr854h0a0eqqrfh/branch/master?svg=true)](https://ci.appveyor.com/project/repi/shadertoy-browser)
![MIT/Apache](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)

Small [Shadertoy](http://shadertoy.com) browser & viewer for Mac built in [Rust](https://www.rust-lang.org).

This application uses the [Shadertoy REST API](http://shadertoy.com/api) to search for Shadertoys and then downloads them locally and converts them using [`shaderc-rs`](https://crates.io/crates/shaderc) and [`spirv-cross`](https://crates.io/crates/spirv_cross) to be natively rendered on Mac using [`metal-rs`](https://crates.io/crates/metal-rs).

Warning: Some of the Shadertoy shaders are extremely demanding of your GPU and cause system to be unresponsive.

The API queries are done through the [`shadertoy`](https://crates.io/crates/shadertoy) crate, which is also part of this repository in [`src/shadertoy`](src/shadertoy)

![Render](https://raw.githubusercontent.com/repi/shadertoy-browser/master/screenshots/render.jpg?token=ABNEZC2kS8a8LqdI5bfJVYkojB7RNz83ks5aWQlAwA%3D%3D)
![Output](https://raw.githubusercontent.com/repi/shadertoy-browser/master/screenshots/output.jpg?token=ABNEZDsvhrXMegCZ4Zy6IaWT4gNDsojOks5aWQkcwA%3D%3D)

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
        --help        Prints help information
    -h, --headless    Don't render, only download shadertoys
    -V, --version     Prints version information
    -v, --verbose     More verbose log output, including list of all shadertoys found

OPTIONS:
    -k, --apikey <key>                Set shadertoy API key to use. Create your key on https://www.shadertoy.com/myapps
                                      [default: BtHtWD]
    -f, --filter <filter>...          Inclusion filters [values: VR, SoundOutput, SoundInput, Webcam, MultiPass,
                                      MusicStream]
    -y, --gridheight <grid_height>    Grid height [default: 4]
    -x, --gridwidth <grid_width>      Grid width [default: 4]
    -l, --limit <limit>               The maximum number of shaders to download. -1 = no limit [default: -1]
    -o, --order <order>               Sort order [default: Popular]  [values: Name, Love, Popular, Newest, Hot]
    -h, --resheight <res_height>      Window resolution height [default: 768]
    -w, --reswidth <res_width>        Window resolution width [default: 1024]
    -s, --search <string>             Search string to filter which shadertoys to get
    -t, --threads <threads>           How many threads to use for downloading & processing shaders. 0 = disables
                                      threading, -1 = use all logical processors [default: -1]
```

To use the Rust shadertoy API directly in another app or library, check out the [`shadertoy`](https://crates.io/crates/shadertoy) crate, [docs](http://docs.rs/shadertoy) and [README](src/shadertoy/README.MD).

## Todo

This repo and app is a bit of an experimental Rust test range, and it is not intended to support everything or all shadertoys. But here are a couple of things I would like to implement going forward:

- [ ] Rendering on Windows using DX12 and DXC
- [ ] Rendering backend using SPIRV
- [ ] Be able to click to select a shadertoy in grid view
- [ ] Basic IMGUI for interactive searching & filtering
- [ ] Async future based version of the Shadertoy client REST API
- [ ] Async background download and building of shadertoys
- [ ] Proper key-value cache store instead of files
- [ ] Support shadertoys that use textures & buffers
- [ ] Support shadertoys that use multiple passes
- [ ] Support shadertoys that use keyboard input

## License

Licensed under either of the following, at your option:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

### Contribution

Contributions are welcome! Please note that your contributions are assumed to be dual-licensed under Apache-2.0/MIT.
