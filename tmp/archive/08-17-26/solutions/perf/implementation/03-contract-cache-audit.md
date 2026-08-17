# 03 — Contract Cache Audit & Plumbing (B05)

> Status: **already implemented (mostly).** This plan documents what
> exists, audits the remaining call sites, and locks in the cache
> behaviour with regression tests so future refactors do not silently
> regress to per-dispatch loads.
>
> Effort: ≈1 h (audit + tests). Risk: low.

---

## Goal & success criteria

A safety contract YAML/JSON file is read from disk **at most once per
role per process**, then served from memory for every tool dispatch.

Done when:

- A regression test is in place that boots a fresh process, dispatches
  multiple tools for the same role, and asserts only one disk read
  occurred.
- All `AgentContract` callers route through `load_for_role` /
  `load_for_role_with_mode` (no rogue `serde_json::from_str` /
  `fs::read_to_string` shortcuts in production code paths).
- The cache invalidation path (test-only `invalidate_contract_cache`) is
  documented as **test-only**, with a clear comment explaining why
  hot-reload is intentionally unsupported.

---

## Background

- Bottleneck source: `BOTTLENECK-ANALYSIS.md` §B05,
  `OPTIMIZATION-PLAYBOOK.md` §3.
- The cache **already exists**:

  ```text
  crates/roko-agent/src/safety/contract.rs
    34  static CONTRACT_CACHE: LazyLock<RwLock<HashMap<String, AgentContract>>> = ...
   120  pub fn load_for_role(role) -> Result<Self, ContractLoadError> {
   125    if let Ok(guard) = CONTRACT_CACHE.read() { /* hit */ }
   132    let source = fs::read_to_string(&path)?;
   144    let mut contract = serde_json::from_str(&source)?;
   162    if let Ok(mut guard) = CONTRACT_CACHE.write() { guard.insert(...) }
  ```

- This means `OPTIMIZATION-PLAYBOOK.md` §3's main proposal is
  *already done*. Two things remain:
  1. Audit that nothing bypasses the cache.
  2. Add a regression test so the fix sticks.

---

## Files to read first

| File | Why |
|---|---|
| `crates/roko-agent/src/safety/contract.rs` | The cache implementation. |
| `crates/roko-agent/src/safety/mod.rs` | Re-exports + which callers consume `AgentContract`. |
| `crates/roko-agent/src/safety/contracts/*.yaml` | Bundled contracts; verify which roles ship by default. |
| `crates/roko-agent/src/dispatcher/` (if present) | Where tool dispatch consults the contract; primary cache consumer. |

---

## Code-level plan

### Step 1 — Audit caller paths

Run:

```bash
rg -n "AgentContract" crates/ --type rust
rg -n "fs::read_to_string\(.*safety/contracts" crates/ --type rust
rg -n "load_for_role" crates/ --type rust
```

For every caller, verify it goes through `load_for_role` or
`load_for_role_with_mode`. If any caller reads the YAML directly (via
`include_str!`, `fs::read_to_string`, or `serde_json::from_str` against
a contract file), refactor it to call the cached loader.

Common offenders to look for:

- Bench / harness code that ships its own contract loader for tests.
- Scripts under `scripts/` (e.g., `layer_check.rs`).
- The `roko-orchestrator` service factory if it bypasses the agent
  crate's safety helpers.

### Step 2 — Document the cache behaviour

Add or refine the doc comment on `CONTRACT_CACHE`:

```rust
/// Process-wide cache of parsed agent contracts.
///
/// Contracts are immutable for the lifetime of a process. Restart
/// `roko` to pick up edits to a YAML file under
/// `crates/roko-agent/src/safety/contracts/`. Hot-reload is
/// intentionally unsupported because contracts gate security-critical
/// tool dispatch — silently swapping the policy mid-run is a footgun
/// (see GH issue #TBD if you disagree).
```

### Step 3 — Add a regression test

In `crates/roko-agent/src/safety/contract.rs` (or a sibling test file):

