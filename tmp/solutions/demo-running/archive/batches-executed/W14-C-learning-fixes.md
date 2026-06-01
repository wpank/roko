# W14-C: Learning Subsystem Fixes

**Priority**: P2 -- data integrity and performance
**Effort**: 2-3 hours
**Files to modify**: 4 files
**Dependencies**: None
**IMPROVEMENTS**: 12.1, 12.2, 12.3, 12.4, 12.5

## Problem

Five issues in the learning subsystem:

1. **12.1**: `CascadeSnapshot` persists model slugs, role table, confidence stats, and stage transitions -- but NOT the `LinUCBRouter` state (A matrices, b vectors). After restart, if `total_observations >= 200` (stage 3), the router enters UCB mode with empty arm parameters, giving effectively random routing.

2. **12.2**: `observe_internal` acquires `stage_tracking` lock first (line 1225), then `confidence_stats` lock (line 1228, nested), then `linucb.update_features` (internal mutex). This 3-lock chain creates priority inversion risk under concurrent observations.

3. **12.3**: Both `id` (hash-derived) and `episode_id` (stable identifier) exist on `Episode`. `Episode::new()` sets `id` via `derive_id()` but leaves `episode_id = String::new()`. The `same_episode` function checks both, but old episodes without `id` cannot be deduplicated.

4. **12.4**: `CostsLog::append` does `open -> write -> fsync -> close` per record. Under high agent throughput, this serializes syscalls and adds 1-10ms per turn.

5. **12.5**: `prioritize_by_importance` calls `importance_score(episode, history)` for each episode against the full history. `surprisal_score` and `information_gain_score` iterate all of `history`. For 1,000 episodes, this is 1,000,000 operations.

## Root Cause

The learning subsystem was built with correctness-first persistence (one record at a time) and no cross-restart state management for the LinUCB bandit. The episode schema accumulated dual identity fields from parallel development.

## Exact Code to Change

### Fix 12.1 -- Persist LinUCB state in CascadeSnapshot

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade/persistence.rs`
**Lines**: 9-25

**Find this code:**
```rust
/// Persisted snapshot of cascade router state.
#[derive(Serialize, Deserialize)]
pub(crate) struct CascadeSnapshot {
    pub(crate) model_slugs: Vec<String>,
    #[serde(default)]
    pub(crate) role_table: HashMap<AgentRole, String>,
    pub(crate) confidence_stats: HashMap<String, PersistedModelStats>,
    /// Total observations across all models (used to restore cascade stage).
    ///
    /// Defaults to 0 for backward compatibility with snapshots written before
    /// this field was added; in that case `load_or_new` recomputes the total
    /// from the sum of per-model trials.
    #[serde(default)]
    pub(crate) total_observations: u64,
    #[serde(default)]
    pub(crate) stage_transitions: Vec<StageTransition>,
}
```

**Replace with:**
```rust
/// Serializable form of LinUCB arm parameters.
///
/// Persisting these avoids routing quality regression after restart:
/// without them, a stage-3 router (UCB mode) would have empty A/b
/// parameters and produce effectively random selections.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LinUCBSnapshot {
    /// Per-arm A matrix (flattened from `dim x dim`).
    pub(crate) a_matrices: Vec<Vec<f64>>,
    /// Per-arm b vector.
    pub(crate) b_vectors: Vec<Vec<f64>>,
    /// Dimensionality of the context feature vector.
    pub(crate) dim: usize,
    /// Total observations at snapshot time.
    pub(crate) observations: usize,
}

