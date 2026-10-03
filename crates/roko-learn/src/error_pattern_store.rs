//! Persistent storage for error patterns discovered during plan execution.
//!
//! # Overview
//!
//! [`ErrorPatternStore`] accumulates compiler and test errors across tasks and
//! plan runs, normalises each error into a stable `key` digest, and exposes
//! the most frequent patterns so that agent prompts can include "known pitfalls"
//! context. This enables agents to learn from each other's failures across
//! different tasks and even different plan runs.
//!
//! # Entry Lifecycle
//!
//! 1. A gate produces a [`GateFailureObservation`] (from a compiler error,
//!    review verdict, or retry classifier).
//! 2. `ErrorPatternStore::upsert_observation` normalises the observation to a
//!    stable `key` (e.g. `E0425::src/lib.rs`) and merges it into an existing
//!    [`ErrorPattern`] or inserts a new one.
//! 3. `occurrences`, `plan_ids`, and `task_ids` are updated atomically.
//! 4. `ErrorPatternStore::top_patterns` returns the most frequent unresolved
//!    patterns for prompt injection.
//! 5. After a fix is confirmed, `ErrorPatternStore::mark_resolved` annotates the
//!    pattern with a resolution string and removes it from future prompt context.
//!
//! # Pattern Categorisation
//!
//! Patterns are classified into coarse categories such as `"unresolved_import"`,
//! `"type_mismatch"`, `"lifetime"`, and `"test_failure"`. Categories are
//! determined by the gate or parser that produced the observation and stored in
//! [`ErrorPattern::category`].
//!
//! # Similarity Matching
//!
//! [`ErrorPatternStore::similar_patterns`] finds patterns whose digest is close
//! to a query string. This allows the store to surface patterns that are not
//! exact matches but describe the same class of error — useful for prompting
//! before a new compile gate runs.
//!
//! # Relationship to PostGateReflectionStore
//!
//! `ErrorPatternStore` tracks **what** went wrong (concrete error signatures).
//! `PostGateReflectionStore` captures **what to do about it** (lessons and
//! playbook candidates). They are complementary:
//!
//! - `ErrorPatternStore` patterns are injected as factual "known errors" context.
//! - `PostGateReflectionStore` lessons are injected as actionable "lessons learned" context.
//!
//! # Persistence
//!
//! The store is a single JSON file at `.roko/learn/error-patterns.json`. Writes
//! use atomic tmp-rename (`error-patterns.json.tmp` → rename) to avoid corruption
//! on crash. There is no upper bound on pattern count, but `mark_resolved` and
//! periodic GC remove stale entries.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The store's file name in `.roko/learn`: the one pattern file plan runs
/// write and every reader loads (backlog 4204).
pub const ERROR_PATTERNS_FILE: &str = "error-patterns.json";

/// The longest fix a verified pass records on a pattern (backlog 4125).
pub const MAX_RESOLUTION_CHARS: usize = 400;

/// Runner-v2's pattern file in `.roko/learn`, which nothing writes any more.
/// [`retire_legacy_discovered_patterns`] sets it aside.
pub const LEGACY_DISCOVERED_PATTERNS_FILE: &str = "discovered-patterns.json";

/// Set aside Runner-v2's pattern file in `learn_dir`: rename it to
/// `discovered-patterns.json.v2-legacy`, the suffix roko-fs migrations use,
/// and log it. Its rows carry no task or command key, so keyed selection
/// would never pick them: they are not imported. A file already set aside is
/// never overwritten. Returns whether a file was set aside (backlog 4204).
///
/// # Errors
///
/// Returns the I/O error of a rename that failed.
pub fn retire_legacy_discovered_patterns(learn_dir: &Path) -> std::io::Result<bool> {
    let legacy = learn_dir.join(LEGACY_DISCOVERED_PATTERNS_FILE);
    let retired = learn_dir.join(format!("{LEGACY_DISCOVERED_PATTERNS_FILE}.v2-legacy"));
    if !legacy.is_file() || retired.exists() {
        return Ok(false);
    }
    std::fs::rename(&legacy, &retired)?;
    tracing::info!(
        from = %legacy.display(),
        to = %retired.display(),
        "set aside Runner-v2's pattern file; plan runs read {ERROR_PATTERNS_FILE}"
    );
    Ok(true)
}

/// A single normalized error pattern with occurrence tracking.
///
/// Patterns are keyed by [`ErrorPattern::key`]. Older digest-only rows are
/// repaired on load by using the digest as the key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorPattern {
    /// Stable key used for de-duplication. New gate failure observations use
    /// normalized gate/parser keys such as `E0425::src/lib.rs`.
    #[serde(default)]
    pub key: String,
    /// Normalized error signature (first line of error, stripped of file
    /// paths and line numbers).
    pub digest: String,
    /// Verify that emitted the pattern, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<String>,
    /// Error category (e.g. `"unresolved_import"`, `"type_mismatch"`,
    /// `"lifetime"`).
    pub category: String,
    /// How many times this pattern has been seen.
    pub occurrences: u32,
    /// ISO 8601 timestamp of the first occurrence.
    pub first_seen_at: String,
    /// ISO 8601 timestamp of the most recent occurrence.
    pub last_seen_at: String,
    /// Plan IDs that have hit this error.
    pub plan_ids: BTreeSet<String>,
    /// Task IDs that have hit this error.
    #[serde(default)]
    pub task_ids: BTreeSet<String>,
    /// Whether this pattern has been resolved.
    #[serde(default)]
    pub resolved: bool,
    /// What fixed the error (filled in from reflection or manual annotation).
    pub resolution: Option<String>,
    /// The key of the verified attempt whose pass recorded `resolution`
    /// ([`ErrorPatternStore::record_resolution`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_by: Option<String>,
    /// Auto-fix hint extracted from rustc output.
    pub suggestion: Option<String>,
}

