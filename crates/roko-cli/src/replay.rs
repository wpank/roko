//! Pure replay traversal and rendering for the `roko replay` command.
//!
//! This module implements breadth-first lineage DAG walking with
//! deterministic lexicographic parent ordering, structured error
//! reporting, and both text and JSON output formatting.
//!
//! The traversal is separated from I/O so that it can be tested with
//! in-memory fixtures.

use roko_core::{ContentHash, Engram as Signal};
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};

// ── Exit codes ──────────────────────────────────────────────────────

/// Successful replay traversal.
pub const REPLAY_EXIT_SUCCESS: i32 = 0;
/// The requested root hash was not found in the substrate.
pub const REPLAY_EXIT_ROOT_NOT_FOUND: i32 = 1;
/// A parent referenced by a present record is missing (corrupt ancestry).
pub const REPLAY_EXIT_ANCESTOR_NOT_FOUND: i32 = 2;

// ── Output format ───────────────────────────────────────────────────

/// The resolved output format for replay rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayFormat {
    /// Human-readable tree (default).
    Tree,
    /// JSON Lines: one JSON object per line on stdout, diagnostics on stderr.
    Json,
}

impl ReplayFormat {
    /// Resolve the output format from the `--format` flag and the global `--json` flag.
    ///
    /// Returns `Err` if both are specified but contradictory (e.g. `--json --format tree`).
    pub fn resolve(format_flag: &str, global_json: bool) -> Result<Self, String> {
        let from_flag = match format_flag {
            "json" => Some(Self::Json),
            "tree" => Some(Self::Tree),
            _ => None,
        };

        match (from_flag, global_json) {
            // --format json and/or --json: both select JSON
            (Some(Self::Json), true) | (Some(Self::Json), false) | (None, true) => Ok(Self::Json),
            // --format tree (no --json): tree
            (Some(Self::Tree), false) | (None, false) => Ok(Self::Tree),
            // --format tree AND --json: contradiction
            (Some(Self::Tree), true) => {
                Err("contradictory output flags: --json and --format tree".to_string())
            }
            // Unknown format value
            _ => Err(format!("unknown format: {format_flag}")),
        }
    }
}

// ── Traversal types ─────────────────────────────────────────────────

/// A single record emitted by the replay traversal.
#[derive(Clone, Debug, Serialize)]
pub struct ReplayRecord {
    /// 1-based traversal index (BFS order).
    pub event: usize,
    /// The signal's content hash.
    pub hash: String,
    /// The signal kind.
    pub kind: String,
    /// The signal author.
    pub author: String,
    /// Unix milliseconds when the signal was created.
    pub created_at_ms: i64,
    /// Tags (omitted when empty in JSON output).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<(String, String)>,
    /// Body preview (first 500 chars for JSON, first 120 for forensic text).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Lineage hashes (for forensic output).
    #[serde(skip)]
    pub lineage: Vec<String>,
    /// BFS depth (for tree indentation, not serialized).
    #[serde(skip)]
    pub depth: usize,
}

/// Structured error from replay traversal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReplayError {
    pub code: &'static str,
    pub message: String,
    pub hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referenced_by: Option<String>,
}

impl ReplayError {
    /// Root hash was not found in the substrate.
    pub fn root_not_found(hash: &str) -> Self {
        Self {
            code: "replay_root_not_found",
            message: format!("replay root not found: {hash}"),
            hash: hash.to_string(),
            referenced_by: None,
        }
    }

    /// A parent referenced by a present record is missing.
    pub fn ancestor_not_found(parent_hash: &str, child_hash: &str) -> Self {
        Self {
            code: "replay_ancestor_not_found",
            message: format!(
                "replay ancestor not found: {parent_hash} (referenced by {child_hash})"
            ),
            hash: parent_hash.to_string(),
            referenced_by: Some(child_hash.to_string()),
        }
    }

    /// Exit code for this error.
    pub fn exit_code(&self) -> i32 {
        match self.code {
            "replay_root_not_found" => REPLAY_EXIT_ROOT_NOT_FOUND,
            "replay_ancestor_not_found" => REPLAY_EXIT_ANCESTOR_NOT_FOUND,
            _ => REPLAY_EXIT_ROOT_NOT_FOUND,
        }
    }