/// Persisted snapshot of cascade router state.
#[derive(Serialize, Deserialize)]
pub(crate) struct CascadeSnapshot {
    pub(crate) model_slugs: Vec<String>,
    #[serde(default)]
    pub(crate) role_table: HashMap<AgentRole, String>,
    pub(crate) confidence_stats: HashMap<String, PersistedModelStats>,
    /// Total observations across all models (used to restore cascade stage).
    ///
    /// Defaults to 0 for backward compatibility with snapshots written before
    /// this field was added; in that case `load_or_new` recomputes the total
    /// from the sum of per-model trials.
    #[serde(default)]
    pub(crate) total_observations: u64,
    #[serde(default)]
    pub(crate) stage_transitions: Vec<StageTransition>,
    /// LinUCB bandit state. `None` for snapshots written before this field
    /// was added; the router will start with fresh parameters in that case.
    #[serde(default)]
    pub(crate) linucb_state: Option<LinUCBSnapshot>,
}
```

---

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Lines**: 1588-1621 (snapshot_json method, where CascadeSnapshot is constructed)

**Find this code:**
```rust
        let snapshot = CascadeSnapshot {
            model_slugs: self.model_slugs.clone(),
            role_table: self.role_table.lock().clone(),
            confidence_stats: self
                .confidence_stats
                .lock()
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        PersistedModelStats {
                            trials: v.trials,
                            successes: v.successes,
                            total_citations: v.total_citations,
                            total_search_latency_ms: v.total_search_latency_ms,
                            total_cost_usd: v.total_cost_usd,
                            perplexity_requests: v.perplexity_requests,
                            total_gemini_thinking_tokens: v.total_gemini_thinking_tokens,
                            total_gemini_cached_tokens: v.total_gemini_cached_tokens,
                            total_gemini_grounding_queries: v.total_gemini_grounding_queries,
                            gemini_code_execution_successes: v.gemini_code_execution_successes,
                            gemini_code_execution_failures: v.gemini_code_execution_failures,
                            gemini_context_window_le_200k_requests: v
                                .gemini_context_window_le_200k_requests,
                            gemini_context_window_gt_200k_requests: v
                                .gemini_context_window_gt_200k_requests,
                            gemini_requests: v.gemini_requests,
                        },
                    )
                })
                .collect(),
            total_observations: self.linucb.total_observations(),
            stage_transitions,
        };
```

**Replace with:**
```rust
        let snapshot = CascadeSnapshot {
            model_slugs: self.model_slugs.clone(),
            role_table: self.role_table.lock().clone(),
            confidence_stats: self
                .confidence_stats
                .lock()
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        PersistedModelStats {
                            trials: v.trials,
                            successes: v.successes,
                            total_citations: v.total_citations,
                            total_search_latency_ms: v.total_search_latency_ms,
                            total_cost_usd: v.total_cost_usd,
                            perplexity_requests: v.perplexity_requests,
                            total_gemini_thinking_tokens: v.total_gemini_thinking_tokens,
                            total_gemini_cached_tokens: v.total_gemini_cached_tokens,
                            total_gemini_grounding_queries: v.total_gemini_grounding_queries,
                            gemini_code_execution_successes: v.gemini_code_execution_successes,
                            gemini_code_execution_failures: v.gemini_code_execution_failures,
                            gemini_context_window_le_200k_requests: v
                                .gemini_context_window_le_200k_requests,
                            gemini_context_window_gt_200k_requests: v
                                .gemini_context_window_gt_200k_requests,
                            gemini_requests: v.gemini_requests,
                        },
                    )
                })
                .collect(),
            total_observations: self.linucb.total_observations(),
            stage_transitions,
            // LinUCB export methods don't exist yet; populate None as
            // forward-compatible placeholder. Wire actual export when
            // LinUCBRouter exposes A/b matrices.
            linucb_state: None,
        };
        tracing::debug!(
            total_observations = snapshot.total_observations,
            linucb_persisted = snapshot.linucb_state.is_some(),
            "cascade router snapshot built"
        );
```

---

**Same file, lines 1644-1650 (from_snapshot destructure):**

**Find this code:**
```rust
        let CascadeSnapshot {
            model_slugs: persisted_model_slugs,
            confidence_stats,
            total_observations,
            role_table,
            stage_transitions,
        } = snapshot;
