//! Shared CF lanes and bounded native-channel recovery for media requests.
use crate::{
    h2_transport::{GLOBAL_BYTES, H2Error, Lane, Reply, SETUP_TIMEOUT},
    outbound::OutboundConnector,
};
use bytes::Bytes;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore, mpsc};
use tokio::task::JoinSet;
use tokio::time::Instant;

type Slot = Arc<Mutex<Option<Arc<Lane>>>>;
const MAX_LANES: usize = 16;
const CHANNEL_BYTES: usize = 8 * 1024 * 1024;
const HISTORY_BYTES: usize = 64 * 1024;
const MAX_AGE: Duration = Duration::from_secs(30);

pub struct Pool {
    lanes: Mutex<HashMap<String, Slot>>,
    cooling: StdMutex<HashMap<String, Instant>>,
    setup_cooling: StdMutex<HashMap<u32, Instant>>,
    pub input_bytes: Arc<Semaphore>,
    reply_bytes: Arc<Semaphore>,
    pub channels: Arc<Semaphore>,
    pub metrics: Metrics,
}
#[derive(Default)]
pub struct Metrics {
    pub opened: std::sync::atomic::AtomicU64,
    pub reused: std::sync::atomic::AtomicU64,
    pub errors: std::sync::atomic::AtomicU64,
    pub fallbacks: std::sync::atomic::AtomicU64,
    pub requests: std::sync::atomic::AtomicU64,
    pub replays: std::sync::atomic::AtomicU64,
    pub active: std::sync::atomic::AtomicU64,
    pub status_errors: std::sync::atomic::AtomicU64,
    pub peak_reserved_bytes: std::sync::atomic::AtomicU64,
}
impl Metrics {
    pub fn summary(&self) -> String {
        use std::sync::atomic::Ordering::Relaxed;
        format!(
            "opened={} reused={} active={} requests={} replays={} errors={} transport_errors={} fallback={}",
            self.opened.load(Relaxed),
            self.reused.load(Relaxed),
            self.active.load(Relaxed),
            self.requests.load(Relaxed),
            self.replays.load(Relaxed),
            self.errors.load(Relaxed),
            self.status_errors.load(Relaxed),
            self.fallbacks.load(Relaxed)
        )
    }
}
pub fn count(counter: &std::sync::atomic::AtomicU64) {
    counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}
