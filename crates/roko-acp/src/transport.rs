//! Stdio transport layer for JSON-RPC messages.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, Stdin, Stdout},
    sync::{Mutex as AsyncMutex, oneshot},
};
use tracing::warn;

use crate::types::{
    JsonRpcError, JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};

/// Errors returned by the ACP stdio transport.
#[derive(Debug, Error)]
pub enum TransportError {
    /// Underlying stdio I/O failed.
    #[error("stdio transport I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization or deserialization failed.
    #[error("stdio transport JSON codec failed: {0}")]
    Json(#[from] serde_json::Error),
    /// The pending request registry could not be accessed.
    #[error("pending request registry is poisoned")]
    PendingRequestsPoisoned,
    /// A client response never arrived for an outbound request.
    #[error("client response channel closed for request id {request_id}")]
    ResponseChannelClosed {
        /// The numeric JSON-RPC request identifier.
        request_id: u64,
    },
    /// A write to the client did not finish in time, or an earlier one did not:
    /// the client stopped reading.
    #[error("stdio transport write did not finish within {after_ms} ms")]
    WriteTimeout {
        /// The write timeout in milliseconds.
        after_ms: u64,
    },
}

/// How long a write to the client may block before the client counts as gone.
pub const DEFAULT_WRITE_TIMEOUT: Duration = Duration::from_secs(60);

/// Result alias for ACP stdio transport operations.
pub type TransportResult<T> = Result<T, TransportError>;

/// A stdio JSON-RPC transport for ACP messages.
#[derive(Debug)]
pub struct StdioTransport<R = Stdin, W = Stdout> {
    /// The reader and the bytes of a line it has started, which survive a
    /// `read_message` call that a `select!` cancels.
    reader: Arc<AsyncMutex<(BufReader<R>, Vec<u8>)>>,
    writer: Arc<AsyncMutex<W>>,
    next_id: Arc<AtomicU64>,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
    write_timeout: Duration,
    /// Set once a write timed out. Part of a message may be on the wire, so no
    /// later write may append to it.
    write_failed: Arc<AtomicBool>,
}

/// Removes an outbound request from the shared pending registry when its
/// waiting future completes or is cancelled.
struct PendingRequestGuard {
    request_id: u64,
    pending_requests: Arc<Mutex<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
}

impl Drop for PendingRequestGuard {
    fn drop(&mut self) {
        match self.pending_requests.lock() {
            Ok(mut pending) => {
                pending.remove(&self.request_id);
            }
            Err(_) => warn!(
                request_id = self.request_id,
                "pending request registry poisoned during cancellation cleanup"
            ),
        }
    }
}

impl<R, W> Clone for StdioTransport<R, W> {
    fn clone(&self) -> Self {
        Self {
            reader: Arc::clone(&self.reader),
            writer: Arc::clone(&self.writer),
            next_id: Arc::clone(&self.next_id),
            pending_requests: Arc::clone(&self.pending_requests),
            write_timeout: self.write_timeout,
            write_failed: Arc::clone(&self.write_failed),
        }
    }
}

impl StdioTransport<Stdin, Stdout> {
    /// Creates a stdio transport bound to the process stdin/stdout streams.
    pub fn new() -> Self {
        Self::from_io(tokio::io::stdin(), tokio::io::stdout())
    }
}

impl Default for StdioTransport<Stdin, Stdout> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R, W> StdioTransport<R, W>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    /// Creates a transport from arbitrary async reader and writer handles.
    pub fn from_io(reader: R, writer: W) -> Self {
        Self {
            reader: Arc::new(AsyncMutex::new((BufReader::new(reader), Vec::new()))),
            writer: Arc::new(AsyncMutex::new(writer)),
            next_id: Arc::new(AtomicU64::new(1)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            write_timeout: DEFAULT_WRITE_TIMEOUT,
            write_failed: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Gives up on a write to the client after `timeout` instead of
    /// [`DEFAULT_WRITE_TIMEOUT`].
    #[must_use]
    pub const fn with_write_timeout(mut self, timeout: Duration) -> Self {
        self.write_timeout = timeout;
        self
    }

    /// Reads one newline-delimited JSON-RPC message from stdin.
    ///
    /// Returns `Ok(None)` when EOF is reached before any bytes are read. A call
    /// that is cancelled mid-line keeps what it has read, and the next call
    /// carries on from there.
    pub async fn read_message(&mut self) -> TransportResult<Option<JsonRpcMessage>> {
        let line = {
            let mut guard = self.reader.lock().await;
            let (reader, partial) = &mut *guard;
            let bytes_read = reader.read_until(b'\n', partial).await?;
            if bytes_read == 0 && partial.is_empty() {
                return Ok(None);
            }
            std::mem::take(partial)
        };

        let message = serde_json::from_slice::<JsonRpcMessage>(&line)?;
        Ok(Some(message))
    }

    /// Sends a successful JSON-RPC response to the client.
    pub async fn send_response(
        &mut self,
        id: JsonRpcId,
        result: serde_json::Value,
    ) -> TransportResult<()> {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_owned(),
            id,
            result: Some(result),
            error: None,
        };

        self.write_message(&response).await
    }

    /// Sends a JSON-RPC error response to the client.
    pub async fn send_error(
        &mut self,
        id: JsonRpcId,
        code: i32,
        message: String,
    ) -> TransportResult<()> {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_owned(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        };

        self.write_message(&response).await
    }

    /// Sends a JSON-RPC notification to the client.
    pub async fn send_notification(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> TransportResult<()> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_owned(),
            method: method.to_owned(),
            params: Some(params),
        };

        self.write_message(&notification).await
    }

    /// Sends a JSON-RPC request to the client and waits for the matching response.
    pub async fn send_request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> TransportResult<JsonRpcResponse> {
        let request_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();

        self.pending_requests
            .lock()
            .map_err(|_| TransportError::PendingRequestsPoisoned)?
            .insert(request_id, sender);
        let _pending_guard = PendingRequestGuard {
            request_id,
            pending_requests: Arc::clone(&self.pending_requests),
        };

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: JsonRpcId::Number(request_id),
            method: method.to_owned(),
            params: Some(params),
        };

        self.write_message(&request).await?;

        receiver
            .await
            .map_err(|_| TransportError::ResponseChannelClosed { request_id })
    }