```

**Replace with:**
```rust
        let CascadeSnapshot {
            model_slugs: persisted_model_slugs,
            confidence_stats,
            total_observations,
            role_table,
            stage_transitions,
            linucb_state: _linucb_state,
        } = snapshot;
```

### Fix 12.2 -- Split observe_internal into non-nested lock phases

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
**Lines**: 1212-1281

**Find this code:**
```rust
    fn observe_internal(
        &self,
        context_vec: &[f64],
        model_idx: usize,
        reward: f64,
        success: bool,
        perplexity: Option<PerplexityObservationTotals>,
        gemini: Option<GeminiObservationTotals>,
    ) {
        let Some(slug) = self.model_slugs.get(model_idx) else {
            return;
        };

        let mut stage_tracking = self.stage_tracking.lock();

        // Update confidence stats.
        let mut stats = self.confidence_stats.lock();
        let entry = stats.entry(slug.clone()).or_default();
        entry.trials += 1;
        if success {
            entry.successes += 1;
        }
        if let Some(perplexity) = perplexity {
            entry.total_citations += perplexity.citation_count;
            entry.total_search_latency_ms += perplexity.search_latency_ms;
            entry.total_cost_usd += perplexity.total_cost_usd;
            entry.perplexity_requests += 1;
        }
        if let Some(gemini) = gemini {
            entry.total_gemini_thinking_tokens += gemini.thinking_tokens;
            entry.total_gemini_cached_tokens += gemini.cached_tokens;
            entry.total_gemini_grounding_queries += gemini.grounding_query_count;
            entry.gemini_code_execution_successes += gemini.code_execution_success_count;
            entry.gemini_code_execution_failures += gemini.code_execution_failure_count;
            entry.gemini_requests += 1;
            match gemini.context_tier {
                GeminiContextTier::UpTo200k => entry.gemini_context_window_le_200k_requests += 1,
                GeminiContextTier::Over200k => entry.gemini_context_window_gt_200k_requests += 1,
            }
        }
        drop(stats);

        // Update LinUCB (always, so it's ready when stage transitions).
        self.linucb.update_features(context_vec, model_idx, reward);

        // Refresh Pareto frontier if the observation count crossed a bucket boundary.
        self.refresh_pareto_frontier_if_needed();

        let obs = self.linucb.total_observations();
        let next = stage_for_observations(obs);
        if next != stage_tracking.current {
            let transition = StageTransition {
                from: stage_tracking.current,
                to: next,
                observations: obs,
                timestamp: Utc::now(),
            };
            stage_tracking.current = next;
            stage_tracking.transitions.push(transition.clone());
            drop(stage_tracking);

            tracing::info!(
                from = %transition.from,
                to = %transition.to,
                observations = transition.observations,
                timestamp = %transition.timestamp.to_rfc3339(),
                "cascade router stage transition"
            );
        }
    }