impl Default for Pool {
    fn default() -> Self {
        Self {
            lanes: Mutex::new(HashMap::new()),
            cooling: StdMutex::new(HashMap::new()),
            setup_cooling: StdMutex::new(HashMap::new()),
            input_bytes: Arc::new(Semaphore::new(GLOBAL_BYTES / 2)),
            reply_bytes: Arc::new(Semaphore::new(GLOBAL_BYTES / 2)),
            channels: Arc::new(Semaphore::new(16)),
            metrics: Metrics::default(),
        }
    }
}
impl Pool {
    pub async fn open(
        &self,
        dc: u32,
        domains: &[String],
        first: usize,
        outbound: &OutboundConnector,
    ) -> Option<Arc<Lane>> {
        if crate::config::default_dc_ip(dc).is_none()
            || self
                .setup_cooling
                .lock()
                .unwrap()
                .get(&dc)
                .is_some_and(|until| Instant::now() < *until)
        {
            count(&self.metrics.fallbacks);
            return None;
        }
        let deadline = Instant::now() + SETUP_TIMEOUT;
        for offset in 0..domains.len() {
            if Instant::now() >= deadline {
                break;
            }
            let host = format!("kws{dc}.{}", domains[(first + offset) % domains.len()]);
            if self
                .cooling
                .lock()
                .unwrap()
                .get(&host)
                .is_some_and(|until| Instant::now() < *until)
            {
                continue;
            }
            let slot = {
                let mut map = self.lanes.lock().await;
                // Only evict lanes with no active native channel or request.
                if map.len() >= MAX_LANES && !map.contains_key(&host) {
                    map.retain(|_, slot| match slot.try_lock() {
                        Ok(lane) => lane
                            .as_ref()
                            .is_some_and(|lane| Arc::strong_count(lane) > 1),
                        Err(_) => true,
                    });
                }
                if map.len() >= MAX_LANES && !map.contains_key(&host) {
                    continue;
                }
                map.entry(host.clone())
                    .or_insert_with(|| Arc::new(Mutex::new(None)))
                    .clone()
            };
            let result = tokio::time::timeout_at(deadline, async {
                let mut current = slot.lock().await;
                if let Some(lane) = current.as_ref()
                    && !lane.is_closed()
                {
                    count(&self.metrics.reused);
                    return Ok(lane.clone());
                }
                let lane = Lane::connect(&host, outbound, self.reply_bytes.clone()).await?;
                lane.preflight().await?;
                count(&self.metrics.opened);
                *current = Some(lane.clone());
                Ok::<_, H2Error>(lane)
            })
            .await;
            if let Ok(Ok(lane)) = result {
                return Some(lane);
            }
            if result.is_err() {
                self.setup_cooling
                    .lock()
                    .unwrap()
                    .insert(dc, Instant::now() + Duration::from_secs(30));
            }
            count(&self.metrics.errors);
            self.fail(&host).await;
        }
        count(&self.metrics.fallbacks);
        None
    }
    pub async fn fail(&self, host: &str) {
        {
            let mut cooling = self.cooling.lock().unwrap();
            cooling.retain(|_, until| Instant::now() < *until);
            cooling.insert(host.to_string(), Instant::now() + Duration::from_secs(30));
        }
        self.lanes.lock().await.remove(host);
    }
    pub async fn close(&self) {
        self.channels.close();
        self.input_bytes.close();
        self.reply_bytes.close();
        self.lanes.lock().await.clear();
    }
    pub fn summary(&self) -> String {
        format!(
            "{} request_bytes={} reply_reserved_bytes={} peak_reserved_bytes={}",
            self.metrics.summary(),
            GLOBAL_BYTES / 2 - self.input_bytes.available_permits(),
            GLOBAL_BYTES / 2 - self.reply_bytes.available_permits(),
            self.metrics
                .peak_reserved_bytes
                .load(std::sync::atomic::Ordering::Relaxed)
        )
    }

    pub fn observe_memory(&self) {
        let reserved = GLOBAL_BYTES
            - self.input_bytes.available_permits()
            - self.reply_bytes.available_permits();
        self.metrics
            .peak_reserved_bytes
            .fetch_max(reserved as u64, std::sync::atomic::Ordering::Relaxed);
    }
}

pub struct Packet {
    pub body: Bytes,
    pub receiving: Arc<std::sync::atomic::AtomicUsize>,
    pub _global: OwnedSemaphorePermit,
    pub _channel: OwnedSemaphorePermit,
}
pub fn channel_budget() -> Arc<Semaphore> {
    Arc::new(Semaphore::new(CHANNEL_BYTES))
}

struct Retained {
    seq: u64,
    packet: Arc<Packet>,
    created: Instant,
    last_sent: Instant,
    attempts: u8,
    active: usize,
    last_reply: Option<Bytes>,
}