/// A structured gate failure observation emitted by gates, review parsing, or
/// retry classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateFailureObservation {
    /// Stable key used to merge repeated observations.
    pub key: String,
    /// Plan that observed the failure.
    pub plan_id: String,
    /// Task that observed the failure, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Verify or parser source that observed the failure.
    pub gate: String,
    /// Coarse failure class.
    pub classification: String,
    /// Compact, bounded signature. Raw logs should not be stored here.
    pub digest: String,
    /// Optional suggested fix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Source subsystem that produced the observation.
    pub source: GateFailureSource,
    /// ISO 8601 timestamp for the observation.
    pub observed_at: String,
}

impl GateFailureObservation {
    /// Build an observation and stamp it with the current time.
    #[must_use]
    pub fn new(
        key: impl Into<String>,
        plan_id: impl Into<String>,
        task_id: Option<String>,
        gate: impl Into<String>,
        classification: impl Into<String>,
        digest: impl Into<String>,
        source: GateFailureSource,
    ) -> Self {
        Self {
            key: key.into(),
            plan_id: plan_id.into(),
            task_id,
            gate: gate.into(),
            classification: classification.into(),
            digest: truncate_chars(&digest.into(), 200),
            suggestion: None,
            source,
            observed_at: Utc::now().to_rfc3339(),
        }
    }

    /// Attach an optional suggestion to this observation.
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: Option<String>) -> Self {
        self.suggestion = suggestion;
        self
    }
}

/// Subsystem that produced a gate failure observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateFailureSource {
    /// Compile/test/lint gate classification.
    GateClassification,
    /// Structured review verdict parsing.
    ReviewVerdict,
    /// Agent dispatch/retry error classification.
    RetryClassifier,
}

/// Result of upserting a failure observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailurePatternUpdate {
    /// Whether this observation created a new pattern.
    pub inserted: bool,
    /// Occurrence count after the update.
    pub occurrences: u32,
}

/// Query used to select relevant failure patterns for retry context.
#[derive(Debug, Clone, Copy, Default)]
pub struct FailurePatternQuery<'a> {
    /// Plan to prefer.
    pub plan_id: Option<&'a str>,
    /// Task to prefer.
    pub task_id: Option<&'a str>,
    /// Verify to prefer.
    pub gate: Option<&'a str>,
    /// Failure class to prefer.
    pub classification: Option<&'a str>,
    /// The task's verify commands. A keyed summary
    /// ([`ErrorPatternStore::bounded_summary_keyed`]) selects the patterns
    /// whose gate is one of them (backlog 4209).
    pub verify_commands: &'a [String],
}

/// A bounded prompt/context summary for failure memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailurePatternSummary {
    /// Selected patterns in display order.
    pub patterns: Vec<FailurePatternSummaryItem>,
    /// Number of candidate patterns considered before bounding.
    pub total_candidates: usize,
}

impl FailurePatternSummary {
    /// Render the summary as retry-context text.
    #[must_use]
    pub fn format_for_prompt(&self) -> String {
        if self.patterns.is_empty() {
            return String::new();
        }

        let mut out = String::from("## Prior Verify Failure Patterns\n");
        out.push_str(
            "Use these concise prior failures as constraints; do not treat them as full logs.\n",
        );
        for (index, pattern) in self.patterns.iter().enumerate() {
            let repeated = if pattern.repeated {
                "repeated"
            } else {
                "one-off"
            };
            let _ = writeln!(
                out,
                "{}. [{}] {} (seen {} time{}, {repeated})",
                index + 1,
                pattern.classification,
                pattern.digest,
                pattern.occurrences,
                if pattern.occurrences == 1 { "" } else { "s" },
            );
            if let Some(gate) = &pattern.gate {
                let _ = writeln!(out, "   Verify: {gate}");
            }
            if let Some(resolution) = &pattern.resolution {
                let _ = writeln!(out, "   Fix: {resolution}");
            }
            if let Some(suggestion) = &pattern.suggestion {
                let _ = writeln!(out, "   Hint: {suggestion}");
            }
        }
        out
    }
}

/// One selected failure pattern for prompt/context use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailurePatternSummaryItem {
    /// Stable key for the pattern.
    pub key: String,
    /// Verify that emitted the pattern, if known.
    pub gate: Option<String>,
    /// Coarse failure class.
    pub classification: String,
    /// Compact signature.
    pub digest: String,
    /// Occurrence count.
    pub occurrences: u32,
    /// Whether this is a repeated pattern rather than a one-off failure.
    pub repeated: bool,
    /// Known resolution from reflection or manual annotation.
    pub resolution: Option<String>,
    /// Optional suggested fix.
    pub suggestion: Option<String>,
}

/// Persistent store of [`ErrorPattern`] records backed by a JSON file.
///
/// The store de-duplicates patterns by key: structured observations use
/// normalized keys, while the legacy [`append`](Self::append) path uses the
/// digest as the key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPatternStore {
    patterns: Vec<ErrorPattern>,
    /// Derived index mapping pattern keys to their position in `patterns`.
    /// Rebuilt on load; not serialized.
    #[serde(skip)]
    key_index: HashMap<String, usize>,
}