```rust
#[test]
fn load_for_role_reads_disk_only_once_per_role() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::OnceLock;

    AgentContract::invalidate_contract_cache();

    // Prime: first call must hit disk.
    let _impl1 = AgentContract::load_for_role("implementer").expect("first load");

    // Subsequent calls (any number) must serve from cache without disk.
    let counter = INSTRUMENT_DISK_READS.get_or_init(|| AtomicUsize::new(0));
    counter.store(0, Ordering::Relaxed);
    for _ in 0..100 {
        let _ = AgentContract::load_for_role("implementer").expect("cached load");
    }
    assert_eq!(counter.load(Ordering::Relaxed), 0,
        "cached loads must not touch disk");
}
```

The disk-read counter requires hooking `read_to_string`. Two options:

- **Cheap:** rely on `fs::metadata` mtime. Stat the file before and
  after; if access time advanced, the cache missed. macOS may not
  update atime by default (`noatime` mounts), so this is unreliable
  cross-platform.
- **Reliable:** wrap `fs::read_to_string` behind a private trait,
  inject a counting impl in tests. Slightly invasive but tracks the
  semantic correctly.

Pick the reliable path.

### Step 4 — Add a per-process startup load (optional)

If you want to amortize the very first load (≈10 ms) outside the
critical path, add a `prewarm_contracts()` helper that loads the six
common roles at startup:

```rust
pub fn prewarm_contracts() {
    for role in [
        "implementer", "reviewer", "researcher",
        "architect", "auditor", "scribe", "auto-fixer",
    ] {
        let _ = AgentContract::load_for_role(role);
    }
}
```

Call it from `main.rs` (CLI) and `lib.rs` (`roko serve` startup). This
moves the 6 × ~10 ms load out of the dispatch hot path. It is optional
because the existing cache already handles steady-state correctly.

---

## Step-by-step execution

1. `git checkout -b perf/03-contract-cache-audit`.
2. Run the audit greps (Step 1). Refactor any rogue loaders.
3. Update the doc comment (Step 2).
4. Add the regression test (Step 3). `cargo test -p roko-agent --release`.
5. (Optional) Add `prewarm_contracts` and call it from CLI startup.
6. Open PR `perf(safety): lock in contract cache + add regression test (B05)`.

---

## Anti-patterns / things NOT to do

- **Do NOT add a TTL to the contract cache.** Contracts are immutable
  per process. A TTL invites the same disk-IO regression we are
  preventing.
- **Do NOT expose `invalidate_contract_cache` outside `#[cfg(test)]`.**
  External callers will misuse it for "reload my updated YAML" — that
  path requires a process restart and an audit log entry, not a cheap
  cache flush.
- **Do NOT switch the cache from `RwLock<HashMap<...>>` to `dashmap` or
  `OnceCell`** without a measured contention problem. The current
  pattern is a textbook "read-heavy with one-shot warm-up", and
  `RwLock` performs perfectly here.
- **Do NOT cache `AgentContract` *outside* the safety module** (e.g.,
  inside the `ToolDispatcher`). That introduces stale references when
  contracts ship updates between releases. The single source of truth
  should remain the safety crate.
- **Do NOT serialize the cache to disk** as a "warm start" file. The
  YAMLs are already in the binary via tests / repo; saving the parsed
  cache to disk only adds another stale copy to maintain.

---

## Test plan

| Level | Test | Where |
|---|---|---|
| Unit | First-load hits disk; subsequent loads do not | `crates/roko-agent/src/safety/contract.rs` (new test) |
| Unit | Different roles each load disk once | extend the test above to load `implementer` and `reviewer` and assert disk reads = 2 |
| Unit | `invalidate_contract_cache` clears state for re-test isolation | existing tests already cover this |
| Smoke | `roko run` with high-tool-count prompt does not regress in flame graph (no `read_to_string` near `safety::`) | `cargo flamegraph --bin roko -- run …` |

---

## Rollback plan

- Audit is read-only: no rollback needed.
- If the regression test trips a CI environment quirk (e.g., a CI
  worker that *does* update atime), gate the test behind
  `#[cfg(not(target_os = "macos"))]` while you investigate, but keep the
  Linux assertion alive — that's where production runs.

---

## Status check (acceptance)

- [ ] No production caller bypasses `AgentContract::load_for_role`.
- [ ] Regression test `load_for_role_reads_disk_only_once_per_role`
      passes.
- [ ] Doc comment on `CONTRACT_CACHE` explains the immutability rule.
- [ ] (Optional) `prewarm_contracts()` is wired into CLI/serve startup.