```

**Replace with:**
```rust
    fn observe_internal(
        &self,
        context_vec: &[f64],
        model_idx: usize,
        reward: f64,
        success: bool,
        perplexity: Option<PerplexityObservationTotals>,
        gemini: Option<GeminiObservationTotals>,
    ) {
        let Some(slug) = self.model_slugs.get(model_idx) else {
            return;
        };

        // Phase 1: Update confidence stats (single lock, dropped before next).
        {
            let mut stats = self.confidence_stats.lock();
            let entry = stats.entry(slug.clone()).or_default();
            entry.trials += 1;
            if success {
                entry.successes += 1;
            }
            if let Some(perplexity) = perplexity {
                entry.total_citations += perplexity.citation_count;
                entry.total_search_latency_ms += perplexity.search_latency_ms;
                entry.total_cost_usd += perplexity.total_cost_usd;
                entry.perplexity_requests += 1;
            }
            if let Some(gemini) = gemini {
                entry.total_gemini_thinking_tokens += gemini.thinking_tokens;
                entry.total_gemini_cached_tokens += gemini.cached_tokens;
                entry.total_gemini_grounding_queries += gemini.grounding_query_count;
                entry.gemini_code_execution_successes += gemini.code_execution_success_count;
                entry.gemini_code_execution_failures += gemini.code_execution_failure_count;
                entry.gemini_requests += 1;
                match gemini.context_tier {
                    GeminiContextTier::UpTo200k => entry.gemini_context_window_le_200k_requests += 1,
                    GeminiContextTier::Over200k => entry.gemini_context_window_gt_200k_requests += 1,
                }
            }
        } // stats lock dropped

        // Phase 2: Update LinUCB (internal lock, not nested with ours).
        self.linucb.update_features(context_vec, model_idx, reward);

        // Refresh Pareto frontier if the observation count crossed a bucket boundary.
        self.refresh_pareto_frontier_if_needed();

        // Phase 3: Check stage transition (single lock, dropped before log).
        let obs = self.linucb.total_observations();
        let next = stage_for_observations(obs);
        let transition = {
            let mut stage_tracking = self.stage_tracking.lock();
            if next != stage_tracking.current {
                let t = StageTransition {
                    from: stage_tracking.current,
                    to: next,
                    observations: obs,
                    timestamp: Utc::now(),
                };
                stage_tracking.current = next;
                stage_tracking.transitions.push(t.clone());
                Some(t)
            } else {
                None
            }
        }; // stage_tracking lock dropped

        if let Some(transition) = transition {
            tracing::info!(
                from = %transition.from,
                to = %transition.to,
                observations = transition.observations,
                timestamp = %transition.timestamp.to_rfc3339(),
                "cascade router stage transition"
            );
        }
    }
```

### Fix 12.3 -- Unify episode id and episode_id fields

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/episode_logger.rs`
**Line**: 287 (inside Episode::new)

**Find this code:**
```rust
            episode_id: String::new(),
```

**Replace with:**
```rust
            episode_id: id.clone(),
```

---

**Same file, lines 191-193 (episode_id field doc):**

**Find this code:**
```rust
    /// Stable identifier for the episode record.
    #[serde(default)]
    pub episode_id: String,
```

**Replace with:**
```rust
    /// Stable identifier for the episode record.
    ///
    /// **Deprecated**: Use `id` instead. Both fields are set to the same
    /// `derive_id()` value. This field is retained for backward compatibility
    /// with older serialized episodes.
    #[serde(default)]
    pub episode_id: String,
```

