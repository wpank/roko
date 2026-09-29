# Field evidence: data and anecdotes from real Roko runs

> Started 2026-09-29. Everything Roko does for real (the portal programme, dogfooding, demos) is observational
> evidence for both papers. This folder accumulates it in a form the papers can cite: frozen, provenance-stamped
> snapshots of each finished run, a structured log of what people had to do, and rollups over time.
> **It never replaces the pre-registered experiments (S09).** It feeds motivation, case studies, pilot priors and
> the discussion.

## What is here

| Path | What | Written by |
|---|---|---|
| `snapshots/<day>/<workspace>__<plan>__<run>/` | One finished plan run: `manifest.json` (provenance), `checkpoint.json` (per-task verdicts), `costs.json`, `derived.json` (whitelisted, scrubbed attempt, cost, gate and event rows), `excerpts.json` (≤280-character output tails), `summary.json` | `tools/field_capture.py` |
| `index.jsonl` | What has been captured (makes capture idempotent) | `field_capture.py` |
| `field-notes.jsonl` | Interventions, escaped defects, anecdotes, loop events, hand measurements, decisions (`roko.field_note/1`) | `tools/field_note.py` (people, the supervising session, backfill) |
| `ROLLUP.md`, `rollup.json` | Per-run, per-day and total metrics, including the autonomy index and escape rate | `tools/field_rollup.py` |
| `CASES.md` | Curated case studies, each promoted from notes plus snapshots and tied to a paper claim | by hand |
| `capture.log`, `.capture.pid` | The background watcher's log and process id | `field_capture.py --watch` |

## How it runs

- **Automatic capture.** A watcher polls every 5 minutes. It snapshots every plan run that reaches a terminal status
  (succeeded, failed or cancelled) in the repo and every git worktree, then refreshes the rollup.
  - Start: `nohup python3 tools/field_capture.py --watch 300 >> evidence/field/capture.log 2>&1 &`
  - Stop: `kill $(cat evidence/field/.capture.pid)`
  - One pass by hand: `python3 tools/field_capture.py`
- **Notes.** Whoever supervises runs logs each intervention and escaped defect as it happens:
  ```bash
  python3 tmp/cybernetic-harness/tools/field_note.py add --kind intervention \
    --summary "Raised turn caps 30→60 after three exit-1s with turns: 0" \
    --plan 03b-backend-workspace-server --task T06 --category turn-cap \
    --detected-by operator --fixed-by operator --minutes-to-detect 69 --automatable yes \
    --evidence commit:3049b7fcf --evidence work:gap-3870d9 --author roko-b6
  python3 tmp/cybernetic-harness/tools/field_note.py add --kind escape \
    --summary "Run button dead in the browser after every gate was green" --plan 08-portal-run \
    --detected-by browser-pass --category product-defect --severity high
  python3 tmp/cybernetic-harness/tools/field_note.py list | stats | validate
  ```
  - **Kinds:** `intervention`, `escape`, `anecdote`, `loop-event`, `measurement`, `decision`.
  - **Actors:** `roko`, `operator`, `user`, `audit`, `browser-pass`, `smoke-test`, `ci`, `review`.
  - **Categories:** the failure modes seen so far, e.g. `false-green`, `weak-gate`, `turn-cap`, `session-limit`,
    `sibling-contamination`, `merge-break`, `spec-defect`, `role-policy`, `product-defect`, `learning-loop`.
    See `field_note.py --help` for the full list.

## Rules

1. **Observational, and labelled so.** Nothing here is randomized. The papers quote it as a field study or as pilot
   data, never as a test of H1–H7.
   - It may set **priors and power** for S09 (pass rates, variances, costs per task).
   - It must not be used to choose analyses after seeing results.
2. **Derived data plus short, scrubbed excerpts only** (the author's decision, 2026-09-29).
   - There are no full transcripts: agents can currently read provider keys (bug-7d7200).
   - Every free-text field passes through a credential scrubber and a length cap.
3. **Provenance on every number.**
   - Each snapshot records the workspace, branch, HEAD, the dirty-tree hash, binary and `roko.toml` hashes, and the
     plan file hash.
   - Roko changes while it runs, so a run's harness version matters as much as its result.
   - Provenance is read at capture time; the watcher keeps that close to the run's end.
4. **Honest labels.**
   - A verified pass is a checkpoint verdict of `passed`; `unverified` never counts.
   - Backfilled runs keep only each plan's latest run, so their first-try rates are upper bounds.
   - Costs are Roko's own records, which lack a cost source, price killed attempts at $0 and use list-price estimates
     for Claude CLI subscriptions (B3). Quote them as "as recorded".
5. **The supervisor's own work counts.** The Claude session that supervises runs is part of the system being
   studied (the "operator loop").
   - Log its interventions as notes.
   - Record its cost with `--kind measurement` when you know it: it belongs in any honest "cheaper" comparison.

## What the papers get from this

| Evidence | Companion report (the audit) | Main paper (the harness) |
|---|---|---|
| False greens before and after the 09-28 verdict fix | RQ3, self-certification: 27% → 0% (B7), now tracked per run | §1 motivation |
| Interventions: who detected, who fixed, how long | RQ1, the operator was the real regulator | §8 discussion: the autonomy index over time, i.e. how much regulation moves from the operator into Roko as M1–M4 land |
| Escapes (defects after green) | "Gates checked parts, not the product" | Motivates M4 audits and whole-plan gates; the escape rate is an outcome measure beside the false-green rate |
| Per-run cost, attempts, first-try rate, concurrency | Descriptive statistics of Roko building its own portal | Pilot priors for S09 power (§4.5); a field-study table; examples in the system section |
| Loop events (a loop observed closing or failing) | RQ2, claimed vs wired vs effective, with dates | The M2 story: loops that earn their place |
| Anecdotes and case studies (`CASES.md`) | Quotable vignettes | Boxes and short examples throughout |

## Native capture (proposed, not built)

Roko should record most of this itself. The proposal is `../../specs/S01-addendum-field-evidence.md`:
- a run manifest at run start;
- structured `roko note` kinds for interventions and escapes;
- a plan-end export hook that writes this snapshot format;
- escapes feeding S05's audit labels.

Until then these tools fill the gap without touching the repo.
