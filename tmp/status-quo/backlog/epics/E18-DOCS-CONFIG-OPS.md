# E18 — Docs, Config, CI & Ops Hygiene

> Executable backlog epic · verified against HEAD `5852c93c05` · sources: `19-DOC-DRIFT-REGISTER`, `65-DOCS-CONVERGENCE-PLAN`, `81-ROOT-DOCS-REWRITE-QUEUE`, `82-COMMAND-EXAMPLE-DRIFT-LEDGER`, `57-CONFIG`, `50-QUALITY-CI-RELEASE`, `71-CI-RELEASE-PROOF-GAPS`, `58-JOBS-DEPLOY`, `77-OPERATIONS-DEPLOY-RUNBOOK`, `83-ENV-VAR-MANIFEST`
> Native task schema: `crates/roko-cli/src/task_parser.rs::TaskDef` · exemplars: `plans/P17-cli-output-format/tasks.toml`, `plans/P30-onboarding-doctor/tasks.toml`
> Subsumes census epics **E-DOCS-TRUTH**, **E-DOC-PROVENANCE**, **E-OPS-PROOF**, **E-CI-PROOF** (see `backlog/02-PLANS-RECONCILIATION.md §3`).
> **Doc-rewrite tasks depend on E01** (engine decision — so docs may honestly name the default engine) **and on this epic's own config/ops fixes** (T05–T08), so the prose describes the *fixed* state, not the current lie.

## Why this epic

This is the "make it trustworthy & shippable" epic. Two failure modes: **the repo lies to its readers**, and
**the pipeline that ships it does not prove what it claims**. The maintained root docs (`CLAUDE.md`,
`README.md`) describe a codebase that no longer exists — 18 crates when there are 35, `orchestrate.rs` as the
canonical loop when it is `#[cfg(feature = "legacy-orchestrate")]` and not compiled by default, "1 noun
(Signal)" when the code noun is `Engram`, ~85 routes when serve exposes ~270, 19 builtin tools when
`TOOL_COUNT = 37`, and a self-host workflow that runs `roko plan run` (a **dry-run Graph**, not the live
Runner v2). Meanwhile CI ships releases with no pre-build test/clippy gate, the MSRV declared in `Cargo.toml`
(1.85) disagrees with the MSRV job (1.91), a `deny.toml` sits unused with no `cargo-deny` workflow, coverage
runs green on failing tests (`--ignore-run-fail`), and the root `Dockerfile` `COPY`s an **untracked**
`roko.toml` so a clean-checkout build fails. Config carries a dual-parser **silent drop** and leaks secrets
in `config show --effective`.

## The one thing that matters

**Order the doc rewrites AFTER the fixes.** Rewriting `CLAUDE.md`/`README.md` is mechanical and high-value,
but if it runs before the engine decision (E01) and the config/ops fixes here (T05–T08) it just replaces one
set of false claims with another (e.g. re-documenting `roko plan run` as the live path while it is still a
dry-run graph). The count/noun/path corrections (35 members, `Engram`, 37 tools, 10 TUI tabs, `roko
knowledge`, `--bind/--port`, `/health`) are true **today**; the engine-default and config-behavior claims are
true **only after** the fix tasks land. The rewrite tasks therefore gate on E01 + T05–T08, and a docs-lint CI
job (T13) then freezes the corrected state so the drift cannot silently return.

## Findings — grouped

### Docs (E-DOCS-TRUTH / E-DOC-PROVENANCE)