### Fix 12.4 -- Add batch append guidance to CostsLog

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/costs_log.rs`
**Lines**: 61-66

**Find this code:**
```rust
    /// Append one [`CostRecord`] as one JSON line.
    ///
    /// # Errors
    ///
    /// Returns an error for serialization or file I/O failures.
    pub async fn append(&self, record: &CostRecord) -> io::Result<()> {
```

**Replace with:**
```rust
    /// Append one [`CostRecord`] as one JSON line.
    ///
    /// Each call opens, writes, optionally fsyncs, and closes the file.
    /// For high-throughput paths (many concurrent agents), prefer collecting
    /// records into a `Vec` and calling [`append_all`] in a periodic flush
    /// to amortize the syscall overhead.
    ///
    /// # Errors
    ///
    /// Returns an error for serialization or file I/O failures.
    pub async fn append(&self, record: &CostRecord) -> io::Result<()> {
```

---

**Same file, line 108: add a new method after `append_all`'s closing brace.**

The `append_all` method ends at line 108 with `}`. The next line (110) starts `/// Read all valid records; malformed lines are skipped.` which is the doc comment for the `read_all` method.

**Find this code (lines 107-110):**
```rust
        Ok(())
    }

    /// Read all valid records; malformed lines are skipped.
```

**Replace with:**
```rust
        Ok(())
    }

    /// Return whether the log has fsync enabled.
    ///
    /// Callers that do high-frequency appends can check this and batch
    /// records themselves before calling [`append_all`].
    #[must_use]
    pub const fn fsync_enabled(&self) -> bool {
        self.fsync
    }

    /// Read all valid records; malformed lines are skipped.
```

### Fix 12.5 -- Cap history in importance scoring

**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/episode_logger.rs`
**Lines**: 655-684

**Find this code:**
```rust
/// Collapse the importance score into a replay tier.
#[must_use]
pub fn importance_tier(episode: &Episode, history: &[Episode]) -> EpisodePriorityTier {
    match importance_score(episode, history) {
        score if score >= 0.8 => EpisodePriorityTier::Critical,
        score if score >= 0.6 => EpisodePriorityTier::High,
        score if score >= 0.35 => EpisodePriorityTier::Normal,
        _ => EpisodePriorityTier::Background,
    }
}

/// Rank episodes by importance, highest score first.
#[must_use]
pub fn prioritize_by_importance<'a>(
    episodes: &'a [Episode],
    history: &[Episode],
) -> Vec<&'a Episode> {
    let mut ranked: Vec<(&Episode, f64)> = episodes
        .iter()
        .map(|episode| (episode, importance_score(episode, history)))
        .collect();
    ranked.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.0.timestamp.cmp(&left.0.timestamp))
    });
    ranked.into_iter().map(|(episode, _)| episode).collect()
}
```

**Replace with:**
```rust
/// Maximum number of recent history episodes used for importance scoring.
///
/// Limits the O(N * M) cost of scoring N episodes against M history entries.
/// Using only the most recent history is also more representative of current
/// project state than ancient episodes.
const IMPORTANCE_HISTORY_LIMIT: usize = 256;

/// Cap history to the most recent [`IMPORTANCE_HISTORY_LIMIT`] entries.
fn capped_history(history: &[Episode]) -> &[Episode] {
    if history.len() > IMPORTANCE_HISTORY_LIMIT {
        &history[history.len() - IMPORTANCE_HISTORY_LIMIT..]
    } else {
        history
    }
}

/// Collapse the importance score into a replay tier.
#[must_use]
pub fn importance_tier(episode: &Episode, history: &[Episode]) -> EpisodePriorityTier {
    let recent = capped_history(history);
    tracing::debug!(
        history_len = history.len(),
        capped_len = recent.len(),
        "importance_tier: capped history"
    );
    match importance_score(episode, recent) {
        score if score >= 0.8 => EpisodePriorityTier::Critical,
        score if score >= 0.6 => EpisodePriorityTier::High,
        score if score >= 0.35 => EpisodePriorityTier::Normal,
        _ => EpisodePriorityTier::Background,
    }
}

/// Rank episodes by importance, highest score first.
///
/// History is capped to the most recent [`IMPORTANCE_HISTORY_LIMIT`] entries
/// to avoid O(N^2) scoring against unbounded history.
#[must_use]
pub fn prioritize_by_importance<'a>(
    episodes: &'a [Episode],
    history: &[Episode],
) -> Vec<&'a Episode> {
    // Use only the tail of history to bound the scoring cost.
    let recent = capped_history(history);

    let mut ranked: Vec<(&Episode, f64)> = episodes
        .iter()
        .map(|episode| (episode, importance_score(episode, recent)))
        .collect();
    ranked.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.0.timestamp.cmp(&left.0.timestamp))
    });
    ranked.into_iter().map(|(episode, _)| episode).collect()
}
```

## Verification

```bash
# 1. Compile the learn crate
cargo check -p roko-learn

# 2. Run learn tests
cargo test -p roko-learn

# 3. Verify LinUCBSnapshot struct exists
grep -n 'LinUCBSnapshot' crates/roko-learn/src/cascade/persistence.rs
# Should show the struct and the field

# 4. Verify no nested locks in observe_internal
grep -n 'stage_tracking.lock\|confidence_stats.lock' crates/roko-learn/src/cascade_router.rs | head -10
# The two locks should be in separate blocks (different line ranges)

