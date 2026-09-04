//! Shared test utilities for roko-agent integration tests.
//!
//! Provides [`MockHttpPoster`] — a multi-response, request-recording mock
//! for the [`HttpPoster`] trait — and [`TestServer`] — a scripted TCP
//! server for tests that need a real HTTP endpoint.

#![allow(dead_code)]

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use async_trait::async_trait;
use roko_agent::http::{HttpPostError, HttpPoster};
use serde_json::Value;

// ─── MockHttpPoster ─────────────────────────────────────────────────

/// A recorded HTTP request captured by [`MockHttpPoster`].
#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub url: String,
    pub headers: Vec<(String, String)>,
    /// Request body parsed as JSON. Panics during capture if body is not
    /// valid JSON.
    pub body: Value,
    pub timeout_ms: u64,
}

/// A multi-response, request-recording mock for [`HttpPoster`].
///
/// Supports:
/// - **Multi-response queue**: supply a `Vec` of responses; each call to
///   `post_json` pops one from the front.
/// - **Request recording**: all received requests are captured and
///   available via [`requests()`](MockHttpPoster::requests).
/// - **Configurable status codes**: queue `Err(HttpPostError::http(...))`
///   for non-2xx responses.
#[derive(Debug)]
pub struct MockHttpPoster {
    responses: Mutex<VecDeque<Result<String, HttpPostError>>>,
    requests: Mutex<Vec<RecordedRequest>>,
}

impl MockHttpPoster {
    /// Create a mock that returns the given responses in order.
    ///
    /// Each response is `Ok(body_string)` for success or
    /// `Err(HttpPostError)` for failure.
    pub fn new(responses: Vec<Result<String, HttpPostError>>) -> Arc<Self> {
        Arc::new(Self {
            responses: Mutex::new(responses.into_iter().collect()),
            requests: Mutex::new(Vec::new()),
        })
    }

    /// Convenience: create a mock from plain success bodies.
    pub fn from_bodies(bodies: Vec<String>) -> Arc<Self> {
        Self::new(bodies.into_iter().map(Ok).collect())
    }

    /// Return all captured requests.
    pub fn requests(&self) -> Vec<RecordedRequest> {
        self.requests.lock().expect("requests lock").clone()
    }
}

#[async_trait]
impl HttpPoster for MockHttpPoster {
    async fn post_json(
        &self,
        url: &str,
        headers: &[(String, String)],
        body: &[u8],
        timeout_ms: u64,
    ) -> Result<String, HttpPostError> {
        let body: Value = serde_json::from_slice(body).expect("request body must be json");
        self.requests
            .lock()
            .expect("requests lock")
            .push(RecordedRequest {
                url: url.to_string(),
                headers: headers.to_vec(),
                body,
                timeout_ms,
            });

        self.responses
            .lock()
            .expect("responses lock")
            .pop_front()
            .unwrap_or_else(|| Err(HttpPostError::transport("no mock response queued")))
    }
}

// ─── TestServer (TCP) ───────────────────────────────────────────────

/// A scripted HTTP response for [`TestServer`].
#[derive(Debug, Clone)]
pub struct ScriptedResponse {
    pub status: u16,
    pub body: String,
}

/// Create a [`ScriptedResponse`] with the given status and JSON body.
pub fn scripted_response(status: u16, body: Value) -> ScriptedResponse {
    ScriptedResponse {
        status,
        body: serde_json::to_string(&body).expect("serialize response body"),
    }
}

/// A recorded request captured by [`TestServer`].
#[derive(Debug, Clone)]
pub struct TcpRecordedRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// An in-process TCP server that replays scripted HTTP responses.
///
/// Each call to `accept` pops one response from the script. Requests are
/// captured for later assertion.
#[derive(Debug)]
pub struct TestServer {
    pub base_url: String,
    captured: Arc<Mutex<Vec<TcpRecordedRequest>>>,
    handle: thread::JoinHandle<()>,
}

impl TestServer {
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn requests(&self) -> Vec<TcpRecordedRequest> {
        self.captured.lock().expect("capture lock").clone()
    }

    pub fn join(self) {
        self.handle.join().expect("server thread");
    }
}

/// Look up a header value by name (case-insensitive).
pub fn header<'a>(request: &'a TcpRecordedRequest, name: &str) -> Option<&'a str> {
    request
        .headers
        .iter()
        .find(|(key, _)| key == &name.to_ascii_lowercase())
        .map(|(_, value)| value.as_str())
}

/// Spawn a TCP server that replays the given scripted responses in order.
pub fn spawn_scripted_server(script: Vec<ScriptedResponse>) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().expect("server addr");
    let captured = Arc::new(Mutex::new(Vec::new()));
    let captured_thread = Arc::clone(&captured);

    let handle = thread::spawn(move || {
        for exchange in script {
            let (mut stream, _) = listener.accept().expect("accept request");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("set read timeout");

            let request = read_request(&mut stream);
            captured_thread.lock().expect("capture lock").push(request);
            write_response(&mut stream, exchange.status, &exchange.body);
        }
    });

    TestServer {
        base_url: format!("http://{addr}"),
        captured,
        handle,
    }
}

fn read_request(stream: &mut TcpStream) -> TcpRecordedRequest {
    let mut buf = Vec::new();
    let mut header_end = None;
    let mut content_length = 0usize;

    loop {
        let mut chunk = [0_u8; 1024];
        let n = stream.read(&mut chunk).expect("read request");
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);

        if header_end.is_none()
            && let Some(pos) = buf.windows(4).position(|window| window == b"\r\n\r\n")
        {
            header_end = Some(pos + 4);
            let headers = String::from_utf8_lossy(&buf[..pos + 4]);
            content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
        }

        if let Some(header_end) = header_end
            && buf.len() >= header_end + content_length
        {
            break;
        }
    }

    let header_end = header_end.unwrap_or(buf.len());
    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let body_len = content_length.min(buf.len().saturating_sub(header_end));
    let body = String::from_utf8_lossy(&buf[header_end..header_end + body_len]).to_string();

    let mut lines = head.lines();
    let request_line = lines.next().expect("request line");
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default().to_string();
    let path = request_parts.next().unwrap_or_default().to_string();
    let headers = lines
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect();

    TcpRecordedRequest {
        method,
        path,
        headers,
        body,
    }
}

fn write_response(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let wire = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(wire.as_bytes()).expect("write response");
    stream.flush().expect("flush response");
}