| # | Finding | Evidence | Fixed by |
|---|---|---|---|
| D1 | `CLAUDE.md` dated 2026-04-20; claims 18 crates / ~177K LOC | actual **35 workspace members** (31 crates + 3 apps + 1 tests pkg); `Cargo.toml [workspace].members` | T10 |
| D2 | `orchestrate.rs` centered as canonical loop | it is `#[cfg(feature = "legacy-orchestrate")]`, **not compiled by default** (`lib.rs:94`); live path is `crates/roko-cli/src/runner/event_loop.rs` | T10, T11 |
| D3 | "1 noun (Signal)" | code noun is `Engram` (`roko-core/src/engram.rs:63`); no `struct Signal` in roko-core; sidecar already returns `engram_id` (README:302 self-contradicts) | T10, T11 |
| D4 | "~85 routes" | serve exposes **~270 routes** (generated manifest); README:152,248 | T10, T11 |
| D5 | "19 builtin tools" | `TOOL_COUNT = 37` | T10, T11 |
| D6 | F1–F7 (7 TUI tabs) | **10 tabs** (Dashboard/Plans/Agents/Git/Logs/Config/Inspect/Marketplace/Atelier/Learning; F1–F9 + `0`); README:72-84 | T10, T11 |
| D7 | "safety falls back to permissive default" | fail-closed `restricted`/deny-everything on missing contract (`contract.rs:88-89,133-140`) — true today, safe to assert | T10 |
| D8 | Self-host workflow teaches default `roko plan run plans/` | default engine is Graph **dry-run** (`main.rs:1361 default_value="graph"`); real path is `--engine runner-v2` | T10, T11 (gate on **E01**) |
| D9 | `.roko/state/executor.json` cited as canonical snapshot | Runner-v2 canonical is `.roko/state/state-snapshot.json` (`persist.rs:45-46`) | T10 |
| D10 | `roko neuro`, `--listen`, `/healthz`/`/readyz` | now `roko knowledge`, `--bind`/`--port`, `/health`/`/ready` (README:224-227,430,245; sidecar) | T10, T11, T12 |
| D11 | Drift can silently return (no lint guard) | doc `82` catalogs stale command examples; no CI grep-guard | T13 |

### Config (E04-adjacent — redaction overlaps E-PERIMETER)

| # | Finding | Evidence | Fixed by |
|---|---|---|---|
| C1 | Dual `roko.toml` schema, silent drop | `load_resolved_config` (config.rs:2896-2911) calls the authoritative core loader at **config.rs:2904** (`let _core_validated = …`), **discards it**, then re-parses the same file via legacy `ConfigLayer` merge (config.rs:927-1050). Neither parser is `deny_unknown_fields` → keys valid in one schema are silently ignored by the other | T06 |
| C2 | `config show --effective` prints secrets **unredacted** | `config_cmd.rs:215-229`; serve masks the same fields (`routes/config.rs:306-336`, `mask_secret_fields`) — inconsistent policy, the P0 leak | T07 |
| C3 | ~2 runtime-dead config surfaces + `cold_storage` **config review** | `conductor.watchers.*` (read only from legacy `orchestrate.rs:6318`, never in runner-v2); `conductor.context_pressure_enabled` (self-documented dead, `schema.rs:1270-1276`). **NB: `cold_storage` is NOT runtime-dead** — the hourly serve timer consumes it (`start_cold_archival_timer`, `serve/lib.rs:344,800,2097`); its copy-not-move growth **bug** is owned by **E02-T12**. Here C3/T09 only reviews the `cold_storage` config surface (field docs / defaults / doctor note), not the runtime path. | T09 (+ E02-T12 for the bug) |
| C4 | Zero-config preflight ignores `BUILTIN_MODELS` | `preflight_provider_for_model` (util.rs:1883) resolves only `config.models`→`config.providers`; never consults `BUILTIN_MODELS` (`config/model_registry.rs:33`, 15 entries) | **owned by P20** (see reconciliation) |

### CI (E-CI-PROOF)

| # | Finding | Evidence | Fixed by |
|---|---|---|---|
| I1 | `release.yml` ships with **no pre-build test/clippy** | `release.yml:61-64` is `cargo build` only; the clippy+test job lives in `ci.yml` which triggers on push/PR, **not on tag push** — a tag can ship un-gated | T02 |
| I2 | MSRV drift | `Cargo.toml:93 rust-version = "1.85"` vs `msrv.yml:21 toolchain "1.91"` (and `Dockerfile:20 rust:1.91`) | T01 |
| I3 | `deny.toml` exists but no `cargo-deny` workflow | `/deny.toml` present; no workflow invokes `cargo deny check` | T03 |
| I4 | Coverage is green on failing tests | `coverage.yml:20` passes `--ignore-run-fail` | T04 |

### Deploy (E-OPS-PROOF)

| # | Finding | Evidence | Fixed by |
|---|---|---|---|
| P1 | Root `roko.toml` untracked but `Dockerfile` COPYs it | `Dockerfile:77 COPY roko.toml /workspace/roko.toml`; file is not in git → clean-checkout `docker build` fails | T05 |
| P2 | `roko deploy docker` has no push | CLI docker deploy builds but never `docker push`; only `docker-publish.yml` (uses `docker/roko.Dockerfile`, not the root Dockerfile) pushes | T08 |
| P3 | Fly / compose port & flag drift | root `Dockerfile:108` HEALTHCHECK hits `/health`; `deploy-fly.yml:71` check path is `/api/health`; `docker/docker-compose.yml` still passes `--listen` (removed flag) | T08, T12 |

