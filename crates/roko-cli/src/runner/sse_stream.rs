//! SSE client for streaming remote `DashboardEvent` progress from `roko serve`.
//!
//! Connects to the `/api/events` SSE endpoint and prints structured events
//! to stderr using the same formatting as [`FormattedStderrSink`].
//!
//! The SSE protocol is trivially simple: each event is one or more `data:` lines
//! separated by a blank line. We also handle `id:` for reconnection and ignore
//! `:` comment lines (keep-alives).
//!
//! [`FormattedStderrSink`]: super::output_sink::FormattedStderrSink

use std::io::Write as _;
use std::time::Duration;

use futures::StreamExt as _;
use roko_core::dashboard_snapshot::DashboardEvent;
use roko_core::obs::LogScrubber;
use roko_core::sse::parse_sse_text;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use super::output_sink::format_dashboard_event;

/// SSE client that streams `DashboardEvent`s from a `roko serve` instance
/// and prints formatted lines to stderr.
pub struct SseStreamClient {
    /// Base URL of the roko serve instance (e.g. `http://localhost:6677`).
    url: String,
    /// Whether to emit ANSI color codes.
    color: bool,
    /// Maximum number of reconnection attempts before giving up.
    max_retries: u32,
    /// Scrubber for redacting secrets from SSE payloads before rendering.
    scrubber: LogScrubber,
}

impl SseStreamClient {
    /// Create a new SSE client pointing at the given `roko serve` base URL.
    pub fn new(url: &str, color: bool) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            color,
            max_retries: 3,
            scrubber: LogScrubber::new(),
        }
    }

    /// Stream events until cancelled or disconnected (with retries exhausted).
    ///
    /// Returns `Ok(())` on clean cancellation, `Err` on connection failure.
    pub async fn stream(&self, cancel: CancellationToken) -> anyhow::Result<()> {
        let mut attempt = 0u32;
        let mut last_event_id: Option<String> = None;

        loop {
            if cancel.is_cancelled() {
                return Ok(());
            }

            let endpoint = format!("{}/api/events", self.url);
            debug!(url = %endpoint, attempt, "connecting to SSE endpoint");

            let mut req = reqwest::Client::new().get(&endpoint);
            if let Some(ref id) = last_event_id {
                req = req.header("Last-Event-ID", id.as_str());
            }

            let response = match req.send().await {
                Ok(resp) if resp.status().is_success() => {
                    attempt = 0; // Reset on successful connect.
                    resp
                }
                Ok(resp) => {
                    let status = resp.status();
                    warn!(status = %status, "SSE endpoint returned non-success status");
                    attempt += 1;
                    if attempt > self.max_retries {
                        anyhow::bail!(
                            "SSE connection failed after {} retries (last status: {status})",
                            self.max_retries
                        );
                    }
                    let backoff = exponential_backoff(attempt);
                    tokio::select! {
                        _ = tokio::time::sleep(backoff) => continue,
                        _ = cancel.cancelled() => return Ok(()),
                    }
                }
                Err(err) => {
                    warn!(error = %err, "SSE connection error");
                    attempt += 1;
                    if attempt > self.max_retries {
                        anyhow::bail!(
                            "SSE connection failed after {} retries: {err}",
                            self.max_retries
                        );
                    }
                    let backoff = exponential_backoff(attempt);
                    tokio::select! {
                        _ = tokio::time::sleep(backoff) => continue,
                        _ = cancel.cancelled() => return Ok(()),
                    }
                }
            };

            // Stream the response body as bytes, accumulating complete SSE
            // frames and dispatching them via the shared parser.
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            loop {
                let chunk = tokio::select! {
                    chunk = stream.next() => chunk,
                    _ = cancel.cancelled() => return Ok(()),
                };

                let Some(chunk_result) = chunk else {
                    // Stream ended; try reconnecting.
                    debug!("SSE stream ended, will attempt reconnect");
                    break;
                };

                let bytes = match chunk_result {
                    Ok(b) => b,
                    Err(err) => {
                        warn!(error = %err, "SSE read error");
                        break;
                    }
                };

                buffer.push_str(&String::from_utf8_lossy(&bytes));

                // Consume all complete SSE frames (terminated by a blank line)
                // from the buffer, leaving any partial frame for the next chunk.
                while let Some(frame_end) = find_sse_frame_end(&buffer) {
                    let frame_text = buffer[..frame_end].to_string();
                    buffer = buffer[frame_end..].to_string();

                    for frame in parse_sse_text(&frame_text) {
                        if let Some(ref id) = frame.id {
                            last_event_id = Some(id.clone());
                        }
                        self.handle_sse_data(&frame.data);
                    }
                }
            }

            // If we got here, the stream ended. Try reconnecting.
            attempt += 1;
            if attempt > self.max_retries {
                anyhow::bail!(
                    "SSE stream disconnected after {} reconnection attempts",
                    self.max_retries
                );
            }
            let backoff = exponential_backoff(attempt);
            debug!(
                attempt,
                backoff_ms = backoff.as_millis(),
                "reconnecting after stream end"
            );
            tokio::select! {
                _ = tokio::time::sleep(backoff) => {},
                _ = cancel.cancelled() => return Ok(()),
            }
        }
    }

    /// Parse a `data:` payload as a `DashboardEvent` and print it.
    ///
    /// Scrubs secrets from the raw payload before deserialisation so that even
    /// if the server-side scrubbing is bypassed the CLI never renders secrets.
    fn handle_sse_data(&self, data: &str) {
        let trimmed = data.trim();
        if trimmed.is_empty() {
            return;
        }

        let scrubbed = self.scrubber.scrub(trimmed);
        match serde_json::from_str::<DashboardEvent>(&scrubbed) {
            Ok(event) => {
                if let Some(line) = format_dashboard_event(&event, self.color) {
                    let mut stderr = std::io::stderr().lock();
                    let _ = writeln!(stderr, "{line}");
                }
            }
            Err(err) => {
                debug!(error = %err, data = %scrubbed, "failed to parse SSE DashboardEvent");
            }
        }
    }
}