/// Original and recovery requests share the same eight-slot channel budget.
/// History is retained only for encrypted packets and is capped independently.
pub async fn run(
    pool: Arc<Pool>,
    lane: Arc<Lane>,
    mut input: mpsc::Receiver<Packet>,
    output: mpsc::Sender<Result<Reply, H2Error>>,
) {
    count(&pool.metrics.active);
    struct Active(Arc<Pool>);
    impl Drop for Active {
        fn drop(&mut self) {
            self.0
                .metrics
                .active
                .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
    let _active = Active(pool.clone());
    let replies = Arc::new(Semaphore::new(CHANNEL_BYTES));
    let mut requests = JoinSet::new();
    let mut history: VecDeque<Retained> = VecDeque::new();
    let mut seq = 0u64;
    let mut replays_inflight = 0usize;
    let mut eof = false;
    let mut tick = tokio::time::interval(Duration::from_millis(250));
    let mut seen: VecDeque<Bytes> = VecDeque::new();
    loop {
        if eof && requests.is_empty() {
            break;
        }
        tokio::select! {
            _ = output.closed() => break,
            packet = input.recv(), if !eof && requests.len() < 6 => {
                let Some(packet) = packet else {
                    eof = true;
                    continue;
                };
                seq += 1;
                let packet = Arc::new(packet);
                pool.observe_memory();
                if packet.body.len() <= HISTORY_BYTES && packet.body[..8] != [0; 8] {
                    // An auth-key change begins a new logical MTProto session.
                    if history.back().is_some_and(|last| last.packet.body[..8] != packet.body[..8]) {
                        history.clear();
                        seen.clear();
                    }
                    while history.len() >= 16
                        || history.iter().map(|h| h.packet.body.len()).sum::<usize>() + packet.body.len() > HISTORY_BYTES
                    {
                        history.pop_front();
                    }
                    history.push_back(Retained {
                        seq,
                        packet: packet.clone(),
                        created: Instant::now(),
                        last_sent: Instant::now(),
                        attempts: 0,
                        active: 1,
                        last_reply: None,
                    });
                }
                spawn(&mut requests, &lane, &pool, seq, packet, false, replies.clone());
            }
            result = requests.join_next(), if !requests.is_empty() => {
                let Some(Ok((id, replay, result))) = result else { break; };
                if replay {
                    replays_inflight = replays_inflight.saturating_sub(1);
                }
                if let Some(entry) = history.iter_mut().find(|entry| entry.seq == id) {
                    entry.active = entry.active.saturating_sub(1);
                }
                match result {
                    Ok(reply) => {
                        pool.observe_memory();
                        if reply.body.is_empty() { continue; }
                        // Copy only the identity, never retain the full reply buffer.
                        if reply.body.len() >= 24 && reply.body[..8] != [0; 8] {
                            let key = Bytes::copy_from_slice(&reply.body[..24]);
                            if seen.contains(&key) { continue; }
                            if let Some(entry) = history.iter_mut().find(|entry| entry.seq == id)
                                && entry.last_reply.as_ref() != Some(&key)
                            {
                                entry.attempts = 0;
                                entry.last_reply = Some(key.clone());
                            }
                            seen.push_back(key);
                            if seen.len() > 64 { seen.pop_front(); }
                        }
                        if output.send(Ok(reply)).await.is_err() { break; }
                    }
                    Err(error) => {
                        count(&pool.metrics.errors);
                        if matches!(error, H2Error::Transport(_)) {
                            count(&pool.metrics.status_errors);
                        }
                        pool.fail(&lane.host).await;
                        let _ = output.send(Err(error)).await;
                        break;
                    }
                }
            }
            _ = tick.tick(), if !eof => {
                history.retain(|entry| entry.created.elapsed() <= MAX_AGE);
                // Six original slots leave two slots for recovery under saturation.
                // Do not replay while a reply is queued or being written natively.
                if requests.len() < 8 && replays_inflight < 2
                    && replies.available_permits() == CHANNEL_BYTES
                {
                    let last = history.back().map(|entry| entry.seq);
                    let candidate = history.iter_mut().find(|entry| {
                        entry.attempts < 3 && entry.active < 2
                            && entry.packet.receiving.load(std::sync::atomic::Ordering::Relaxed) == 0
                            && entry.last_sent.elapsed() >= Duration::from_secs(
                                if entry.attempts > 0 {2} else if entry.active > 0 {3} else {1})
                            && (entry.active > 0 || (Some(entry.seq) == last && requests.is_empty()))
                    });
                    if let Some(entry) = candidate {
                        replays_inflight += 1;
                        entry.attempts += 1;
                        entry.active += 1;
                        entry.last_sent = Instant::now();
                        spawn(&mut requests, &lane, &pool, entry.seq, entry.packet.clone(), true, replies.clone());
                    }
                }
            }
        }
    }
    requests.shutdown().await;
}
type RequestResult = (u64, bool, Result<Reply, H2Error>);
fn spawn(
    tasks: &mut JoinSet<RequestResult>,
    lane: &Arc<Lane>,
    pool: &Pool,
    id: u64,
    packet: Arc<Packet>,
    replay: bool,
    replies: Arc<Semaphore>,
) {
    count(&pool.metrics.requests);
    if replay {
        count(&pool.metrics.replays);
    }
    let lane = lane.clone();
    tasks.spawn(async move {
        (
            id,
            replay,
            lane.post_with_activity(
                packet.body.clone(),
                replies,
                Some(packet.receiving.clone()),
                replay,
            )
            .await,
        )
    });
}
