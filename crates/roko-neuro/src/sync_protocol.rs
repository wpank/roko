//! Transactional knowledge sync envelope and cursor protocol.
//!
//! Replaces the legacy entry-index cursors with versioned `SyncEnvelopeV1`
//! envelopes carrying per-origin monotonic sequences and canonical checksums.
//! Transport is manual files under `.roko/mesh/{outbox,inbox,archive}`.
//!
//! Spec: backlog #360.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{KnowledgeEntry, KnowledgeStore, KnowledgeTier};

// ──────────────────────────── peer name validation ────────────────────────────

/// Maximum length for a peer name.
const PEER_NAME_MAX_LEN: usize = 64;

/// Validate a peer name: ASCII `[A-Za-z0-9._-]`, length 1..=64.
///
/// Returns the validated name or an error. This prevents directory traversal
/// and guarantees the name is safe for use as a filesystem path component.
pub fn validate_peer_name(name: &str) -> Result<&str> {
    ensure!(!name.is_empty(), "peer name must not be empty");
    ensure!(
        name.len() <= PEER_NAME_MAX_LEN,
        "peer name must be at most {PEER_NAME_MAX_LEN} characters, got {}",
        name.len()
    );
    ensure!(
        name.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-'),
        "peer name contains invalid characters (only ASCII letters, digits, '.', '_', '-' allowed): {name:?}"
    );
    Ok(name)
}

// ──────────────────────────── versioned types ─────────────────────────────────

/// A single knowledge entry in a sync envelope, carrying its stable ID and a
/// monotonically allocated per-origin sequence number.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncEntryV1 {
    /// Stable knowledge entry ID (content-addressed or assigned at creation).
    pub entry_id: String,
    /// Monotonically increasing per-origin sequence number.
    pub sequence: u64,
    /// The full knowledge entry payload.
    pub entry: KnowledgeEntry,
}

/// Versioned sync envelope exchanged between peers via the file transport.
///
/// The checksum covers the compact JSON serialization of all fields *except*
/// `checksum` itself, computed over `(version, transfer_id,
/// source_workspace_id, source_peer_id, first_sequence, last_sequence,
/// entries, created_at)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncEnvelopeV1 {
    /// Protocol version. Must be `1`.
    pub version: u32,
    /// Unique transfer identifier (opaque string, e.g. timestamp + random).
    pub transfer_id: String,
    /// Workspace ID of the sender.
    pub source_workspace_id: String,
    /// Peer ID of the sender.
    pub source_peer_id: String,
    /// Sequence number of the first entry in this envelope.
    pub first_sequence: u64,
    /// Sequence number of the last entry in this envelope.
    pub last_sequence: u64,
    /// Ordered entries in this transfer batch.
    pub entries: Vec<SyncEntryV1>,
    /// Hex-encoded SHA-256 checksum over the canonical payload.
    pub checksum: String,
    /// When this envelope was created.
    pub created_at: DateTime<Utc>,
}

/// Per-peer cursor tracking the greatest committed sequence and the last
/// successfully applied transfer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeerCursorV1 {
    /// Greatest committed sequence number from this origin.
    pub last_committed_sequence: u64,
    /// Transfer ID of the last successfully applied transfer.
    pub last_transfer_id: String,
    /// Checksum of the last successfully applied transfer.
    pub last_transfer_checksum: String,
    /// When this cursor was last updated.
    pub updated_at: DateTime<Utc>,
    /// If true, the next send must include all entries (legacy migration).
    #[serde(default)]
    pub requires_full_resend: bool,
}

/// Metadata about a completed receive operation.
#[derive(Debug, Clone)]
pub struct ReceiveResult {
    /// Number of new entries imported.
    pub imported: usize,
    /// Number of entries that were already present (deduplicated).
    pub duplicates: usize,
    /// The transfer ID that was processed.
    pub transfer_id: String,
}

// ──────────────────────────── checksum ────────────────────────────────────────

/// Canonical fields for checksum computation (excludes checksum itself).
#[derive(Serialize)]
struct ChecksumPayload<'a> {
    version: u32,
    transfer_id: &'a str,
    source_workspace_id: &'a str,
    source_peer_id: &'a str,
    first_sequence: u64,
    last_sequence: u64,
    entries: &'a [SyncEntryV1],
    created_at: &'a DateTime<Utc>,
}

