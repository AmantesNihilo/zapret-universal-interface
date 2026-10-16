//! Bounded HTTP/2 transport. Every lane owns its connection driver and budgets.
use crate::{native_packet::MAX_PACKET, outbound::OutboundConnector};
use bytes::{Bytes, BytesMut};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use tokio::task::AbortHandle;

pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(65);
pub const SETUP_TIMEOUT: Duration = Duration::from_secs(8);
pub const LANE_BYTES: usize = 32 * 1024 * 1024;
pub const GLOBAL_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug)]
pub enum H2Error {
    Transport(i32),
    Network(String),
}
impl std::fmt::Display for H2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport(code) => write!(f, "MTProto transport error {code}"),
            Self::Network(message) => f.write_str(message),
        }
    }
}
impl From<String> for H2Error {
    fn from(value: String) -> Self {
        Self::Network(value)
    }
}

pub struct Reply {
    pub body: Bytes,
    // Held until the native client has consumed the reply, including queued replies.
    _lane: OwnedSemaphorePermit,
    _global: OwnedSemaphorePermit,
    _channel: OwnedSemaphorePermit,
}

pub struct Lane {
    sender: Mutex<h2::client::SendRequest<Bytes>>,
    driver: AbortHandle,
    pub host: String,
    requests: Arc<Semaphore>,
    original_requests: Arc<Semaphore>,
    original_bytes: Arc<Semaphore>,
    request_bytes: Arc<Semaphore>,
    reply_bytes: Arc<Semaphore>,
    global: Arc<Semaphore>,
}
impl Drop for Lane {
    fn drop(&mut self) {
        self.driver.abort();
    }
}

impl Lane {
    pub fn is_closed(&self) -> bool {
        self.driver.is_finished()
    }
    pub async fn connect(
        host: &str,
        outbound: &OutboundConnector,
        global: Arc<Semaphore>,
    ) -> Result<Arc<Self>, H2Error> {
        tokio::time::timeout(SETUP_TIMEOUT, async {
            let tcp = outbound
                .connect(host, 443, SETUP_TIMEOUT)
                .await
                .map_err(|e| H2Error::Network(e.to_string()))?;
            let _ = tcp.set_nodelay(true);
            static H2_TLS: std::sync::OnceLock<Arc<rustls::ClientConfig>> =
                std::sync::OnceLock::new();
            let config = H2_TLS
                .get_or_init(|| {
                    let mut config = (*crate::ws_client::verifying_rustls_config()).clone();
                    config.alpn_protocols = vec![b"h2".to_vec()];
                    Arc::new(config)
                })
                .clone();
            let name = rustls::pki_types::ServerName::try_from(host.to_string())
                .map_err(|e| H2Error::Network(e.to_string()))?;
            let tls = tokio_rustls::TlsConnector::from(config)
                .connect(name, tcp)
                .await
                .map_err(|e| H2Error::Network(e.to_string()))?;
            if tls.get_ref().1.alpn_protocol() != Some(b"h2") {
                return Err(H2Error::Network("server did not negotiate h2".into()));
            }
            Self::handshake(tls, host.to_string(), global).await
        })
        .await
        .map_err(|_| H2Error::Network("H2 setup timeout".into()))?
    }

