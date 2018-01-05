# Shadertoy API

[![Crate](https://img.shields.io/crates/v/shadertoy.svg)](https://crates.io/crates/shadertoy)
[![Docs.rs](https://docs.rs/shadertoy/badge.svg)](https://docs.rs/shadertoy)
[![Build Status](https://travis-ci.com/repi/shadertoy-browser.svg?token=8SzG1tHkq3FpBRftmohU&branch=master)](https://travis-ci.com/repi/shadertoy-browser)
![MIT/Apache](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)

Rust library wrapping the [Shadertoy REST API](http://shadertoy.com/api) to be able to easily search through and download Shadertoy assets.

## Usage

In `Cargo.toml`:

```toml
[dependencies]
shadertoy = "0.5"
```

Example usage:

```rust
extern crate shadertoy;

fn test() {
    let client = shadertoy::Client::new("Bd8tWD"); // insert your own API key here
    
	let search_params = shadertoy::SearchParams {
        string: "car",
        sort_order: shadertoy::SearchSortOrder::Love,
        filters: vec![],
    };

    match client.search(&search_params) {
        Ok(shader_ids) => println!("\"Car\" shadertoys: {:?}", shader_ids),
        Err(err) => println!("Search failed: {}", err),
    }
}
```

TODO

## License

Licensed under either of the following, at your option:

* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

### Contribution

Contributions are welcome! Please note that your contributions are assumed to be dual-licensed under Apache-2.0/MIT.
