# PERF_05: Adopt FileSubstrate::put_batch everywhere (B10)

## Task

`FileSubstrate::put_batch` already exists at
`crates/roko-fs/src/file_substrate.rs:137`. Most of the `run.rs` hot path
already uses it. Audit the remaining `substrate.put(...)` call sites,
batch the eligible ones, and add a partial-write replay test that
documents crash-safety.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_05](../ISSUE-TRACKER.md#perf_05)
- Plan: `tmp/solutions/perf/implementation/05-batch-substrate-writes.md`
- Bottleneck: B10 (BOTTLENECK-ANALYSIS.md §B10)
- Performance contract: **C-5** (no ≥3 sequential single-puts in production)
- Priority: P2
- Effort: ≈2 h
- Depends on: none
- Wave: 1

## Problem

`crates/roko-fs/src/file_substrate.rs:137`:

```rust
pub async fn put_batch(&self, signals: Vec<Engram>) -> Result<Vec<ContentHash>> {
    // Phase 1: deduplicate and serialize without holding the write lock.
    // Phase 2: single lock acquire -> write -> flush.
    // Phase 3: update the in-memory index after the write succeeds.
    // ...
}
```

This API is already optimised. Today's `crates/roko-cli/src/run.rs`
already uses it at lines 1156, 1180, 1188, 1245, 2872, 2901.

The remaining single-shot `substrate.put(...)` call sites — typically
in iteration-over-verdicts patterns — should be coalesced. Each `put`:

```text
serde_json::to_string : ~1 ms
File append           : ~3 ms
flush                 : ~4 ms
                       = ~8 ms × N
```

For N=10 that's 80 ms; one `put_batch` does it in ~12 ms.

## Exact Changes

### Step 1 — Run the audit grep

```bash
rg -n '\.put\(' crates/ --type rust \
  | rg -v 'put_batch|tests|test\.rs|/tests/'
```

For each result, classify:

- **Already in a batch context** → leave alone.
- **Eligible to batch** (≥3 sequential `put` calls in the same function
  with no observable side effect between them) → convert.
- **Genuinely single-shot** (one put per function call, or with an
  awaitable side effect between puts that observers depend on) →
  annotate with `// SAFETY-ORDER: ...` comment explaining why batching
  would change semantics.

### Step 2 — Convert eligible call sites

Typical offender pattern:

```rust
// BEFORE:
substrate.put(gate_input.clone()).await?;
for verdict in verdicts {
    let sig = build_verdict_signal(verdict);
    substrate.put(sig).await?;
}
```

Convert to:

```rust
// AFTER:
let mut batch = Vec::with_capacity(1 + verdicts.len());
batch.push(gate_input.clone());
for verdict in verdicts {
    batch.push(build_verdict_signal(verdict));
}
if !batch.is_empty() {
    substrate
        .put_batch(batch)
        .await
        .map_err(|e| anyhow!("persist gate verdicts: {e}"))?;
}
```

> **Note on `if !batch.is_empty()`.** The plan emphasizes guarding
> against empty `put_batch` calls — they are no-ops but still acquire
> the write lock and allocate. Always guard.

> **Where to look first.** Per the audit reference in `01-FILE-INVENTORY.md`,
> single-put sites cluster in:
> - `crates/roko-cli/src/run.rs` (a few stragglers around lines 1224, 1268, 2780, 2870-2900)
> - `crates/roko-cli/src/orchestrate.rs` (per-task verdict loops)
>
> Run the grep yourself; the line numbers drift.

### Step 3 — Add the partial-write replay test

Append to `crates/roko-fs/src/file_substrate.rs` (in the existing
`#[cfg(test)] mod tests` block; if the file has no test module, append
one at the bottom of the file):

```rust
#[tokio::test]
async fn replay_skips_partial_last_line() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let log_path = dir.path().join("engrams.jsonl");

    // Write one valid engram followed by a truncated line.
    {
        let mut f = std::fs::File::create(&log_path).unwrap();
        let valid = roko_core::Engram::builder(roko_core::Kind::Prompt)
            .body(roko_core::Body::text("ok"))
            .build();
        let valid_json = serde_json::to_string(&valid).unwrap();
        writeln!(f, "{valid_json}").unwrap();
        write!(f, r#"{{"id":"abc","body":"trun"#).unwrap();   // truncated, no newline
    }

    // Open succeeds; index has the 1 valid entry; partial line is
    // silently skipped per the JSONL "ignore non-parsing lines" pattern.
    let sub = FileSubstrate::open(dir.path()).await.unwrap();
    let snapshot = sub.snapshot();
    assert_eq!(snapshot.len(), 1, "expected exactly the valid engram");
}

#[tokio::test]
async fn put_batch_empty_input_is_noop() {
    let dir = tempfile::tempdir().unwrap();
    let sub = FileSubstrate::open(dir.path()).await.unwrap();
    let ids = sub.put_batch(vec![]).await.unwrap();
    assert!(ids.is_empty());

    // The log file should not have been created (or at least not grown).
    let log_path = dir.path().join("engrams.jsonl");
    let len = std::fs::metadata(&log_path)
        .map(|m| m.len())
        .unwrap_or(0);
    assert_eq!(len, 0, "empty put_batch should not write anything");
}
```

> If `FileSubstrate::snapshot()` does not exist, use whatever the
> existing tests use to count entries (e.g., `sub.query(&Query::all(),
> &Context::now()).len()` or a private accessor).

### Step 4 — Document `// SAFETY-ORDER:` annotations

For every single `put` call you decide to leave alone in Step 1, add a
short comment above it explaining why batching would break the semantic.
Examples:

```rust
// SAFETY-ORDER: the agent below reads this engram from the substrate
// before the next put runs. Batching would lose the happens-before.
substrate.put(gate_input.clone()).await?;
let agent_result = dispatch_agent(...).await?;

// SAFETY-ORDER: failures must persist independently of subsequent
// success signals so a partial run leaves a consistent log.
substrate.put(failure_signal).await?;
```

### Step 5 — (OPTIONAL, NOT scope-expanding) Consider a `Store` trait default

The plan §Step 3 suggests adding a default `put_batch` impl on the
`Store` trait so all backends inherit a (slow) baseline impl that
`FileSubstrate` overrides. This involves `crates/roko-core/` changes —
**out of scope for this batch**. Document as a follow-up:

```text
followup: Add default Store::put_batch impl on the trait so
MemorySubstrate / ColdSubstrate inherit the API without each rolling
their own. Skipped here to keep scope tight.
```

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-fs/src/file_substrate.rs`

## Read-Only Context

- `crates/roko-runtime/src/effect_driver.rs`
- `tmp/solutions/perf/implementation/05-batch-substrate-writes.md`
- `tmp/runners/perf/context-pack/02-ANTI-PATTERNS.md` (AP-PERSIST-2)

## Acceptance Criteria

- [ ] No production code path issues ≥3 sequential `substrate.put` calls without batching (verified by `rg`).
- [ ] Sequential `put` call sites that genuinely need ordering are annotated with a `// SAFETY-ORDER:` comment explaining why.
- [ ] Test `replay_skips_partial_last_line` exists and passes.
- [ ] Test `put_batch_empty_input_is_noop` exists and passes.
- [ ] `if !batch.is_empty()` guard present on every new `put_batch` call.
- [ ] Optional `Store::put_batch` default impl explicitly deferred or implemented (commit body documents which).

## Verify

```bash
# After your edits, this should print only single-shot puts (≤2 in a row),
# all annotated with SAFETY-ORDER, and put_batch usages.
rg -n '\.put\(|\.put_batch\(' crates/roko-cli/src/ \
  | rg -v 'tests|test\.rs|/tests/'

# Multiline grep for ≥3 consecutive puts in the same function:
rg -nU --multiline 'substrate\.put\([^_].*?\.await.*?substrate\.put\([^_].*?\.await.*?substrate\.put\([^_]' \
  crates/ --type rust
# Expected: empty.
```

## Do NOT

- Do NOT batch puts that span an `.await` on something the caller might
  observe (see `// SAFETY-ORDER:` rule).
- Do NOT collect engrams into a `Vec` over a long await chain (a panic
  mid-chain loses everything). Batching is for synchronous sequences
  (≤a few millis between puts).
- Do NOT call `put_batch(vec![])`. The empty-input guard above prevents
  the wasted lock acquire + allocation.
- Do NOT remove the in-memory dedup inside `put_batch`. It catches
  accidental double-writes (e.g., a retry loop). The cost is one HashMap
  lookup per signal, well worth it.
- Do NOT switch the file backend from JSONL to a binary format. JSONL
  is grep-able and load-bearing for debugging.
- Do NOT batch across substrate instances. `put_batch` works on a
  single `FileSubstrate`. If the workflow involves writing to two
  substrates (rare), batch each separately.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_05 done <commit-sha>
```