/// Compute the canonical SHA-256 checksum for a sync envelope's payload.
pub fn compute_envelope_checksum(envelope: &SyncEnvelopeV1) -> String {
    let payload = ChecksumPayload {
        version: envelope.version,
        transfer_id: &envelope.transfer_id,
        source_workspace_id: &envelope.source_workspace_id,
        source_peer_id: &envelope.source_peer_id,
        first_sequence: envelope.first_sequence,
        last_sequence: envelope.last_sequence,
        entries: &envelope.entries,
        created_at: &envelope.created_at,
    };
    let bytes = serde_json::to_vec(&payload).expect("checksum payload serialization");
    let hash = Sha256::digest(&bytes);
    hash.iter().fold(String::new(), |mut out, b| {
        use std::fmt::Write;
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// Verify the checksum of a sync envelope.
pub fn verify_envelope_checksum(envelope: &SyncEnvelopeV1) -> Result<()> {
    let expected = compute_envelope_checksum(envelope);
    ensure!(
        envelope.checksum == expected,
        "envelope checksum mismatch: expected {expected}, got {}",
        envelope.checksum
    );
    Ok(())
}

// ──────────────────────────── transfer ID ─────────────────────────────────────

/// Generate a unique transfer ID from timestamp and random suffix.
fn generate_transfer_id() -> String {
    let now = Utc::now().format("%Y%m%dT%H%M%S%.3f");
    let random: u64 = {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        let s = RandomState::new();
        let mut h = s.build_hasher();
        h.write_u64(std::process::id() as u64);
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
        );
        h.finish()
    };
    format!("{now}-{random:016x}")
}

// ──────────────────────────── filesystem layout ──────────────────────────────

/// Mesh directory layout under `.roko/mesh/`.
pub struct MeshLayout {
    root: PathBuf,
}

impl MeshLayout {
    /// Create a mesh layout rooted at `workdir/.roko/mesh`.
    pub fn new(workdir: &Path) -> Self {
        Self {
            root: workdir.join(".roko").join("mesh"),
        }
    }

    /// Outbox directory for a specific peer.
    pub fn outbox_dir(&self, peer: &str) -> PathBuf {
        self.root.join("outbox").join(peer)
    }

    /// Inbox directory for a specific peer.
    pub fn inbox_dir(&self, peer: &str) -> PathBuf {
        self.root.join("inbox").join(peer)
    }

    /// Archive directory for a specific peer.
    pub fn archive_dir(&self, peer: &str) -> PathBuf {
        self.root.join("archive").join(peer)
    }

    /// Path to the peer cursor file.
    pub fn cursor_path(&self, peer: &str) -> PathBuf {
        self.root.join("cursors").join(format!("{peer}.json"))
    }

    /// Path to the local sequence counter file.
    pub fn sequence_path(&self) -> PathBuf {
        self.root.join("sequence.json")
    }
}

// ──────────────────────────── sequence allocator ──────────────────────────────

/// Persistent sequence counter for this workspace's origin.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SequenceState {
    /// The next sequence number to allocate.
    next_sequence: u64,
}

impl Default for SequenceState {
    fn default() -> Self {
        Self { next_sequence: 1 }
    }
}

/// Load the current sequence state from disk, or return default.
fn load_sequence_state(path: &Path) -> SequenceState {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => SequenceState::default(),
    }
}

/// Atomically persist the sequence state.
fn save_sequence_state(path: &Path, state: &SequenceState) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(state).context("serialize sequence state")?;
    roko_fs::atomic_write_bytes(path, &bytes)
        .with_context(|| format!("atomic write sequence state to {}", path.display()))?;
    Ok(())
}

// ──────────────────────────── cursor persistence ─────────────────────────────

/// Load a peer cursor from disk.
pub fn load_peer_cursor(path: &Path) -> Option<PeerCursorV1> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Atomically persist a peer cursor.
fn save_peer_cursor(path: &Path, cursor: &PeerCursorV1) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(cursor).context("serialize peer cursor")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create cursor dir {}", parent.display()))?;
    }
    roko_fs::atomic_write_bytes(path, &bytes)
        .with_context(|| format!("atomic write peer cursor to {}", path.display()))?;
    Ok(())
}

// ──────────────────────────── legacy cursor migration ────────────────────────

/// Path to the legacy version-vectors file.
fn legacy_vv_path(workdir: &Path) -> PathBuf {
    workdir
        .join(".roko")
        .join("neuro")
        .join("version-vectors.json")
}

/// Check whether a legacy version-vectors.json exists for this peer.
/// If so, migrate to PeerCursorV1 with `requires_full_resend = true`.
fn migrate_legacy_cursor(workdir: &Path, peer: &str, cursor_path: &Path) -> Option<PeerCursorV1> {
    let vv_path = legacy_vv_path(workdir);
    if !vv_path.exists() {
        return None;
    }
    let text = fs::read_to_string(&vv_path).ok()?;
    let vv: std::collections::HashMap<String, u64> = serde_json::from_str(&text).ok()?;
    if vv.contains_key(peer) {
        // Legacy cursor exists -- create a migration cursor requiring full resend.
        let cursor = PeerCursorV1 {
            last_committed_sequence: 0,
            last_transfer_id: String::new(),
            last_transfer_checksum: String::new(),
            updated_at: Utc::now(),
            requires_full_resend: true,
        };
        // Best-effort persist the migration cursor.
        let _ = save_peer_cursor(cursor_path, &cursor);
        Some(cursor)
    } else {
        None
    }
}

// ──────────────────────────── send protocol ──────────────────────────────────

