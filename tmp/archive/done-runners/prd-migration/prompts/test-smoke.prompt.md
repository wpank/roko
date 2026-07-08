# Prompt: test-smoke (END-TO-END RUNNER TEST)

> This is a minimal end-to-end smoke-test topic. It exercises the full
> `run-migration.sh` pipeline (arg parsing, preflight, subshell, backgrounding,
> spawn_topic, verify_topic) at minimal cost. It is NOT part of the real
> migration — it's only loaded when `ROKO_MIGRATION_TEST_MODE=1`.

You are a fresh Claude Opus agent. Do NOT try to produce the full Roko PRD
documentation. This is a tiny scoped test.

## Your mission

Read a small set of context files, then write an INDEX.md plus 3 short sub-docs
to `/Users/will/dev/nunchi/roko/roko/docs/test-smoke/`. The output will be
verified by the same `verify_topic()` function used for real topics, but with
relaxed thresholds set by the test runner.

## Step 1 — Read these files (use the Read tool)

Read each file in full. Do not skip:

1. `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/context-pack/01-naming-map.md`
2. `/Users/will/dev/nunchi/roko/refactoring-prd/00-overview.md`
3. `/Users/will/dev/nunchi/roko/refactoring-prd/01-synapse-architecture.md`

## Step 2 — Create the output directory

Use the Bash tool to run:

```bash
mkdir -p /Users/will/dev/nunchi/roko/roko/docs/test-smoke
```

## Step 3 — Write exactly these 4 files

### File 1: `/Users/will/dev/nunchi/roko/roko/docs/test-smoke/INDEX.md`

Minimum 20 lines. Must contain:
- An H1 heading starting with `# `
- A reference to each of the 3 sub-doc filenames below (e.g., `00-engram.md`)
- The words: Roko, Engram, Synapse
- At least 2 citations (e.g., "Meta-Harness Lee et al. 2026", "Kanerva 2009")

### File 2: `/Users/will/dev/nunchi/roko/roko/docs/test-smoke/00-engram.md`

Minimum 40 lines. Content: the Engram data type. Must contain:
- An H1 heading
- A brief explanation of what an Engram is, written for a zero-context reader
- The 7 Score axes in order (confidence, novelty, utility, reputation, precision, salience, coherence)
- At least one verbatim quote or Rust snippet from `refactoring-prd/01-synapse-architecture.md`
- The word "Roko" at least once
- No forbidden terms (no "Thanatopsis", "Necrocracy", "GNOS token", "fleet" as agent group, "1 noun 6 verbs")

### File 3: `/Users/will/dev/nunchi/roko/roko/docs/test-smoke/01-synapse-traits.md`

Minimum 40 lines. Content: the 6 Synapse traits. Must contain:
- An H1 heading
- All 6 trait names (Substrate, Scorer, Gate, Router, Composer, Policy)
- A note that Gate returns `Verdict` directly (not wrapped in `Result`)
- The words "Roko" and "Synapse"
- At least one citation

### File 4: `/Users/will/dev/nunchi/roko/roko/docs/test-smoke/02-naming.md`

Minimum 40 lines. Content: the naming map. Must contain:
- An H1 heading
- Table or list showing at least 5 old→new renames (Bardo→Roko, Golem→Agent, etc.)
- Explicit statement that Clade → Collective / Mesh (NOT "fleet")
- Explicit statement that GNOS is replaced by KORAI (mainnet) / DAEJI (testnet)
- The words "Roko", "Engram", "Synapse"

## Step 4 — Self-check before finishing

Before you finish, make sure:
- [ ] All 4 files exist at the absolute paths above
- [ ] Each file meets its minimum line count
- [ ] INDEX.md references all 3 sub-docs by filename
- [ ] The words Roko, Engram, Synapse all appear somewhere in the topic
- [ ] No forbidden terms (Thanatopsis, Necrocracy, GNOS token, Thriving → Terminal, terminal requiem) appear anywhere
- [ ] At least 3 citation-like patterns total across all files (e.g., "Lee et al. 2026", "arXiv:2309.02427", "Kanerva 2009")

## Constraints

- Do not write any files outside `/Users/will/dev/nunchi/roko/roko/docs/test-smoke/`.
- Do not run any bash commands except `mkdir`.
- Do not ask clarifying questions — make decisions and proceed.
- This is a smoke test. Keep output short. 40-80 lines per sub-doc is plenty.
