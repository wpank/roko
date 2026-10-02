# 08-learning/01 -- Episode Logger

> Append-only JSONL pipeline, HDC fingerprinting, crash-safe persistence,
> and the raw data substrate for every downstream learning subsystem.

**Parent:** [08-LEARNING](../../08-LEARNING.md)

**Source:** `crates/roko-learn/src/episode_logger.rs`

**Persistence:** `.roko/episodes.jsonl` (append-only JSONL)

**Cross-references:** [cascade-router](cascade-router.md),
[pattern-discovery-trigram](pattern-discovery-trigram.md),
[task-metrics-and-baselines](task-metrics-and-baselines.md)

---

## 1. Purpose

The episode logger is the foundational data substrate for all learning in Roko.
Every agent turn -- regardless of outcome -- produces exactly one `Episode`
record appended to a JSONL file on disk. This append-only log is the raw
material from which every other learning subsystem draws its observations:
pattern discovery mines trigrams from episode sequences, the cascade router
updates bandit arms from episode outcomes, the regression detector computes
baselines from episode metrics, and the skill library extracts reusable
capabilities from successful episodes.

The design prioritizes durability and simplicity over query performance.
Episodes are never modified in place. Concurrent writers are serialized through
a process-wide mutex. The reader is tolerant: lines that fail to parse (a
common outcome of a crash mid-write or of forward-compatible schema changes)
are surfaced through a dedicated error variant rather than corrupting the whole
stream.

---

## 2. Episode Schema

The canonical `Episode` struct captures the full context of a single agent
turn:

```rust
pub struct Episode {
    /// Unique episode identifier (UUID v4).
    pub id: String,
    /// Agent identifier that produced this episode.
    pub agent_id: String,
    /// Task identifier this episode belongs to.
    pub task_id: String,
    /// Plan identifier containing the task.
    pub plan_id: String,
    /// Agent role (e.g. "Implementer", "Reviewer").
    pub role: String,
    /// Model slug used for this turn (e.g. "claude-sonnet-4-20250514").
    pub model: String,
    /// Backend provider (e.g. "anthropic", "openrouter").
    pub backend: String,
    /// Whether the episode ended in a successful gate pass.
    pub success: bool,
    /// Zero-based iteration index within the task.
    pub iteration: u32,
    /// Input token count from the provider response.
    pub input_tokens: u64,
    /// Output token count from the provider response.
    pub output_tokens: u64,
    /// Actual cost in USD after cache discounts.
    pub cost_usd: f64,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Gate verdicts produced by the verification pipeline.
    pub gate_verdicts: Vec<GateVerdict>,
    /// Timestamp when the episode was recorded.
    pub timestamp: DateTime<Utc>,
    /// 10,240-bit HDC fingerprint of the episode content.
    pub hdc_fingerprint: Option<HdcVector>,
    /// Free-form metadata map (capped at 16 KB serialized).
    pub extra: HashMap<String, Value>,
}
```

### 2.1 GateVerdict

Each gate execution within an episode produces a verdict:

```rust
pub struct GateVerdict {
    /// Gate identifier ("compile", "test", "lint", "diff", etc.).
    pub gate: String,
    /// Whether the gate passed.
    pub passed: bool,
    /// Optional short diagnostic (hashed, never raw output).
    pub signature: Option<String>,
}
```

The `signature` field stores a content hash of the error output rather than the
raw text. This serves two purposes: it keeps the log compact (error outputs can
be megabytes), and it enables exact-match deduplication across episodes without
exposing potentially sensitive build output.

---

## 3. Append Pipeline

The append path is designed for crash-safety and concurrency:

```
Agent Turn Completes
    |
    v
EpisodeLogger::append(&episode)
    |
    +-- 1. Validate: extra field <= MAX_EXTRA_BYTES (16 KB)
    |       -> LoggerError::ExtraTooLarge if exceeded
    |
    +-- 2. Compute HDC fingerprints:
    |       text_fingerprint: roko_primitives::hdc::text_fingerprint(content)
    |       metadata_fingerprint: text_fingerprint(agent_id + task_id + role)
    |       -> stored in episode.extra["text_fingerprint"] and
    |         episode.extra["metadata_fingerprint"]
    |
    +-- 3. Serialize: serde_json::to_string(&episode) + "\n"
    |
    +-- 4. Acquire process-wide parking_lot::Mutex
    |
    +-- 5. Open file with O_APPEND | O_CREAT
    |
    +-- 6. Write serialized line
    |
    +-- 7. Release mutex
```

