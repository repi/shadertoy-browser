//! Rust library wrapping the [Shadertoy REST API](http://shadertoy.com/api) to be able to easily search through and download Shadertoy assets.

#![warn(clippy::all)]
#![warn(rust_2018_idioms)]

mod query;
mod types;

pub use query::*;
pub use types::*;

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "client")]
pub use client::*;