/// Exponential backoff: 1s, 2s, 4s, capped at 8s.
fn exponential_backoff(attempt: u32) -> Duration {
    let secs = (1u64 << attempt.min(3)).min(8);
    Duration::from_secs(secs)
}

/// Find the byte offset just past the end of the first complete SSE frame
/// (terminated by a blank line `\n\n` or `\r\n\r\n`) in `buf`.
///
/// Returns `None` when no complete frame is available yet.
fn find_sse_frame_end(buf: &str) -> Option<usize> {
    // A blank line is `\n\n` or `\r\n\r\n`; we look for the simpler `\n\n`
    // because `trim_end_matches('\r')` is applied per-line by `str::lines()`.
    buf.find("\n\n").map(|pos| pos + 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::sse::parse_sse_text;

    #[test]
    fn parse_single_frame() {
        let input = "id: 1\ndata: {\"type\":\"plan_started\",\"plan_id\":\"p1\"}\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].id.as_deref(), Some("1"));
        assert!(frames[0].data.contains("plan_started"));
    }

    #[test]
    fn parse_multiple_frames() {
        let input = concat!(
            "id: 1\n",
            "data: {\"type\":\"plan_started\",\"plan_id\":\"p1\"}\n",
            "\n",
            "id: 2\n",
            "data: {\"type\":\"plan_completed\",\"plan_id\":\"p1\",\"success\":true}\n",
            "\n",
        );
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].id.as_deref(), Some("1"));
        assert_eq!(frames[1].id.as_deref(), Some("2"));
    }

    #[test]
    fn parse_multiline_data() {
        let input = "data: line1\ndata: line2\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].data, "line1\nline2");
    }

    #[test]
    fn parse_ignores_comments() {
        let input = ": keepalive\ndata: {\"type\":\"error\",\"message\":\"oops\"}\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 1);
        assert!(frames[0].data.contains("error"));
    }

    #[test]
    fn parse_no_id() {
        let input = "data: hello\n\n";
        let frames = parse_sse_text(input);
        assert_eq!(frames.len(), 1);
        assert!(frames[0].id.is_none());
        assert_eq!(frames[0].data, "hello");
    }

    #[test]
    fn parse_invalid_json_does_not_panic() {
        let client = SseStreamClient::new("http://localhost:6677", false);
        // This should not panic, just log a debug warning.
        client.handle_sse_data("not valid json");
    }

    #[test]
    fn parse_valid_event_formats() {
        let client = SseStreamClient::new("http://localhost:6677", false);
        // Should not panic and should produce output on stderr.
        client.handle_sse_data(r#"{"type":"plan_started","plan_id":"test-plan"}"#);
    }

    #[test]
    fn exponential_backoff_values() {
        assert_eq!(exponential_backoff(0), Duration::from_secs(1));
        assert_eq!(exponential_backoff(1), Duration::from_secs(2));
        assert_eq!(exponential_backoff(2), Duration::from_secs(4));
        assert_eq!(exponential_backoff(3), Duration::from_secs(8));
        assert_eq!(exponential_backoff(4), Duration::from_secs(8)); // capped
        assert_eq!(exponential_backoff(100), Duration::from_secs(8)); // capped
    }

    #[test]
    fn parse_bulk_event_filtered() {
        let client = SseStreamClient::new("http://localhost:6677", false);
        // Bulk data events should be silently filtered (no output).
        client.handle_sse_data(r#"{"type":"cascade_router_updated","snapshot_json":"{}"}"#);
    }
}