    /// Render as a JSON error object.
    pub fn to_json(&self) -> String {
        let obj = serde_json::json!({"error": self});
        serde_json::to_string(&obj).unwrap_or_default()
    }

    /// Render as a stderr text error.
    pub fn to_text(&self) -> String {
        format!("error: {}", self.message)
    }
}

/// The result of a complete replay traversal.
#[derive(Clone, Debug)]
pub enum ReplayResult {
    /// Traversal succeeded; records are in BFS order.
    Ok(Vec<ReplayRecord>),
    /// Traversal failed (root not found or corrupt ancestry).
    Err(ReplayError),
}

// ── Pure traversal ──────────────────────────────────────────────────

/// Walk the lineage DAG rooted at `root_hash` using breadth-first
/// traversal with lexicographic parent ordering.
///
/// `lookup` maps content hashes to signals. This is the pure core of
/// the replay command -- no I/O, no printing.
///
/// `from_event` is an inclusive 1-based traversal-index lower bound:
/// records with index < from_event are traversed but not emitted.
pub fn traverse_dag(
    root_hash: &ContentHash,
    lookup: &HashMap<ContentHash, Signal>,
    from_event: usize,
) -> ReplayResult {
    // Check root exists.
    let root_signal = match lookup.get(root_hash) {
        Some(sig) => sig,
        None => {
            return ReplayResult::Err(ReplayError::root_not_found(&root_hash.to_string()));
        }
    };

    let mut visited = HashSet::new();
    let mut queue: VecDeque<(ContentHash, usize)> = VecDeque::new();
    let mut records = Vec::new();
    let mut index: usize = 0;
    // Track missing ancestors: (parent_hash, child_hash) for the first one found.
    let mut first_missing_ancestor: Option<(String, String)> = None;

    // Seed with root.
    queue.push_back((*root_hash, 0));

    // Pre-borrow root to avoid double-lookup in the loop.
    drop(root_signal);

    while let Some((id, depth)) = queue.pop_front() {
        if !visited.insert(id) {
            continue;
        }

        if let Some(sig) = lookup.get(&id) {
            index += 1;

            // Sort parent hashes lexicographically before enqueueing so
            // branching output is deterministic.
            let mut parents: Vec<ContentHash> = sig.lineage.clone();
            parents.sort_by(|a, b| a.to_string().cmp(&b.to_string()));

            for parent in &parents {
                if !visited.contains(parent) && !lookup.contains_key(parent) {
                    // Record the first missing ancestor under deterministic order.
                    if first_missing_ancestor.is_none() {
                        first_missing_ancestor =
                            Some((parent.to_string(), sig.id.to_string()));
                    }
                }
                queue.push_back((*parent, depth + 1));
            }

            // Apply --from-event filter: skip records before the target index.
            if index < from_event {
                continue;
            }

            let body_preview = sig
                .body
                .as_text()
                .ok()
                .map(|text| text.chars().take(500).collect::<String>());

            let tags: Vec<(String, String)> = sig
                .tags
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            let lineage: Vec<String> = sig.lineage.iter().map(|h| h.to_string()).collect();

            records.push(ReplayRecord {
                event: index,
                hash: sig.id.to_string(),
                kind: sig.kind.to_string(),
                author: sig.provenance.author.clone(),
                created_at_ms: sig.created_at_ms,
                tags,
                body: body_preview,
                lineage,
                depth,
            });
        } else if first_missing_ancestor.is_none() {
            // This shouldn't happen for non-root since we check above,
            // but guard against it anyway.
            // The id itself is missing and it's not the root (we checked that),
            // so it must be referenced by some parent we already visited.
            // We already track this above, so this branch is defensive only.
        }
    }

    // Report the first missing ancestor if any.
    if let Some((parent_hash, child_hash)) = first_missing_ancestor {
        return ReplayResult::Err(ReplayError::ancestor_not_found(&parent_hash, &child_hash));
    }

    ReplayResult::Ok(records)
}

// ── Rendering ───────────────────────────────────────────────────────