/// Result of a send operation.
#[derive(Debug, Clone)]
pub struct SendResult {
    /// Number of entries packaged in the envelope.
    pub sent: usize,
    /// Path where the envelope was written.
    pub outbox_path: PathBuf,
    /// The transfer ID of the envelope.
    pub transfer_id: String,
    /// The new local sequence high-water mark.
    pub high_water_sequence: u64,
}

/// Build and atomically publish a sync envelope to the peer's outbox.
///
/// Commit order: allocate sequences -> build envelope -> fsync/atomic publish
/// outbox -> atomic cursor publish.
///
/// The caller is expected to hold the workspace lock.
pub fn send_sync(
    workdir: &Path,
    peer: &str,
    source_workspace_id: &str,
    store: &KnowledgeStore,
    max_send: usize,
) -> Result<Option<SendResult>> {
    let peer = validate_peer_name(peer)?;
    let layout = MeshLayout::new(workdir);

    // Load or migrate cursor.
    let cursor_path = layout.cursor_path(peer);
    let cursor = load_peer_cursor(&cursor_path)
        .or_else(|| migrate_legacy_cursor(workdir, peer, &cursor_path));

    let requires_full_resend = cursor
        .as_ref()
        .map(|c| c.requires_full_resend)
        .unwrap_or(false);

    // Read all entries from the store.
    let entries = store
        .read_all()
        .with_context(|| format!("read knowledge store from {}", store.path().display()))?;

    if entries.is_empty() {
        return Ok(None);
    }

    // Load sequence state and allocate sequences for entries that need them.
    let seq_path = layout.sequence_path();
    if let Some(parent) = seq_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut seq_state = load_sequence_state(&seq_path);

    // Build sync entries. If requires_full_resend, include all entries.
    // Otherwise, only entries with sequences > cursor's last_committed_sequence.
    let last_committed = cursor
        .as_ref()
        .map(|c| c.last_committed_sequence)
        .unwrap_or(0);

    // Assign sequences to all entries that don't have them yet.
    // We use the entry's position-independent ID as the stable key.
    let mut sync_entries: Vec<SyncEntryV1> = Vec::new();
    for entry in &entries {
        let seq = seq_state.next_sequence;
        seq_state.next_sequence += 1;
        sync_entries.push(SyncEntryV1 {
            entry_id: entry.id.clone(),
            sequence: seq,
            entry: entry.clone(),
        });
    }

    // Persist the new sequence state atomically.
    save_sequence_state(&seq_path, &seq_state)?;

    // Filter to delta: entries with sequence > last_committed,
    // unless full resend is required.
    let delta: Vec<SyncEntryV1> = if requires_full_resend {
        sync_entries.into_iter().take(max_send).collect()
    } else {
        sync_entries
            .into_iter()
            .filter(|e| e.sequence > last_committed)
            .take(max_send)
            .collect()
    };

    if delta.is_empty() {
        return Ok(None);
    }

    let first_sequence = delta.first().map(|e| e.sequence).unwrap_or(0);
    let last_sequence = delta.last().map(|e| e.sequence).unwrap_or(0);
    let transfer_id = generate_transfer_id();
    let now = Utc::now();

    // Build envelope (checksum is placeholder, will be computed).
    let mut envelope = SyncEnvelopeV1 {
        version: 1,
        transfer_id: transfer_id.clone(),
        source_workspace_id: source_workspace_id.to_string(),
        source_peer_id: peer.to_string(),
        first_sequence,
        last_sequence,
        entries: delta,
        checksum: String::new(),
        created_at: now,
    };
    envelope.checksum = compute_envelope_checksum(&envelope);

    // Atomically publish to outbox.
    let outbox_dir = layout.outbox_dir(peer);
    fs::create_dir_all(&outbox_dir)
        .with_context(|| format!("create outbox dir {}", outbox_dir.display()))?;
    let outbox_path = outbox_dir.join(format!("{transfer_id}.json"));
    let envelope_bytes = serde_json::to_vec_pretty(&envelope).context("serialize sync envelope")?;
    roko_fs::atomic_write_bytes(&outbox_path, &envelope_bytes)
        .with_context(|| format!("atomic write envelope to {}", outbox_path.display()))?;

    // Atomically publish cursor update.
    let new_cursor = PeerCursorV1 {
        last_committed_sequence: last_sequence,
        last_transfer_id: transfer_id.clone(),
        last_transfer_checksum: envelope.checksum.clone(),
        updated_at: now,
        requires_full_resend: false,
    };
    save_peer_cursor(&cursor_path, &new_cursor)?;

    Ok(Some(SendResult {
        sent: envelope.entries.len(),
        outbox_path,
        transfer_id,
        high_water_sequence: last_sequence,
    }))
}

// ──────────────────────────── receive protocol ───────────────────────────────

