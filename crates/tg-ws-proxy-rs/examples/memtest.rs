//! Memory harness: fake Telegram WebSocket upstream + CONNECT proxy + clients.
//!
//! `cargo run --release --example memtest -- <proxy-port> <clients> <frames> <frame-kib>`
//!
//! Point the proxy under test at the printed CONNECT address with:
//! `--outbound-proxy http://127.0.0.1:<port> --danger-accept-invalid-certs`
//! `--cf-domain fake.local --pool-size 0`

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use futures_util::SinkExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::{self, ServerConfig};
use tungstenite::Message;

use tg_ws_proxy_rs::crypto::{ProtoTag, generate_client_handshake};

#[allow(clippy::result_large_err)]
fn select_binary_protocol(
    _request: &tungstenite::handshake::server::Request,
    mut response: tungstenite::handshake::server::Response,
) -> Result<tungstenite::handshake::server::Response, tungstenite::handshake::server::ErrorResponse>
{
    response.headers_mut().insert(
        "Sec-WebSocket-Protocol",
        tungstenite::http::HeaderValue::from_static("binary"),
    );
    Ok(response)
}

#[tokio::main]
async fn main() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let args: Vec<String> = std::env::args().collect();
    let proxy_port: u16 = args[1].parse().unwrap();
    let clients: usize = args[2].parse().unwrap();
    let frames: usize = args[3].parse().unwrap();
    let frame_kib: usize = args[4].parse().unwrap();
    let pool_spares: usize = args.get(5).map_or(0, |value| value.parse().unwrap());

    let cert = rcgen::generate_simple_self_signed(vec!["fake.local".to_string()]).unwrap();
    let cert_der = CertificateDer::from(cert.serialize_der().unwrap());
    let key_der = PrivateKeyDer::try_from(cert.serialize_private_key_der()).unwrap();
    let tls = TlsAcceptor::from(Arc::new(
        ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .unwrap(),
    ));

    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = upstream.local_addr().unwrap();
    let upstream_connections = Arc::new(AtomicUsize::new(0));
    let accepted_upstreams = Arc::clone(&upstream_connections);
    tokio::spawn(async move {
        while let Ok((stream, _)) = upstream.accept().await {
            let tls = tls.clone();
            let accepted_upstreams = Arc::clone(&accepted_upstreams);
            tokio::spawn(async move {
                let Ok(stream) = tls.accept(stream).await else {
                    return;
                };
                let Ok(mut ws) =
                    tokio_tungstenite::accept_hdr_async(stream, select_binary_protocol).await
                else {
                    return;
                };
                accepted_upstreams.fetch_add(1, Ordering::Relaxed);
                let payload = vec![0x7e; frame_kib * 1024];
                for _ in 0..frames {
                    if ws.send(Message::Binary(payload.clone())).await.is_err() {
                        return;
                    }
                }
                tokio::time::sleep(Duration::from_secs(120)).await;
            });
        }
    });

    let connect = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let connect_addr = connect.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut inbound, _)) = connect.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                let mut seen = 0;
                while let Ok(n) = inbound.read(&mut buf[seen..]).await {
                    if n == 0 {
                        return;
                    }
                    seen += n;
                    if buf[..seen].windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                if inbound.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await.is_err() {
                    return;
                }
                let Ok(outbound) = TcpStream::connect(upstream_addr).await else {
                    return;
                };
                let (mut inbound_reader, mut inbound_writer) = inbound.split();
                let (mut outbound_reader, mut outbound_writer) = tokio::io::split(outbound);
                let _ = tokio::join!(
                    tokio::io::copy(&mut inbound_reader, &mut outbound_writer),
                    tokio::io::copy(&mut outbound_reader, &mut inbound_writer),
                );
            });
        }
    });

    println!("CONNECT_PORT={}", connect_addr.port());
    println!("START_PROXY=1");
    tokio::time::sleep(Duration::from_secs(2)).await;

    let secret = hex::decode("0ea7201141bf2763a7dee49ba68eeb4c").unwrap();
    let mut held = Vec::new();
    for _ in 0..clients {
        let Ok(mut stream) = TcpStream::connect(("127.0.0.1", proxy_port)).await else {
            continue;
        };
        let (handshake, _, _) = generate_client_handshake(&secret, 2, ProtoTag::PaddedIntermediate);
        if stream.write_all(&handshake).await.is_ok() {
            held.push(stream);
        }
    }
    println!("CLIENTS_CONNECTED={}", held.len());

    let expected_upstreams = held.len() + pool_spares;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while upstream_connections.load(Ordering::Relaxed) < expected_upstreams
        && tokio::time::Instant::now() < deadline
    {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    println!(
        "UPSTREAM_CONNECTED={}",
        upstream_connections.load(Ordering::Relaxed)
    );

    let expected_bytes = frames * frame_kib * 1024;
    let mut readers = Vec::new();
    for mut stream in held {
        readers.push(tokio::spawn(async move {
            if expected_bytes == 0 {
                std::future::pending::<()>().await;
            }

            let mut buf = vec![0u8; 64 * 1024];
            let mut received = 0;
            while received < expected_bytes {
                let Ok(n) = stream.read(&mut buf).await else {
                    return false;
                };
                if n == 0 {
                    return false;
                }
                received += n;
            }
            true
        }));
    }

    if expected_bytes > 0 {
        let mut completed = 0;
        for reader in readers {
            if reader.await.unwrap_or(false) {
                completed += 1;
            }
        }
        println!("TRANSFER_DONE={completed}");
    } else {
        println!("TRANSFER_DONE=0");
    }

    tokio::time::sleep(Duration::from_secs(30)).await;
    println!("DONE=1");
}