    /// Routes an incoming JSON-RPC response to the matching pending outbound request.
    pub fn handle_incoming_response(&mut self, response: JsonRpcResponse) {
        let JsonRpcId::Number(request_id) = response.id.clone() else {
            warn!(
                response_id = ?response.id,
                "received outbound response with non-numeric id"
            );
            return;
        };

        let pending = match self.pending_requests.lock() {
            Ok(mut pending_requests) => pending_requests.remove(&request_id),
            Err(_) => {
                warn!("pending request registry is poisoned");
                None
            }
        };

        match pending {
            Some(sender) => {
                if sender.send(response).is_err() {
                    warn!(request_id, "response receiver dropped before delivery");
                }
            }
            None => {
                warn!(request_id, "received response for unknown outbound request");
            }
        }
    }

    async fn write_message<T>(&self, message: &T) -> TransportResult<()>
    where
        T: serde::Serialize,
    {
        let after_ms = u64::try_from(self.write_timeout.as_millis()).unwrap_or(u64::MAX);
        if self.write_failed.load(Ordering::Acquire) {
            return Err(TransportError::WriteTimeout { after_ms });
        }
        let bytes = serde_json::to_vec(message)?;
        let write = async {
            let mut writer = self.writer.lock().await;
            writer.write_all(&bytes).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await
        };
        match tokio::time::timeout(self.write_timeout, write).await {
            Ok(result) => Ok(result?),
            Err(_) => {
                self.write_failed.store(true, Ordering::Release);
                Err(TransportError::WriteTimeout { after_ms })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, duplex, empty, sink};

    use super::*;
    use crate::types::JsonRpcNotification;

    #[tokio::test]
    async fn reads_valid_json_rpc_request() {
        let (client, server) = duplex(1024);
        let writer_task = tokio::spawn(async move {
            let mut client = client;
            client
                .write_all(br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1}}"#)
                .await
                .expect("write request bytes");
            client.write_all(b"\n").await.expect("write newline");
        });

        let mut transport = StdioTransport::from_io(server, sink());
        let message = transport
            .read_message()
            .await
            .expect("read message")
            .expect("message present");

        writer_task.await.expect("writer task");

        let JsonRpcMessage::Request(request) = message else {
            panic!("expected request message");
        };
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, JsonRpcId::Number(1));
        assert_eq!(request.method, "initialize");
        assert_eq!(request.params, Some(json!({ "protocolVersion": 1 })));
    }

