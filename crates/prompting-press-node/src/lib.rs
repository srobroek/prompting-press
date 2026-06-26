//! # prompting-press-node
//!
//! The Node.js binding for Prompting Press, built with [napi-rs] (Node-API). This crate
//! exposes the Rust consumer surface ([`prompting_press`]) to Node.js as a native addon.
//!
//! It is one of exactly two crates (the other being `prompting-press-py`) permitted to
//! depend on an FFI toolkit; the kernel and the Rust consumer stay FFI-free
//! (constitution Principle II / C-02).
//!
//! This is a spec-001 stub: it exports a single trivial function to prove the napi
//! addon wiring works end to end.
//!
//! [napi-rs]: https://napi.rs

use napi_derive::napi;

/// Returns the kernel version, reached through the Rust consumer surface.
///
/// Placeholder so the addon exports something callable from JavaScript and so the
/// dependency edges onto `prompting-press`/`prompting-press-core` are exercised.
#[napi]
pub fn core_version() -> &'static str {
    prompting_press::core_version()
}
