//! Asset primitives shared by the Next Rust build pipeline, procedural
//! macros and the runtime server.
//!
//! This crate has **no dependencies**. It provides:
//!
//! * [`hash`] – a stable, non-cryptographic content hash used for
//!   fingerprinting and cache busting.
//! * [`mime`] – file-extension based MIME detection.
//! * [`css`] – a small CSS tokenizer used for minification and for
//!   CSS-module class scoping.
//! * [`names`] – the short names release builds give `data-*` attributes,
//!   custom properties, ids, keyframes and layers.

#![forbid(unsafe_code)]

pub mod css;
pub mod hash;
pub mod html;
pub mod mime;
pub mod names;

pub use hash::{Fnv64, content_hash, fingerprint_name};
