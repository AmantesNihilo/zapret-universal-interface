use tg_ws_proxy_rs::ws_client::cf_worker_path;

#[test]
fn cf_worker_path_carries_destination_dc_and_media_flag() {
    // The Worker is a TCP tunnel: dst tells it which Telegram DC IP to open,
    // while dc/media are kept for compatibility with the Python Worker code.
    let path = cf_worker_path("149.154.167.51", 2, true);

    assert_eq!(path, "/apiws?dst=149.154.167.51&dc=2&media=1");
}
