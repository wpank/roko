//! Resumable loop state (§36.57).
//!
//! A [`Checkpoint`] captures the tool loop's mutable state (iteration
//! count, accumulated tool calls, and conversation messages) so the
//! loop can be serialized to disk and resumed later — e.g. after a
//! crash, manual pause, or context-window rotation.

use roko_core::Result;
use roko_core::tool::ToolCall;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;

use crate::dispatcher::truncate::bounded_serialized_bytes;
use crate::translate::SessionState;

/// Absolute on-disk and in-memory serialization limit for a checkpoint.
pub const MAX_CHECKPOINT_BYTES: usize = 8 * 1024 * 1024;

/// Serializable snapshot of a [`ToolLoop`](super::ToolLoop) mid-execution.
///
/// Created by the loop when it stops for any reason other than
/// [`StopReason::Stop`](super::StopReason::Stop) (i.e. the normal
/// "final answer" path).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    /// Number of tool-call iterations completed before this snapshot.
    pub iterations: usize,
    /// All tool calls dispatched so far (across all iterations).
    pub tool_calls: Vec<ToolCall>,
    /// The full conversation message history at snapshot time.
    pub messages: Vec<serde_json::Value>,
    /// Provider-issued session identifiers required to resume the conversation.
    #[serde(default)]
    pub session: SessionState,
}

impl Checkpoint {
    /// Create a new checkpoint from the loop's current state.
    #[must_use]
    pub fn new(
        iterations: usize,
        tool_calls: Vec<ToolCall>,
        messages: Vec<serde_json::Value>,
    ) -> Self {
        Self {
            iterations,
            tool_calls,
            messages,
            session: SessionState::default(),
        }
    }

    /// Attach provider session continuity state to the checkpoint.
    #[must_use]
    pub fn with_session(mut self, session: SessionState) -> Self {
        self.session = session;
        self
    }

    /// Serialize to JSON bytes for persistence.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails or the fixed checkpoint byte
    /// budget would be exceeded.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bounded_serialized_bytes(self, MAX_CHECKPOINT_BYTES).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "tool-loop checkpoint exceeds fixed byte budget",
            )
            .into()
        })
    }

    /// Deserialize from JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes exceed the fixed budget or are not a
    /// valid exact `Checkpoint` wire shape.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_CHECKPOINT_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "tool-loop checkpoint exceeds fixed byte budget",
            )
            .into());
        }
        Ok(serde_json::from_slice(bytes)?)
    }

    /// Persist the checkpoint to disk as bounded atomic JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails, the parent directory
    /// cannot be created, or the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<()> {
        // Serialize and enforce the ceiling before touching an existing file.
        // The sibling staging file + rename keeps readers from observing a
        // partial checkpoint after a crash or concurrent read.
        let bytes = self.to_bytes()?;
        roko_fs::atomic_write_bytes(path, &bytes)?;
        Ok(())
    }

    /// Load a checkpoint from a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or the JSON is invalid.
    pub fn load(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)?;
        if file.metadata()?.len() > MAX_CHECKPOINT_BYTES as u64 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "tool-loop checkpoint exceeds fixed byte budget",
            )
            .into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_CHECKPOINT_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        Self::from_bytes(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_serde() {
        let call = ToolCall::new("c1", "echo", serde_json::json!({"x": 1}));
        let cp = Checkpoint::new(
            3,
            vec![call],
            vec![
                serde_json::json!({"role": "system", "content": "sys"}),
                serde_json::json!({"role": "user", "content": "usr"}),
            ],
        );
        let bytes = cp.to_bytes().expect("serialize");
        let recovered = Checkpoint::from_bytes(&bytes).expect("deserialize");
        assert_eq!(recovered.iterations, 3);
        assert_eq!(recovered.tool_calls.len(), 1);
        assert_eq!(recovered.tool_calls[0].name, "echo");
        assert_eq!(recovered.messages.len(), 2);
        assert_eq!(recovered.session, SessionState::default());
    }

    #[test]
    fn empty_checkpoint_round_trips() {
        let cp = Checkpoint::new(0, vec![], vec![]);
        let bytes = cp.to_bytes().expect("serialize");
        let recovered = Checkpoint::from_bytes(&bytes).expect("deserialize");
        assert_eq!(recovered.iterations, 0);
        assert!(recovered.tool_calls.is_empty());
        assert!(recovered.messages.is_empty());
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("state").join("checkpoint.json");
        let cp = Checkpoint::new(
            2,
            vec![ToolCall::new("c1", "echo", serde_json::json!({"value": 7}))],
            vec![serde_json::json!({"role": "user", "content": "resume me"})],
        );

        cp.save(&path).expect("save checkpoint");
        let loaded = Checkpoint::load(&path).expect("load checkpoint");

        assert_eq!(loaded.iterations, cp.iterations);
        assert_eq!(loaded.tool_calls.len(), 1);
        assert_eq!(loaded.tool_calls[0].name, "echo");
        assert_eq!(loaded.messages, cp.messages);
    }

    #[test]
    fn malformed_and_oversized_checkpoint_loads_fail_without_rewriting_source() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("checkpoint.json");

        let malformed = b"{\"iterations\":not-json}".to_vec();
        std::fs::write(&path, &malformed).expect("write malformed checkpoint");
        assert!(Checkpoint::load(&path).is_err());
        assert_eq!(std::fs::read(&path).expect("read malformed"), malformed);

        let oversized = vec![b'x'; MAX_CHECKPOINT_BYTES + 1];
        std::fs::write(&path, &oversized).expect("write oversized checkpoint");
        assert!(Checkpoint::load(&path).is_err());
        assert_eq!(
            std::fs::metadata(&path).expect("oversized metadata").len(),
            oversized.len() as u64
        );
        assert_eq!(
            std::fs::read(&path).expect("read oversized checkpoint"),
            oversized
        );
    }

    #[test]
    fn oversized_checkpoint_save_preserves_last_valid_atomic_snapshot() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("checkpoint.json");
        let valid = Checkpoint::new(1, vec![], vec![serde_json::json!({"role": "user"})]);
        valid.save(&path).expect("save valid checkpoint");
        let before = std::fs::read(&path).expect("read valid checkpoint");

        let oversized = Checkpoint::new(
            2,
            vec![],
            vec![serde_json::json!({"content": "x".repeat(MAX_CHECKPOINT_BYTES)})],
        );
        assert!(oversized.save(&path).is_err());
        assert_eq!(
            std::fs::read(&path).expect("read preserved checkpoint"),
            before
        );
        assert_eq!(
            Checkpoint::load(&path).expect("load preserved").iterations,
            1
        );
    }

    #[test]
    fn checkpoint_wire_rejects_unknown_fields() {
        let raw = serde_json::json!({
            "iterations": 0,
            "tool_calls": [],
            "messages": [],
            "session": {},
            "unexpected": "must fail closed",
        });
        assert!(Checkpoint::from_bytes(raw.to_string().as_bytes()).is_err());
    }
}
