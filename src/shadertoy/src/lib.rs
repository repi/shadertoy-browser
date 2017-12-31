//! Rust library wrapping the [Shadertoy REST API](http://shadertoy.com/api) to be able to easily search through and download Shadertoy assets.

#[macro_use]
extern crate serde_derive;

mod types;
mod client;

pub use types::*;
pub use client::*;
