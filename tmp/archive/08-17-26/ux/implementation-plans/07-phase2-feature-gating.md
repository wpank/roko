# 07 — Feature-Gate Phase-2 Crates

> **Source plan**: `tmp/ux/ux-followup/05-partially-wired-subsystems.md`
> items 32 (`roko-dreams`) and 33 (`roko-chain`, `roko-daimon`).
>
> **Status as of 2026-05-01**: `roko-dreams`, `roko-chain`, `roko-daimon`,
> `roko-conductor` (in part), and `roko-plugin` are workspace members
> built on every `cargo build --workspace`. CLAUDE.md marks them
> "Phase 2+". Their tests run in CI, contributing measurable build/test
> time without runtime benefit. Everything but `roko-chain` lacks a
> real production call site.
>
> **Effort**: 4-6 hours.
>
> **Risk**: Low (cosmetic / CI cost). The danger is creating
> downstream-breaking compile errors when feature flags don't compose;
> mitigate by running `--all-features` and `--no-default-features` in
> CI.

---

## What this plan accomplishes

Phase-2 crates gain a workspace-level feature gate so the default
`cargo build --workspace` excludes them. Tests, clippy, and binary
emission all narrow to the actively-shipped slice. The crates remain
buildable on demand (`cargo build --workspace --features phase2`)
without disrupting their authors.

## Why this matters

CI cost is real but secondary. The first-order win is signal:
`cargo build --workspace` becomes a true smoke test of *shipped* code.
A failing build is a real regression, not noise from a Phase-2 module
mid-refactor.

## A note on `roko-chain`

`roko-chain` is in the open list (item 33) but is *also* the dependency
target of track `02` (aggregator knowledge backends) and track `04`
(chain discovery). Once those tracks land, `roko-chain` is firmly in the
shipped slice — gate it as part of the default feature, not the
phase2 feature.

This plan covers `roko-dreams`, `roko-daimon`, `roko-plugin` only. The
audit on `roko-chain` is a one-line check after track `04` ships.

---

## Required reading

```
Cargo.toml                                 (workspace root, members + features)
crates/roko-dreams/Cargo.toml
crates/roko-dreams/src/lib.rs              (entry points; cycle.rs, hypnagogia.rs, imagination.rs)
crates/roko-daimon/Cargo.toml
crates/roko-daimon/src/lib.rs
crates/roko-plugin/Cargo.toml
crates/roko-plugin/src/lib.rs
.github/workflows/*.yml                    (CI matrix)
CLAUDE.md (Key crates table — confirms which are Phase 2+)
tmp/ux/ux-followup/05-partially-wired-subsystems.md
```

---

## Deliverables

1. **Workspace-level `phase2` feature** in the root `Cargo.toml`:

   ```toml
   [workspace.package]
   # ...

   # Per-member features are local; this is a meta-feature on the
   # workspace consumer crates that depend on phase2 modules.
   ```

   Workspace `Cargo.toml` doesn't actually carry features (cargo
   doesn't support workspace features yet). Instead:

   - Each crate that *consumes* a phase-2 module (almost certainly
     `roko-cli` and `roko-serve` if any) declares the dependency as
     `optional = true` and a `phase2` feature that turns it on.
   - The phase-2 crates themselves stay workspace members but are
     **not built by default**. Achieved via:

     - Move them from `[workspace] members = [...]` to
       `[workspace] members = ["..."]` plus `default-members = [<the
       shipped subset>]`.
     - Then `cargo build --workspace` builds default-members; explicit
       `--workspace --all` builds everything.

2. **`default-members` block** in root `Cargo.toml`:

   ```toml
   [workspace]
   members = [
     "apps/agent-relay",
     "apps/mirage-rs",
     "apps/roko-chain-watcher",
     "crates/roko-acp",
     "crates/roko-agent",
     "crates/roko-agent-server",
     "crates/roko-chain",          # active in Phase 1+
     "crates/roko-cli",
     "crates/roko-compose",
     "crates/roko-conductor",
     "crates/roko-core",
     "crates/roko-daimon",         # phase 2
     "crates/roko-demo",
     "crates/roko-dreams",         # phase 2
     "crates/roko-fs",
     "crates/roko-gate",
     "crates/roko-index",
     "crates/roko-lang-go",
     "crates/roko-lang-rust",
     "crates/roko-lang-typescript",
     "crates/roko-learn",
     "crates/roko-mcp-code",
     "crates/roko-mcp-github",     # see plan 06
     "crates/roko-mcp-scripts",    # see plan 06
     "crates/roko-mcp-slack",      # see plan 06
     "crates/roko-mcp-stdio",      # see plan 06
     "crates/roko-neuro",
     "crates/roko-orchestrator",
     "crates/roko-plugin",         # phase 2 (audit; see below)
     "crates/roko-primitives",
     "crates/roko-runtime",
     "crates/roko-serve",
     "crates/roko-std",
   ]
   default-members = [
     # All except phase-2 candidates.
     # Edit this list whenever a crate's status flips.
     "apps/agent-relay",
     "apps/mirage-rs",
     "apps/roko-chain-watcher",
     "crates/roko-acp",
     "crates/roko-agent",
     "crates/roko-agent-server",
     "crates/roko-chain",
     "crates/roko-cli",
     "crates/roko-compose",
     "crates/roko-conductor",
     "crates/roko-core",
     "crates/roko-demo",
     "crates/roko-fs",
     "crates/roko-gate",
     "crates/roko-index",
     "crates/roko-lang-go",
     "crates/roko-lang-rust",
     "crates/roko-lang-typescript",
     "crates/roko-learn",
     "crates/roko-mcp-code",
     "crates/roko-neuro",
     "crates/roko-orchestrator",
     "crates/roko-primitives",
     "crates/roko-runtime",
     "crates/roko-serve",
     "crates/roko-std",
   ]
   ```