/// Render replay records as JSON Lines (one JSON object per line).
pub fn render_json(records: &[ReplayRecord]) -> Vec<String> {
    records
        .iter()
        .map(|rec| {
            let mut obj = serde_json::Map::new();
            obj.insert("event".into(), serde_json::json!(rec.event));
            obj.insert("hash".into(), serde_json::json!(rec.hash));
            obj.insert("kind".into(), serde_json::json!(rec.kind));
            obj.insert("author".into(), serde_json::json!(rec.author));
            obj.insert("created_at_ms".into(), serde_json::json!(rec.created_at_ms));
            if !rec.tags.is_empty() {
                let tags: serde_json::Map<String, serde_json::Value> = rec
                    .tags
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::json!(v)))
                    .collect();
                obj.insert("tags".into(), serde_json::Value::Object(tags));
            }
            if let Some(body) = &rec.body {
                obj.insert("body".into(), serde_json::json!(body));
            }
            serde_json::Value::Object(obj).to_string()
        })
        .collect()
}

/// Render replay records as forensic text (detailed, indented).
pub fn render_forensic_text(records: &[ReplayRecord]) -> Vec<String> {
    let mut lines = Vec::new();
    for rec in records {
        let indent = "  ".repeat(rec.depth);
        lines.push(format!("{indent}{} {}", rec.kind, rec.hash));
        lines.push(format!("{indent}  event:     {}", rec.event));
        lines.push(format!("{indent}  hash:      {}", rec.hash));
        lines.push(format!("{indent}  author:    {}", rec.author));
        lines.push(format!("{indent}  created:   {}", rec.created_at_ms));
        lines.push(format!(
            "{indent}  lineage:   [{}]",
            rec.lineage.join(", ")
        ));
        if !rec.tags.is_empty() {
            let tag_strs: Vec<String> = rec
                .tags
                .iter()
                .map(|(k, v)| format!("{k:?}: {v:?}"))
                .collect();
            lines.push(format!("{indent}  tags:      {{{}}}", tag_strs.join(", ")));
        }
        if let Some(body) = &rec.body {
            let body_preview: String = body.chars().take(120).collect();
            lines.push(format!("{indent}  body:      {body_preview}"));
        }
        lines.push(String::new());
    }
    lines
}

/// Render replay records as compact tree text (one line per record).
pub fn render_tree_text(records: &[ReplayRecord]) -> Vec<String> {
    records
        .iter()
        .map(|rec| {
            let indent = "  ".repeat(rec.depth);
            format!(
                "{indent}{} {}  (event={}, author={})",
                rec.kind, rec.hash, rec.event, rec.author
            )
        })
        .collect()
}

// ── Parse filter ────────────────────────────────────────────────────