    pub(crate) async fn handshake<T>(
        io: T,
        host: String,
        global: Arc<Semaphore>,
    ) -> Result<Arc<Self>, H2Error>
    where
        T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let (sender, connection) = h2::client::Builder::new()
            .initial_window_size(256 * 1024)
            .initial_connection_window_size(1024 * 1024)
            .max_concurrent_streams(64)
            .max_header_list_size(16 * 1024)
            .max_send_buffer_size(64 * 1024)
            .handshake(io)
            .await
            .map_err(|e| H2Error::Network(e.to_string()))?;
        let driver = tokio::spawn(async move {
            let _ = connection.await;
        });
        Ok(Arc::new(Self {
            sender: Mutex::new(sender),
            driver: driver.abort_handle(),
            host,
            requests: Arc::new(Semaphore::new(64)),
            original_requests: Arc::new(Semaphore::new(6)),
            original_bytes: Arc::new(Semaphore::new(LANE_BYTES - 1024 * 1024)),
            request_bytes: Arc::new(Semaphore::new(LANE_BYTES)),
            reply_bytes: Arc::new(Semaphore::new(LANE_BYTES)),
            global,
        }))
    }

    pub async fn preflight(&self) -> Result<(), H2Error> {
        let (response, _) = self.start_request("HEAD", 0).await?;
        let response = response
            .await
            .map_err(|e| H2Error::Network(e.to_string()))?;
        let status = response.status().as_u16();
        if !(200..300).contains(&status) && !matches!(status, 405 | 501) {
            return Err(H2Error::Network(format!("H2 preflight HTTP {status}")));
        }
        Ok(())
    }

    async fn start_request(
        &self,
        method: &str,
        length: usize,
    ) -> Result<(h2::client::ResponseFuture, h2::SendStream<Bytes>), H2Error> {
        let request = http::Request::builder()
            .method(method)
            .uri(format!("https://{}/api", self.host))
            .header("content-type", "application/octet-stream")
            .header("accept-encoding", "identity")
            .header("content-length", length)
            .body(())
            .map_err(|e| H2Error::Network(e.to_string()))?;
        let mut sender = self.sender.lock().await;
        std::future::poll_fn(|cx| sender.poll_ready(cx))
            .await
            .map_err(|e| H2Error::Network(e.to_string()))?;
        sender
            .send_request(request, method == "HEAD")
            .map_err(|e| H2Error::Network(e.to_string()))
    }

    pub async fn post(&self, body: Bytes, channel: Arc<Semaphore>) -> Result<Reply, H2Error> {
        self.post_with_activity(body, channel, None, false).await
    }

    pub(crate) async fn post_with_activity(
        &self,
        body: Bytes,
        channel: Arc<Semaphore>,
        activity: Option<Arc<std::sync::atomic::AtomicUsize>>,
        replay: bool,
    ) -> Result<Reply, H2Error> {
        if body.is_empty() || body.len() > MAX_PACKET || !body.len().is_multiple_of(4) {
            return Err(H2Error::Network("invalid H2 request body".into()));
        }
        tokio::time::timeout(
            REQUEST_TIMEOUT,
            self.post_inner(body, channel, activity, replay),
        )
        .await
        .map_err(|_| H2Error::Network("H2 request timeout".into()))?
    }
    async fn post_inner(
        &self,
        mut body: Bytes,
        channel: Arc<Semaphore>,
        activity: Option<Arc<std::sync::atomic::AtomicUsize>>,
        replay: bool,
    ) -> Result<Reply, H2Error> {
        // Keep recovery capacity available even if every original is stalled.
        let _original = if !replay {
            Some(
                self.original_requests
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|_| H2Error::Network("lane closed".into()))?,
            )
        } else {
            None
        };
        let _original_bytes = if !replay {
            Some(
                self.original_bytes
                    .clone()
                    .acquire_many_owned(body.len() as u32)
                    .await
                    .map_err(|_| H2Error::Network("lane closed".into()))?,
            )
        } else {
            None
        };
        let _request = self
            .requests
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| H2Error::Network("lane closed".into()))?;
        let _bytes = self
            .request_bytes
            .clone()
            .acquire_many_owned(body.len() as u32)
            .await
            .map_err(|_| H2Error::Network("lane closed".into()))?;
        // Reserve reply space before opening a stream. Waiting for memory
        // after response headers can deadlock the connection window when DATA
        // for an unconsumed stream blocks otherwise runnable streams.
        let channel = channel
            .acquire_many_owned(MAX_PACKET as u32)
            .await
            .map_err(|_| H2Error::Network("channel closed".into()))?;
        let lane = self
            .reply_bytes
            .clone()
            .acquire_many_owned(MAX_PACKET as u32)
            .await
            .map_err(|_| H2Error::Network("lane closed".into()))?;
        let global = self
            .global
            .clone()
            .acquire_many_owned(MAX_PACKET as u32)
            .await
            .map_err(|_| H2Error::Network("H2 budget closed".into()))?;
        let (response, mut upload) = self.start_request("POST", body.len()).await?;
        while !body.is_empty() {
            upload.reserve_capacity(body.len().min(64 * 1024));
            let capacity = std::future::poll_fn(|cx| upload.poll_capacity(cx))
                .await
                .ok_or_else(|| H2Error::Network("H2 upload closed".into()))?
                .map_err(|e| H2Error::Network(e.to_string()))?;
            if capacity == 0 {
                continue;
            }
            let count = capacity.min(body.len());
            let chunk = body.split_to(count);
            upload
                .send_data(chunk, body.is_empty())
                .map_err(|e| H2Error::Network(e.to_string()))?;
        }
        let response = response
            .await
            .map_err(|e| H2Error::Network(e.to_string()))?;
        let status = response.status().as_u16();
        if matches!(status, 403 | 404 | 429 | 444) {
            return Err(H2Error::Transport(-(status as i32)));
        }
        if status != 200 {
            return Err(H2Error::Network(format!("H2 HTTP {status}")));
        }
        if response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<usize>().ok())
            .is_some_and(|v| v > MAX_PACKET)
        {
            return Err(H2Error::Network("H2 reply too large".into()));
        }
        // Reserve before reading: peer responses cannot grow an unbounded heap.
        struct Receiving(Option<Arc<std::sync::atomic::AtomicUsize>>);
        impl Drop for Receiving {
            fn drop(&mut self) {
                if let Some(counter) = &self.0 {
                    counter.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
        if let Some(counter) = &activity {
            counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        let _receiving = Receiving(activity);
        let mut stream = response.into_body();
        let mut data = BytesMut::new();
        while let Some(chunk) = stream.data().await {
            let chunk = chunk.map_err(|e| H2Error::Network(e.to_string()))?;
            if data.len() + chunk.len() > MAX_PACKET {
                return Err(H2Error::Network("H2 reply too large".into()));
            }
            data.extend_from_slice(&chunk);
            stream
                .flow_control()
                .release_capacity(chunk.len())
                .map_err(|e| H2Error::Network(e.to_string()))?;
        }
        if data.len() == 4 {
            let code = i32::from_le_bytes(data[..].try_into().unwrap());
            if code < 0 {
                return Err(H2Error::Transport(code));
            }
        }
        if !data.len().is_multiple_of(4) {
            return Err(H2Error::Network("unaligned H2 reply".into()));
        }
        Ok(Reply {
            body: data.freeze(),
            _lane: lane,
            _global: global,
            _channel: channel,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests;