    #[tokio::test]
    async fn returns_none_on_eof() {
        let mut transport = StdioTransport::from_io(empty(), sink());

        let message = transport.read_message().await.expect("read EOF");

        assert!(message.is_none());
    }

    #[tokio::test]
    async fn writes_json_rpc_notification() {
        let (mut client_reader, server_writer) = duplex(1024);
        let mut transport = StdioTransport::from_io(empty(), server_writer);

        transport
            .send_notification("session/update", json!({ "sessionUpdate": "plan" }))
            .await
            .expect("write notification");

        let mut line = String::new();
        BufReader::new(&mut client_reader)
            .read_line(&mut line)
            .await
            .expect("read notification line");

        let notification: JsonRpcNotification =
            serde_json::from_str(&line).expect("parse notification payload");
        assert_eq!(notification.method, "session/update");
    }

    #[tokio::test]
    async fn cancelled_outbound_request_cleans_pending_registry() {
        let (client, server_writer) = duplex(1024);
        let mut transport = StdioTransport::from_io(empty(), server_writer);
        let pending = Arc::clone(&transport.pending_requests);

        let request_task = tokio::spawn(async move {
            transport
                .send_request("session/request_permission", json!({}))
                .await
        });

        let mut reader = BufReader::new(client);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .expect("read outbound request");
        assert_eq!(pending.lock().expect("pending registry").len(), 1);

        request_task.abort();
        assert!(
            request_task
                .await
                .expect_err("request should be aborted")
                .is_cancelled()
        );
        assert!(
            pending
                .lock()
                .expect("pending registry after abort")
                .is_empty(),
            "cancelling send_request must not leak a pending response sender"
        );
    }

    #[tokio::test]
    async fn bridge_under_load_read_resumes_after_a_cancelled_read() {
        let (mut client, server) = duplex(1024);
        let mut transport = StdioTransport::from_io(server, sink());
        let line = br#"{"jsonrpc":"2.0","id":3,"method":"session/list","params":{}}"#;
        let (head, tail) = line.split_at(20);

        // Half a line arrives, and the read is cancelled before the rest does.
        client.write_all(head).await.expect("write first half");
        let cancelled =
            tokio::time::timeout(Duration::from_millis(20), transport.read_message()).await;
        assert!(cancelled.is_err(), "half a line must not parse");

        // The next read carries on from the bytes already taken.
        client.write_all(tail).await.expect("write second half");
        client.write_all(b"\n").await.expect("write newline");
        let message = transport
            .read_message()
            .await
            .expect("read message")
            .expect("message present");
        let JsonRpcMessage::Request(request) = message else {
            panic!("expected a request");
        };
        assert_eq!(request.id, JsonRpcId::Number(3));
        assert_eq!(request.method, "session/list");
    }

    #[tokio::test]
    async fn bridge_under_load_write_gives_up_on_a_client_that_stopped_reading() {
        // The client never reads, so the 64-byte pipe fills and the write stalls.
        let (_client, server) = duplex(64);
        let mut transport =
            StdioTransport::from_io(empty(), server).with_write_timeout(Duration::from_millis(50));
        let started = std::time::Instant::now();
        let error = transport
            .send_notification("session/update", json!({ "text": "x".repeat(1_024) }))
            .await
            .expect_err("the write must give up");
        assert!(matches!(
            error,
            TransportError::WriteTimeout { after_ms: 50 }
        ));
        assert!(started.elapsed() < Duration::from_secs(5));

        // Later writes fail at once instead of adding to a half-written line.
        let again = transport
            .send_notification("session/update", json!({}))
            .await;
        assert!(matches!(again, Err(TransportError::WriteTimeout { .. })));
    }
}