The `MAX_EXTRA_BYTES` guard (16 KB) prevents a runaway optimizer from blowing
up the log by stuffing arbitrary data into the `extra` map. This is a hard
limit enforced at write time -- the episode is rejected with
`LoggerError::ExtraTooLarge` if the serialized `extra` field exceeds 16,384
bytes.

### 3.1 Concurrency Model

The logger uses `parking_lot::Mutex` (not `tokio::Mutex`) for the write
serialization lock. This is deliberate: the critical section is a single
`write_all` syscall, which is fast enough that the synchronous mutex avoids the
overhead of task scheduling. The mutex is process-wide (held by the
`EpisodeLogger` instance), so concurrent agent tasks within the same process
are serialized, while separate processes append independently (the OS
guarantees atomicity for `O_APPEND` writes below `PIPE_BUF`).

---

## 4. HDC Fingerprinting

Every episode is fingerprinted with a 10,240-bit hyperdimensional computing
(HDC) vector from `roko-primitives::hdc`. Two fingerprints are computed:

1. **Text fingerprint** -- encodes the semantic content of the episode (task
   description, gate verdicts) into a binary vector via `text_fingerprint`.
2. **Metadata fingerprint** -- encodes structural identity (agent_id, task_id,
   role) for structural similarity matching.

HDC fingerprints enable sub-microsecond similarity search: comparing two
10,240-bit vectors via Hamming distance takes ~50ns, compared to ~1us for
cosine distance on 768-dimensional float embeddings. This speed advantage is
critical for real-time pattern matching during task dispatch, where the system
must scan hundreds of historical episodes to find relevant patterns before the
agent begins work.

The fingerprints are stored in the `extra` map under reserved keys
(`text_fingerprint` and `metadata_fingerprint`) rather than as top-level
fields. This keeps the `Episode` struct backward-compatible with older log
entries that predate HDC support.

### 4.1 Template Suggestion

The episode logger supports template suggestion via HDC similarity. Given a new
task context, the system can scan recent episodes (within
`TEMPLATE_SUGGESTION_MAX_AGE_DAYS` = 30 days, up to
`TEMPLATE_SUGGESTION_MAX_CANDIDATES` = 256 candidates) and find episodes with
HDC similarity above `TEMPLATE_SUGGESTION_MIN_SIMILARITY` = 0.7. Successful
episodes matching this threshold can be used to suggest prompt templates or
skill patterns for the new task.

---

## 5. Reading and Tolerance

The reader is designed to be tolerant of corruption:

```rust
impl EpisodeLogger {
    pub async fn read_all(path: impl AsRef<Path>) -> Result<Vec<Episode>, LoggerError> {
        // Opens file, reads line-by-line
        // Each line: serde_json::from_str::<Episode>(line)
        // On parse failure: LoggerError::Parse { line, source }
        // Caller decides whether to skip or abort
    }
}
```

The `LoggerError::Parse` variant includes the 1-based line number and the
`serde_json` diagnostic, so callers can decide whether to skip corrupt lines or
abort. In practice, the `LearningRuntime` skips corrupt lines and logs a
warning -- this is the right default for a system that must remain operational
even after a crash mid-write.

### 5.1 Why JSONL

| Format | Append-safe | Schema-flexible | Grep-friendly | Corruption-isolated |
|--------|-------------|-----------------|---------------|---------------------|
| JSONL  | Yes         | Yes             | Yes           | Yes (per-line)      |
| SQLite | No (WAL)    | Limited         | No            | No (whole-DB)       |
| Parquet| No          | Limited         | No            | No (whole-file)     |
| CSV    | Yes         | No              | Yes           | Yes (per-line)      |

JSONL's key advantage is corruption isolation: a crash during write corrupts at
most one line. The next line is a fresh JSON object that parses independently.
This property is essential for an append-only log that may be written to during
agent crashes, OOM kills, or power failures.

---

## 6. Retention and Compaction

