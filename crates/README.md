# Vendored Rust engines

This directory contains Rust crates that are compiled into Z2 instead of being launched as separate GUI/tray executables.

## tg-ws-proxy-rs

- Source: https://github.com/valnesfjord/tg-ws-proxy-rs
- Integrated version: 2.2.5-zui.1, based on upstream 2.2.4 (`641c6b6bd028b17c1f238f9339103345c2887b66`)
- Upstream reference: https://github.com/Flowseal/tg-ws-proxy
- Reviewed against Flowseal release: 1.10.0 (`b2a8074c59c52cabde7fe295280b614cc6c01fce`)

The crate is used as a path dependency by `src-tauri`. Keep the code isolated in `crates/tg-ws-proxy-rs` so updating it is a replace-and-check operation: update the vendored crate, keep Z2's adapter in `src-tauri/src/runtime/tg_ws.rs`, then run the Rust and Svelte checks.
