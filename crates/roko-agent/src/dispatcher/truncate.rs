//! Strict tool-result budgeting across content, errors, and artifacts.

use std::io::{self, Write};
use std::path::PathBuf;

use roko_core::Body;
use roko_core::tool::{Artifact, ToolError, ToolResult};
use serde::Serialize;

/// Marker appended when enough budget remains.
pub const TRUNCATION_MARKER: &str = "\n...[truncated]";
/// Maximum artifact count retained from one handler result.
pub const MAX_RESULT_ARTIFACTS: usize = 16;
/// Maximum bytes retained for an artifact name or MIME label.
pub const MAX_ARTIFACT_LABEL_BYTES: usize = 256;
/// Absolute aggregate ceiling even when a caller requests a larger context cap.
pub const MAX_TOOL_RESULT_BYTES: usize = 4 * 1024 * 1024;

/// Bound every attacker-influenced portion of a tool result.
///
/// The supplied byte budget is shared by primary content, typed error text,
/// artifact labels, and artifact bodies. No returned string or byte body can
/// exceed it, and at most [`MAX_RESULT_ARTIFACTS`] artifacts survive.
#[must_use]
pub fn truncate_result(result: ToolResult, max_bytes: usize) -> ToolResult {
    let max_bytes = max_bytes.min(MAX_TOOL_RESULT_BYTES);
    match result {
        ToolResult::Ok {
            content,
            is_structured,
            artifacts,
        } => truncate_ok(content, is_structured, artifacts, max_bytes),
        ToolResult::Err(error) => ToolResult::Err(truncate_error(error, max_bytes)),
    }
}

fn truncate_ok(
    content: String,
    is_structured: bool,
    artifacts: Vec<Artifact>,
    max_bytes: usize,
) -> ToolResult {
    let mut remaining = max_bytes;
    let content = take_text(content, remaining);
    remaining = remaining.saturating_sub(content.len());
    let mut retained = Vec::with_capacity(artifacts.len().min(MAX_RESULT_ARTIFACTS));
    for artifact in artifacts.into_iter().take(MAX_RESULT_ARTIFACTS) {
        if remaining == 0 {
            break;
        }
        let name_budget = remaining.min(MAX_ARTIFACT_LABEL_BYTES);
        let name = take_text(artifact.name, name_budget);
        remaining = remaining.saturating_sub(name.len());
        let mime_budget = remaining.min(MAX_ARTIFACT_LABEL_BYTES);
        let mime_type = take_text(artifact.mime_type, mime_budget);
        remaining = remaining.saturating_sub(mime_type.len());
        let (body, body_bytes) = take_body(artifact.body, remaining);
        remaining = remaining.saturating_sub(body_bytes);
        retained.push(Artifact::new(name, mime_type, body));
    }
    ToolResult::Ok {
        content,
        is_structured,
        artifacts: retained,
    }
}

fn truncate_error(error: ToolError, max_bytes: usize) -> ToolError {
    match error {
        ToolError::PermissionDenied(message) => {
            ToolError::PermissionDenied(take_text(message, max_bytes))
        }
        ToolError::SchemaInvalid(message) => {
            ToolError::SchemaInvalid(take_text(message, max_bytes))
        }
        ToolError::HandlerPanic(message) => ToolError::HandlerPanic(take_text(message, max_bytes)),
        ToolError::PathOutsideWorktree(path) => ToolError::PathOutsideWorktree(PathBuf::from(
            take_text(path.to_string_lossy().into_owned(), max_bytes),
        )),
        ToolError::CommandNotAllowed(message) => {
            ToolError::CommandNotAllowed(take_text(message, max_bytes))
        }
        ToolError::NetworkBlocked(message) => {
            ToolError::NetworkBlocked(take_text(message, max_bytes))
        }
        ToolError::Other(message) => ToolError::Other(take_text(message, max_bytes)),
        ToolError::Timeout { after_ms } => ToolError::Timeout { after_ms },
        ToolError::Cancelled => ToolError::Cancelled,
        other => ToolError::Other(take_text(other.to_string(), max_bytes)),
    }
}

fn take_body(body: Body, max_bytes: usize) -> (Body, usize) {
    match body {
        Body::Empty => (Body::Empty, 0),
        Body::Text(text) => {
            let text = take_text(text, max_bytes);
            let size = text.len();
            (Body::Text(text), size)
        }
        Body::Bytes(mut bytes) => {
            bytes.truncate(max_bytes);
            let size = bytes.len();
            (Body::Bytes(bytes), size)
        }
        Body::Json(value) => match bounded_json_bytes(&value, max_bytes) {
            Ok(encoded) => {
                let size = encoded.len();
                (Body::Json(value), size)
            }
            Err(prefix) => {
                let mut truncated = String::from_utf8_lossy(&prefix).into_owned();
                truncated.push_str(TRUNCATION_MARKER);
                let text = take_text(truncated, max_bytes);
                let size = text.len();
                (Body::Text(text), size)
            }
        },
    }
}