The current implementation does not compact or rotate the episode log. For a
system running 100 tasks per day at ~2 KB per episode, this produces ~200
KB/day or ~73 MB/year -- well within filesystem limits.

Future compaction strategies under consideration:

1. **Time-based rotation** -- archive episodes older than 90 days to a
   compressed file, keeping the active log small for fast `read_all` scans.
2. **Summary compaction** -- replace old episodes with aggregate summaries
   (pass rate, cost distribution, pattern counts) that preserve learning signal
   without individual records.
3. **HDC compaction** -- merge similar episodes into a single representative
   episode using HDC superposition (element-wise majority of fingerprint bits),
   reducing storage while preserving the similarity search index.

### 6.1 Tiered Architecture (Target)

```
Hot tier   (0-7 days)   -> episodes.jsonl          Raw JSONL, full fidelity
Warm tier  (7-90 days)  -> episodes-warm.jsonl.zst  Zstandard compressed
Cold tier  (90+ days)   -> episodes-cold.bin         HDC superposition summaries
```

The cold tier achieves extreme compression by exploiting HDC superposition:
merging N episode fingerprints into a single 10,240-bit vector preserves
statistical properties (similarity search still works on the superposition)
while discarding individual records. This is analogous to the "compressed
sensing" property of high-dimensional random projections
(Johnson-Lindenstrauss lemma).

---

## 7. Episode Importance Scoring

Not all episodes are equally valuable for learning. Episode importance scoring
quantifies this, inspired by prioritized experience replay (Schaul et al. 2016)
and the information-theoretic concept of surprisal.

### 7.1 Composite Scoring

```
importance = 0.30 * surprisal
           + 0.20 * novelty
           + 0.15 * difficulty_signal
           + 0.20 * information_gain
           + 0.15 * diversity
```

| Component | Definition |
|-----------|-----------|
| Surprisal | abs(predicted_probability - actual_outcome) |
| Novelty | HDC Hamming distance to nearest neighbor in last 100 episodes |
| Difficulty signal | abs(complexity_adjusted_expected_rate - actual_outcome) |
| Information gain | Approximate gradient magnitude for bandit updates |
| Diversity | 1.0 / sqrt(count_in_same_slice) |

### 7.2 Applications

| Consumer | How importance is used |
|----------|----------------------|
| Pattern discovery | Weight trigram support by episode importance |
| Skill extraction | Prioritize extraction from high-importance successes |
| Cascade router | Weight bandit updates by importance |
| Compaction | Keep high-importance episodes in hot tier longer |
| Dashboard | Surface high-importance episodes as "Notable events" |

---

## 8. Integration with LearningRuntime

The episode logger is the first subsystem updated by
`LearningRuntime::record_completed_run()`. The runtime constructs an `Episode`
from the `CompletedRunInput` payload and appends it before updating any
downstream subsystem:

```
CompletedRunInput
    |
    +-- 1. EpisodeLogger::append(episode)          <-- you are here
    +-- 2. CostsLog::append(cost_record)
    +-- 3. PlaybookStore::record_outcome()
    +-- 4. PlaybookRules::validate() / contradict()
    +-- 5. SkillLibrary::record_use()
    +-- 6. TaskMetric -> regression history
    +-- 7. ExperimentStore::record_outcome()
    +-- 8. (removed: PatternMiner::ingest_episode(), gap-4adfa7)
    +-- 9. CascadeRouter::update()
    +-- 10. CFactor::compute()
```

This ordering ensures that the raw episode is always persisted before any
derived computation runs. If the process crashes during step 5, the episode is
already on disk and can be replayed on restart to reconstruct downstream state.

---

## 9. Error Handling

| Variant | When | Recovery |
|---------|------|----------|
| `LoggerError::Io` | Filesystem call failed | Retry or alert operator |
| `LoggerError::Serde` | Serialization failed | Fix the caller (programming error) |
| `LoggerError::ExtraTooLarge` | `extra` map exceeds 16 KB | Trim `extra` before appending |
| `LoggerError::Parse` | JSONL line corrupt | Skip the line and continue reading |

The `Parse` variant is the most common in practice. It occurs when a crash
interrupted a write, when a schema change added new fields, or when manual
editing introduced syntax errors. The tolerant reader handles all three by
surfacing the error with the line number, letting the caller decide.
