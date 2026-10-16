use super::*;
use tokio::task::JoinHandle;

pub(crate) async fn server(
    status: u16,
    reply: Option<Bytes>,
    length: Option<usize>,
    stall: bool,
) -> (
    Arc<Lane>,
    JoinHandle<()>,
    Arc<std::sync::atomic::AtomicUsize>,
) {
    let (client, server) = tokio::io::duplex(64 * 1024);
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let requests = count.clone();
    let task = tokio::spawn(async move {
        let mut connection = h2::server::handshake(server).await.unwrap();
        let mut tasks = tokio::task::JoinSet::new();
        while let Some(Ok((request, mut respond))) = connection.accept().await {
            while tasks.try_join_next().is_some() {}
            requests.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let reply = reply.clone();
            tasks.spawn(async move {
                let head = request.method() == http::Method::HEAD;
                let mut input = request.into_body();
                let mut echo = BytesMut::new();
                while let Some(Ok(chunk)) = input.data().await {
                    echo.extend_from_slice(&chunk);
                    input.flow_control().release_capacity(chunk.len()).unwrap();
                }
                if stall {
                    std::future::pending::<()>().await;
                }
                let mut response = http::Response::builder().status(status);
                if let Some(length) = length {
                    response = response.header("content-length", length);
                }
                let Ok(mut stream) = respond.send_response(response.body(()).unwrap(), head) else {
                    return;
                };
                if !head {
                    let mut reply = reply.unwrap_or_else(|| echo.freeze());
                    if reply.is_empty() {
                        let _ = stream.send_data(Bytes::new(), true);
                        return;
                    }
                    while !reply.is_empty() {
                        stream.reserve_capacity(reply.len().min(16 * 1024));
                        let Some(Ok(capacity)) =
                            std::future::poll_fn(|cx| stream.poll_capacity(cx)).await
                        else {
                            return;
                        };
                        if capacity == 0 {
                            continue;
                        }
                        let n = capacity.min(reply.len());
                        let bytes = reply.split_to(n);
                        if stream.send_data(bytes, reply.is_empty()).is_err() {
                            return;
                        }
                    }
                }
            });
        }
    });
    let lane = Lane::handshake(
        client,
        "mock.local".into(),
        Arc::new(Semaphore::new(GLOBAL_BYTES / 2)),
    )
    .await
    .unwrap();
    (lane, task, count)
}
fn budget() -> Arc<Semaphore> {
    Arc::new(Semaphore::new(8 * 1024 * 1024))
}

#[tokio::test]
async fn load_one_thousand_posts_and_one_hundred_concurrent_requests() {
    let (lane, server, count) = server(200, None, None, false).await;
    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let reply = lane
            .post(Bytes::from_static(&[1; 24]), budget())
            .await
            .unwrap();
        assert_eq!(reply.body.len(), 24);
    }
    let mut requests = tokio::task::JoinSet::new();
    for _ in 0..100 {
        let lane = lane.clone();
        requests.spawn(async move {
            let reply = lane
                .post(Bytes::from(vec![1; 1024 * 1024]), budget())
                .await
                .unwrap();
            assert_eq!(reply.body.len(), 1024 * 1024);
        });
    }
    while let Some(result) = requests.join_next().await {
        result.unwrap();
    }
    assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 1100);
    assert_eq!(lane.requests.available_permits(), 64);
    assert_eq!(lane.request_bytes.available_permits(), LANE_BYTES);
    assert_eq!(lane.reply_bytes.available_permits(), LANE_BYTES);
    println!(
        "H2 load: 1000 sequential + 100 concurrent, elapsed_ms={}, retained_request_bytes=0, retained_reply_bytes=0",
        start.elapsed().as_millis()
    );
    drop(lane);
    server.abort();
}

#[tokio::test]
#[ignore = "sustained RAM sampling; run explicitly"]
async fn sustained_memory_sampling() {
    let (lane, server, _) = server(200, None, None, false).await;
    println!("phase=baseline");
    tokio::time::sleep(Duration::from_secs(2)).await;
    for round in 0..5 {
        println!("phase=load round={round}");
        let mut tasks = tokio::task::JoinSet::new();
        for id in 1..=100 {
            let lane = lane.clone();
            tasks.spawn(async move {
                let reply = lane
                    .post(Bytes::from(vec![id; 1024 * 1024]), budget())
                    .await
                    .unwrap();
                assert_eq!(reply.body.len(), 1024 * 1024);
            });
        }
        while let Some(result) = tasks.join_next().await {
            result.unwrap();
        }
        assert_eq!(lane.requests.available_permits(), 64);
        assert_eq!(lane.request_bytes.available_permits(), LANE_BYTES);
        assert_eq!(lane.reply_bytes.available_permits(), LANE_BYTES);
        println!("phase=idle round={round} requests=100 released_bytes=all");
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    drop(lane);
    server.abort();
    println!("phase=closed");
    tokio::time::sleep(Duration::from_secs(2)).await;
}

#[tokio::test]
#[ignore = "requires a live CF domain and network"]
async fn live_cloudflare_h2_preflight() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let pool = crate::cf_h2::Pool::default();
    let domains = crate::default_domains::fallback_domains();
    let lane = pool
        .open(
            2,
            &domains,
            0,
            &crate::outbound::OutboundConnector::direct(),
        )
        .await;
    assert!(
        lane.is_some(),
        "No bundled CF domain passed TLS ALPN h2 + HEAD /api within 8 seconds"
    );
    println!("live CF: verified TLS, negotiated h2, successful /api preflight");
    // Telegram's documented initial DH query needs no account or auth key:
    // https://core.telegram.org/mtproto/auth_key
    let nonce: [u8; 16] = rand::random();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let id = ((now.as_secs() << 32) + ((now.subsec_nanos() as u64) << 32) / 1_000_000_000) & !3;
    let mut request = vec![0; 8];
    request.extend_from_slice(&id.to_le_bytes());
    request.extend_from_slice(&20u32.to_le_bytes());
    request.extend_from_slice(&0xbe7e8ef1u32.to_le_bytes());
    request.extend_from_slice(&nonce);
    let reply = lane
        .as_ref()
        .unwrap()
        .post(Bytes::from(request), budget())
        .await
        .expect("live req_pq_multi response");
    assert!(reply.body.len() >= 56, "short resPQ response");
    assert_eq!(&reply.body[..8], &[0; 8]);
    assert_eq!(&reply.body[20..24], &0x05162463u32.to_le_bytes());
    assert_eq!(&reply.body[24..40], &nonce);
    println!("live CF: req_pq_multi -> resPQ, matching random nonce");
    drop(lane);
    pool.close().await;
}

