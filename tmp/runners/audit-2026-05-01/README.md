# audit-2026-05-01 runner

**Purpose**: Land the open items from the 2026-05-01 audit
(`tmp/subsystem-audits/05-01/` and per-subsystem audits) using the same
parallel-runner pattern as `productionizing/` and `post-parity/`.

**Source plans** (canonical, with full implementation detail):
`tmp/subsystem-audits/implementation-plans/`. Every batch in this
runner cites a section of one of those plans.

**Issue tracker**: [`ISSUES.md`](./ISSUES.md). Single source of truth
for what is open vs. done. Re-grep before ticking a box.

**Runner format**: parallel-template (codex worktrees, cherry-pick,
12 concurrent). Verifies via `--list`/`--dry-run`/`--continue` like
the other runners.

---

## Summary

| Group | Tier / Topic | Open batches | Source plan |
|---|---|---|---|
| `T2` | Delete dead code | 7 | `12-tier2-delete-dead-code.md` |
| `T3` | Security hardening | 7 | `13-tier3-security-hardening.md` |
| `T4` | Feedback loop completion | 10 | `14-tier4-feedback-loops.md` |
| `T5` | Architectural extraction | 16 | `15-tier5-architectural.md`, `20-orchestrate-rs-extraction.md` |
| `S` | Subsystem cross-cutting | 52 | `21-..34-...` plans |
| **Total** | | **92** | |

(The 92 includes 3 inventory-only batches that produce audit documents
under `logs/`: S-prompt-1, S-cog-1, S-chain-1. These are prerequisites
for their sister batches.)

Forward-looking work (`40-..-42-...md`) is **not** in this runner. Pick
those up only after the engine plans land.

Already-done items (T0-1 through T0-7, T1-8 through T1-15) are NOT in
this runner. They are listed under "Already done in tree" in
[`ISSUES.md`](./ISSUES.md) with `git log` SHAs for reference.

---

## Execution DAG

```
Wave 1 (independent, ~50 batches, run with PARALLEL=12):
  T2: T2-16, T2-17a..n, T2-18, T2-19, T2-20, T2-21
  T3: T3-22, T3-23, T3-24, T3-25, T3-26, T3-27, T3-28
  T4: T4-29, T4-30, T4-31a..e, T4-32, T4-33, T4-34
  T5: T5-35a, T5-37, T5-39, T5-41
  S:  S-acp1..4, S-config-1..2, S-ledger-1, S-learn-A..C,
      S-term-1..2, S-ci-1..2, S-safety-1, S-gate-1, S-prompt-1,
      S-cog-1, S-http-1, S-codeintel-1, S-chain-1

Wave 2 (deps satisfied):
  T5-35b (←T5-35a)
  T5-36a..f (←T5-35a; serve route migrations)
  T5-38 (←S-config-2)
  T5-40a..d (←S-ledger-1)
  T5-42a..e (←T5-35a)
  S-acp-test (←S-acp1)
  S-config-3..7 (←S-config-2)
  S-ledger-2..5 (←S-ledger-1)
  S-learn-D..F (←T2-17 + T4-29)
  S-term-3..5 (←S-term-1)

Wave 3 (deeper):
  T5-35c (←T5-35b)
  T5-35d (←T5-35c)
  S-cog-2..5 (←S-cog-1 inventory)
  S-prompt-2..6 (←S-prompt-1 audit)
```

Cross-group deps are fine — `deps = [...]` in `batches.toml` enforces.

---

## Running

```bash
# Inventory + sanity-check the tree
bash tmp/runners/audit-2026-05-01/run.sh --list
bash tmp/runners/audit-2026-05-01/run.sh --dry-run

# Full run, 12 concurrent
bash tmp/runners/audit-2026-05-01/run.sh

# One group at a time (recommended starting order: T2 → T3 → T4 → T5 → S)
bash tmp/runners/audit-2026-05-01/run.sh --group T2
bash tmp/runners/audit-2026-05-01/run.sh --group T3

# Specific batches
bash tmp/runners/audit-2026-05-01/run.sh --only T2-16,T3-23,T4-29

# Resume after interrupt
bash tmp/runners/audit-2026-05-01/run.sh --continue

# Cherry-pick to wp-arch2 as batches succeed (optional, alongside)
bash tmp/runners/parallel-template/lib/auto-pick.sh \
  --interval 90 --target-branch wp-arch2

# Status / disk / cleanup
bash tmp/runners/audit-2026-05-01/run.sh --status
bash tmp/runners/audit-2026-05-01/run.sh --disk
bash tmp/runners/audit-2026-05-01/run.sh --cleanup
```

---

## After a green run

1. Open [`ISSUES.md`](./ISSUES.md). Re-grep the codebase for each
   unticked box. Tick only what survives the grep.
2. Run the workspace-wide sanity:
   ```bash
   cargo +nightly fmt --all
   cargo clippy --workspace --no-deps -- -D warnings
   cargo test --workspace
   bash scripts/roko-fitness-checks.sh
   ```
3. Move stale items from "open" to "done in tree" with a fresh
   line-number citation.

---

## Conventions

- **Group letter** = wave gate scope. `T2`, `T3`, `T4`, `T5`, `S` each
  get their own `cargo check + clippy` after their batches complete.
- **Batch IDs** match the originating implementation-plan section IDs
  (T2-16, T3-23, T5-35a, S-acp1, …). Easy to bisect.
- `deps = [...]` enforces the DAG. Cross-group is fine
  (e.g. `T5-36a` deps on `T5-35a`).
- Every prompt:
  - Names the **plan file** it derives from.
  - Names exact files + line numbers.
  - Shows BEFORE / AFTER snippets where relevant.
  - Lists explicit "Do NOT" constraints.
  - Ends with a `Verification` block (no compile, just grep / file
    presence / test name).
- Context-pack (`context-pack/`) injects `00-RULES.md`, `01-CRATE-MAP.md`,
  `02-ANTI-PATTERNS.md` into every prompt.

## Anti-pattern carry-forward

The 10 global anti-patterns from
`tmp/subsystem-audits/implementation-plans/02-ANTI-PATTERNS.md` apply to
**every** batch in this runner. They are also in `context-pack/02-ANTI-PATTERNS.md`.
Most failure modes from the 661-batch runner damage report come from
violating these.

Critical ones to keep in mind:

1. Skeletons ≠ migrations. New type compiling does not mean it is used.
2. Unknown ≠ zero (`Option<u64>`/`Option<f64>`).
3. No silent fallback (typed errors only).
4. Missing config → restricted, never permissive.
5. No regex prompt scraping.
6. No string-interpolated payloads (use serde / toml).
7. No fifth dispatch path.
8. One item per commit.
9. No `unwrap()`/`panic!()` in changed code.
10. No unrelated edits.

If a batch prompt seems to require violating any of these, **stop and
report**, do not improvise.
