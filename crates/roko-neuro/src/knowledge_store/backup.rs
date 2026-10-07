//! Knowledge store export, import, and Merkle verification.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::{KnowledgeEntry, KnowledgeKind, KnowledgeTier};

use super::KnowledgeStore;
use super::scoring::{entry_similarity, join_replayed_security_labels, normalize_entry_security};
use super::types::*;

#[cfg(feature = "hdc")]
use super::scoring::fingerprint_content;

// ── Merkle tree ──────────────────────────────────────────────────────

/// Compute a Merkle root over a sorted list of entry IDs.
///
/// The IDs are first sorted lexicographically so that the root is
/// deterministic regardless of export order. Each leaf is the SHA-256 hash
/// of the UTF-8 entry ID. The tree is built bottom-up by hashing pairs of
/// adjacent nodes; an odd node is promoted unchanged (a "lone sibling" carry).
/// Returns the hex-encoded root hash, or an empty string for an empty set.
pub fn compute_merkle_root(ids: &[String]) -> String {
    if ids.is_empty() {
        return String::new();
    }
    // Sort IDs for deterministic ordering.
    let mut sorted = ids.to_vec();
    sorted.sort();

    // Build leaves: SHA-256 of each entry ID.
    let mut layer: Vec<[u8; 32]> = sorted
        .iter()
        .map(|id| {
            let mut h = Sha256::new();
            h.update(id.as_bytes());
            h.finalize().into()
        })
        .collect();

    // Reduce pairs until one node remains.
    while layer.len() > 1 {
        let mut next: Vec<[u8; 32]> = Vec::with_capacity(layer.len().div_ceil(2));
        let mut i = 0;
        while i < layer.len() {
            if i + 1 < layer.len() {
                let mut h = Sha256::new();
                h.update(layer[i]);
                h.update(layer[i + 1]);
                next.push(h.finalize().into());
                i += 2;
            } else {
                // Odd node: promote as-is.
                next.push(layer[i]);
                i += 1;
            }
        }
        layer = next;
    }

    // Encode the single root as lowercase hex.
    layer[0].iter().fold(String::new(), |mut out, b| {
        use std::fmt::Write;
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// Compute the version-2 Merkle root over complete canonical entry JSON.
///
/// Entries are sorted by ID and then by their serialized bytes. Both content
/// and identity changes therefore invalidate the root.
pub(crate) fn compute_entry_merkle_root(entries: &[KnowledgeEntry]) -> Result<String> {
    if entries.is_empty() {
        return Ok(String::new());
    }

    let mut canonical = entries
        .iter()
        .map(|entry| {
            serde_json::to_vec(entry)
                .map(|bytes| (entry.id.clone(), bytes))
                .context("serialize knowledge entry for Merkle root")
        })
        .collect::<Result<Vec<_>>>()?;
    canonical.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    let mut layer = canonical
        .into_iter()
        .map(|(_, bytes)| {
            let mut hash = Sha256::new();
            hash.update(bytes);
            <[u8; 32]>::from(hash.finalize())
        })
        .collect::<Vec<_>>();

    while layer.len() > 1 {
        let mut next = Vec::with_capacity(layer.len().div_ceil(2));
        let mut index = 0;
        while index < layer.len() {
            if let Some(right) = layer.get(index + 1) {
                let mut hash = Sha256::new();
                hash.update(layer[index]);
                hash.update(right);
                next.push(hash.finalize().into());
            } else {
                next.push(layer[index]);
            }
            index += 2;
        }
        layer = next;
    }

    Ok(layer[0].iter().fold(String::new(), |mut output, byte| {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
        output
    }))
}

// ── Import helpers ───────────────────────────────────────────────────

pub(crate) fn read_import_entries(
    input: &Path,
    allow_legacy: bool,
) -> Result<(Vec<KnowledgeEntry>, bool)> {
    let file =
        File::open(input).with_context(|| format!("open import file at {}", input.display()))?;
    let lines = BufReader::new(file)
        .lines()
        .enumerate()
        .map(|(index, line)| {
            line.with_context(|| format!("read import line {} from {}", index + 1, input.display()))
                .map(|line| (index + 1, line))
        })
        .collect::<Result<Vec<_>>>()?;

    ensure!(!lines.is_empty(), "import file is empty");

    if let Ok(header) = serde_json::from_str::<BackupHeader>(&lines[0].1) {
        let entries = parse_strict_import_lines(&lines[1..])?;
        ensure!(
            header.entry_count == entries.len(),
            "backup entry_count mismatch: header={}, actual={}",
            header.entry_count,
            entries.len()
        );

        match header.version {
            KNOWLEDGE_BACKUP_VERSION => {
                let actual_root = compute_entry_merkle_root(&entries)?;
                ensure!(
                    !header.merkle_root.is_empty() || entries.is_empty(),
                    "canonical backup is missing its Merkle root"
                );
                ensure!(
                    header.merkle_root == actual_root,
                    "backup Merkle verification failed"
                );
                return Ok((entries, false));
            }
            1 if allow_legacy => {
                ensure!(
                    !header.merkle_root.is_empty() || entries.is_empty(),
                    "legacy version-1 backup has no integrity root"
                );
                let ids = entries
                    .iter()
                    .map(|entry| entry.id.clone())
                    .collect::<Vec<_>>();
                ensure!(
                    header.merkle_root == compute_merkle_root(&ids),
                    "legacy backup ID Merkle verification failed"
                );
                return Ok((entries, true));
            }
            1 => {
                anyhow::bail!("legacy version-1 backup requires explicit allow_legacy migration");
            }
            version => {
                anyhow::bail!(
                    "unsupported backup version {version} (this build supports version {KNOWLEDGE_BACKUP_VERSION})"
                );
            }
        }
    }

    ensure!(
        allow_legacy,
        "import is not a canonical versioned backup; use explicit allow_legacy migration for a trusted raw JSONL store"
    );
    Ok((parse_strict_import_lines(&lines)?, true))
}

fn parse_strict_import_lines(lines: &[(usize, String)]) -> Result<Vec<KnowledgeEntry>> {
    let mut entries = Vec::with_capacity(lines.len());
    for (line_number, line) in lines {
        ensure!(
            !line.trim().is_empty(),
            "malformed_entries=1: blank import record at line {line_number}"
        );
        let entry = serde_json::from_str::<KnowledgeEntry>(line).with_context(|| {
            format!("malformed_entries=1: invalid knowledge entry at line {line_number}")
        })?;
        ensure!(
            !entry.id.trim().is_empty(),
            "malformed_entries=1: empty knowledge entry ID at line {line_number}"
        );
        entries.push(entry);
    }
    Ok(entries)
}

pub(crate) fn resolved_transfer_path(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut cursor = absolute.as_path();
    let mut suffix = Vec::new();
    while !cursor.exists() {
        let name = cursor
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("resolve transfer path {}", path.display()))?;
        suffix.push(name.to_os_string());
        cursor = cursor
            .parent()
            .ok_or_else(|| anyhow::anyhow!("resolve parent of {}", path.display()))?;
    }
    let mut resolved = std::fs::canonicalize(cursor)
        .with_context(|| format!("resolve existing path ancestor {}", cursor.display()))?;
    for component in suffix.into_iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

pub(crate) fn import_entry_is_contradicted(
    candidate: &KnowledgeEntry,
    existing: &[KnowledgeEntry],
    admitted: &[KnowledgeEntry],
) -> bool {
    if candidate.kind == KnowledgeKind::AntiKnowledge {
        return false;
    }

    existing.iter().chain(admitted).any(|entry| {
        entry.kind == KnowledgeKind::AntiKnowledge
            && entry.confidence > 0.8
            && import_contradiction_similarity(entry, candidate) > 0.9
    })
}

pub(crate) fn import_entry_is_semantic_duplicate(
    candidate: &KnowledgeEntry,
    existing: &[KnowledgeEntry],
    admitted: &[KnowledgeEntry],
) -> bool {
    existing.iter().chain(admitted).any(|entry| {
        // AntiKnowledge is a refutation, not a duplicate of the ordinary
        // knowledge it contradicts. Never discard it merely because its
        // content is highly similar to the claim being refuted.
        (entry.kind == KnowledgeKind::AntiKnowledge)
            == (candidate.kind == KnowledgeKind::AntiKnowledge)
            && import_semantic_similarity(entry, candidate) > 0.95
    })
}

#[cfg(feature = "hdc")]
fn import_semantic_similarity(left: &KnowledgeEntry, right: &KnowledgeEntry) -> f64 {
    use crate::hdc::KnowledgeHdcEncoder;
    let encoder = KnowledgeHdcEncoder;
    let structured = encoder
        .encode_entry(left)
        .similarity(&encoder.encode_entry(right));
    let content =
        fingerprint_content(&left.content).similarity(&fingerprint_content(&right.content));
    f64::from(structured.max(content))
}

#[cfg(not(feature = "hdc"))]
fn import_semantic_similarity(left: &KnowledgeEntry, right: &KnowledgeEntry) -> f64 {
    entry_similarity(left, right)
}

fn import_contradiction_similarity(left: &KnowledgeEntry, right: &KnowledgeEntry) -> f64 {
    let lexical = entry_similarity(left, right);
    #[cfg(feature = "hdc")]
    {
        lexical.max(import_semantic_similarity(left, right))
    }
    #[cfg(not(feature = "hdc"))]
    {
        lexical
    }
}

// ── KnowledgeStore export/import methods ─────────────────────────────

impl KnowledgeStore {
    /// Export the knowledge store to a JSONL file with versioned backup header.
    ///
    /// Entries are filtered by the provided [`ExportFilter`] (including optional
    /// secret filtering), then sorted by confidence descending so bounded exports
    /// retain the most valuable knowledge. A SHA-256 Merkle root over complete
    /// canonical entry JSON is included in the header.
    ///
    /// Returns the number of entries exported.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read or the output cannot be
    /// written.
    pub fn export(&self, output: &Path, filter: &ExportFilter) -> Result<usize> {
        let source = resolved_transfer_path(&self.path)?;
        let destination = resolved_transfer_path(output)?;
        ensure!(
            source != destination,
            "knowledge export destination resolves to the live store at {}",
            self.path.display()
        );

        let entries = self.read_all_strict()?;
        let mut filtered: Vec<_> = entries.into_iter().filter(|e| filter.matches(e)).collect();

        // Sort highest confidence first so bounded exports retain the best entries.
        filtered.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
        if let Some(max_entries) = filter.max_entries {
            filtered.truncate(max_entries);
        }

        let count = filtered.len();

        ensure!(
            filtered.iter().all(|entry| !entry.id.trim().is_empty()),
            "cannot export a knowledge entry with an empty ID"
        );
        let unique_ids = filtered
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<HashSet<_>>();
        ensure!(
            unique_ids.len() == filtered.len(),
            "cannot export duplicate knowledge entry IDs"
        );

        // Version 2 commits the complete serialized entries, not only IDs.
        let merkle_root = compute_entry_merkle_root(&filtered)?;

        let header = BackupHeader {
            version: KNOWLEDGE_BACKUP_VERSION,
            created_at: Utc::now(),
            entry_count: count,
            source_path: self.path.display().to_string(),
            merkle_root,
        };

        // Serialize the complete artifact before opening any staging file. The
        // shared atomic writer then fsyncs a unique same-directory temporary
        // file, renames it over the target, and fsyncs the parent directory.
        let mut bytes = Vec::new();
        serde_json::to_writer(&mut bytes, &header).context("serialize backup header")?;
        bytes.push(b'\n');
        for entry in &filtered {
            serde_json::to_writer(&mut bytes, entry).context("serialize knowledge entry")?;
            bytes.push(b'\n');
        }
        roko_fs::atomic_write_bytes(output, &bytes)
            .with_context(|| format!("atomically write export file at {}", output.display()))?;

        Ok(count)
    }

    /// Export all entries with integrity verification and return an [`ExportBundle`].
    ///
    /// This is a higher-level alternative to [`export`] for callers that need the
    /// exported data in memory (e.g. replication, sync) rather than written to a file.
    ///
    /// Applies a default [`ExportFilter`] with `filter_secrets = true`, sorts entries
    /// by confidence descending, and computes a SHA-256 Merkle root over complete
    /// canonical entry JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if the store cannot be read.
    pub fn export_with_verification(&self) -> Result<ExportBundle> {
        let filter = ExportFilter {
            filter_secrets: true,
            ..Default::default()
        };
        let entries = self.read_all_strict()?;
        let mut filtered: Vec<KnowledgeEntry> =
            entries.into_iter().filter(|e| filter.matches(e)).collect();

        // Sort highest confidence first.
        filtered.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });

        let merkle_root = compute_entry_merkle_root(&filtered)?;
        Ok(ExportBundle {
            entries: filtered,
            merkle_root,
        })
    }

    /// Import knowledge entries from a versioned JSONL backup file.
    ///
    /// The complete input is parsed and its count and Merkle root are verified
    /// before the destination is read or written. Restored entries are reset to
    /// [`KnowledgeTier::Transient`] and their confidence is multiplied by the
    /// configured discount factor. Deduplication and contradiction checks are
    /// completed before one atomic destination rewrite.
    ///
    /// Returns exact admitted and skipped counts.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, aliases the live store, has
    /// an unsupported version, or entries cannot be ingested.
    pub fn import(&self, input: &Path, options: &ImportOptions) -> Result<ImportResult> {
        let source = resolved_transfer_path(input)?;
        let destination = resolved_transfer_path(&self.path)?;
        ensure!(
            source != destination,
            "knowledge import source resolves to the live store at {}",
            self.path.display()
        );
        ensure!(
            options.confidence_discount.is_finite()
                && (0.0..=1.0).contains(&options.confidence_discount),
            "confidence discount must be between 0.0 and 1.0"
        );
        if let Some(minimum) = options.min_confidence {
            ensure!(
                minimum.is_finite() && (0.0..=1.0).contains(&minimum),
                "minimum confidence must be between 0.0 and 1.0"
            );
        }

        let (entries, legacy_input) = read_import_entries(input, options.allow_legacy)?;
        let source_entries = entries.len();
        let mut skipped_filter = 0;
        let mut transformed = Vec::with_capacity(entries.len());
        let mut source_contradictions = Vec::new();
        for mut entry in entries {
            let kind_matches = options
                .kinds
                .as_ref()
                .is_none_or(|kinds| kinds.contains(&entry.kind));
            let confidence_matches = options
                .min_confidence
                .is_none_or(|minimum| entry.confidence >= minimum);
            if !kind_matches || !confidence_matches {
                skipped_filter += 1;
                continue;
            }
            // Preserve the source confidence for contradiction enforcement.
            // Applying the import discount must not weaken a high-confidence
            // refutation or make the result depend on source record order.
            if entry.kind == KnowledgeKind::AntiKnowledge && entry.confidence > 0.8 {
                source_contradictions.push(normalize_entry_security(entry.clone()));
            }
            if options.reset_tier {
                entry.tier = KnowledgeTier::Transient;
            }
            entry.confidence = (entry.confidence * options.confidence_discount).clamp(0.0, 1.0);
            entry.confidence_weight =
                (entry.confidence_weight * options.confidence_discount).clamp(0.0, 1.0);
            entry.source = Some(options.source_label.clone());
            transformed.push(normalize_entry_security(entry));
        }

        let _guard = self.lock_writes();
        let mut merged = self.read_all_strict()?;
        let security_upgraded = join_replayed_security_labels(&mut merged, &transformed);
        let mut admitted = Vec::new();
        let mut skipped_dedup = 0;
        let mut skipped_contradiction = 0;
        let mut seen_ids = merged
            .iter()
            .filter(|entry| !entry.id.trim().is_empty())
            .map(|entry| entry.id.clone())
            .collect::<HashSet<_>>();

        for entry in transformed {
            if entry.id.trim().is_empty() {
                anyhow::bail!("malformed_entries=1: imported knowledge entry has an empty ID");
            }
            if !seen_ids.insert(entry.id.clone()) {
                skipped_dedup += 1;
                continue;
            }
            if import_entry_is_contradicted(&entry, &merged, &source_contradictions) {
                skipped_contradiction += 1;
                continue;
            }
            if import_entry_is_semantic_duplicate(&entry, &merged, &admitted) {
                skipped_dedup += 1;
                continue;
            }
            admitted.push(entry);
        }

        let imported = admitted.len();
        if imported > 0 || security_upgraded {
            merged.extend(admitted.iter().cloned());
            self.rewrite_all(&merged)?;
            self.register_temporal_entries(&admitted);
        }

        let result = ImportResult {
            source_entries,
            imported,
            skipped_dedup,
            skipped_contradiction,
            skipped_filter,
            malformed: 0,
            legacy_input,
        };
        tracing::info!(
            entries_imported = result.imported,
            entries_skipped_dedup = result.skipped_dedup,
            entries_skipped_contradiction = result.skipped_contradiction,
            entries_skipped_filter = result.skipped_filter,
            malformed_entries = result.malformed,
            legacy_input = result.legacy_input,
            "knowledge import completed"
        );
        Ok(result)
    }
}