3. **CI matrix update**: `.github/workflows/*.yml` learn about the
   new shape. Default jobs use `cargo build --workspace`. A new
   "phase2" job runs weekly (cron) with `cargo build --workspace --all`
   so phase-2 code doesn't bit-rot.

4. **Documentation**: `CLAUDE.md` Key crates table gets a
   "Default-built" column indicating shipped vs phase 2.
   `docs/v2/WORKSPACE.md` documents `default-members` semantics.

5. **`roko-plugin` audit**: it's listed under item 54 as "vestigial
   or in-progress". Quick decision in this plan: gate or delete. If
   gated, add a tracking issue.

---

## Step-by-step

### Step 1 — Verify nothing accidentally depends on phase-2 crates (30 min)

```bash
rg -t toml '^(roko-dreams|roko-daimon|roko-plugin)\b' crates/*/Cargo.toml apps/*/Cargo.toml \
  | grep -v 'crates/roko-(dreams|daimon|plugin)/'
```

Any hit means a *shipped* crate currently imports a phase-2 crate. That
import must be cleaned up before gating, or the dependency must become
optional. Read the consumer; in most cases the import is from a test
module and can be feature-gated locally.

### Step 2 — Edit the workspace `Cargo.toml` (15 min)

Apply Deliverable 2. Verify with:

```bash
cargo build --workspace                  # only default-members
cargo build --workspace --all            # everything including phase 2
cargo build --workspace --no-default-features   # sanity
```

All three must succeed.

### Step 3 — Update CI (30 min)

In each workflow that runs `cargo build --workspace`:

- Default-members job: leave as `cargo build --workspace` (now narrower).
- New `phase2` job: triggered on `workflow_dispatch` and on a weekly
  schedule (`schedule: cron: '0 6 * * 0'`); runs
  `cargo build --workspace --all` and `cargo test --workspace --all`.
  Failures notify the owners of the failing crate but do not block PRs.

### Step 4 — Decide `roko-plugin`'s fate (1-2 hrs)

Read `crates/roko-plugin/src/lib.rs` and `Cargo.toml`. Three options:

| Option | When to choose |
|--------|----------------|
| Gate as phase 2 | There is a near-term plan to use it. File the issue. |
| Delete | No documented use, no team owner, no PR in flight. |
| Promote to default | An existing PR or shipped feature uses it. |

Pick one, document the decision in `tmp/ux/ux-followup/08-phase-2-vision.md`
item 54 with a "Decided YYYY-MM-DD: <X>" line.

### Step 5 — Update CLAUDE.md and docs (30 min)

Per Deliverable 4. Specifically include a section "Building only the
shipped slice":

```markdown
- `cargo build --workspace`         — shipped slice (default-members).
- `cargo build --workspace --all`   — including Phase 2+ crates.
- `cargo test --workspace`          — shipped slice tests.
```

### Step 6 — Close the followup item (5 min)

`tmp/ux/ux-followup/05-partially-wired-subsystems.md` items 32 and 33:
add a "Closed YYYY-MM-DD" header, link to merged PR. Bump
`00-INDEX.md` totals.

---

## Anti-patterns to avoid

- **Don't gate `roko-chain`.** Track `02` and `04` make it a hard
  shipped dependency.
- **Don't silently drop CI coverage of phase-2 code.** The weekly job
  catches bit-rot. Skipping it altogether means the next promotion
  attempt finds two months of accumulated breakage.
- **Don't introduce per-crate features that fragment the build matrix
  beyond `default-members` vs `--all`.** Cargo gets exponentially
  noisier as feature flags multiply. The two-knob model is enough.
- **Don't hand-edit `default-members` without updating CLAUDE.md** in
  the same commit. The list and the table go out of sync immediately.
- **Don't move phase-2 crates to a sibling repo "for cleanliness".**
  Splitting repos creates real coordination cost. Workspace members
  with `default-members` is the canonical Cargo solution.

## Done when

1. `cargo build --workspace` builds and tests only the shipped slice
   (verify via `--timings` JSON output: phase-2 crate names absent).
2. `cargo build --workspace --all` still works.
3. Weekly CI job `phase2-build` exists and was green at least once.
4. `CLAUDE.md` reflects the gating; `docs/v2/WORKSPACE.md` exists.
5. `tmp/ux/ux-followup/05-partially-wired-subsystems.md` items 32, 33
   closed.
6. `roko-plugin` decision documented (gated, deleted, or promoted).