impl ErrorPatternStore {
    /// Load patterns from a JSON file at `path`.
    ///
    /// Returns an empty store if the file does not exist or cannot be
    /// parsed (e.g. after a crash that left a partial write).
    pub fn load(path: &Path) -> Self {
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(_) => return Self::empty(),
        };
        serde_json::from_slice::<Self>(&bytes)
            .map(|mut store| {
                store.repair_loaded_patterns();
                store.rebuild_key_index();
                store
            })
            .unwrap_or_else(|_| Self::empty())
    }

    /// Persist the store to `path` as pretty-printed JSON.
    ///
    /// Uses atomic write (tmp file + rename) so readers never see a
    /// partially-written file.
    ///
    /// # Errors
    ///
    /// Returns the underlying [`std::io::Error`] if the parent directory
    /// cannot be created, serialization fails, or the filesystem write fails.
    pub fn save(&mut self, path: &Path) -> Result<(), std::io::Error> {
        self.gc(Duration::from_hours(2160), 10_000);
        roko_fs::atomic_write_json(path, self)
    }

    /// Upsert an error pattern by digest.
    ///
    /// If a pattern with the same `digest` already exists, its occurrence
    /// counter is incremented, `last_seen_at` is updated, and `plan_id` and
    /// `suggestion` are merged. Otherwise a new pattern is created.
    pub fn append(
        &mut self,
        digest: &str,
        category: &str,
        plan_id: &str,
        suggestion: Option<&str>,
    ) {
        self.append_at(digest, category, plan_id, suggestion, Utc::now());
    }

    /// [`ErrorPatternStore::append`] with the time of the occurrence given,
    /// so `first_seen_at` and `last_seen_at` do not depend on the clock.
    pub fn append_at(
        &mut self,
        digest: &str,
        category: &str,
        plan_id: &str,
        suggestion: Option<&str>,
        now: DateTime<Utc>,
    ) {
        let observation = GateFailureObservation::new(
            digest,
            plan_id,
            None,
            "unknown",
            category,
            digest,
            GateFailureSource::RetryClassifier,
        )
        .with_suggestion(suggestion.map(str::to_string));
        let _ = self.observe_gate_failure_at(observation, now);
    }

    /// Upsert a structured gate failure observation.
    ///
    /// Repeated observations with the same key increment evidence on the
    /// existing pattern instead of producing duplicate prompt noise.
    pub fn observe_gate_failure(
        &mut self,
        observation: GateFailureObservation,
    ) -> FailurePatternUpdate {
        self.observe_gate_failure_at(observation, Utc::now())
    }

    /// [`ErrorPatternStore::observe_gate_failure`] with the time of the
    /// observation given instead of read from the clock.
    pub fn observe_gate_failure_at(
        &mut self,
        observation: GateFailureObservation,
        now: DateTime<Utc>,
    ) -> FailurePatternUpdate {
        let now = now.to_rfc3339();
        let key = observation.key.trim().to_string();
        if key.is_empty() {
            return FailurePatternUpdate {
                inserted: false,
                occurrences: 0,
            };
        }

        if let Some(&idx) = self.key_index.get(&key) {
            let existing = &mut self.patterns[idx];
            existing.occurrences = existing.occurrences.saturating_add(1);
            existing.last_seen_at = now;
            existing.plan_ids.insert(observation.plan_id);
            if let Some(task_id) = observation.task_id {
                existing.task_ids.insert(task_id);
            }
            if existing.gate.is_none() && !observation.gate.trim().is_empty() {
                existing.gate = Some(observation.gate);
            }
            if !observation.digest.trim().is_empty() {
                existing.digest = truncate_chars(&observation.digest, 200);
            }
            if existing.suggestion.is_none() {
                existing.suggestion = observation.suggestion;
            }
            return FailurePatternUpdate {
                inserted: false,
                occurrences: existing.occurrences,
            };
        }

        let idx = self.patterns.len();
        self.key_index.insert(key.clone(), idx);
        self.patterns.push(ErrorPattern {
            key,
            digest: truncate_chars(&observation.digest, 200),
            gate: (!observation.gate.trim().is_empty()).then_some(observation.gate),
            category: observation.classification,
            occurrences: 1,
            first_seen_at: now.clone(),
            last_seen_at: now,
            plan_ids: std::iter::once(observation.plan_id).collect(),
            task_ids: observation.task_id.into_iter().collect(),
            resolved: false,
            resolution: None,
            resolved_by: None,
            suggestion: observation.suggestion,
        });
        FailurePatternUpdate {
            inserted: true,
            occurrences: 1,
        }
    }

    /// Record on the pattern `key` what fixed it: `resolution`, cut to
    /// [`MAX_RESOLUTION_CHARS`], from the verified attempt `resolved_by`
    /// (backlog 4125). The pattern stays unresolved, because prompts leave
    /// resolved patterns out, which would hide the fix exactly when there is
    /// one to show. Returns whether `key` names a pattern.
    pub fn record_resolution(&mut self, key: &str, resolution: &str, resolved_by: &str) -> bool {
        let Some(&index) = self.key_index.get(key.trim()) else {
            return false;
        };
        let pattern = &mut self.patterns[index];
        pattern.resolution = Some(truncate_chars(resolution.trim(), MAX_RESOLUTION_CHARS));
        pattern.resolved_by = Some(resolved_by.to_string());
        true
    }

    /// The patterns with a recorded fix, seen at least twice, that are about
    /// a crate `paths` name (backlog 4126), most frequent first and at most
    /// `limit`. A pattern is about the crates its verify command, digest and
    /// fix name: `crates/<name>` paths, and the package of a cargo `-p` or
    /// `--package` flag.
    pub fn resolved_for(&self, paths: &[String], limit: usize) -> Vec<&ErrorPattern> {
        let wanted: BTreeSet<String> = paths.iter().flat_map(|path| crates_named(path)).collect();
        if wanted.is_empty() {
            return Vec::new();
        }
        let mut found: Vec<&ErrorPattern> = self
            .patterns
            .iter()
            .filter(|pattern| pattern.resolution.is_some() && pattern.occurrences >= 2)
            .filter(|pattern| !pattern.crates().is_disjoint(&wanted))
            .collect();
        found.sort_by(|a, b| {
            b.occurrences
                .cmp(&a.occurrences)
                .then_with(|| b.last_seen_at.cmp(&a.last_seen_at))
        });
        found.truncate(limit);
        found
    }

    /// Return the most frequent patterns, sorted by descending occurrence
    /// count.
    pub fn top_patterns(&self, limit: usize) -> Vec<&ErrorPattern> {
        let mut sorted: Vec<&ErrorPattern> = self.patterns.iter().collect();
        sorted.sort_by_key(|p| std::cmp::Reverse(p.occurrences));
        sorted.truncate(limit);
        sorted
    }

    /// Return all patterns matching the given `category`.
    pub fn patterns_for_category(&self, category: &str) -> Vec<&ErrorPattern> {
        self.patterns
            .iter()
            .filter(|p| p.category == category)
            .collect()
    }

    /// Return a bounded, relevance-ranked summary for retry prompt context.
    /// Every pattern that scores against `query` qualifies, or every pattern
    /// when the query is empty; prompts use [`Self::bounded_summary_keyed`].
    #[must_use]
    pub fn bounded_summary(
        &self,
        query: FailurePatternQuery<'_>,
        limit: usize,
        max_chars: usize,
    ) -> FailurePatternSummary {
        self.summary_where(query, limit, max_chars, |pattern| {
            pattern.relevance_score(query) > 0 || query.is_empty()
        })
    }

    /// [`Self::bounded_summary`] for a prompt (backlog 4209): a pattern
    /// qualifies only when it was seen on the query's task, or its gate is
    /// one of the query's verify commands, compared with whitespace
    /// collapsed. Plan and class only order the qualifying patterns. A query
    /// with neither a task nor commands selects none.
    #[must_use]
    pub fn bounded_summary_keyed(
        &self,
        query: FailurePatternQuery<'_>,
        limit: usize,
        max_chars: usize,
    ) -> FailurePatternSummary {
        self.summary_where(query, limit, max_chars, |pattern| pattern.keyed_to(query))
    }

    /// The bounded summary of the unresolved patterns that `qualifies`
    /// accepts, ranked by relevance to `query`.
    fn summary_where(
        &self,
        query: FailurePatternQuery<'_>,
        limit: usize,
        max_chars: usize,
        qualifies: impl Fn(&ErrorPattern) -> bool,
    ) -> FailurePatternSummary {
        let mut candidates: Vec<(usize, &ErrorPattern)> = self
            .patterns
            .iter()
            .filter(|pattern| !pattern.resolved && qualifies(pattern))
            .map(|pattern| (pattern.relevance_score(query), pattern))
            .collect();
        candidates.sort_by(|(score_a, a), (score_b, b)| {
            score_b
                .cmp(score_a)
                .then_with(|| b.occurrences.cmp(&a.occurrences))
                .then_with(|| b.last_seen_at.cmp(&a.last_seen_at))
        });

        let total_candidates = candidates.len();
        let mut used_chars = 0usize;
        let mut patterns = Vec::new();
        for (_, pattern) in candidates.into_iter().take(limit) {
            let item = FailurePatternSummaryItem {
                key: pattern.key.clone(),
                gate: pattern.gate.clone(),
                classification: pattern.category.clone(),
                digest: truncate_chars(&pattern.digest, 200),
                occurrences: pattern.occurrences,
                repeated: pattern.occurrences > 1,
                resolution: pattern.resolution.clone(),
                suggestion: pattern.suggestion.clone(),
            };
            let projected = item.digest.chars().count()
                + item.resolution.as_ref().map_or(0, |s| s.chars().count())
                + item.suggestion.as_ref().map_or(0, |s| s.chars().count())
                + item.gate.as_ref().map_or(0, |s| s.chars().count())
                + 80;
            if !patterns.is_empty() && used_chars.saturating_add(projected) > max_chars {
                break;
            }
            used_chars = used_chars.saturating_add(projected);
            patterns.push(item);
        }

        FailurePatternSummary {
            patterns,
            total_candidates,
        }
    }

    /// Format the top patterns as a markdown-ish block.
    ///
    /// Each entry shows the digest, category, occurrence count, and any
    /// known resolution or suggestion. Output is capped at `limit` entries.
    /// It is unkeyed, every pattern qualifying, so it suits display
    /// (`roko learn`); prompts use [`Self::bounded_summary_keyed`]
    /// (backlog 4209).
    pub fn format_for_prompt(&self, limit: usize) -> String {
        self.bounded_summary(FailurePatternQuery::default(), limit, 2_000)
            .format_for_prompt()
    }

    /// Return the number of distinct patterns in the store.
    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    /// Return `true` if the store contains no patterns.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// Return a new, empty store with no patterns.
    pub fn empty() -> Self {
        Self {
            patterns: Vec::new(),
            key_index: HashMap::new(),
        }
    }

    fn repair_loaded_patterns(&mut self) {
        // A turn cap or a timeout says nothing about the code; older runs
        // recorded them as patterns (backlog 4208).
        self.patterns
            .retain(|pattern| !matches!(pattern.category.as_str(), "turn_cap" | "timeout"));
        for pattern in &mut self.patterns {
            if pattern.key.is_empty() {
                pattern.key = pattern.digest.clone();
            }
            pattern.digest = truncate_chars(&pattern.digest, 200);
        }
    }

    /// Rebuild the `key_index` from the current `patterns` vec.
    fn rebuild_key_index(&mut self) {
        self.key_index.clear();
        self.key_index.reserve(self.patterns.len());
        for (idx, pattern) in self.patterns.iter().enumerate() {
            self.key_index.insert(pattern.key.clone(), idx);
        }
    }

    /// Evict stale or excess patterns to bound store growth.
    ///
    /// 1. Removes patterns whose `last_seen_at` is older than `max_age`.
    /// 2. If the store still exceeds `max_patterns`, removes the oldest
    ///    *resolved* patterns first (by `last_seen_at`), then the oldest
    ///    unresolved patterns until the limit is satisfied.
    /// 3. Rebuilds the `key_index` after any removals.
    pub fn gc(&mut self, max_age: Duration, max_patterns: usize) {
        let cutoff =
            Utc::now() - chrono::Duration::from_std(max_age).unwrap_or(chrono::Duration::days(90));
        let cutoff_str = cutoff.to_rfc3339();

        let before = self.patterns.len();
        self.patterns.retain(|p| p.last_seen_at >= cutoff_str);

        if self.patterns.len() > max_patterns {
            // Sort indices by eviction priority: resolved first, then oldest last_seen_at.
            let mut indices: Vec<usize> = (0..self.patterns.len()).collect();
            indices.sort_by(|&a, &b| {
                let pa = &self.patterns[a];
                let pb = &self.patterns[b];
                // Resolved patterns are evicted before unresolved ones.
                pb.resolved
                    .cmp(&pa.resolved)
                    .then_with(|| pa.last_seen_at.cmp(&pb.last_seen_at))
            });
            // Mark the first (len - max_patterns) indices for removal.
            let to_remove = self.patterns.len() - max_patterns;
            let mut remove_set: Vec<bool> = vec![false; self.patterns.len()];
            for &idx in indices.iter().take(to_remove) {
                remove_set[idx] = true;
            }
            let mut i = 0;
            self.patterns.retain(|_| {
                let keep = !remove_set[i];
                i += 1;
                keep
            });
        }

        if self.patterns.len() != before {
            self.rebuild_key_index();
        }
    }
}

