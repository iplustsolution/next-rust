//! Core building blocks shared by every Next Rust crate.
//!
//! * [`config`] – `next-rust.toml` / `next-rust.json` discovery and parsing.
//! * [`env`] – `.env` file loading with a strict server/public split.
//! * [`diagnostic`] – structured, human friendly error reporting.
//! * [`RenderingMode`] – static / dynamic / auto rendering.

#![deny(unsafe_code)]

pub mod config;
pub mod diagnostic;
pub mod env;
mod mode;

pub use config::{Config, ConfigError};
pub use diagnostic::{Diagnostic, Diagnostics, Severity};
pub use mode::{Environment, RenderingMode};
