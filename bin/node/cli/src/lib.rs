
//! yeeroot2 CLI library.
//!
//! This package has two Cargo features:
//!
//! - `cli` (default): exposes functions that parse command-line options, then start and run the
//! node as a CLI application.
//!
//! - `browser`: exposes the content of the `browser` module, which consists of exported symbols
//! that are meant to be passed through the `wasm-bindgen` utility and called from JavaScript.
//! Despite its name the produced WASM can theoretically also be used from NodeJS, although this
//! hasn't been tested.

#![warn(missing_docs)]

pub mod chain_spec;

#[macro_use]
mod service;
// #[cfg(feature = "browser")]
// mod browser;
#[cfg(feature = "cli")]
mod cli;
//#[cfg(feature = "cli")]
//mod factory_impl;
#[cfg(feature = "cli")]
mod command;

#[cfg(feature = "browser")]
pub use browser::*;
#[cfg(feature = "cli")]
pub use cli::*;
#[cfg(feature = "cli")]
pub use command::*;