impl ErrorPattern {
    /// The crates the pattern is about: those its verify command, digest
    /// and fix name ([`crates_named`]).
    fn crates(&self) -> BTreeSet<String> {
        [
            self.gate.as_deref(),
            Some(self.digest.as_str()),
            self.resolution.as_deref(),
        ]
        .into_iter()
        .flatten()
        .flat_map(crates_named)
        .collect()
    }

    fn relevance_score(&self, query: FailurePatternQuery<'_>) -> usize {
        let mut score = 0usize;
        if let Some(task_id) = query.task_id
            && self.task_ids.contains(task_id)
        {
            score += 8;
        }
        if let Some(plan_id) = query.plan_id
            && self.plan_ids.contains(plan_id)
        {
            score += 4;
        }
        if let Some(gate) = query.gate
            && self.gate.as_deref() == Some(gate)
        {
            score += 2;
        }
        if let Some(classification) = query.classification
            && self.category == classification
        {
            score += 1;
        }
        score
    }

    /// Whether the pattern is about the query's work (backlog 4209): it was
    /// seen on the query's task, or it is the failure of one of the query's
    /// verify commands (its gate), compared with whitespace collapsed.
    fn keyed_to(&self, query: FailurePatternQuery<'_>) -> bool {
        let same_task = query
            .task_id
            .is_some_and(|task_id| self.task_ids.contains(task_id));
        let same_command = self.gate.as_deref().is_some_and(|gate| {
            let gate = collapse_whitespace(gate);
            query
                .verify_commands
                .iter()
                .any(|command| collapse_whitespace(command) == gate)
        });
        same_task || same_command
    }
}