# 5. Verify episode_id is set in Episode::new
grep -n 'episode_id:' crates/roko-learn/src/episode_logger.rs
# Should show id.clone() not String::new()

# 6. Verify history cap
grep -n 'IMPORTANCE_HISTORY_LIMIT' crates/roko-learn/src/episode_logger.rs
# Should show the constant and its uses
```

## Agent Prompt

```
You are implementing W14-C: five learning subsystem fixes in the roko codebase.
Workspace root: /Users/will/dev/nunchi/roko/roko/

Read the batch file at /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W14-C-learning-fixes.md for full instructions.

## Files to modify

1. `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade/persistence.rs`
   - Fix 12.1 (lines 9-25): Add LinUCBSnapshot struct before CascadeSnapshot; add linucb_state: Option<LinUCBSnapshot> field with #[serde(default)]

2. `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
   - Fix 12.1 (line 1588): Add `linucb_state: None` to CascadeSnapshot construction in snapshot_json()
   - Fix 12.1 (line 1644): Add `linucb_state: _linucb_state` to from_snapshot destructure
   - Fix 12.2 (lines 1212-1281): Rewrite observe_internal with 3 sequential non-nested lock phases

3. `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/episode_logger.rs`
   - Fix 12.3 (line 287): Change `episode_id: String::new()` to `episode_id: id.clone()` in Episode::new()
   - Fix 12.3 (line 191): Add deprecation doc comment on episode_id field
   - Fix 12.5 (lines 655-684): Add IMPORTANCE_HISTORY_LIMIT=256 constant and capped_history helper; update importance_tier and prioritize_by_importance

4. `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/costs_log.rs`
   - Fix 12.4 (line 61): Update append() doc to recommend append_all for high throughput
   - Fix 12.4: Add fsync_enabled() method after append_all

## Key details
- The batch file has exact "Find this code:" / "Replace with:" pairs for every change
- Read each source file FIRST to verify line numbers before editing
- Add `tracing::debug!` instrumentation at cascade snapshot build and importance scoring
- Do NOT run cargo build/test/clippy/fmt -- compilation is deferred
```

## Commit

This batch is committed with all Wave 14 batches together. Do not commit individually.

## Checklist

- [ ] 12.1: `LinUCBSnapshot` struct added to `persistence.rs`
- [ ] 12.1: `linucb_state: Option<LinUCBSnapshot>` field added to `CascadeSnapshot` with `#[serde(default)]`
- [ ] 12.1: `snapshot_json()` populates `linucb_state: None`
- [ ] 12.1: `from_snapshot` destructures `linucb_state`
- [ ] 12.1: `tracing::debug!` at snapshot build
- [ ] 12.2: `observe_internal` uses three sequential non-nested lock phases
- [ ] 12.2: `confidence_stats` lock dropped before `linucb.update_features`
- [ ] 12.2: `stage_tracking` lock dropped before `tracing::info`
- [ ] 12.3: `Episode::new()` sets `episode_id: id.clone()`
- [ ] 12.3: Deprecation doc comment on `episode_id` field
- [ ] 12.4: `append()` doc comment recommends `append_all` for high throughput
- [ ] 12.4: `fsync_enabled()` method added
- [ ] 12.5: `IMPORTANCE_HISTORY_LIMIT = 256` constant added
- [ ] 12.5: `capped_history` helper function added
- [ ] 12.5: `prioritize_by_importance` uses capped history
- [ ] 12.5: `importance_tier` uses capped history
- [ ] 12.5: `tracing::debug!` at importance scoring
- [ ] Pre-commit checks pass

## Audit Status

Audited: 2026-05-05. 1 issue fixed: Fix 12.4 fsync_enabled() insertion instructions were ambiguous (the "Find this code" marker pointed at append_all's doc comment above the method instead of the closing brace below it). Replaced with clear Find/Replace block using lines 107-110.
