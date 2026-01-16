//! FFI module for native app integration
//!
//! Provides C-compatible functions for iOS and macOS Swift integration.

#[cfg(any(target_os = "ios", target_os = "macos"))]
mod apple;

#[cfg(any(target_os = "ios", target_os = "macos"))]
pub use apple::*;
