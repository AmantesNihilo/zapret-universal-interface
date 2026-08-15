//! Manual live check used by ZUI maintainers. It intentionally is not part of
//! the offline test suite because it needs Internet access.

use std::time::Duration;

use tg_ws_proxy_rs::{check, default_domains, outbound::OutboundConnector};

#[tokio::main]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let outbound = OutboundConnector::direct();
    let domains = default_domains::fetch_default_domains_with_outbound(&outbound).await;
    let Some(domain) = domains.last() else {
        eprintln!("No default Cloudflare domains available");
        std::process::exit(1);
    };

    println!("Testing {domain} against DC 1/2/3/4/5/203");
    let probes =
        check::probe_cf_domain_all_dcs(domain, false, Duration::from_secs(10), &outbound).await;
    for probe in &probes {
        println!(
            "DC{:>3}: {:<4} {} {}",
            probe.dc,
            if probe.ok { "OK" } else { "FAIL" },
            probe
                .latency_ms
                .map(|value| format!("{value}ms"))
                .unwrap_or_default(),
            probe.target
        );
        if !probe.ok {
            println!("       {}", probe.detail);
        }
    }
    if probes.iter().any(|probe| !probe.ok) {
        std::process::exit(1);
    }
}
