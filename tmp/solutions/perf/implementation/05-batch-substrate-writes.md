# 05 — Adopt `FileSubstrate::put_batch` Everywhere (B10)

> Status: **partially implemented.** `FileSubstrate::put_batch` already
> exists and the main `run.rs` hot path uses it. This plan audits the
> remaining single-write call sites, converts them, and adds tests
> guaranteeing crash-safety semantics.
>
> Effort: ≈2 h. Risk: medium (data integrity on crash).

---

## Goal & success criteria

After this change, every `put_batch`-eligible place in the codebase
batches its writes. No production hot path issues 5+ consecutive
single-engram `Store::put` calls.

Done when:

- `rg "substrate.put\("` and `rg "store\.put\("` show only single-shot
  writes (≤2 puts in sequence) or test code.
- A new test verifies `replay_log` tolerates a partial last line in
  `engrams.jsonl` (mirrors the JSONL logger test in plan 04).
- Macro-benchmark p50 wall-time shows ≥40 ms improvement on runs that
  previously did 10 sequential puts.

---

## Background

- Bottleneck source: `BOTTLENECK-ANALYSIS.md` §B10,
  `OPTIMIZATION-PLAYBOOK.md` §4.
- `put_batch` is implemented at
  `crates/roko-fs/src/file_substrate.rs:137` with crash-safe semantics:
  serialize-then-single-write-then-flush + dedup against the in-memory
  index.
- Already adopted at:

  ```text
  crates/roko-cli/src/run.rs
    1156 substrate.put_batch(prompt_signals)
    1180 substrate.put_batch(agent_result.trace.clone())
    1188 substrate.put_batch(batch)              # output + traces
    1245 substrate.put_batch(verdict_sigs.clone())
    2872 substrate.put_batch(vec![agent_result.output.clone()])
    2901 substrate.put_batch(vec![raw_trace, clean_sig.clone()])
  ```

- Remaining `substrate.put(...)` (single-shot) call sites need an audit
  to confirm they really are single-shot or should join a batch.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-fs/src/file_substrate.rs` | `put`, `put_batch`, `replay_log` — primary file. |
| `crates/roko-cli/src/run.rs` | Most callers of substrate. |
| `crates/roko-cli/src/orchestrate.rs` | Plan-execution writes; high single-put counts likely. |
| `crates/roko-runtime/src/effect_driver.rs` | Some writes happen during effect dispatch. |
| `crates/roko-cli/src/inject.rs` | Writes for `roko inject`; may batch. |

---

## Code-level plan

### Step 1 — Find all single-put hot paths

```bash
rg -n "\.put\(" crates/ --type rust | rg -v "put_batch|tests|test\.rs"
```

For every result, look at the surrounding code. If two or more `put`
calls happen consecutively (no awaitable side effect between them), they
should be coalesced.

A typical offender pattern:

```rust
substrate.put(gate_input.clone()).await?;       // single
// ... compute verdicts ...
for verdict in verdicts {
    substrate.put(verdict).await?;              // N×1 — should batch
}
```

Should become:

```rust
let mut batch = Vec::with_capacity(1 + verdicts.len());
batch.push(gate_input.clone());
batch.extend(verdicts.into_iter());
substrate.put_batch(batch).await?;
```

### Step 2 — Decide whether to batch across awaits

If a `put` is followed by a network call, it is fine to keep it single.
The IO time of network calls dominates anyway. Only batch consecutive
puts.

Heuristic: **if removing the await between puts changes observable
behaviour (e.g., a downstream agent consumes the substrate read state
between writes), keep them single.** Document the reason in a code
comment.

### Step 3 — Add the `put_batch` API to other Store implementors (optional)

If `MemorySubstrate` or `ColdSubstrate` exist and don't yet expose
`put_batch`, add it for parity. The `Store` trait should grow a default
implementation that loops over `put` so every implementor gets the API
automatically:

```rust
// crates/roko-core/src/lib.rs (or wherever the Store trait lives)
#[async_trait]
pub trait Store: Send + Sync {
    async fn put(&self, sig: Engram) -> Result<ContentHash>;