#[tokio::test]
async fn multiplexes_large_posts_on_one_lane_and_releases_budgets() {
    let (lane, server, count) = server(200, None, None, false).await;
    lane.preflight().await.unwrap();
    let mut jobs = tokio::task::JoinSet::new();
    for i in 1..=8 {
        let lane = lane.clone();
        jobs.spawn(async move {
            let body = Bytes::from(vec![i; 1024 * 1024]);
            let reply = lane.post(body.clone(), budget()).await.unwrap();
            assert_eq!(reply.body, body);
        });
    }
    while let Some(result) = jobs.join_next().await {
        result.unwrap();
    }
    assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 9);
    assert_eq!(lane.request_bytes.available_permits(), LANE_BYTES);
    assert_eq!(lane.reply_bytes.available_permits(), LANE_BYTES);
    drop(lane);
    server.abort();
}
#[tokio::test]
async fn maps_http_statuses_and_negative_bodies_to_native_transport_errors() {
    for status in [403, 404, 429, 444] {
        let (lane, task, _) = server(status, Some(Bytes::new()), None, false).await;
        assert!(
            matches!(lane.post(Bytes::from_static(&[1;24]),budget()).await,Err(H2Error::Transport(code)) if code==-(status as i32))
        );
        task.abort();
    }
    let (lane, task, _) = server(
        200,
        Some(Bytes::copy_from_slice(&(-404i32).to_le_bytes())),
        None,
        false,
    )
    .await;
    assert!(matches!(
        lane.post(Bytes::from_static(&[1; 24]), budget()).await,
        Err(H2Error::Transport(-404))
    ));
    task.abort();
}
#[tokio::test]
async fn rejects_oversized_declared_and_streamed_replies() {
    for declared in [true, false] {
        let (lane, task, _) = server(
            200,
            Some(Bytes::from(vec![1; MAX_PACKET + 4])),
            if declared { Some(MAX_PACKET + 4) } else { None },
            false,
        )
        .await;
        assert!(
            lane.post(Bytes::from_static(&[1; 24]), budget())
                .await
                .is_err()
        );
        assert_eq!(lane.reply_bytes.available_permits(), LANE_BYTES);
        task.abort();
    }
}
#[tokio::test(start_paused = true)]
async fn stalled_request_has_a_deadline_and_releases_its_capacity() {
    let (lane, task, _) = server(200, None, None, true).await;
    assert!(
        matches!(lane.post(Bytes::from_static(&[1;24]),budget()).await,Err(H2Error::Network(message)) if message.contains("timeout"))
    );
    assert_eq!(lane.requests.available_permits(), 64);
    task.abort();
}
#[tokio::test]
async fn dropping_lane_closes_the_driver() {
    let (lane, task, _) = server(200, None, None, false).await;
    let driver = lane.driver.clone();
    drop(lane);
    tokio::time::timeout(Duration::from_secs(1), async {
        while !driver.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    task.abort();
}
#[tokio::test(start_paused = true)]
async fn recovery_replays_bounded_history_and_deduplicates_replies() {
    use crate::cf_h2::{Packet, Pool, channel_budget};
    let (lane, server, count) = server(200, None, None, false).await;
    let pool = Arc::new(Pool::default());
    let (input_tx, input_rx) = tokio::sync::mpsc::channel(1);
    let (output_tx, mut output_rx) = tokio::sync::mpsc::channel(2);
    let bytes = Bytes::from_static(&[1; 24]);
    let packet = Packet {
        receiving: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        body: bytes.clone(),
        _global: pool
            .input_bytes
            .clone()
            .acquire_many_owned(24)
            .await
            .unwrap(),
        _channel: channel_budget().acquire_many_owned(24).await.unwrap(),
    };
    input_tx.send(packet).await.unwrap();
    let session = tokio::spawn(crate::cf_h2::run(pool.clone(), lane, input_rx, output_tx));
    assert_eq!(output_rx.recv().await.unwrap().unwrap().body, bytes);
    tokio::time::sleep(Duration::from_secs(16)).await;
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }
    assert!(
        count.load(std::sync::atomic::Ordering::Relaxed) >= 2,
        "recovery should poll last encrypted packet"
    );
    assert!(
        output_rx.try_recv().is_err(),
        "same encrypted reply must not be delivered twice"
    );
    drop(input_tx);
    session.await.unwrap();
    server.abort();
    assert_eq!(pool.input_bytes.available_permits(), GLOBAL_BYTES / 2);
    assert_eq!(
        pool.metrics
            .active
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
}