## Reconciliation with existing P-plans

| Plan | Relationship to E18 | Action |
|---|---|---|
| **P17-cli-output-format** | `CliOutput` wrapper / `eprintln!` cleanup — pure terminal cosmetics, no doc/config/CI overlap | Independent — run on its own track |
| **P20-zero-config** | Owns finding **C4** (consult `BUILTIN_MODELS` in preflight, `util.rs:1883`) | **E18 does NOT re-own C4** — defer to P20; E18 only documents the resulting zero-config path in T10/T11 once P20 lands |
| **P30-onboarding-doctor** | OpenAI/Gemini key checks + per-provider validation + init/setup hints — provider/onboarding UX | Overlaps the *secrets/provider* doc surface; E18 config tasks (T06/T07) are orthogonal to P30's doctor checks. Keep separate; cross-link in docs (T11) |
| **P32-cli-polish** | `skip_serializing_if` on `ModelProfile` bools; emoji swap — cosmetic | Independent |
| **P33-model-ux** | `max_tokens` auto-recovery in `CodexAgent` — provider UX | Independent |

Net: E18 claims the **docs / CI / deploy** hygiene and the **dual-config + secret-redaction** config fixes.
It cedes zero-config preflight (C4) to **P20** and provider onboarding to **P30**, and touches none of the
cosmetic CLI plans. Secret redaction (C2/T07) also appears in **E04-SECURITY-PERIMETER / E-PERIMETER** — E18
is the executing owner; E04 should mark it satisfied by `E18-T07`.

## Task breakdown (E18-Txx)

| Task | Tier | Summary | Depends |
|---|---|---|---|
| **E18-T01** | mechanical | Fix MSRV drift: `Cargo.toml [workspace].rust-version` 1.85→1.91 (match `msrv.yml` + `Dockerfile rust:1.91`) | — |
| **E18-T02** | small | Add a pre-build test+clippy gate job to `release.yml`; make `build` `needs:` it, so no tag ships un-gated | — |
| **E18-T03** | small | Add `.github/workflows/deny.yml` running `cargo deny check` against the existing `deny.toml` | — |
| **E18-T04** | mechanical | Drop `--ignore-run-fail` from `coverage.yml` so coverage fails when tests fail | — |
| **E18-T05** | small | Fix Docker clean-checkout: track a canonical `docker/roko.toml` and `COPY docker/roko.toml …` (or generate at startup), so `git archive`-based builds succeed | — |
| **E18-T06** | medium | Collapse the dual-config silent drop: make `load_resolved_config` *use* the authoritative `_core_validated` (or make `roko_cli::config::Config` a projection of `RokoConfig`); at minimum stop discarding it and warn on unknown keys | — |
| **E18-T07** | small | Redact secrets in CLI `config show`/`--effective` by reusing serve's `mask_secret_fields` (config_cmd.rs:215-229) | — |
| **E18-T08** | medium | `roko deploy docker` push flag + align Fly/compose port & health: `--listen`→`--bind`/`--port`, `/api/health`↔`/health` consistency | — |
| **E18-T09** | small | Document/deprecate the 2 runtime-dead config surfaces (`conductor.watchers.*`, `conductor.context_pressure_enabled`): doc-comment + `roko doctor` note, or delete. Also **review** the `cold_storage` config surface (field docs / sane defaults / doctor note) — but note `cold_storage` **is** runtime-live (hourly serve timer) and its copy-not-move growth bug is owned by **E02-T12**, not this task | — |
| **E18-T10** | mechanical | Rewrite `CLAUDE.md` from the status-quo truth (counts, `Engram`, Runner v2, fail-closed safety, snapshot path, `roko knowledge`) | E01, T06, T07 |
| **E18-T11** | mechanical | Rewrite `README.md` likewise (scrub the 5 cross-cutting drift threads); use `--engine runner-v2` in every real run example | E01, T06, T07, T10 |
| **E18-T12** | small | Patch `docs/v2/{CLI-REFERENCE,04-EXECUTION,INTEGRATION-GUIDE,25-DEPLOYMENT}` + `docker/README.md` with engine semantics, `/health`/`/ready`, `--bind`/`--port`, clean-checkout | E01, T05, T08 |
| **E18-T13** | small | Add a docs-lint CI job (grep-guard the doc `82` forbidden patterns: `roko neuro`, `--listen`, `18 crates`, `F1-F7`, `~85 routes`, default `plan run plans/`) so drift cannot silently return | T10, T11 |

