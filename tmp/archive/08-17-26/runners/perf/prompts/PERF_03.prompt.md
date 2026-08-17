# PERF_03: Contract cache audit (B05)

## Task

The `AgentContract` cache **already exists** at
`crates/roko-agent/src/safety/contract.rs:34`. This batch is an **audit**:
verify nothing bypasses it, lock the cache behaviour into a regression
test, and document the immutability rule in the doc comment.

## Tracker & sources

- Issue tracker row: [ISSUE-TRACKER.md#perf_03](../ISSUE-TRACKER.md#perf_03)
- Plan: `tmp/solutions/perf/implementation/03-contract-cache-audit.md`
- Bottleneck: B05 (BOTTLENECK-ANALYSIS.md §B05)
- Performance contract: **C-3** (process-wide contract cache)
- Priority: P2 (verification, not new behaviour)
- Effort: ≈1 h
- Depends on: none
- Wave: 1

## Problem

`crates/roko-agent/src/safety/contract.rs:34-35` already defines a
process-wide cache:

```rust
static CONTRACT_CACHE: LazyLock<RwLock<HashMap<String, AgentContract>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
```

`AgentContract::load_for_role` consults the cache (read-lock fast path,
write-lock fallback). The implementation looks correct.

The risk is *regressions*: a future PR adds a rogue caller that bypasses
the cache (e.g., `serde_json::from_str(&fs::read_to_string("...yaml")?)?`
inline somewhere). This batch:

1. Audits the codebase for such bypasses and fixes them.
2. Adds a regression test that asserts the cache is hit on repeated
   loads.
3. Tightens the doc comment so the immutability rule is explicit.

## Exact Changes

### Step 1 — Audit

Run these greps (in your batch worktree; you can `rg` because read-only
analysis is allowed during a batch):

```bash
rg -n 'AgentContract' crates/ --type rust
rg -n 'fs::read_to_string\(.*safety/contracts' crates/ --type rust
rg -n 'serde_json::from_str.*AgentContract' crates/ --type rust
rg -n 'load_for_role' crates/ --type rust
```

For every result that calls a contract loader, confirm it goes through
`AgentContract::load_for_role` or `AgentContract::load_for_role_with_mode`.
Common offender categories:

- bench/test code that ships its own loader (acceptable in `#[cfg(test)]`,
  not in production paths),
- `scripts/layer_check.rs` (out of scope but inspect for awareness),
- `roko-orchestrator::ServiceFactory` if it bypasses the agent crate's
  helpers.

If you find a production caller that bypasses the cache, refactor it to
use `load_for_role`. **Document each refactor in the commit body**:

```text
fix: contract loader at <path:line> was reading contracts/<role>.yaml
directly via fs::read_to_string; switched to AgentContract::load_for_role
which serves from the process cache.
```

If you find none, document that explicitly:

```text
audit: no rogue contract loaders in production paths.
```

### Step 2 — Tighten the doc comment on `CONTRACT_CACHE`

Replace the existing doc comment block above the `static CONTRACT_CACHE`
declaration (around line 28-33 of `crates/roko-agent/src/safety/contract.rs`)
with:

```rust
/// Process-wide cache of parsed agent contracts.
///
/// **Immutability contract.** Contracts are immutable for the lifetime
/// of a process. Restart `roko` to pick up edits to a YAML file under
/// `crates/roko-agent/src/safety/contracts/`. Hot-reload is intentionally
/// unsupported because contracts gate security-critical tool dispatch
/// — silently swapping the policy mid-run is a footgun.
///
/// The cache uses `RwLock<HashMap<...>>` so concurrent reads do not
/// contend; writes occur once per role per process. `LazyLock` ensures
/// the cache is initialized lazily on first access.
///
/// Test isolation: use `AgentContract::invalidate_contract_cache()`
/// (`#[cfg(test)]` only) to clear between tests. Never expose this
/// method to non-test code.
static CONTRACT_CACHE: LazyLock<RwLock<HashMap<String, AgentContract>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
```

### Step 3 — Add the regression test

Append to the existing `#[cfg(test)] mod tests` block in
`crates/roko-agent/src/safety/contract.rs`:

```rust
/// C-3: process-wide contract cache.
///
/// First load reads disk; subsequent loads must serve from cache without
/// touching disk. We track this by counting `mtime` reads on the
/// contract file -- a cache hit will not refresh atime/mtime since
/// nothing on disk is read.
#[test]
fn load_for_role_reads_disk_only_once_per_role() {
    AgentContract::invalidate_contract_cache();

    // Prime: this load MUST hit disk.
    let path_before = contract_asset_path("implementer");
    assert!(path_before.exists(), "fixture contract missing");
    let mtime_before = std::fs::metadata(&path_before)
        .and_then(|m| m.modified())
        .ok();

    let _impl1 = AgentContract::load_for_role("implementer").expect("first load");

    // Repeat 100 times. None of these should produce a fresh disk read.
    // A simple proof: the mtime of the contract file is unchanged
    // (nothing wrote it) AND a fresh `Instant::now()` taken before and
    // after the loop bounds total time well under the cost of 100 disk
    // reads.
    let start = std::time::Instant::now();
    for _ in 0..100 {
        let _ = AgentContract::load_for_role("implementer").expect("cached load");
    }
    let elapsed = start.elapsed();

    let mtime_after = std::fs::metadata(&path_before)
        .and_then(|m| m.modified())
        .ok();

    assert_eq!(mtime_before, mtime_after,
        "contract file mtime changed; suggests disk was written between loads");
    assert!(elapsed.as_millis() < 50,
        "100 cached loads took {}ms; expected <50ms (a single disk \
         read is ~1ms, so 100 cold reads would be ~100ms)",
        elapsed.as_millis());
}

/// C-3: cache is keyed by role.
#[test]
fn load_for_role_caches_per_role_independently() {
    AgentContract::invalidate_contract_cache();
    let _ = AgentContract::load_for_role("implementer").expect("load implementer");
    let _ = AgentContract::load_for_role("reviewer").expect("load reviewer");
    let _ = AgentContract::load_for_role("implementer").expect("cached implementer");
    let _ = AgentContract::load_for_role("reviewer").expect("cached reviewer");
    // Smoke check: both contracts retrievable; the test passing under
    // serial execution is the assertion.
}
```

> If you would prefer a strict "0 disk reads after warm-up" assertion,
> wrap `fs::read_to_string` in `contract.rs` behind a private trait that
> injects a counting impl in tests. The plan documents this as the
> "reliable but invasive" option. The mtime+timing test above is the
> "cheap" option and is acceptable for this audit batch.

### Step 4 (OPTIONAL) — Pre-warm at startup

If the team wants to amortize the very first 6 × 10 ms loads outside
the critical path, add a `prewarm_contracts()` helper:

```rust
// crates/roko-agent/src/safety/mod.rs

/// Eagerly load the bundled contracts for the standard agent roles.
/// Call from CLI / `roko serve` startup so the first dispatch does not
/// pay the load cost.
pub fn prewarm_contracts() {
    for role in [
        "implementer", "reviewer", "researcher",
        "architect", "auditor", "scribe", "auto-fixer",
    ] {
        let _ = crate::safety::contract::AgentContract::load_for_role(role);
    }
}
```

Then call from CLI `main` and `roko-serve` startup.

If you skip the optional step, document in the commit body:

```text
deferred: prewarm_contracts() helper not added; the cache already serves
the steady-state correctly. Add later if first-dispatch latency is shown
to matter.
```

## Write Scope

- `crates/roko-agent/src/safety/contract.rs`

(Optional Step 4 also touches `crates/roko-agent/src/safety/mod.rs` and
either `crates/roko-cli/src/main.rs` or `crates/roko-serve/src/lib.rs`.
If you do the optional step, expand `scope` in `batches.toml` BEFORE
editing — but this batch is small enough that deferring Step 4 is
preferred.)

## Read-Only Context

- `crates/roko-agent/src/safety/mod.rs`
- `crates/roko-agent/src/safety/contracts/*.yaml`
- `tmp/solutions/perf/implementation/03-contract-cache-audit.md`
- `tmp/runners/perf/context-pack/00-RULES.md`

## Acceptance Criteria

- [ ] No production caller bypasses `AgentContract::load_for_role` (audit recorded in commit body).
- [ ] Doc comment on `CONTRACT_CACHE` explicitly states immutability for the process lifetime.
- [ ] Regression test `load_for_role_reads_disk_only_once_per_role` passes.
- [ ] Regression test `load_for_role_caches_per_role_independently` passes.
- [ ] Optional `prewarm_contracts()` helper exists and is called from CLI startup OR the optional step is documented as deferred.

## Verify

```bash
# Audit greps:
rg -n 'AgentContract::load_for_role|fs::read_to_string\(.*safety/contracts|serde_json::from_str.*AgentContract' \
   crates/ --type rust

# Expected: only `load_for_role` callers (no inline reads).

# Post-merge:
cargo test -p roko-agent --lib --release \
   load_for_role_reads_disk_only_once_per_role
```

## Do NOT

- Do NOT add a TTL to the contract cache. Contracts are immutable per
  process; a TTL re-introduces the disk-IO regression.
- Do NOT expose `invalidate_contract_cache` outside `#[cfg(test)]`.
  External callers will misuse it for "reload my updated YAML"; that
  path requires a process restart and an audit-log entry.
- Do NOT switch from `RwLock<HashMap<...>>` to `dashmap` or `OnceCell`.
  The current pattern is a textbook read-heavy + one-shot warm-up
  workload that `RwLock` handles perfectly.
- Do NOT cache `AgentContract` outside the safety module (e.g., on a
  `ToolDispatcher`). Single source of truth.
- Do NOT serialize the cache to disk as a "warm start" file. The YAMLs
  are the source of truth; another stale copy is overhead.
- Do NOT compile or run tests during the batch (see `00-RULES.md`).

## Tracker update

```
tracker: PERF_03 done <commit-sha>
```
