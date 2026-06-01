# Vendored Rust engines

This directory contains Rust crates that are compiled into Z2 instead of being launched as separate GUI/tray executables.

## tg-ws-proxy-rs

- Source: https://github.com/valnesfjord/tg-ws-proxy-rs
- Integrated version: 1.5.0
- Upstream reference: https://github.com/Flowseal/tg-ws-proxy

The crate is used as a path dependency by `src-tauri`. Keep the code isolated in `crates/tg-ws-proxy-rs` so updating it is a replace-and-check operation: update the vendored crate, keep Z2's adapter in `src-tauri/src/runtime/tg_ws.rs`, then run the Rust and Svelte checks.