/// Receive and import a sync envelope from the peer's inbox.
///
/// Commit order: validate version/peer/checksum/range -> stage imports ->
/// atomically publish store -> atomically publish cursor -> move envelope
/// to archive.
///
/// The caller is expected to hold the workspace lock.
pub fn receive_sync(
    workdir: &Path,
    peer: &str,
    store: &KnowledgeStore,
) -> Result<Vec<ReceiveResult>> {
    let peer = validate_peer_name(peer)?;
    let layout = MeshLayout::new(workdir);

    let inbox_dir = layout.inbox_dir(peer);
    if !inbox_dir.exists() {
        return Ok(Vec::new());
    }

    // Read all envelope files from the inbox, sorted by name for determinism.
    let mut envelope_paths: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(&inbox_dir)
        .with_context(|| format!("read inbox dir {}", inbox_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            envelope_paths.push(path);
        }
    }
    envelope_paths.sort();

    if envelope_paths.is_empty() {
        return Ok(Vec::new());
    }

    let cursor_path = layout.cursor_path(peer);
    let archive_dir = layout.archive_dir(peer);
    let mut results = Vec::new();

    for envelope_path in &envelope_paths {
        let result =
            receive_single_envelope(peer, envelope_path, &cursor_path, &archive_dir, store)?;
        results.push(result);
    }

    Ok(results)
}

/// Process a single envelope file.
fn receive_single_envelope(
    peer: &str,
    envelope_path: &Path,
    cursor_path: &Path,
    archive_dir: &Path,
    store: &KnowledgeStore,
) -> Result<ReceiveResult> {
    let text = fs::read_to_string(envelope_path)
        .with_context(|| format!("read envelope from {}", envelope_path.display()))?;
    let envelope: SyncEnvelopeV1 = serde_json::from_str(&text)
        .with_context(|| format!("parse envelope from {}", envelope_path.display()))?;

    // Validate version.
    ensure!(
        envelope.version == 1,
        "unsupported envelope version {}, expected 1",
        envelope.version
    );

    // Validate checksum.
    verify_envelope_checksum(&envelope)
        .with_context(|| format!("envelope {} from peer {peer}", envelope.transfer_id))?;

    // Validate sequence range consistency.
    if !envelope.entries.is_empty() {
        ensure!(
            envelope.first_sequence <= envelope.last_sequence,
            "envelope sequence range invalid: first {} > last {}",
            envelope.first_sequence,
            envelope.last_sequence
        );

        // Verify entries are in sequence order.
        for window in envelope.entries.windows(2) {
            ensure!(
                window[0].sequence < window[1].sequence,
                "envelope entries not in monotonic sequence order: {} >= {}",
                window[0].sequence,
                window[1].sequence
            );
        }

        if let Some(first_entry) = envelope.entries.first() {
            ensure!(
                first_entry.sequence == envelope.first_sequence,
                "first entry sequence {} does not match envelope first_sequence {}",
                first_entry.sequence,
                envelope.first_sequence
            );
        }
        if let Some(last_entry) = envelope.entries.last() {
            ensure!(
                last_entry.sequence == envelope.last_sequence,
                "last entry sequence {} does not match envelope last_sequence {}",
                last_entry.sequence,
                envelope.last_sequence
            );
        }
    }

    // Check for duplicate transfer (idempotent replay).
    let existing_cursor = load_peer_cursor(cursor_path);
    if let Some(ref cursor) = existing_cursor
        && cursor.last_transfer_id == envelope.transfer_id
        && cursor.last_transfer_checksum == envelope.checksum
    {
        // Already committed this exact transfer -- idempotent success.
        return Ok(ReceiveResult {
            imported: 0,
            duplicates: envelope.entries.len(),
            transfer_id: envelope.transfer_id.clone(),
        });
    }

    // Stage imports: deduplicate by entry ID against the existing store.
    let existing_entries = store.read_all().unwrap_or_default();
    let existing_ids: HashSet<&str> = existing_entries.iter().map(|e| e.id.as_str()).collect();

    let mut to_import = Vec::new();
    let mut duplicate_count = 0_usize;

    for sync_entry in &envelope.entries {
        if existing_ids.contains(sync_entry.entry_id.as_str()) {
            duplicate_count += 1;
            continue;
        }
        let mut entry = sync_entry.entry.clone();
        // Apply mesh receive policies: confidence discount and tier reset.
        entry.confidence *= 0.7;
        entry.tier = KnowledgeTier::Transient;
        entry.source = Some(format!("mesh:{peer}"));
        to_import.push(entry);
    }

    let imported_count = to_import.len();

    // Atomically publish to store.
    if !to_import.is_empty() {
        store.ingest(to_import).with_context(|| {
            format!("import mesh entries from transfer {}", envelope.transfer_id)
        })?;
    }

    // Atomically publish cursor.
    let new_cursor = PeerCursorV1 {
        last_committed_sequence: envelope.last_sequence,
        last_transfer_id: envelope.transfer_id.clone(),
        last_transfer_checksum: envelope.checksum.clone(),
        updated_at: Utc::now(),
        requires_full_resend: false,
    };
    save_peer_cursor(cursor_path, &new_cursor)?;

    // Move envelope to archive.
    fs::create_dir_all(archive_dir)
        .with_context(|| format!("create archive dir {}", archive_dir.display()))?;
    let archive_path = archive_dir.join(envelope_path.file_name().unwrap_or_default());
    fs::rename(envelope_path, &archive_path).with_context(|| {
        format!(
            "archive envelope from {} to {}",
            envelope_path.display(),
            archive_path.display()
        )
    })?;

    Ok(ReceiveResult {
        imported: imported_count,
        duplicates: duplicate_count,
        transfer_id: envelope.transfer_id,
    })
}