    async fn put_batch(&self, sigs: Vec<Engram>) -> Result<Vec<ContentHash>> {
        let mut ids = Vec::with_capacity(sigs.len());
        for sig in sigs {
            ids.push(self.put(sig).await?);
        }
        Ok(ids)
    }
    // ... other methods ...
}
```

`FileSubstrate::put_batch` overrides this default with the optimized
impl. This means callers can write `store.put_batch(...)` without
caring which backend they have.

> **Caveat.** Adding a default trait method is a non-breaking change
> for trait *implementors* but a breaking change for **trait objects in
> some pathological cases** (deprecated method dispatch). Audit
> `dyn Store` usages first; if any exist with non-default impls, you may
> need to add `put_batch` explicitly to each.

### Step 4 — Verify crash safety

Add a partial-write test next to the existing `replay_log` tests:

```rust
#[tokio::test]
async fn replay_skips_partial_last_line() {
    let dir = tempfile::tempdir().unwrap();
    let log_path = dir.path().join("engrams.jsonl");
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&log_path).unwrap();
        let valid = serde_json::to_string(&dummy_engram()).unwrap();
        writeln!(f, "{valid}").unwrap();
        write!(f, r#"{{"id":"abc","body":"trun"#).unwrap();   // truncated
    }
    let sub = FileSubstrate::open(dir.path()).await.unwrap();
    // Open succeeds; index has the 1 valid entry; partial line ignored.
    assert_eq!(sub.len_for_test(), 1);
}
```

(`len_for_test` may not exist; use a query that returns 1 result for
the dummy engram instead.)

---

## Step-by-step execution

1. `git checkout -b perf/05-batch-substrate-writes`.
2. Run the audit grep (Step 1). Categorise each call site as:
   - already in a batch (skip),
   - eligible for batching (convert),
   - genuinely single-shot (annotate with comment).
3. Convert eligible call sites; one logical group per commit so review
   stays manageable.
4. (Optional) Add the default `put_batch` to `Store`.
5. Add the partial-write replay test.
6. Macro-benchmark before/after on a workflow that emits many signals
   (e.g., `roko plan run` against a 5-task plan, or `roko run` with
   gates enabled so verdict signals exist).
7. Open PR `perf(fs): adopt put_batch across remaining hot paths (B10)`.

---

## Anti-patterns / things NOT to do

- **Do NOT batch puts that span an await on something the caller might
  observe.** Example: writing a "task started" engram, then awaiting an
  agent that may read the substrate. Batching destroys the happens-before.
- **Do NOT collect engrams into a `Vec` over a long await chain.** A
  panic mid-chain loses everything. Batching is for synchronous
  sequences (≤a few millis between puts), not for "save up everything
  for the entire run".
- **Do NOT call `put_batch` with `Vec::new()`.** It is a no-op but
  acquires a write lock and allocates an empty buffer needlessly. Guard
  with `if !batch.is_empty()`.
- **Do NOT remove the in-memory dedup** in `put_batch`. It catches
  accidental double-writes (e.g., a retry loop). The cost is one
  HashMap lookup per signal, well worth it.
- **Do NOT switch the file backend from JSONL to a binary format** as
  part of this plan. JSONL is grep-able and load-bearing for debugging;
  changing the format is a major architectural decision.
- **Do NOT batch across substrate instances.** `put_batch` works on a
  single `FileSubstrate`. If the workflow involves writing to two
  substrates (rare but possible), batch each separately.

---

## Test plan

| Level | Test | Where |
|---|---|---|
| Unit | `put_batch` deduplicates pre-existing engrams | already exists in `file_substrate.rs` tests |
| Unit | `put_batch([])` is a no-op (no lock, no flush) | new test in `file_substrate.rs` |
| Unit | `replay_log` skips truncated last line | new test (Step 4) |
| Integration | After `roko run --gates compile,test`, `engrams.jsonl` has all expected signals | shell test or `cargo test` against a fixture workdir |
| Macro-bench | Wall-clock improvement ≥40 ms on multi-signal runs | `/usr/bin/time -l roko run …` |

---

## Rollback plan

- Per-call-site conversion: `git revert` of an individual commit
  restores that call site only. Keep commits small.
- `Store` default-method addition (optional Step 3): if it breaks an
  external trait implementor, remove the default impl and require each
  implementor to provide its own. The `FileSubstrate` impl is unaffected.

---

## Status check (acceptance)

- [ ] No production code path issues ≥3 sequential `substrate.put`
      calls without batching (verified by grep + manual review).
- [ ] Partial-write replay test green.
- [ ] Macro-benchmark improvement recorded.
