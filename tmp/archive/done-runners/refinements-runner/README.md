# Roko Refinements Runner

Overnight Codex batch runner that propagates the 35 refinement proposals in
`tmp/refinements/` into the canonical `docs/` tree. Docs-only — the runner
never touches code.

Follows the same pattern as `tmp/ux-followup-runner/` (sibling). Shares the
Codex-exec plumbing, retry semantics, worktree lifecycle, and status
accounting. If you have run `run-ux-followup.sh`, everything here behaves
identically except for the verify gate: instead of `cargo check`, the runner
uses terminology/scope/diff/required-vocab gates.

## Quick start

```bash
# List all batches (REF01..REF35)
bash tmp/refinements-runner/run-refinements.sh --list

# Dry run (renders prompt snapshots; no Codex spawn)
bash tmp/refinements-runner/run-refinements.sh --dry-run

# Run everything in dependency order (multi-night)
bash tmp/refinements-runner/run-refinements.sh --model gpt-5.4 --reasoning high

# Run a single phase (e.g. foundation first night)
bash tmp/refinements-runner/run-refinements.sh --group foundation

# Run a specific refinement
bash tmp/refinements-runner/run-refinements.sh --only REF02

# Resume a prior run
bash tmp/refinements-runner/run-refinements.sh --continue last

# Verify-only (re-run gates, no Codex)
bash tmp/refinements-runner/run-refinements.sh --verify-only --continue last

# Cap at N batches per session
bash tmp/refinements-runner/run-refinements.sh --max-batches 10 --continue last
```

## Architecture

```
tmp/refinements-runner/
├── run-refinements.sh            # Main runner
├── README.md                      # This file
├── BATCHES.md                     # Batch manifest + dependency graph
├── lib/
│   ├── common.sh                  # Batch metadata, retired terms, required vocab
│   ├── spawn.sh                   # Codex exec + prompt-snapshot composition
│   └── verify.sh                  # Terminology + scope + diff + required-term gates
├── context-pack/
│   ├── 00-REFINEMENTS-RULES.md   # Rules every batch must follow
│   ├── 01-TWO-FABRIC-PRIMER.md   # Canonical vocabulary
│   ├── 02-TERMINOLOGY-TABLE.md   # Retired → current terms
│   ├── 03-DOCS-TREE-MAP.md       # docs/ subsystem map
│   ├── 04-SYNERGY-SUMMARY.md     # Condensed 31-synergy doc
│   └── 05-REFINEMENTS-INDEX.md   # One-sentence summaries of all 35
├── prompts/
│   ├── REF01.prompt.md           # Critique one-noun across docs/00-architecture
│   ├── REF02.prompt.md           # Introduce Pulse
│   ├── …                         # 33 more
│   └── REF35.prompt.md           # Consolidated roadmap
└── logs/                          # Created at runtime; one dir per run
```

## Groups (6 phases, 35 batches)

| Group | Batches | Count | Theme |
|-------|---------|-------|-------|
| `foundation`  | REF01–REF09 | 9 | Two-medium / two-fabric kernel reframing |
| `learning`    | REF10–REF16 | 7 | Self-learning, HDC, demurrage, c-factor, heuristics, scaling, research |
| `moat`        | REF17–REF21 | 5 | Plugin SPI, moat, innovations, modularity, rewrites |
| `ux-core`     | REF22–REF25 | 4 | Dev UX, user UX, deployment UX, domain profiles |
| `ux-surface`  | REF26–REF30 | 5 | StateHub, realtime, CLI, web UI, rich primitives |
| `integrator`  | REF31–REF35 | 5 | Synergy, safety, observability, glossary, roadmap |

See [BATCHES.md](BATCHES.md) for the full per-batch manifest + dependency
graph + required vocabulary.

## Execution cadence (recommended)

### Night 1 — Foundation (REF01–REF09)

```bash
bash tmp/refinements-runner/run-refinements.sh --group foundation
```

Lands the two-medium / two-fabric reframing into `docs/00-architecture/`.
This is the highest-impact pass — every subsequent batch relies on it.

### Night 2 — Learning (REF10–REF16)

```bash
bash tmp/refinements-runner/run-refinements.sh --continue last --group learning
```

Demurrage, HDC, heuristics, c-factor, compounding, replication-ledger
framing lands into `docs/05-learning/`, `docs/06-neuro/`,
`docs/13-coordination/`, `docs/21-references/`.

### Night 3 — Moat (REF17–REF21)

```bash
bash tmp/refinements-runner/run-refinements.sh --continue last --group moat
```

Plugin SPI + modularity + rewrite-candidates.

### Night 4 — UX core (REF22–REF25)

```bash
bash tmp/refinements-runner/run-refinements.sh --continue last --group ux-core
```

Developer SDK + user UX + deployment UX + domain profiles.

### Night 5 — UX surface (REF26–REF30)

```bash
bash tmp/refinements-runner/run-refinements.sh --continue last --group ux-surface
```

StateHub + realtime + CLI + web UI + rich primitives. Mostly lands in
`docs/12-interfaces/`.

### Night 6 — Integrators (REF31–REF35)

```bash
bash tmp/refinements-runner/run-refinements.sh --continue last --group integrator
```

Synergy, safety, observability, glossary, roadmap. These finalize the
narrative thread across earlier batches.