## First 3 tasks (executable TOML)

```toml
[meta]
plan = "E18-DOCS-CONFIG-OPS"
total = 13
done = 0
status = "ready"
max_parallel = 3

# ─────────────────────────────────────────────────────────────────────────────
# E18-T01: Fix MSRV drift (Cargo.toml 1.85 -> 1.91)
# ─────────────────────────────────────────────────────────────────────────────
#
# The workspace declares rust-version = "1.85" (Cargo.toml:93) but the MSRV CI
# job pins toolchain "1.91" (.github/workflows/msrv.yml:21) and the release
# Docker image is rust:1.91-slim-bookworm (Dockerfile:20). The alloy deps also
# require 1.91+ (CLAUDE.md "Known blockers"). Bump the single declared MSRV so
# the manifest, the MSRV gate, and the build image all agree.
#
[[task]]
id = "E18-T01"
title = "Bump workspace rust-version 1.85 -> 1.91 to match msrv.yml + Dockerfile"
status = "ready"
tier = "mechanical"
model_hint = "claude-haiku-4-5"
max_loc = 3
files = ["Cargo.toml"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "Cargo.toml", lines = "88-98", why = "[workspace.package] rust-version = \"1.85\" at :93 — the stale MSRV" },
    { path = ".github/workflows/msrv.yml", lines = "16-23", why = "MSRV job pins toolchain \"1.91\"; comment says it must match Cargo.toml rust-version" },
    { path = "Dockerfile", lines = "18-21", why = "Release image is rust:1.91-slim-bookworm — the effective floor" },
]
symbols = []
anti_patterns = [
    "Do NOT change msrv.yml or the Dockerfile — Cargo.toml is the one that is wrong.",
    "Do NOT bump edition or any other version key; only rust-version.",
    "Do NOT add per-crate rust-version overrides; the workspace key is authoritative.",
]

[[task.verify]]
phase = "structural"
command = "grep -q 'rust-version = \"1.91\"' Cargo.toml"
fail_msg = "Cargo.toml workspace rust-version must be 1.91"

[[task.verify]]
phase = "structural"
command = "! grep -q 'rust-version = \"1.85\"' Cargo.toml"
fail_msg = "The stale 1.85 MSRV must be gone"

[[task.verify]]
phase = "compile"
command = "cargo metadata --no-deps --format-version 1 >/dev/null 2>&1"
fail_msg = "Cargo.toml must still parse after the MSRV bump"

acceptance = "Cargo.toml declares rust-version 1.91, matching msrv.yml (toolchain 1.91) and Dockerfile (rust:1.91). `cargo metadata` succeeds."


# ─────────────────────────────────────────────────────────────────────────────
# E18-T02: Add a pre-build test+clippy gate to the release workflow
# ─────────────────────────────────────────────────────────────────────────────
#
# release.yml triggers on tag push (v*) and jumps straight to `cargo build`
# (:61-64) — it never runs clippy or tests. The clippy+test job lives in ci.yml,
# which triggers on push/PR to main, NOT on tags. So a tag cut from a commit that
# never went through CI ships un-gated binaries. Add a `check` job (clippy -D
# warnings + workspace test) and make the `build` matrix job `needs: check`, so
# the release aborts before packaging if the tagged commit is red.
#
[[task]]
id = "E18-T02"
title = "Gate release.yml on cargo clippy + cargo test before building binaries"
status = "ready"
tier = "small"
model_hint = "claude-sonnet-4-5"
max_loc = 30
files = [".github/workflows/release.yml"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = ".github/workflows/release.yml", lines = "25-77", why = "The `build` job goes straight to cargo build with no test/clippy gate — insert a `check` job it depends on" },
    { path = ".github/workflows/ci.yml", lines = "12-24", why = "Reuse this exact clippy+test recipe (toolchain@stable + components: clippy; cargo clippy --workspace --no-deps -D warnings; cargo test --workspace)" },
]
symbols = []
anti_patterns = [
    "Do NOT weaken the existing ci.yml — this is an ADDITIONAL gate scoped to the tag/release trigger.",
    "Do NOT run the gate per-target in the build matrix (wasteful); one ubuntu-latest `check` job that `build` needs is enough.",
    "Do NOT drop `-D warnings` from clippy; the release gate must be as strict as CI.",
    "Do NOT change the release/artifact jobs beyond adding the `needs: check` edge on `build`.",
]

[[task.verify]]
phase = "structural"
command = "grep -q 'cargo test --workspace' .github/workflows/release.yml && grep -q 'cargo clippy --workspace' .github/workflows/release.yml"
fail_msg = "release.yml must run clippy and tests before building"

[[task.verify]]
phase = "structural"
command = "grep -q 'needs: check' .github/workflows/release.yml || grep -qE 'needs:\\s*\\[?\\s*check' .github/workflows/release.yml"
fail_msg = "The build job must depend on the new check gate"

[[task.verify]]
phase = "structural"
command = "python3 -c \"import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml'))\""
fail_msg = "release.yml must remain valid YAML"

acceptance = "A release tag now runs clippy -D warnings + `cargo test --workspace` and refuses to build/publish binaries if either fails."


# ─────────────────────────────────────────────────────────────────────────────
# E18-T03: Add a cargo-deny CI workflow (the unused deny.toml)
# ─────────────────────────────────────────────────────────────────────────────
#
# A deny.toml sits at the repo root but NO workflow ever runs cargo-deny, so the
# advisory/license/bans policy is dead. Add .github/workflows/deny.yml that
# installs cargo-deny and runs `cargo deny check` on push/PR, using the existing
# deny.toml as-is (do not author a new policy here).
#
[[task]]
id = "E18-T03"
title = "Add cargo-deny workflow running `cargo deny check` against deny.toml"
status = "ready"
tier = "small"
model_hint = "claude-sonnet-4-5"
max_loc = 35
files = [".github/workflows/deny.yml"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "deny.toml", lines = "1-40", why = "The existing policy this workflow enforces — do NOT rewrite it, just run it" },
    { path = ".github/workflows/ci.yml", lines = "1-24", why = "Match the on: push[main]/pull_request trigger + actions/checkout@v4 + Swatinem/rust-cache@v2 conventions used across this repo" },
    { path = ".github/workflows/msrv.yml", lines = "1-24", why = "Reference for a small single-job workflow shape" },
]
symbols = []
anti_patterns = [
    "Do NOT edit deny.toml — the task is to wire the existing policy into CI, not tune it.",
    "Do NOT use a bespoke `cargo install cargo-deny` if a maintained action exists (prefer taiki-e/install-action@cargo-deny or EmbarkStudios/cargo-deny-action).",
    "Do NOT make the job non-blocking (no continue-on-error) — a supply-chain violation must fail the build.",
]

[[task.verify]]
phase = "structural"
command = "test -f .github/workflows/deny.yml && grep -q 'cargo deny check' .github/workflows/deny.yml"
fail_msg = "deny.yml must exist and run `cargo deny check`"

[[task.verify]]
phase = "structural"
command = "python3 -c \"import yaml,sys; yaml.safe_load(open('.github/workflows/deny.yml'))\""
fail_msg = "deny.yml must be valid YAML"

[[task.verify]]
phase = "structural"
command = "! grep -q 'continue-on-error' .github/workflows/deny.yml"
fail_msg = "The cargo-deny gate must be blocking, not advisory"

acceptance = "`.github/workflows/deny.yml` runs `cargo deny check` on push/PR against the existing deny.toml and blocks on any advisory/license/ban violation."
```