/// `text` with each run of whitespace made one space, and none at the ends.
impl FailurePatternQuery<'_> {
    fn is_empty(self) -> bool {
        self.plan_id.is_none()
            && self.task_id.is_none()
            && self.gate.is_none()
            && self.classification.is_none()
            && self.verify_commands.is_empty()
    }
}

/// Normalize raw error text into a stable digest.
///
/// The normalization pipeline:
/// 1. Takes the first non-empty line of `raw`.
/// 2. Strips ANSI escape codes.
/// 3. Replaces file-path-with-line-number tokens (e.g.
///    `/path/to/file.rs:42:10`) with `<file>`.
/// 4. Collapses runs of whitespace into single spaces.
/// 5. Truncates to 200 characters.
pub fn normalize_error_digest(raw: &str) -> String {
    let first_line = raw
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim();

    let stripped = strip_ansi(first_line);
    let no_paths = replace_file_paths(&stripped);
    let collapsed = collapse_whitespace(&no_paths);

    truncate_chars(&collapsed, 200)
}

/// Strip ANSI escape sequences (CSI and OSC) from `text`.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // CSI sequence: ESC [ ... final byte
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                while let Some(&next) = chars.peek() {
                    chars.next();
                    // CSI terminates at an ASCII letter or '~'.
                    if next.is_ascii_alphabetic() || next == '~' {
                        break;
                    }
                }
            } else {
                // OSC or other: consume until BEL or ST.
                while let Some(&next) = chars.peek() {
                    if next == '\x07' {
                        chars.next();
                        break;
                    }
                    if next == '\x1b' {
                        break;
                    }
                    chars.next();
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Replace file-path tokens like `/foo/bar.rs:42:10` or `src/lib.rs:7`
/// with `<file>`.
fn replace_file_paths(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for token in text.split_whitespace() {
        if !result.is_empty() {
            result.push(' ');
        }
        if is_file_path_token(token) {
            result.push_str("<file>");
        } else {
            result.push_str(token);
        }
    }
    result
}

/// Check whether `token` looks like a file path with a line number,
/// e.g. `crates/roko-learn/src/lib.rs:42:10` or `-->
/// src/main.rs:7:1`.
fn is_file_path_token(token: &str) -> bool {
    let cleaned = token
        .trim_start_matches("-->")
        .trim_start()
        .trim_end_matches([',', ';', ':']);

    // Must contain a `.rs:` or `.ts:` or `.go:` etc followed by digits.
    for ext in [".rs:", ".ts:", ".go:", ".py:", ".js:", ".toml:", ".json:"] {
        if let Some((_prefix, tail)) = cleaned.rsplit_once(ext) {
            let first_part = tail.split(':').next().unwrap_or("");
            if !first_part.is_empty() && first_part.chars().all(|ch| ch.is_ascii_digit()) {
                return true;
            }
        }
    }
    false
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_chars(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// Where a workspace keeps its crates.
const CRATES_DIR: &str = "crates/";

/// The crates `text` names: each `crates/<name>` path, and the package of
/// each cargo `-p <name>`, `--package <name>` or `--package=<name>` flag.
fn crates_named(text: &str) -> BTreeSet<String> {
    let mut crates = BTreeSet::new();
    let mut words = text.split_whitespace().peekable();
    while let Some(word) = words.next() {
        for (start, _) in word.match_indices(CRATES_DIR) {
            crates.insert(crate_name(&word[start + CRATES_DIR.len()..]));
        }
        let package = match word {
            "-p" | "--package" => words.peek().copied(),
            _ => word.strip_prefix("--package="),
        };
        if let Some(package) = package {
            crates.insert(crate_name(package));
        }
    }
    crates.remove("");
    crates
}

/// The crate name `text` starts with: its leading ASCII letters, digits,
/// dashes and underscores.
fn crate_name(text: &str) -> String {
    text.chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .collect()
}

// NOTE: The `unique_tmp_path` helper that lived here has been replaced by
// `roko_fs::atomic_write_json`.

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// backlog 4209: a keyed summary selects a pattern of the same task, or
    /// of one of the task's verify commands from another task, and skips a
    /// pattern of another task and command, though it shares the plan.
    #[test]
    fn keyed_summary_skips_patterns_of_other_tasks_and_commands() {
        let mut store = ErrorPatternStore::empty();
        for (task, command) in [
            ("T1", "cargo test -p app"),
            ("T2", "cargo  clippy -p app"),
            ("T3", "cargo test -p other"),
        ] {
            store.observe_gate_failure(GateFailureObservation::new(
                format!("verify::{task}"),
                "plan-1",
                Some(task.to_string()),
                command,
                "verify",
                format!("{command} failed"),
                GateFailureSource::GateClassification,
            ));
        }
        let commands = vec!["cargo clippy -p app".to_string()];
        let query = FailurePatternQuery {
            plan_id: Some("plan-1"),
            task_id: Some("T1"),
            verify_commands: &commands,
            ..FailurePatternQuery::default()
        };

        let summary = store.bounded_summary_keyed(query, 5, 2_000);
        let mut gates: Vec<&str> = summary
            .patterns
            .iter()
            .filter_map(|pattern| pattern.gate.as_deref())
            .collect();
        gates.sort_unstable();
        assert_eq!(gates, ["cargo  clippy -p app", "cargo test -p app"]);
        let unkeyed = FailurePatternQuery::default();
        let keyed = store.bounded_summary_keyed(unkeyed, 5, 2_000);
        assert!(keyed.patterns.is_empty(), "no key selects nothing");
        assert_eq!(store.bounded_summary(unkeyed, 5, 2_000).patterns.len(), 3);
    }

    /// backlog 4208: loading drops the turn-cap and timeout rows older runs
    /// recorded, and keeps verify failures.
    #[test]
    fn load_drops_turn_cap_and_timeout_patterns() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join(ERROR_PATTERNS_FILE);
        let mut store = ErrorPatternStore::empty();
        for class in ["verify", "turn_cap", "timeout"] {
            store.observe_gate_failure(GateFailureObservation::new(
                format!("{class}::digest"),
                "plan-1",
                Some("T1".to_string()),
                class,
                class,
                format!("{class} failure"),
                GateFailureSource::RetryClassifier,
            ));
        }
        assert_eq!(store.len(), 3);
        store.save(&path).expect("save");

        let loaded = ErrorPatternStore::load(&path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.top_patterns(1)[0].category, "verify");
    }

    /// backlog 4204: Runner-v2's pattern file is renamed aside, never
    /// deleted or imported, and a second call finds nothing to do.
    #[test]
    fn legacy_discovered_patterns_file_is_set_aside() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let legacy = tmp.path().join(LEGACY_DISCOVERED_PATTERNS_FILE);
        std::fs::write(&legacy, "{\"patterns\":{}}").expect("write the legacy file");

        assert!(retire_legacy_discovered_patterns(tmp.path()).expect("set aside"));
        assert!(!legacy.exists());
        let retired = tmp.path().join("discovered-patterns.json.v2-legacy");
        let kept = std::fs::read_to_string(&retired).expect("the file set aside");
        assert_eq!(kept, "{\"patterns\":{}}");
        let imported = tmp.path().join(ERROR_PATTERNS_FILE);
        assert!(!imported.exists(), "nothing imported");
        assert!(!retire_legacy_discovered_patterns(tmp.path()).expect("nothing to do"));
    }

    #[test]
    fn append_upserts_by_digest() {
        let mut store = ErrorPatternStore::empty();
        store.append(
            "error[E0433]: unresolved import",
            "unresolved_import",
            "plan-1",
            None,
        );
        store.append(
            "error[E0433]: unresolved import",
            "unresolved_import",
            "plan-2",
            Some("did you mean `std::io`?"),
        );
        store.append(
            "error[E0433]: unresolved import",
            "unresolved_import",
            "plan-2",
            None,
        );

        assert_eq!(store.len(), 1);
        let pattern = &store.patterns[0];
        assert_eq!(pattern.occurrences, 3);
        assert_eq!(
            pattern.plan_ids,
            BTreeSet::from(["plan-1".to_string(), "plan-2".to_string()])
        );
        assert_eq!(
            pattern.suggestion.as_deref(),
            Some("did you mean `std::io`?")
        );
    }

    #[test]
    fn top_patterns_sorts_by_occurrence() {
        let mut store = ErrorPatternStore::empty();
        store.append("rare error", "misc", "p1", None);

        store.append("common error", "misc", "p1", None);
        store.append("common error", "misc", "p2", None);
        store.append("common error", "misc", "p3", None);

        store.append("medium error", "misc", "p1", None);
        store.append("medium error", "misc", "p2", None);

        let top = store.top_patterns(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].digest, "common error");
        assert_eq!(top[0].occurrences, 3);
        assert_eq!(top[1].digest, "medium error");
        assert_eq!(top[1].occurrences, 2);
    }

    #[test]
    fn normalize_strips_paths() {
        let raw = "error[E0433]: failed to resolve: use of undeclared crate or module `foo` --> crates/roko-learn/src/lib.rs:42:10";
        let digest = normalize_error_digest(raw);
        assert!(
            !digest.contains("crates/roko-learn/src/lib.rs:42:10"),
            "file path should be replaced, got: {digest}"
        );
        assert!(
            digest.contains("<file>"),
            "should contain <file> placeholder, got: {digest}"
        );
        assert!(
            digest.contains("error[E0433]"),
            "error code should be preserved, got: {digest}"
        );
    }

    #[test]
    fn normalize_strips_ansi_codes() {
        let raw = "\x1b[1m\x1b[38;5;9merror[E0308]\x1b[0m: mismatched types";
        let digest = normalize_error_digest(raw);
        assert_eq!(digest, "error[E0308]: mismatched types");
    }

    #[test]
    fn normalize_collapses_whitespace_and_truncates() {
        let raw = format!("error:   too   many   spaces   {}", "x".repeat(300));
        let digest = normalize_error_digest(&raw);
        assert!(!digest.contains("  "), "should collapse whitespace");
        assert!(
            digest.chars().count() <= 200,
            "should truncate to 200 chars, got {}",
            digest.chars().count()
        );
    }

    #[test]
    fn save_load_roundtrip() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("error-patterns.json");

        let mut store = ErrorPatternStore::empty();
        store.append("digest-a", "type_mismatch", "plan-1", Some("try Into"));
        store.append("digest-b", "lifetime", "plan-2", None);

        store.save(&path).expect("save");
        let loaded = ErrorPatternStore::load(&path);

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.patterns[0].digest, "digest-a");
        assert_eq!(loaded.patterns[0].category, "type_mismatch");
        assert_eq!(loaded.patterns[0].suggestion.as_deref(), Some("try Into"));
        assert_eq!(loaded.patterns[1].digest, "digest-b");
        assert_eq!(loaded.patterns[1].category, "lifetime");
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("nonexistent.json");
        let store = ErrorPatternStore::load(&path);
        assert!(store.is_empty());
    }

    #[test]
    fn format_for_prompt_limits_output() {
        let mut store = ErrorPatternStore::empty();
        for i in 0..10 {
            for _ in 0..(10 - i) {
                store.append(&format!("error-{i}"), "misc", &format!("plan-{i}"), None);
            }
        }

        let formatted = store.format_for_prompt(3);
        let lines: Vec<&str> = formatted.lines().collect();

        // Header + 3 entries = 4 lines minimum.
        assert!(
            lines.len() >= 4,
            "expected at least 4 lines, got {}",
            lines.len()
        );
        assert!(formatted.contains("error-0"), "most frequent should appear");
        assert!(formatted.contains("error-1"));
        assert!(formatted.contains("error-2"));
        assert!(
            !formatted.contains("error-9"),
            "least frequent should be excluded"
        );
    }

    #[test]
    fn format_for_prompt_empty_store() {
        let store = ErrorPatternStore::empty();
        let formatted = store.format_for_prompt(5);
        assert!(formatted.is_empty());
    }

    #[test]
    fn format_for_prompt_includes_resolution_and_suggestion() {
        let mut store = ErrorPatternStore::empty();
        store.append("digest-r", "type_mismatch", "p1", Some("use `.into()`"));
        store.patterns[0].resolution = Some("Added explicit type annotation".to_string());

        let formatted = store.format_for_prompt(5);
        assert!(
            formatted.contains("Added explicit type annotation"),
            "resolution should appear"
        );
        assert!(
            formatted.contains("use `.into()`"),
            "suggestion should appear"
        );
    }

    #[test]
    fn patterns_for_category_filters() {
        let mut store = ErrorPatternStore::empty();
        store.append("error A", "type_mismatch", "p1", None);
        store.append("error B", "lifetime", "p1", None);
        store.append("error C", "type_mismatch", "p2", None);
        store.append("error D", "unresolved_import", "p3", None);

        let type_errors = store.patterns_for_category("type_mismatch");
        assert_eq!(type_errors.len(), 2);
        assert!(type_errors.iter().all(|p| p.category == "type_mismatch"));

        let lifetime_errors = store.patterns_for_category("lifetime");
        assert_eq!(lifetime_errors.len(), 1);
        assert_eq!(lifetime_errors[0].digest, "error B");

        let empty = store.patterns_for_category("nonexistent");
        assert!(empty.is_empty());
    }

    #[test]
    fn append_preserves_first_seen_timestamp() {
        // Two explicit, distinct times: two appends that read the clock can
        // land in the same tick under load.
        let first = Utc::now();
        let later = first + chrono::Duration::seconds(5);
        let mut store = ErrorPatternStore::empty();
        store.append_at("same-digest", "misc", "p1", None, first);

        // A later occurrence of the same pattern.
        store.append_at("same-digest", "misc", "p2", None, later);
        assert_eq!(
            store.patterns[0].first_seen_at,
            first.to_rfc3339(),
            "first_seen_at must not change on upsert"
        );
        assert_eq!(
            store.patterns[0].last_seen_at,
            later.to_rfc3339(),
            "last_seen_at must move to the later occurrence"
        );
    }

    #[test]
    fn append_does_not_overwrite_existing_suggestion() {
        let mut store = ErrorPatternStore::empty();
        store.append("d", "misc", "p1", Some("original hint"));
        store.append("d", "misc", "p2", Some("new hint"));

        assert_eq!(
            store.patterns[0].suggestion.as_deref(),
            Some("original hint"),
            "first suggestion wins"
        );
    }

    #[test]
    fn structured_observations_dedupe_by_key() {
        let mut store = ErrorPatternStore::empty();
        let first = GateFailureObservation::new(
            "E0425::src/lib.rs",
            "plan-a",
            Some("task-a".to_string()),
            "compile:cargo",
            "unresolved_import",
            "E0425: cannot find value `foo` [src/lib.rs]",
            GateFailureSource::GateClassification,
        );
        let second = GateFailureObservation::new(
            "E0425::src/lib.rs",
            "plan-b",
            Some("task-b".to_string()),
            "compile:cargo",
            "unresolved_import",
            "E0425: cannot find value `bar` [src/lib.rs]",
            GateFailureSource::GateClassification,
        );
        let different_file = GateFailureObservation::new(
            "E0425::src/other.rs",
            "plan-a",
            Some("task-a".to_string()),
            "compile:cargo",
            "unresolved_import",
            "E0425: cannot find value `foo` [src/other.rs]",
            GateFailureSource::GateClassification,
        );

        assert!(store.observe_gate_failure(first).inserted);
        let update = store.observe_gate_failure(second);
        assert!(!update.inserted);
        assert_eq!(update.occurrences, 2);
        assert!(store.observe_gate_failure(different_file).inserted);

        assert_eq!(store.len(), 2);
        assert_eq!(
            store.patterns[0].plan_ids,
            BTreeSet::from(["plan-a".to_string(), "plan-b".to_string()])
        );
        assert_eq!(
            store.patterns[0].task_ids,
            BTreeSet::from(["task-a".to_string(), "task-b".to_string()])
        );
    }

    #[test]
    fn bounded_summary_limits_patterns_and_size() {
        let mut store = ErrorPatternStore::empty();
        for i in 0..8 {
            let observation = GateFailureObservation::new(
                format!("E04{i:02}::src/lib.rs"),
                "plan-a",
                Some("task-a".to_string()),
                "compile:cargo",
                "type_error",
                format!("E04{i:02}: {}", "x".repeat(500)),
                GateFailureSource::GateClassification,
            );
            store.observe_gate_failure(observation);
        }

        let summary = store.bounded_summary(
            FailurePatternQuery {
                plan_id: Some("plan-a"),
                task_id: Some("task-a"),
                gate: Some("compile:cargo"),
                classification: Some("type_error"),
                ..FailurePatternQuery::default()
            },
            5,
            500,
        );

        assert!(summary.patterns.len() <= 5);
        assert!(
            summary
                .patterns
                .iter()
                .all(|p| p.digest.chars().count() <= 200)
        );
        let prompt = summary.format_for_prompt();
        assert!(prompt.contains("Prior Verify Failure Patterns"));
        assert!(!prompt.contains(&"x".repeat(500)));
    }

    #[test]
    fn summary_distinguishes_repeated_from_one_off() {
        let mut store = ErrorPatternStore::empty();
        let observation = GateFailureObservation::new(
            "test::panic::snapshot",
            "plan-a",
            Some("task-a".to_string()),
            "test:cargo",
            "test_expectation_failure",
            "snapshot mismatch",
            GateFailureSource::GateClassification,
        );
        store.observe_gate_failure(observation.clone());
        store.observe_gate_failure(observation);

        let summary = store.bounded_summary(
            FailurePatternQuery {
                gate: Some("test:cargo"),
                ..FailurePatternQuery::default()
            },
            3,
            1_000,
        );

        assert_eq!(summary.patterns.len(), 1);
        assert!(summary.patterns[0].repeated);
        assert!(summary.format_for_prompt().contains("repeated"));
    }
}