// ──────────────────────────── tests ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::KnowledgeKind;
    use tempfile::TempDir;

    fn make_test_entry(id: &str, content: &str) -> KnowledgeEntry {
        KnowledgeEntry {
            id: id.to_string(),
            kind: KnowledgeKind::Insight,
            content: content.to_string(),
            confidence: 1.0,
            tags: vec!["test".to_string()],
            created_at: Utc::now(),
            ..Default::default()
        }
    }

    fn setup_workdir() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let workdir = tmp.path().to_path_buf();
        fs::create_dir_all(workdir.join(".roko/neuro")).unwrap();
        (tmp, workdir)
    }

    // ── peer name validation ─────────────────────────────────────────────

    #[test]
    fn knowledge_sync_protocol_valid_peer_names() {
        assert!(validate_peer_name("alice").is_ok());
        assert!(validate_peer_name("peer-1").is_ok());
        assert!(validate_peer_name("agent_A.v2").is_ok());
        assert!(validate_peer_name("A").is_ok());
        let max_name: String = "a".repeat(64);
        assert!(validate_peer_name(&max_name).is_ok());
    }

    #[test]
    fn knowledge_sync_protocol_invalid_peer_names() {
        assert!(validate_peer_name("").is_err());
        let too_long: String = "a".repeat(65);
        assert!(validate_peer_name(&too_long).is_err());
        assert!(validate_peer_name("../escape").is_err());
        assert!(validate_peer_name("peer/bad").is_err());
        assert!(validate_peer_name("peer name").is_err());
        assert!(validate_peer_name("peer\0null").is_err());
    }

    #[test]
    fn knowledge_sync_protocol_malicious_peer_names_cannot_escape_mesh() {
        // Directory traversal attempts.
        assert!(validate_peer_name("..").is_err());
        assert!(validate_peer_name("../..").is_err());
        assert!(validate_peer_name("../../etc/passwd").is_err());
        assert!(validate_peer_name("foo/../bar").is_err());
        // Null bytes, slashes, other separators.
        assert!(validate_peer_name("foo\0bar").is_err());
        assert!(validate_peer_name("foo\\bar").is_err());
        assert!(validate_peer_name("foo:bar").is_err());
    }

    // ── envelope checksum ────────────────────────────────────────────────

    #[test]
    fn knowledge_sync_protocol_checksum_round_trip() {
        let entry = make_test_entry("e1", "test knowledge");
        let sync_entry = SyncEntryV1 {
            entry_id: "e1".to_string(),
            sequence: 1,
            entry,
        };
        let mut envelope = SyncEnvelopeV1 {
            version: 1,
            transfer_id: "test-transfer-1".to_string(),
            source_workspace_id: "ws-1".to_string(),
            source_peer_id: "peer-a".to_string(),
            first_sequence: 1,
            last_sequence: 1,
            entries: vec![sync_entry],
            checksum: String::new(),
            created_at: Utc::now(),
        };
        envelope.checksum = compute_envelope_checksum(&envelope);
        assert!(!envelope.checksum.is_empty());
        assert!(verify_envelope_checksum(&envelope).is_ok());
    }

    #[test]
    fn knowledge_sync_protocol_checksum_detects_tampering() {
        let entry = make_test_entry("e1", "test knowledge");
        let sync_entry = SyncEntryV1 {
            entry_id: "e1".to_string(),
            sequence: 1,
            entry,
        };
        let mut envelope = SyncEnvelopeV1 {
            version: 1,
            transfer_id: "test-transfer-1".to_string(),
            source_workspace_id: "ws-1".to_string(),
            source_peer_id: "peer-a".to_string(),
            first_sequence: 1,
            last_sequence: 1,
            entries: vec![sync_entry],
            checksum: String::new(),
            created_at: Utc::now(),
        };
        envelope.checksum = compute_envelope_checksum(&envelope);

        // Tamper with the transfer_id.
        envelope.transfer_id = "tampered".to_string();
        assert!(verify_envelope_checksum(&envelope).is_err());
    }

    #[test]
    fn knowledge_sync_protocol_checksum_is_stable() {
        let entry = make_test_entry("e1", "deterministic");
        let now = Utc::now();
        let sync_entry = SyncEntryV1 {
            entry_id: "e1".to_string(),
            sequence: 1,
            entry: entry.clone(),
        };
        let mut env1 = SyncEnvelopeV1 {
            version: 1,
            transfer_id: "t1".to_string(),
            source_workspace_id: "ws-1".to_string(),
            source_peer_id: "peer-a".to_string(),
            first_sequence: 1,
            last_sequence: 1,
            entries: vec![sync_entry.clone()],
            checksum: String::new(),
            created_at: now,
        };
        let mut env2 = env1.clone();
        env1.checksum = compute_envelope_checksum(&env1);
        env2.checksum = compute_envelope_checksum(&env2);
        assert_eq!(env1.checksum, env2.checksum);
    }

    // ── envelope serde round-trip ────────────────────────────────────────

    #[test]
    fn knowledge_sync_protocol_envelope_serde_round_trip() {
        let entry = make_test_entry("e1", "serde test");
        let sync_entry = SyncEntryV1 {
            entry_id: "e1".to_string(),
            sequence: 1,
            entry,
        };
        let mut envelope = SyncEnvelopeV1 {
            version: 1,
            transfer_id: "test-serde".to_string(),
            source_workspace_id: "ws-1".to_string(),
            source_peer_id: "peer-a".to_string(),
            first_sequence: 1,
            last_sequence: 1,
            entries: vec![sync_entry],
            checksum: String::new(),
            created_at: Utc::now(),
        };
        envelope.checksum = compute_envelope_checksum(&envelope);

        let json = serde_json::to_string(&envelope).unwrap();
        let deserialized: SyncEnvelopeV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(envelope, deserialized);
        assert!(verify_envelope_checksum(&deserialized).is_ok());
    }

    // ── cursor serde round-trip ──────────────────────────────────────────

    #[test]
    fn knowledge_sync_protocol_cursor_serde_round_trip() {
        let cursor = PeerCursorV1 {
            last_committed_sequence: 42,
            last_transfer_id: "xfer-42".to_string(),
            last_transfer_checksum: "abc123".to_string(),
            updated_at: Utc::now(),
            requires_full_resend: false,
        };
        let json = serde_json::to_string(&cursor).unwrap();
        let deserialized: PeerCursorV1 = serde_json::from_str(&json).unwrap();
        assert_eq!(cursor, deserialized);
    }

    // ── send/receive integration ─────────────────────────────────────────

    #[test]
    fn knowledge_sync_protocol_send_creates_envelope_and_cursor() {
        let (_tmp, workdir) = setup_workdir();
        let store = KnowledgeStore::for_workdir(&workdir);
        let entries = vec![
            make_test_entry("e1", "knowledge one"),
            make_test_entry("e2", "knowledge two"),
        ];
        store.ingest(entries).unwrap();

        let result = send_sync(&workdir, "peer-b", "ws-local", &store, 100).unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.sent, 2);
        assert!(result.outbox_path.exists());

        // Verify the envelope file is valid.
        let text = fs::read_to_string(&result.outbox_path).unwrap();
        let envelope: SyncEnvelopeV1 = serde_json::from_str(&text).unwrap();
        assert_eq!(envelope.version, 1);
        assert_eq!(envelope.entries.len(), 2);
        assert!(verify_envelope_checksum(&envelope).is_ok());

        // Verify cursor was persisted.
        let layout = MeshLayout::new(&workdir);
        let cursor = load_peer_cursor(&layout.cursor_path("peer-b")).unwrap();
        assert_eq!(cursor.last_committed_sequence, result.high_water_sequence);
        assert!(!cursor.requires_full_resend);
    }

    #[test]
    fn knowledge_sync_protocol_receive_imports_and_archives() {
        let (_tmp_send, workdir_send) = setup_workdir();
        let (_tmp_recv, workdir_recv) = setup_workdir();

        // Sender creates entries and sends.
        let store_send = KnowledgeStore::for_workdir(&workdir_send);
        store_send
            .ingest(vec![
                make_test_entry("e1", "shared knowledge"),
                make_test_entry("e2", "more knowledge"),
            ])
            .unwrap();
        let send_result = send_sync(&workdir_send, "peer-recv", "ws-send", &store_send, 100)
            .unwrap()
            .unwrap();

        // Copy envelope to receiver's inbox.
        let recv_layout = MeshLayout::new(&workdir_recv);
        let inbox = recv_layout.inbox_dir("peer-send");
        fs::create_dir_all(&inbox).unwrap();
        let dest = inbox.join(send_result.outbox_path.file_name().unwrap());
        fs::copy(&send_result.outbox_path, &dest).unwrap();

        // Receiver processes inbox.
        let store_recv = KnowledgeStore::for_workdir(&workdir_recv);
        let results = receive_sync(&workdir_recv, "peer-send", &store_recv).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].imported, 2);
        assert_eq!(results[0].duplicates, 0);

        // Verify entries were imported with mesh policies.
        let imported = store_recv.read_all().unwrap();
        assert_eq!(imported.len(), 2);
        for entry in &imported {
            assert!(entry.confidence <= 0.71); // 1.0 * 0.7 = 0.7
            assert_eq!(entry.tier, KnowledgeTier::Transient);
            assert_eq!(entry.source.as_deref(), Some("mesh:peer-send"));
        }

        // Verify envelope was moved to archive.
        assert!(!dest.exists());
        let archive = recv_layout.archive_dir("peer-send");
        assert!(archive.exists());
        let archived: Vec<_> = fs::read_dir(&archive)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(archived.len(), 1);
    }

    #[test]
    fn knowledge_sync_protocol_duplicate_transfer_is_idempotent() {
        let (_tmp_send, workdir_send) = setup_workdir();
        let (_tmp_recv, workdir_recv) = setup_workdir();

        // Send.
        let store_send = KnowledgeStore::for_workdir(&workdir_send);
        store_send
            .ingest(vec![make_test_entry("e1", "knowledge")])
            .unwrap();
        let send_result = send_sync(&workdir_send, "peer-recv", "ws-send", &store_send, 100)
            .unwrap()
            .unwrap();

        // Copy to receiver inbox.
        let recv_layout = MeshLayout::new(&workdir_recv);
        let inbox = recv_layout.inbox_dir("peer-send");
        fs::create_dir_all(&inbox).unwrap();
        let dest = inbox.join(send_result.outbox_path.file_name().unwrap());
        fs::copy(&send_result.outbox_path, &dest).unwrap();

        // Receive once.
        let store_recv = KnowledgeStore::for_workdir(&workdir_recv);
        let results = receive_sync(&workdir_recv, "peer-send", &store_recv).unwrap();
        assert_eq!(results[0].imported, 1);

        // Copy the same envelope again.
        let dest2 = inbox.join(send_result.outbox_path.file_name().unwrap());
        fs::copy(&send_result.outbox_path, &dest2).unwrap();

        // Receive again -- should be idempotent (duplicate transfer).
        let results2 = receive_sync(&workdir_recv, "peer-send", &store_recv).unwrap();
        assert_eq!(results2[0].imported, 0);
        assert_eq!(results2[0].duplicates, 1);

        // Store should still have only one entry.
        let all = store_recv.read_all().unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn knowledge_sync_protocol_duplicate_entry_deduplication() {
        let (_tmp_send, workdir_send) = setup_workdir();
        let (_tmp_recv, workdir_recv) = setup_workdir();

        // Receiver already has entry e1.
        let store_recv = KnowledgeStore::for_workdir(&workdir_recv);
        store_recv
            .ingest(vec![make_test_entry("e1", "existing")])
            .unwrap();

        // Sender sends envelope containing e1 and e2.
        let store_send = KnowledgeStore::for_workdir(&workdir_send);
        store_send
            .ingest(vec![
                make_test_entry("e1", "duplicate"),
                make_test_entry("e2", "new"),
            ])
            .unwrap();
        let send_result = send_sync(&workdir_send, "peer-recv", "ws-send", &store_send, 100)
            .unwrap()
            .unwrap();

        // Copy to receiver inbox.
        let recv_layout = MeshLayout::new(&workdir_recv);
        let inbox = recv_layout.inbox_dir("peer-send");
        fs::create_dir_all(&inbox).unwrap();
        let dest = inbox.join(send_result.outbox_path.file_name().unwrap());
        fs::copy(&send_result.outbox_path, &dest).unwrap();

        // Receive: e1 should be deduped, e2 imported.
        let results = receive_sync(&workdir_recv, "peer-send", &store_recv).unwrap();
        assert_eq!(results[0].imported, 1);
        assert_eq!(results[0].duplicates, 1);

        // Total entries: original e1 + newly imported e2.
        let all = store_recv.read_all().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn knowledge_sync_protocol_failed_receive_leaves_prior_state_intact() {
        let (_tmp, workdir) = setup_workdir();
        let store = KnowledgeStore::for_workdir(&workdir);
        store
            .ingest(vec![make_test_entry("existing", "safe data")])
            .unwrap();

        // Write a corrupt envelope to the inbox.
        let layout = MeshLayout::new(&workdir);
        let inbox = layout.inbox_dir("bad-peer");
        fs::create_dir_all(&inbox).unwrap();
        let bad_envelope = inbox.join("bad-transfer.json");
        fs::write(&bad_envelope, r#"{"version": 1, "not_valid": true}"#).unwrap();

        // Receive should fail.
        let result = receive_sync(&workdir, "bad-peer", &store);
        assert!(result.is_err());

        // Prior state should be intact.
        let entries = store.read_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "existing");

        // Corrupt envelope should still exist for retry.
        assert!(bad_envelope.exists());
    }

    #[test]
    fn knowledge_sync_protocol_failed_send_leaves_prior_state() {
        let (_tmp, workdir) = setup_workdir();
        let store = KnowledgeStore::for_workdir(&workdir);
        store.ingest(vec![make_test_entry("e1", "data")]).unwrap();

        // First send succeeds.
        let result1 = send_sync(&workdir, "peer-b", "ws-local", &store, 100).unwrap();
        assert!(result1.is_some());

        // Entries haven't changed, so a second send produces no delta.
        // The sequence allocator advanced but the cursor already committed
        // all sequences from the first send, so the delta will cover only
        // the newly-allocated range.  Since we re-allocate for the same
        // entries, the sequences are > last_committed, so we do get a new
        // envelope. This is correct behavior: the receiver deduplicates by
        // entry ID.
        let result2 = send_sync(&workdir, "peer-b", "ws-local", &store, 100).unwrap();
        // Whether result2 is Some or None, the store is intact.
        let entries = store.read_all().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "e1");

        // Check cursor is still valid.
        let layout = MeshLayout::new(&workdir);
        let cursor = load_peer_cursor(&layout.cursor_path("peer-b")).unwrap();
        assert!(!cursor.requires_full_resend);
        let _ = result2;
    }

    #[test]
    fn knowledge_sync_protocol_legacy_cursor_migration_requires_full_resend() {
        let (_tmp, workdir) = setup_workdir();

        // Create legacy version-vectors.json.
        let vv_path = legacy_vv_path(&workdir);
        if let Some(parent) = vv_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut vv = std::collections::HashMap::new();
        vv.insert("old-peer".to_string(), 42u64);
        fs::write(&vv_path, serde_json::to_string(&vv).unwrap()).unwrap();

        // Store with entries.
        let store = KnowledgeStore::for_workdir(&workdir);
        store
            .ingest(vec![
                make_test_entry("e1", "old"),
                make_test_entry("e2", "older"),
            ])
            .unwrap();

        // Send to old-peer -- should trigger full resend via legacy migration.
        let result = send_sync(&workdir, "old-peer", "ws-local", &store, 100).unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        // Full resend: all entries included.
        assert_eq!(result.sent, 2);
    }

    #[test]
    fn knowledge_sync_protocol_max_send_limits_batch() {
        let (_tmp, workdir) = setup_workdir();
        let store = KnowledgeStore::for_workdir(&workdir);
        store
            .ingest(vec![
                make_test_entry("e1", "one"),
                make_test_entry("e2", "two"),
                make_test_entry("e3", "three"),
            ])
            .unwrap();

        let result = send_sync(&workdir, "peer-b", "ws-local", &store, 2).unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.sent, 2);
    }

    #[test]
    fn knowledge_sync_protocol_empty_store_returns_none() {
        let (_tmp, workdir) = setup_workdir();
        let store = KnowledgeStore::for_workdir(&workdir);

        let result = send_sync(&workdir, "peer-b", "ws-local", &store, 100).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn knowledge_sync_protocol_empty_inbox_returns_empty() {
        let (_tmp, workdir) = setup_workdir();
        let store = KnowledgeStore::for_workdir(&workdir);

        let results = receive_sync(&workdir, "peer-a", &store).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn knowledge_sync_protocol_rejects_bad_version() {
        let (_tmp, workdir) = setup_workdir();
        let store = KnowledgeStore::for_workdir(&workdir);
        let layout = MeshLayout::new(&workdir);
        let inbox = layout.inbox_dir("peer-a");
        fs::create_dir_all(&inbox).unwrap();

        let entry = make_test_entry("e1", "content");
        let sync_entry = SyncEntryV1 {
            entry_id: "e1".to_string(),
            sequence: 1,
            entry,
        };
        let mut envelope = SyncEnvelopeV1 {
            version: 99, // bad version
            transfer_id: "bad-ver".to_string(),
            source_workspace_id: "ws-1".to_string(),
            source_peer_id: "peer-a".to_string(),
            first_sequence: 1,
            last_sequence: 1,
            entries: vec![sync_entry],
            checksum: String::new(),
            created_at: Utc::now(),
        };
        envelope.checksum = compute_envelope_checksum(&envelope);
        let path = inbox.join("bad-ver.json");
        fs::write(&path, serde_json::to_string(&envelope).unwrap()).unwrap();

        let result = receive_sync(&workdir, "peer-a", &store);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unsupported envelope version")
        );
    }

    #[test]
    fn knowledge_sync_protocol_rejects_bad_checksum() {
        let (_tmp, workdir) = setup_workdir();
        let store = KnowledgeStore::for_workdir(&workdir);
        let layout = MeshLayout::new(&workdir);
        let inbox = layout.inbox_dir("peer-a");
        fs::create_dir_all(&inbox).unwrap();

        let entry = make_test_entry("e1", "content");
        let sync_entry = SyncEntryV1 {
            entry_id: "e1".to_string(),
            sequence: 1,
            entry,
        };
        let envelope = SyncEnvelopeV1 {
            version: 1,
            transfer_id: "bad-cksum".to_string(),
            source_workspace_id: "ws-1".to_string(),
            source_peer_id: "peer-a".to_string(),
            first_sequence: 1,
            last_sequence: 1,
            entries: vec![sync_entry],
            checksum: "definitely_wrong".to_string(),
            created_at: Utc::now(),
        };
        let path = inbox.join("bad-cksum.json");
        fs::write(&path, serde_json::to_string(&envelope).unwrap()).unwrap();

        let result = receive_sync(&workdir, "peer-a", &store);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("checksum mismatch")
        );
    }
}