### Alternatively — run uncapped overnight

```bash
bash tmp/refinements-runner/run-refinements.sh --continue last
```

Let it go until Codex exhausts or all batches complete.

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--model` | `gpt-5.4` | Codex model |
| `--reasoning` | `high` | Reasoning effort |
| `--timeout` | `5400` | Per-batch timeout seconds (90 min) |
| `--retries` | `2` | Automatic retries per batch |
| `--base-ref` | `HEAD` | Git ref for new worktree |
| `--only LIST` | — | Comma-separated batch IDs (REF01-REF35) |
| `--group LIST` | — | Comma-separated group names |
| `--continue RUN` | — | Resume prior run (ID or 'last') |
| `--max-batches N` | 0 (∞) | Cap batches per session |
| `--dry-run` | — | Preview without Codex |
| `--force` | — | Re-run successful batches |
| `--verify-only` | — | Re-run verification gates only |
| `--list` | — | Show batch manifest |

Environment overrides: `REF_MODEL`, `REF_REASONING`, `REF_TIMEOUT`,
`REF_MAX_RETRIES`, `REF_BASE_REF`, `REF_MAX_BATCHES`,
`REF_LINK_CHECK_STRICT` (when `1`, broken internal links fail the batch;
default soft warning), `NO_COLOR`.

## Verify gates (no cargo; docs only)

Each batch goes through four gates after Codex finishes:

1. **`scope_gate`** — fails if any file outside `docs/` is modified.
   Hard failure. Protects the runner from silently editing code.
2. **`diff_gate`** — fails if no file under `docs/` was changed. Batch
   must produce substantive edits.
3. **`terminology_check`** — fails if retired terms (see
   `context-pack/02-TERMINOLOGY-TABLE.md`) appear in changed lines
   *outside* allowed contexts ("retired", "deprecated", "formerly",
   "historical", "legacy", "old name", "renamed", "see also").
4. **`required_terms_check`** — fails if the refinement's expected new
   vocabulary (per `batch_required_terms` in `lib/common.sh`) is absent
   from every changed file.
5. **`internal_link_check`** — soft warning by default; set
   `REF_LINK_CHECK_STRICT=1` to fail the batch on broken internal `.md`
   links.

All gates run in sequence; first failure aborts the batch and triggers a
retry (worktree state preserved).

## Resumption & failure handling

Same as the sibling runners. Each batch writes `<batch>.result` with one
of: `success`, `success_noop`, `dry_run`, `verify_only`,
`verify_failed`, `spawn_failed`, `timeout`, `commit_failed`, `blocked`.

- Failure keeps the worktree dirty; `--continue last` resumes from the
  same batch using the preserved state plus the failure summary.
- Dependencies (`batch_deps` in `lib/common.sh`) are respected; a blocked
  batch does not retry until its dependency lands.
- Per-batch status events append to `logs/<run-id>/status.tsv`.
- Prompt snapshots include the shared context pack + the canonical
  refinement source so agents don't need prior chat history.

## What the agent sees per batch

Each spawn assembles a prompt with:

1. Run metadata (id, attempt, model, reasoning, target docs).
2. Shared context pack (6 files above).
3. Delegation guidance (suggested explorer/worker splits).
4. The canonical refinement source — the verbatim proposal from
   `tmp/refinements/NN-*.md`.
5. The per-batch prompt from `prompts/REFnn.prompt.md`.

The agent has everything it needs to propagate the refinement into docs
with zero reliance on prior conversation.

## Overnight expectations

At the recommended defaults (90 min timeout, 2 retries):

- Foundation (9 batches): ~10–15 Codex hours, good for one long night.
- Learning (7): ~8–12 hours.
- Moat (5): ~6–9 hours.
- UX core (4): ~5–7 hours.
- UX surface (5): ~6–9 hours.
- Integrators (5): ~6–9 hours.

Total: 40–60 Codex hours across 6 sessions — ~6 sleep-sized nights.

## Exit codes

- `0` — all selected batches succeeded (or were blocked by deps in a
  dep-respecting run).
- `1` — at least one batch failed terminally after all retries.

## Safety

- Worktrees land on `codex/refinements-<run-id>` branches. Never on
  `main`.
- Manual merge required after review.
- Scope gate enforces docs-only edits — if the runner catches a batch
  touching code, the batch fails and the run stops.
- Branches preserved after failures; manual investigation always
  possible.

## See also

- [tmp/ux-followup-runner/](../ux-followup-runner/) — sibling runner for
  code-level P0/P1 closures (different verify gate, same plumbing).
- [tmp/refinements/](../refinements/) — the source of all 35 proposals.
- [tmp/refinements/00-INDEX.md](../refinements/00-INDEX.md) — master
  index with suggested reading orders.

## Known limitations

1. Agents occasionally interpret "aggressive edit posture" as license to
   restructure a doc's headings. Prefer the dry-run first on new
   refinement batches and inspect the prompt snapshot before a real run.
2. Dense cross-references across many refinement docs can produce
   inconsistent link targets when batches land out of order. The
   dep-DAG tries to minimize this; verify manually after the
   `integrator` group lands.
3. The terminology check is a grep; it can flag false positives when a
   retired term appears in a quoted code sample. Mark such lines with
   a clear "retired: " prefix or move them into a fenced code block
   with a comment indicating historical context.
