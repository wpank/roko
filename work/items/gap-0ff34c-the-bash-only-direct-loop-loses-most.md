+++
id = "gap-0ff34c"
kind = "gap"
title = "The bash-only direct loop loses most of gpt-oss-120b's F1 edits (apply_patch calls, broken one-block replies), holding G0's cheap level 1 at 0.50 after the re-tune"
status = "open"
triage = "verified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
source = "gap-dd670b worker (claude-agent), 2026-10-19: PILOT-A-RECAL, benchmarks/viabilitybench/reports/pilot_a_recal/go-no-go.md and its transcripts"
discovered_from = "gap-dd670b"
anchors = ["benchmarks/viabilitybench/driver/mini_loop.py::SYSTEM_PROMPT", "benchmarks/viabilitybench/arms/cheap_direct.toml", "benchmarks/viabilitybench/arms/cheap_direct_msa.toml"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-dd670b", "gap-327242", "gap-129862"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "test -f benchmarks/viabilitybench/reports/direct_loop/metrics.json && benchmarks/viabilitybench/.venv/bin/python benchmarks/viabilitybench/analysis/report.py --check benchmarks/viabilitybench/reports/direct_loop"
+++

## Problem

After gap-dd670b's ℓ1–ℓ2 re-tune (`f1-1.2.0`, `f4-1.2.0`), the re-run PILOT-A-RECAL (2026-10-19,
`benchmarks/viabilitybench/reports/pilot_a_recal/`) has `cheap_direct` gpt-oss-120b solving F4 at ℓ1–ℓ2 in 11 of 12
runs, but F1 in 1 of 12. G0's level-1 check pools both: (1 + 5) / 12 = 0.50, under 0.6.

F1's runs fail before the convention matters. Of its 12 runs at ℓ1–ℓ2:

- every one had at least one reply with no bash block or several (the loop's format error, which costs a turn);
- 8 called `apply_patch`, gpt-oss's own patch tool, which the loop's shell does not have (`apply_patch: command not
  found`);
- 4 left no diff, and 5 never ran the visible check;
- 10 ended with the visible check failing: 9 submitted, after 3 to 15 turns, and 1 hit the 30-call cap.

The transcripts show replies that hold a whole imagined session, up to 89 bash blocks with invented outputs, and
patches written against file contents the model never read. Where a reply shows the edit it meant, the edit follows
the convention that AC3 now spells out: an `AppError` subclass with an `E`-code in `ERROR_CODES` and a docs row. The
edit just never reaches the files.

Pilot A had the same pattern: 52 of its 60 cheap runs had format errors, 23 called `apply_patch` and 12 left no
diff. F4's one-script task gets through the same loop: in the re-run, 11 of its 12 ℓ1–ℓ2 runs pass.

## Why it matters

- G0 gates LOG1's $152 (S09 §4.7), and its cheap ℓ1 check cannot pass while F1's edits do not land. gap-dd670b stays
  open on that check.
- S08 decision 3, the direct-loop choice, is one of G0's own checks. This is the evidence it needs: the bash-only
  loop may understate what a cheap model does alone, and that is H1's baseline.
- Related: gap-327242 (Pilot B's optional `mini-swe` block) and gap-129862 (the SC2 file's `[direct_loop]` choice).

## Where

- `benchmarks/viabilitybench/driver/mini_loop.py`: `SYSTEM_PROMPT`, `FORMAT_ERROR` and `parse_command`, the
  one-bash-block protocol. `PROMPT_VERSION` is `mini-loop-2`, and every record pins `PROMPT_SHA256`.
- `benchmarks/viabilitybench/arms/cheap_direct.toml`: it sets no reasoning effort, so gpt-oss-120b runs at Cerebras'
  default.
- `benchmarks/viabilitybench/arms/cheap_direct_msa.toml` and `driver/run_msa.py`: the pinned mini-swe-agent loop.
  Its records keep arm id `cheap_direct`, so G0's ladder check counts them.
- Evidence: `reports/pilot_a_recal/` (records) and `reports/pilot_a/`. The transcripts are in
  `~/.roko-bench/viability/PILOT-A-RECAL/cheap-direct-1/transcripts/` and `.../PILOT-A/*/transcripts/`, which are
  not committed.

## Current state

- The loop's prompt asks for exactly one ```` ```bash ```` block, and suggests `cat <<'EOF'`, `sed -i` or a python3
  script for edits. It never mentions `apply_patch`. A reply with 0 or 2+ blocks gets `FORMAT_ERROR` and costs a
  turn.
- mini-swe-agent's venv (`~/.local/share/mini-swe-agent/venv`) does not exist yet (gap-327242), so
  `cheap_direct_msa` has never run.

## Plan

1. **Run the decision-3 comparison** on the pilot slice's ℓ1–ℓ2: 8 instances × 3 seeds, about $0.40 an arm at
   PILOT-A-RECAL's $0.0176 a task. Use `cheap_direct_msa` once its venv exists, and optionally `cheap_direct` with
   reasoning effort `high`, as an arm variant. That variant needs the arm format and `provider.OpenAICompatible` to
   pass `reasoning_effort`. Bundle the runs as one experiment in `reports/direct_loop/`.
2. **Compare** F1's VS, and the protocol counts (format errors, `apply_patch` calls, empty diffs, submissions with a
   failing visible check), with PILOT-A-RECAL's.
3. **Decide S08 decision 3's direct loop**, and record it in the SC2 file's `[direct_loop]` (gap-129862). If the
   bash-only loop stays, a prompt that says `apply_patch` does not exist would be a new prompt version. It changes the
   instrument, so it needs Will's decision.
4. **Re-run the chosen loop on the pilot stream** and repoint gap-dd670b's verify at that bundle.

## Done when

- [ ] The comparison is bundled in `reports/direct_loop/` and passes `report.py --check` (the `[[verify]]`).
- [ ] S08 decision 3's choice is recorded, with F1's numbers from both loops.

## Notes

- **It spends money** (about $1), so it needs Will's go-ahead. Run one runner at a time, and never through roko.
- **Do not edit `mini_loop.py`'s prompt without a decision.** Every `cheap_direct` record pins `PROMPT_SHA256`.
