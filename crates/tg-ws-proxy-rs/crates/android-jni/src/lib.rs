//! Android JNI shim for [`tg_ws_proxy_rs`].
//!
//! Its own crate because `crate-type` is a per-package setting: a `cdylib` on
//! the library everything else depends on made every desktop and Docker build
//! additionally link an `.so` nobody loads.  Off Android this compiles to an
//! empty cdylib, which is what lets `cargo fmt` and `cargo metadata` cover it
//! on a host with no NDK.

#[cfg(target_os = "android")]
mod android;