/// Parse the `--from-event` / `--as-of` filter value into a 1-based
/// inclusive lower bound. Returns 0 (no filtering) if the value is
/// `None` or cannot be parsed.
pub fn parse_event_filter(value: Option<&str>) -> usize {
    value
        .and_then(|s| {
            let stripped = s
                .trim_start_matches("step")
                .trim_start_matches('#')
                .trim();
            stripped.parse().ok()
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::{Body, ContentHash, Kind, Engram as Signal};

    /// Build a test signal with a given body text, author, and lineage.
    fn make_signal(body_text: &str, author: &str, lineage: Vec<ContentHash>) -> Signal {
        Signal::builder(Kind::Task)
            .body(Body::text(body_text))
            .provenance(roko_core::Provenance::trusted(author))
            .lineage(lineage)
            .created_at_ms(1_000_000)
            .build()
    }

    /// Build a lookup map from a slice of signals.
    fn make_lookup(signals: &[Signal]) -> HashMap<ContentHash, Signal> {
        signals.iter().map(|s| (s.id, s.clone())).collect()
    }

    // ── Traversal order tests ───────────────────────────────────────

    #[test]
    fn linear_chain_traversal_order() {
        // C -> B -> A
        let a = make_signal("A", "alice", vec![]);
        let b = make_signal("B", "bob", vec![a.id]);
        let c = make_signal("C", "carol", vec![b.id]);
        let lookup = make_lookup(&[a.clone(), b.clone(), c.clone()]);

        let result = traverse_dag(&c.id, &lookup, 0);
        let records = match result {
            ReplayResult::Ok(r) => r,
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        };

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].hash, c.id.to_string());
        assert_eq!(records[0].event, 1);
        assert_eq!(records[1].hash, b.id.to_string());
        assert_eq!(records[1].event, 2);
        assert_eq!(records[2].hash, a.id.to_string());
        assert_eq!(records[2].event, 3);
    }

    #[test]
    fn diamond_dag_deterministic_order() {
        //   D
        //  / \
        // B   C
        //  \ /
        //   A
        let a = make_signal("A", "alice", vec![]);
        let b = make_signal("B", "bob", vec![a.id]);
        let c = make_signal("C", "carol", vec![a.id]);

        // D has two parents: B and C. The traversal should sort them
        // lexicographically before enqueueing.
        let d = make_signal("D", "dave", vec![b.id, c.id]);
        let lookup = make_lookup(&[a.clone(), b.clone(), c.clone(), d.clone()]);

        let result = traverse_dag(&d.id, &lookup, 0);
        let records = match result {
            ReplayResult::Ok(r) => r,
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        };

        assert_eq!(records.len(), 4);
        // D is always first (the root)
        assert_eq!(records[0].hash, d.id.to_string());

        // B and C are at depth 1; their order depends on lexicographic
        // hash ordering. Collect them.
        let depth_1: Vec<&str> = records[1..3]
            .iter()
            .map(|r| r.hash.as_str())
            .collect();

        // Verify they are sorted lexicographically
        assert!(depth_1[0] < depth_1[1], "BFS parents should be lexicographically sorted");

        // Verify both B and C are present
        let bc_hashes: HashSet<String> =
            [b.id.to_string(), c.id.to_string()].into_iter().collect();
        let actual_hashes: HashSet<String> = depth_1.iter().map(|s| s.to_string()).collect();
        assert_eq!(bc_hashes, actual_hashes);

        // A is always last (depth 2, shared ancestor visited once)
        assert_eq!(records[3].hash, a.id.to_string());
    }

    #[test]
    fn cycle_handled_gracefully() {
        // A -> B -> A (cycle)
        let mut a = make_signal("A", "alice", vec![]);
        let b = make_signal("B", "bob", vec![a.id]);
        // Manually create a cycle: A references B
        a.lineage.push(b.id);
        // Recompute hash after mutation would change the id, but for
        // test purposes we keep the original hash in the lookup.
        let lookup = make_lookup(&[a.clone(), b.clone()]);

        let result = traverse_dag(&a.id, &lookup, 0);
        let records = match result {
            ReplayResult::Ok(r) => r,
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        };

        // Both should be visited exactly once despite the cycle.
        assert_eq!(records.len(), 2);
        let hashes: HashSet<String> = records.iter().map(|r| r.hash.clone()).collect();
        assert!(hashes.contains(&a.id.to_string()));
        assert!(hashes.contains(&b.id.to_string()));
    }

    // ── Filter tests ────────────────────────────────────────────────

    #[test]
    fn from_event_filter_zero_emits_all() {
        let a = make_signal("A", "alice", vec![]);
        let b = make_signal("B", "bob", vec![a.id]);
        let lookup = make_lookup(&[a.clone(), b.clone()]);

        let result = traverse_dag(&b.id, &lookup, 0);
        match result {
            ReplayResult::Ok(r) => assert_eq!(r.len(), 2),
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn from_event_filter_one_emits_all() {
        // from_event=1 is an inclusive lower bound on index 1; all
        // records start at index 1, so everything is emitted.
        let a = make_signal("A", "alice", vec![]);
        let b = make_signal("B", "bob", vec![a.id]);
        let lookup = make_lookup(&[a.clone(), b.clone()]);

        let result = traverse_dag(&b.id, &lookup, 1);
        match result {
            ReplayResult::Ok(r) => assert_eq!(r.len(), 2),
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn from_event_filter_skips_early_records() {
        let a = make_signal("A", "alice", vec![]);
        let b = make_signal("B", "bob", vec![a.id]);
        let c = make_signal("C", "carol", vec![b.id]);
        let lookup = make_lookup(&[a.clone(), b.clone(), c.clone()]);

        // from_event=2 should skip event 1 (C) and emit events 2 and 3.
        let result = traverse_dag(&c.id, &lookup, 2);
        match result {
            ReplayResult::Ok(r) => {
                assert_eq!(r.len(), 2);
                assert_eq!(r[0].event, 2);
                assert_eq!(r[1].event, 3);
            }
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn from_event_filter_beyond_count_emits_nothing() {
        let a = make_signal("A", "alice", vec![]);
        let lookup = make_lookup(&[a.clone()]);

        // from_event=999 is beyond the single record.
        let result = traverse_dag(&a.id, &lookup, 999);
        match result {
            ReplayResult::Ok(r) => assert!(r.is_empty()),
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    // ── Error tests ─────────────────────────────────────────────────

    #[test]
    fn missing_root_returns_error() {
        let lookup: HashMap<ContentHash, Signal> = HashMap::new();
        let fake_hash = ContentHash([0xAA; 32]);

        let result = traverse_dag(&fake_hash, &lookup, 0);
        match result {
            ReplayResult::Err(e) => {
                assert_eq!(e.code, "replay_root_not_found");
                assert_eq!(e.exit_code(), REPLAY_EXIT_ROOT_NOT_FOUND);
                assert!(e.referenced_by.is_none());
            }
            ReplayResult::Ok(_) => panic!("expected error for missing root"),
        }
    }

    #[test]
    fn missing_ancestor_returns_error() {
        let phantom_parent = ContentHash([0xBB; 32]);
        let a = make_signal("A", "alice", vec![phantom_parent]);
        let lookup = make_lookup(&[a.clone()]);

        let result = traverse_dag(&a.id, &lookup, 0);
        match result {
            ReplayResult::Err(e) => {
                assert_eq!(e.code, "replay_ancestor_not_found");
                assert_eq!(e.exit_code(), REPLAY_EXIT_ANCESTOR_NOT_FOUND);
                assert_eq!(e.hash, phantom_parent.to_string());
                assert_eq!(e.referenced_by.as_deref(), Some(a.id.to_string().as_str()));
            }
            ReplayResult::Ok(_) => panic!("expected error for missing ancestor"),
        }
    }

    #[test]
    fn missing_root_json_shape() {
        let hash = "aaaa".repeat(16);
        let err = ReplayError::root_not_found(&hash);
        let json_str = err.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["error"]["code"], "replay_root_not_found");
        assert_eq!(
            parsed["error"]["message"],
            format!("replay root not found: {hash}")
        );
        assert_eq!(parsed["error"]["hash"], hash);
        assert!(parsed["error"]["referenced_by"].is_null());
    }

    #[test]
    fn missing_ancestor_json_shape() {
        let parent = "bbbb".repeat(16);
        let child = "cccc".repeat(16);
        let err = ReplayError::ancestor_not_found(&parent, &child);
        let json_str = err.to_json();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["error"]["code"], "replay_ancestor_not_found");
        assert_eq!(parsed["error"]["hash"], parent);
        assert_eq!(parsed["error"]["referenced_by"], child);
    }

    #[test]
    fn missing_root_text_shape() {
        let hash = "aaaa".repeat(16);
        let err = ReplayError::root_not_found(&hash);
        assert_eq!(err.to_text(), format!("error: replay root not found: {hash}"));
    }

    #[test]
    fn missing_ancestor_text_shape() {
        let parent = "bbbb".repeat(16);
        let child = "cccc".repeat(16);
        let err = ReplayError::ancestor_not_found(&parent, &child);
        assert_eq!(
            err.to_text(),
            format!("error: replay ancestor not found: {parent} (referenced by {child})")
        );
    }

    // ── Render tests ────────────────────────────────────────────────

    #[test]
    fn json_render_is_valid_jsonl() {
        let a = make_signal("A", "alice", vec![]);
        let b = make_signal("B", "bob", vec![a.id]);
        let lookup = make_lookup(&[a.clone(), b.clone()]);

        let result = traverse_dag(&b.id, &lookup, 0);
        let records = match result {
            ReplayResult::Ok(r) => r,
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        };

        let lines = render_json(&records);
        assert_eq!(lines.len(), 2);
        for line in &lines {
            let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(parsed.is_object(), "each JSONL line must be a JSON object");
            assert!(parsed["event"].is_number());
            assert!(parsed["hash"].is_string());
            assert!(parsed["kind"].is_string());
            assert!(parsed["author"].is_string());
            assert!(parsed["created_at_ms"].is_number());
        }
    }

    #[test]
    fn json_render_omits_empty_tags() {
        let a = make_signal("A", "alice", vec![]);
        let lookup = make_lookup(&[a.clone()]);

        let result = traverse_dag(&a.id, &lookup, 0);
        let records = match result {
            ReplayResult::Ok(r) => r,
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        };

        let lines = render_json(&records);
        let parsed: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        assert!(
            parsed.get("tags").is_none(),
            "empty tags should be omitted from JSON output"
        );
    }

    #[test]
    fn json_render_includes_body_preview() {
        let a = make_signal("hello world", "alice", vec![]);
        let lookup = make_lookup(&[a.clone()]);

        let result = traverse_dag(&a.id, &lookup, 0);
        let records = match result {
            ReplayResult::Ok(r) => r,
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        };

        let lines = render_json(&records);
        let parsed: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(parsed["body"], "hello world");
    }

    #[test]
    fn forensic_text_includes_lineage() {
        let a = make_signal("A", "alice", vec![]);
        let b = make_signal("B", "bob", vec![a.id]);
        let lookup = make_lookup(&[a.clone(), b.clone()]);

        let result = traverse_dag(&b.id, &lookup, 0);
        let records = match result {
            ReplayResult::Ok(r) => r,
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        };

        let lines = render_forensic_text(&records);
        let joined = lines.join("\n");
        assert!(joined.contains("lineage:"));
        assert!(joined.contains(&a.id.to_string()));
    }

    #[test]
    fn forensic_text_body_truncated_to_120_chars() {
        let long_body: String = "x".repeat(200);
        let a = make_signal(&long_body, "alice", vec![]);
        let lookup = make_lookup(&[a.clone()]);

        let result = traverse_dag(&a.id, &lookup, 0);
        let records = match result {
            ReplayResult::Ok(r) => r,
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        };

        let lines = render_forensic_text(&records);
        let body_line = lines.iter().find(|l| l.contains("body:")).unwrap();
        // The body preview is 120 chars, plus the "  body:      " prefix
        let body_content = body_line.split("body:").nth(1).unwrap().trim();
        assert_eq!(body_content.len(), 120);
    }

    #[test]
    fn tree_text_one_line_per_record() {
        let a = make_signal("A", "alice", vec![]);
        let b = make_signal("B", "bob", vec![a.id]);
        let lookup = make_lookup(&[a.clone(), b.clone()]);

        let result = traverse_dag(&b.id, &lookup, 0);
        let records = match result {
            ReplayResult::Ok(r) => r,
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        };

        let lines = render_tree_text(&records);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("event=1"));
        assert!(lines[1].contains("event=2"));
    }

    // ── Parse filter tests ──────────────────────────────────────────

    #[test]
    fn parse_event_filter_none_returns_zero() {
        assert_eq!(parse_event_filter(None), 0);
    }

    #[test]
    fn parse_event_filter_numeric() {
        assert_eq!(parse_event_filter(Some("5")), 5);
        assert_eq!(parse_event_filter(Some("0")), 0);
        assert_eq!(parse_event_filter(Some("1")), 1);
    }

    #[test]
    fn parse_event_filter_step_prefix() {
        assert_eq!(parse_event_filter(Some("step5")), 5);
        assert_eq!(parse_event_filter(Some("step 5")), 5);
        assert_eq!(parse_event_filter(Some("step05")), 5);
    }

    #[test]
    fn parse_event_filter_hash_prefix() {
        assert_eq!(parse_event_filter(Some("#3")), 3);
    }

    #[test]
    fn parse_event_filter_invalid_returns_zero() {
        assert_eq!(parse_event_filter(Some("abc")), 0);
        assert_eq!(parse_event_filter(Some("")), 0);
    }

    // ── Format resolution tests ─────────────────────────────────────

    #[test]
    fn format_resolve_tree_default() {
        assert_eq!(
            ReplayFormat::resolve("tree", false).unwrap(),
            ReplayFormat::Tree
        );
    }

    #[test]
    fn format_resolve_json_flag() {
        assert_eq!(
            ReplayFormat::resolve("tree", true).unwrap_err(),
            "contradictory output flags: --json and --format tree"
        );
    }

    #[test]
    fn format_resolve_format_json() {
        assert_eq!(
            ReplayFormat::resolve("json", false).unwrap(),
            ReplayFormat::Json
        );
    }

    #[test]
    fn format_resolve_both_json() {
        assert_eq!(
            ReplayFormat::resolve("json", true).unwrap(),
            ReplayFormat::Json
        );
    }

    #[test]
    fn format_resolve_global_json_alone() {
        // global --json with default format "tree" text doesn't override;
        // but if format is left as default we must handle it in the caller.
        // Here we test with "tree" explicitly:
        assert!(ReplayFormat::resolve("tree", true).is_err());
    }

    // ── Diamond fixture with FileSubstrate ──────────────────────────

    #[test]
    fn diamond_bfs_order_is_deterministic_across_runs() {
        //   D
        //  / \
        // B   C
        //  \ /
        //   A
        let a = make_signal("A", "alice", vec![]);
        let b = make_signal("B", "bob", vec![a.id]);
        let c = make_signal("C", "carol", vec![a.id]);
        let d = make_signal("D", "dave", vec![b.id, c.id]);
        let lookup = make_lookup(&[a.clone(), b.clone(), c.clone(), d.clone()]);

        // Run traversal 10 times and verify the order is identical.
        let mut first_order: Option<Vec<String>> = None;
        for _ in 0..10 {
            let result = traverse_dag(&d.id, &lookup, 0);
            let records = match result {
                ReplayResult::Ok(r) => r,
                ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
            };
            let order: Vec<String> = records.iter().map(|r| r.hash.clone()).collect();
            if let Some(ref expected) = first_order {
                assert_eq!(&order, expected, "traversal order must be deterministic");
            } else {
                first_order = Some(order);
            }
        }
    }

    #[test]
    fn single_node_no_parents() {
        let a = make_signal("solo", "alice", vec![]);
        let lookup = make_lookup(&[a.clone()]);

        let result = traverse_dag(&a.id, &lookup, 0);
        match result {
            ReplayResult::Ok(r) => {
                assert_eq!(r.len(), 1);
                assert_eq!(r[0].event, 1);
                assert_eq!(r[0].depth, 0);
            }
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn empty_body_signal_has_no_body_in_json() {
        let sig = Signal::builder(Kind::Task)
            .body(Body::empty())
            .provenance(roko_core::Provenance::trusted("test"))
            .created_at_ms(1_000_000)
            .build();
        let lookup = make_lookup(&[sig.clone()]);

        let result = traverse_dag(&sig.id, &lookup, 0);
        let records = match result {
            ReplayResult::Ok(r) => r,
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        };

        let lines = render_json(&records);
        let parsed: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        assert!(
            parsed.get("body").is_none(),
            "empty body should be omitted from JSON"
        );
    }

    #[test]
    fn tagged_signal_includes_tags_in_json() {
        let sig = Signal::builder(Kind::Task)
            .body(Body::text("tagged"))
            .provenance(roko_core::Provenance::trusted("test"))
            .tag("priority", "high")
            .tag("domain", "infra")
            .created_at_ms(1_000_000)
            .build();
        let lookup = make_lookup(&[sig.clone()]);

        let result = traverse_dag(&sig.id, &lookup, 0);
        let records = match result {
            ReplayResult::Ok(r) => r,
            ReplayResult::Err(e) => panic!("unexpected error: {e:?}"),
        };

        let lines = render_json(&records);
        let parsed: serde_json::Value = serde_json::from_str(&lines[0]).unwrap();
        assert_eq!(parsed["tags"]["priority"], "high");
        assert_eq!(parsed["tags"]["domain"], "infra");
    }
}