/// Serialize JSON into a fixed-capacity sink that aborts as soon as the
/// canonical representation exceeds `max_bytes`. This avoids allocating an
/// unbounded intermediate `String` merely to discover that an artifact or
/// audit payload is too large.
pub(crate) fn bounded_json_bytes(
    value: &serde_json::Value,
    max_bytes: usize,
) -> Result<Vec<u8>, Vec<u8>> {
    bounded_serialized_bytes(value, max_bytes)
}

/// Serialize any persisted value without allocating beyond `max_bytes`.
pub(crate) fn bounded_serialized_bytes<T: Serialize + ?Sized>(
    value: &T,
    max_bytes: usize,
) -> Result<Vec<u8>, Vec<u8>> {
    let mut writer = BoundedWriter::new(max_bytes);
    match serde_json::to_writer(&mut writer, value) {
        Ok(()) => Ok(writer.bytes),
        Err(_) if writer.exceeded => Err(writer.bytes),
        Err(_) => Err(Vec::new()),
    }
}

struct BoundedWriter {
    bytes: Vec<u8>,
    max_bytes: usize,
    exceeded: bool,
}

impl BoundedWriter {
    fn new(max_bytes: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(max_bytes.min(64 * 1024)),
            max_bytes,
            exceeded: false,
        }
    }
}

impl Write for BoundedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let remaining = self.max_bytes.saturating_sub(self.bytes.len());
        if buffer.len() > remaining {
            self.bytes.extend_from_slice(&buffer[..remaining]);
            self.exceeded = true;
            return Err(io::Error::other("JSON byte budget exceeded"));
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn take_text(mut text: String, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text;
    }
    if max_bytes == 0 {
        return String::new();
    }
    let marker = if max_bytes >= TRUNCATION_MARKER.len() {
        TRUNCATION_MARKER
    } else {
        ""
    };
    let mut cut = max_bytes.saturating_sub(marker.len()).min(text.len());
    while cut > 0 && !text.is_char_boundary(cut) {
        cut -= 1;
    }
    text.truncate(cut);
    text.push_str(marker);
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_remote_error_text_without_erasing_type() {
        let output = truncate_result(ToolResult::err(ToolError::Other("x".repeat(100))), 24);
        assert!(matches!(
            output,
            ToolResult::Err(ToolError::Other(message))
                if message.len() <= 24 && message.contains("[truncated]")
        ));
    }

    #[test]
    fn result_budget_covers_artifact_count_labels_and_bodies() {
        let artifacts = (0..32)
            .map(|index| {
                Artifact::new(
                    format!("{}-{index}", "n".repeat(500)),
                    "m".repeat(500),
                    Body::text("payload".repeat(500)),
                )
            })
            .collect();
        let output = truncate_result(ToolResult::with_artifacts("main", artifacts), 1_024);
        let ToolResult::Ok {
            content, artifacts, ..
        } = output
        else {
            panic!("expected successful result");
        };
        assert!(artifacts.len() <= MAX_RESULT_ARTIFACTS);
        assert!(artifacts.iter().all(|artifact| {
            artifact.name.len() <= MAX_ARTIFACT_LABEL_BYTES
                && artifact.mime_type.len() <= MAX_ARTIFACT_LABEL_BYTES
        }));
        let total = content.len()
            + artifacts
                .iter()
                .map(|artifact| {
                    artifact.name.len() + artifact.mime_type.len() + artifact.body.byte_size()
                })
                .sum::<usize>();
        assert!(total <= 1_024);
    }

    #[test]
    fn truncate_preserves_utf8_and_strict_total_cap() {
        let output = truncate_result(ToolResult::text(format!("日本語{}", "x".repeat(200))), 20);
        let ToolResult::Ok { content, .. } = output else {
            panic!("expected successful result");
        };
        assert!(content.len() <= 20);
        assert!(content.is_char_boundary(content.len()));
        assert!(content.contains(TRUNCATION_MARKER));
    }

    #[test]
    fn json_artifact_serialization_aborts_at_the_byte_budget() {
        let artifact = Artifact::new(
            "data.json",
            "application/json",
            Body::Json(serde_json::json!({ "payload": "x".repeat(2_000_000) })),
        );
        let output = truncate_result(ToolResult::with_artifacts("", vec![artifact]), 512);
        let ToolResult::Ok { artifacts, .. } = output else {
            panic!("expected successful result")
        };
        assert_eq!(artifacts.len(), 1);
        assert!(matches!(&artifacts[0].body, Body::Text(_)));
        let Body::Text(body) = &artifacts[0].body else {
            unreachable!()
        };
        assert!(body.len() <= 512);
        assert!(body.contains(TRUNCATION_MARKER));
    }
}