## Remaining tasks (E18-T04 .. E18-T13)

Authored in the same schema when scheduled; key parameters and proof gates:

- **E18-T04** (mechanical, `.github/workflows/coverage.yml:20`): remove `--ignore-run-fail` from the
  `cargo llvm-cov` invocation. Verify: `! grep -q 'ignore-run-fail' .github/workflows/coverage.yml`; coverage
  job now fails when a test fails.
- **E18-T05** (small, `Dockerfile:77` + new `docker/roko.toml`): stop `COPY roko.toml` of an untracked file —
  track a canonical `docker/roko.toml` (or generate a default in `start-railway`) and `COPY docker/roko.toml
  /workspace/roko.toml`. Verify: `git ls-files docker/roko.toml` is non-empty; clean-checkout
  `git archive HEAD | tar -x -C /tmp/clean && docker build /tmp/clean` succeeds.
- **E18-T06** (medium, `crates/roko-cli/src/config.rs:2896-2911,927-1050`): make `load_resolved_config` consume
  the authoritative `_core_validated` result (port `auto_plan/repos/[[gate]]/dreams/daimon/runner.plan_timeout_secs`
  into `RokoConfig` or expose them through it) and drop the discard + legacy `ConfigLayer` re-parse. Verify:
  `grep -c '_core_validated' crates/roko-cli/src/config.rs` → 0 (no unused-binding discard);
  `cargo run -p roko-cli -- config show` still resolves the CLI-only fields.
