# Roko UX Follow-up Runner

Overnight Codex batch runner that implements every open P0/P1 item in the
post-PR-13 catalogue at `tmp/ux-followup/`. 47 batches, grouped into 8 phases,
designed to run unattended for multiple hours per night and resume cleanly.

Follows the same pattern as `tmp/tui-parity/` (the sibling TUI-parity runner
that shipped PR #13). Shares the same Codex-exec plumbing, retry/preserve-dirty
semantics, and verify-gate flow. If you have run `run-tui-parity.sh` before,
everything here behaves identically — only the batch IDs and prompts differ.

## Quick start

```bash
# List all batches
bash tmp/ux-followup-runner/run-ux-followup.sh --list

# Dry run (preview prompts; no Codex)
bash tmp/ux-followup-runner/run-ux-followup.sh --dry-run

# Run everything in dependency order (multi-night)
bash tmp/ux-followup-runner/run-ux-followup.sh --model gpt-5.4 --reasoning high

# Run a single group (e.g. self-hosting closure first night)
bash tmp/ux-followup-runner/run-ux-followup.sh --group selfhost

# Run a specific batch
bash tmp/ux-followup-runner/run-ux-followup.sh --only UX01

# Resume a prior run
bash tmp/ux-followup-runner/run-ux-followup.sh --continue last

# Verify only (re-run gates on existing worktree, no Codex)
bash tmp/ux-followup-runner/run-ux-followup.sh --verify-only --continue last

# Cap a session at N batches (night-at-a-time cadence)
bash tmp/ux-followup-runner/run-ux-followup.sh --max-batches 10 --continue last
```

## Architecture

```
tmp/ux-followup-runner/
├── run-ux-followup.sh           # Main runner
├── README.md                     # This file
├── BATCHES.md                    # Batch manifest + dependency graph + verify gates
├── lib/
│   ├── common.sh                 # Batch metadata (titles, deps, verify cmds, catalog refs)
│   ├── spawn.sh                  # Codex exec wrapper with per-batch delegation hints
│   └── verify.sh                 # Verify gate + commit + dirty-state backup
├── context-pack/
│   ├── 00-UX-FOLLOWUP-RULES.md  # Rules every batch must follow
│   ├── 01-CATALOG-MAP.md         # Which catalog items each batch closes
│   ├── 02-WORKSPACE-TOPOLOGY.md  # Key crates + their responsibilities
│   ├── 03-STATE-FLOW.md          # StateHub / DashboardSnapshot / EventBus primer
│   ├── 04-SAFETY-LAYER.md        # SafetyLayer + AgentContract + role context
│   └── 05-MORI-REFERENCE-APPENDIX.md # Current Mori anchors + Roko counterparts
├── prompts/
│   ├── UX01.prompt.md            # Gate-failure → plan-revision feedback loop
│   ├── UX02.prompt.md            # PRD-publish event → orchestrator
│   ├── …                         # 45 more
│   └── UX47.prompt.md            # tui-parity runner hardening + CI dry-run
└── logs/                         # Created at runtime; contains one dir per run
```

## Groups (8 phases, 47 batches)

| Group | Batches | Count | Max severity | Theme |
|-------|---------|-------|--------------|-------|
| `selfhost`   | UX01–UX04 | 4  | **P0** | Close CLAUDE.md items 10–11 + smoke test |
| `tui-stream` | UX05–UX11 | 7  | **P0** | Replace TUI polling with streaming / file-watch |
| `state`      | UX12–UX14 | 3  | P1     | Snapshot version / migration / process supervision |
| `observ`     | UX15–UX22 | 8  | P1     | Dashboard + metrics consumers |
| `wired`      | UX23–UX29 | 7  | P1     | Wire subsystems currently built-but-unused |
| `backends`   | UX30–UX34 | 5  | P1     | Codex / Cursor / cascade-router test parity |
| `hygiene`    | UX35–UX42 | 8  | P1     | Unwraps, coverage, validation, unused config |
| `docs`       | UX43–UX47 | 5  | P1     | Terminology, stale snapshots, runner hardening |

See [BATCHES.md](BATCHES.md) for the full per-batch manifest and dependency graph.
Prompt snapshots prepend `context-pack/00-05` before the batch prompt so each
run starts with the same zero-context-ready shared sources.

## Execution cadence (recommended)

### Night 1 — self-hosting closure (blocker)

```bash
bash tmp/ux-followup-runner/run-ux-followup.sh \
  --group selfhost --max-batches 4
```

Takes ~4–6 hours. Lands UX01 + UX02 (the last two CLAUDE.md "What to work on"
items) and UX03 + UX04 (smoke-test + plan-validate CLI).

### Night 2 — TUI streaming

```bash
bash tmp/ux-followup-runner/run-ux-followup.sh \
  --continue last --group tui-stream
```

Lands UX05–UX11. Deletes the TUI's polling code paths the user flagged as bugs.

### Night 3+ — everything else (one group per session)

```bash
# state + observability
bash tmp/ux-followup-runner/run-ux-followup.sh --continue last --group state,observ

# wired subsystems + backends
bash tmp/ux-followup-runner/run-ux-followup.sh --continue last --group wired,backends

# hygiene + docs
bash tmp/ux-followup-runner/run-ux-followup.sh --continue last --group hygiene,docs
```

Or run uncapped and let it go until Codex exhausts or the batches complete:

```bash
bash tmp/ux-followup-runner/run-ux-followup.sh --continue last
```

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--model` | `gpt-5.4` | Codex model |
| `--reasoning` | `high` | Reasoning effort |
| `--timeout` | `5400` | Per-batch timeout seconds (90 min) |
| `--retries` | `2` | Automatic retries per batch |
| `--base-ref` | `HEAD` | Git ref for new worktree |
| `--only LIST` | — | Comma-separated batch IDs (UX01-UX47) |
| `--group LIST` | — | Comma-separated group names |
| `--continue RUN` | — | Resume prior run (ID or 'last') |
| `--max-batches N` | 0 (∞) | Cap batches per session |
| `--include-p2` | off | Include P2-severity batches (none exist today) |
| `--dry-run` | — | Preview without Codex; does not update latest run |
| `--force` | — | Re-run successful batches |
| `--verify-only` | — | Re-run verification gates only; does not mark completion |
| `--list` | — | Show batch manifest |

## Environment overrides

All flags above can be set via env vars (`UX_MODEL`, `UX_REASONING`,
`UX_TIMEOUT`, `UX_MAX_RETRIES`, `UX_BASE_REF`, `UX_MAX_BATCHES`, `UX_SKIP_P2`).
`NO_COLOR=1` disables ANSI codes.

## Resumption & failure handling

- Each batch writes `<batch>.result` with one of `success`, `success_noop`,
  `dry_run`, `verify_only`, `commit_failed`, `spawn_failed`,
  `verify_failed`, `timeout`, `blocked`.
- A failure inside a batch keeps the dirty worktree state; `--continue last`
  resumes from the same batch, reusing the preserved changes.
- Dry-run manifests are ignored by `--continue last`, and `logs/latest`
  continues to point at the latest real run.
- `--verify-only` reruns the verification gates without marking the batch as
  complete, so a later normal run will still execute the batch.
- Verification dependencies (`batch_deps` in `lib/common.sh`) are respected;
  a blocked batch does not retry until its dependency lands.
- Per-batch status events append to `logs/<run-id>/status.tsv`, one line per
  transition.
- The prompt snapshot already includes the shared context pack and Mori
  appendix, so agents do not depend on prior chat to reconstruct the setup.

## Verification per batch

Every batch declares its own `cargo` verification commands in
`lib/common.sh::batch_verify_commands`. These run post-Codex; the commit is
only created after they pass. For example, UX01 runs
`cargo check -p roko-cli -p roko-orchestrator -p roko-runtime -p roko-learn`
plus `cargo clippy` with `-D warnings`.

Workspace-wide verification (for UX29 and a final pass) runs
`cargo check --workspace && cargo clippy --workspace --no-deps -- -D warnings`.
UX29 also verifies the ordinary root build surface with a plain `cargo check`
so any `default-members` cleanup is exercised too.

## Catalog mapping

Each prompt includes a `Catalog refs:` line (visible on dry-run output) that
names the items in `tmp/ux-followup/` it closes. See
[context-pack/01-CATALOG-MAP.md](context-pack/01-CATALOG-MAP.md) for the
full cross-reference.

## Overnight expectations

At the recommended defaults (90 min timeout, 2 retries), a full end-to-end
pass takes ~70–140 Codex hours across all 47 batches. Allocate:

- 1 night for Group A (selfhost) — P0
- 1–2 nights for Group B (tui-stream) — P0
- 1 night for Group C (state)
- 1–2 nights for Group D (observ)
- 1–2 nights for Group E (wired)
- 1 night for Group F (backends)
- 1–2 nights for Groups G+H (hygiene + docs)

Plan on ~7–10 sleep-sized sessions end-to-end.

## Exit codes

- `0` — all selected batches succeeded
- `1` — at least one batch failed terminally after all retries

## See also

- [tmp/tui-parity/](../tui-parity/) — sibling runner for T1–T19 TUI-parity work
  (already merged via PR #13; retained as reference)
- [tmp/ux-followup/](../ux-followup/) — the source catalogue (files 01–15)
- [tmp/ux-followup/00-INDEX.md](../ux-followup/00-INDEX.md) — severity matrix
  + post-PR-13 delta

## Known limitations

1. Batches assume `rustup toolchain list` has a stable version ≥ 1.91
   (`alloy` dep requirement per CLAUDE.md).
2. Codex worktrees land on `codex/ux-followup-<run-id>` branches, never on
   `main`. You manually merge after review.
3. P2 items from `tmp/ux-followup/08-phase-2-vision.md` are **not** in this
   runner — they are parked until the P0/P1 work is green.