- **E18-T07** (small, `crates/roko-cli/src/commands/config_cmd.rs:215-229`): reuse serve's `mask_secret_fields`
  (`routes/config.rs:306-336`) in `config show`/`--effective`. Verify:
  `ANTHROPIC_API_KEY=sk-test cargo run -p roko-cli -- config show --effective | grep -c 'sk-test'` → 0.
  (Also closes the E04-PERIMETER redaction finding.)
- **E18-T08** (medium, deploy command source + `docker/docker-compose.yml` + `deploy-fly.yml`): add a
  `--push` to `roko deploy docker`; unify health/flag drift (`--listen`→`--bind`/`--port`; make the Fly check
  path and Dockerfile HEALTHCHECK agree). Verify: `roko deploy docker --push` reaches a `docker push`;
  `! grep -q -- '--listen' docker/docker-compose.yml`.
- **E18-T09** (small, `schema.rs` doc-comments + `roko doctor`): mark `conductor.watchers.*` and
  `conductor.context_pressure_enabled` as deprecated/runtime-dead with a doctor note, or delete. **Do NOT
  mark `cold_storage` as runtime-dead** — it is consumed hourly by the serve cold-archival timer
  (`start_cold_archival_timer`, `serve/lib.rs:344,800,2097`); scope for `cold_storage` here is a **config-surface
  review** only (field docs, sane defaults, optional doctor note). The `cold_storage` copy-not-move growth **bug**
  (archival copies instead of moves → unbounded cold-file growth) is a real, runtime-live defect owned by
  **E02-T12** — cross-reference it, do not double-own it. Verify: `roko doctor` surfaces the dead-config warning
  for the 2 conductor keys; schema comments name the runtime gap; `cold_storage` is not labelled runtime-dead.
- **E18-T10** (mechanical, `CLAUDE.md`) — **depends on E01, T06, T07**: rewrite from the status-quo truth —
  35 workspace members, `Engram` noun, Runner v2 as the live path (`orchestrate.rs` = legacy/opt-in),
  fail-closed safety, `state-snapshot.json`, 37 tools, 10 TUI tabs, `roko knowledge`. Verify:
  `! grep -qE '18 crates|1 noun \\(Signal\\)|F1.?F7' CLAUDE.md` and `grep -q '35 workspace members' CLAUDE.md`.
- **E18-T11** (mechanical, `README.md`) — **depends on E01, T06, T07, T10**: scrub the 5 cross-cutting drift
  threads; every real run example uses `--engine runner-v2`; `roko knowledge`, `--bind`/`--port`. Verify:
  `! grep -qE '18 crates|roko neuro|--listen|~85 routes|F1.?F7' README.md`; README crate count reads 35.
- **E18-T12** (small, `docs/v2/{CLI-REFERENCE,04-EXECUTION,INTEGRATION-GUIDE,25-DEPLOYMENT}.md`,
  `docker/README.md`) — **depends on E01, T05, T08**: engine semantics, `/health`/`/ready`, `--bind`/`--port`,
  clean-checkout. Verify: `! grep -rqE '/healthz|/readyz|--listen' docs/v2 docker/README.md`.
- **E18-T13** (small, new `.github/workflows/docs-lint.yml` or a `ci.yml` step) — **depends on T10, T11**:
  grep-guard the doc `82` forbidden patterns so drift cannot silently return. Verify: the job greps
  `README.md CLAUDE.md docs/` for `roko neuro|--listen|18 crates|F1-F7|~85 routes|roko plan run plans/`
  (default-engine example) and exits non-zero on a hit; run it against the rewritten docs to confirm it is green.

## Proof gate for the epic

Per `backlog/02-PLANS-RECONCILIATION.md §4`, a type/route/doc existing is not "done" — the default path must
exercise it. E18 closes when: (a) a release tag runs clippy+test+`cargo deny check` and cannot ship red;
(b) `cargo metadata` MSRV == `msrv.yml` == Docker image; (c) a clean `git archive` checkout `docker build`
succeeds; (d) `config show --effective` leaks zero secret bytes; (e) `README`/`CLAUDE.md` pass the docs-lint
grep-guard with the corrected counts/nouns/engine.
