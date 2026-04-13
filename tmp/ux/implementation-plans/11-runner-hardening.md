# 11 — Runner Hardening (T-batch wrapper)

> **Source plan**: `tmp/ux/ux-followup/04-t9-t19-residuals.md` items 27,
> 27a, 28; `tmp/ux/ux-followup/11-execution-plan.md` Phase B carry-forward
> (T23).
>
> **Status as of 2026-05-01**: The TUI-parity batch runner stopped
> silently after T14 in 2026-04-16 (`tmp/tui-parity/logs/run-20260416-101433/`).
> Item 28a is closed (`.github/workflows/tui-parity-dry-run.yml` runs
> `--dry-run` on PRs). Items 27 (root-cause documentation), 27a (log
> retention), and 28 (env-var knobs for max-batch / max-retry) are
> still open.
>
> **Effort**: 1-2 days.
>
> **Risk**: Low. Runner-wrapper changes only; no production code paths.

---

## What this plan accomplishes

Make the batch runner self-explanatory and safe against the silent-stop
pattern that bricked the 2026-04-16 run. After this plan:

- The 2026-04-16 stop has a documented post-mortem in
  `tmp/tui-parity/POSTMORTEM-20260416.md`.
- `TUI_PARITY_MAX_BATCHES` and `TUI_PARITY_MAX_RETRIES` env vars (with
  sensible defaults) gate runner extent. Logged at startup.
- On unexpected termination (no `.result` file, no follow-on event), the
  runner emits a tail trailer to `status.tsv` so the next reader sees
  "runner aborted at T<N>" instead of silence.
- `tmp/tui-parity/logs/` has a documented retention policy enforced by
  cleanup script (`keep last 5 runs`) AND is gitignored so accidental
  commits don't bloat the repo.

## Why this matters

Batch runners are the day-to-day execution surface for AI-assisted
work. A silent stop means a half-finished feature with no
explanation; the next maintainer wastes hours diagnosing instead of
working.

---

## Required reading

```
tmp/tui-parity/run-tui-parity.sh
tmp/tui-parity/lib/common.sh
tmp/tui-parity/BATCHES.md
tmp/tui-parity/prompts/T*.prompt.md             (template shape)
tmp/tui-parity/logs/run-20260416-101433/        (the failed run)
tmp/tui-parity/logs/run-20260416-101433/status.tsv
.github/workflows/tui-parity-dry-run.yml        (the closed item 28a)
.gitignore                                      (current rules)
tmp/ux/ux-followup/04-t9-t19-residuals.md       (items 27, 27a, 28)
tmp/ux/ux-followup/11-execution-plan.md         (T23 skeleton)
tmp/ux-followup-runner/                         (the sister runner; also affected)
```

If `tmp/ux-followup-runner/` exists with a similar shape, apply the same
fixes there (per `28a`'s mention).

---

## Deliverables

### Postmortem (item 27)

`tmp/tui-parity/POSTMORTEM-20260416.md`:

```markdown
# Runner Stop Post-Mortem — 2026-04-16

## Context
The `run-20260416-101433` TUI-parity run advanced T1-T13 successfully,
recorded `attempt_started` for T14, then never wrote a follow-on event
or `.result` file. T14, T17, T19 were re-queued under PR #13.

## Root cause
[FILL IN AFTER READING THE LOGS — likely a hardcoded batch cap, or a
bash subshell exit propagation issue, or a network timeout that caused
the wrapper to abort without trapping. Document what you find.]

## Mitigations landed under this plan
- `TUI_PARITY_MAX_BATCHES` and `TUI_PARITY_MAX_RETRIES` env vars
  (defaults 25 / 3 respectively).
- `status.tsv` always receives a "trailer" line on exit (success,
  failure, or signal).
- `--dry-run` exercised on every PR via the existing CI workflow.

## Open follow-ups
- (e.g. richer per-batch failure breadcrumbs)
```

The `[FILL IN ...]` block requires reading the logs and tracing the
shell. Don't skip that step — the post-mortem is the only artefact that
prevents recurrence.

### Env knobs + logging (item 28)

In `tmp/tui-parity/lib/common.sh`:

```bash
# Default values; override via env.
: "${TUI_PARITY_MAX_BATCHES:=25}"
: "${TUI_PARITY_MAX_RETRIES:=3}"

# Log at startup (idempotent — only once per run).
if [[ -z "${TUI_PARITY_BANNER_PRINTED:-}" ]]; then
    echo "[runner] TUI_PARITY_MAX_BATCHES=${TUI_PARITY_MAX_BATCHES}"
    echo "[runner] TUI_PARITY_MAX_RETRIES=${TUI_PARITY_MAX_RETRIES}"
    export TUI_PARITY_BANNER_PRINTED=1
fi
```

Each batch loop in `run-tui-parity.sh` checks `$TUI_PARITY_MAX_BATCHES`
before continuing and logs "reached cap" if exceeded:

```bash
if (( batches_run >= TUI_PARITY_MAX_BATCHES )); then
    echo "[runner] hit TUI_PARITY_MAX_BATCHES=${TUI_PARITY_MAX_BATCHES}; stopping"
    write_trailer "max-batches-reached"
    exit 0
fi
```

Each retry honors `$TUI_PARITY_MAX_RETRIES`. On exhaustion, write a
trailer.

### Trailer-on-exit (item 28 plus item 27 mitigation)

A bash `trap` on `EXIT` emits a final line to `status.tsv` so a silent
stop is impossible:

```bash
write_trailer() {
    local reason="$1"
    {
        printf 'trailer\t%s\t%s\n' "$(date +%s)" "$reason"
    } >> "$STATUS_TSV"
}

trap 'write_trailer "${LAST_REASON:-signal-or-eof}"' EXIT
```

Document the trailer rows in `BATCHES.md` so consumers know what
they mean.

### Log retention (item 27a)

1. Add a `.gitignore` entry:

   ```gitignore
   tmp/tui-parity/logs/
   tmp/ux-followup-runner/logs/
   ```

   (If they're already partially committed, `git rm --cached -r
   tmp/tui-parity/logs/`. Mention in the PR; reviewers expect the
   delete.)

2. Add `tmp/tui-parity/lib/cleanup-logs.sh`:

   ```bash
   #!/usr/bin/env bash
   # Keep the last 5 runs; delete the rest.
   set -euo pipefail
   logs_dir="$(dirname "$0")/../logs"
   if [[ ! -d "$logs_dir" ]]; then exit 0; fi
   cd "$logs_dir"
   ls -1dt run-* 2>/dev/null | tail -n +6 | xargs -I {} rm -rf {}
   ```

3. The runner calls it on startup:

   ```bash
   # In run-tui-parity.sh, near the top.
   bash "$(dirname "$0")/lib/cleanup-logs.sh" || true
   ```

4. Document the policy in `tmp/tui-parity/BATCHES.md`:
   "Retention: the last 5 runs. Older runs are pruned on startup."

### Apply to sister runner

Repeat the same changes in `tmp/ux-followup-runner/` if it exists. The
two runners share patterns; one should not drift from the other.

---

## Step-by-step

### Step 1 — Post-mortem investigation (3-4 hrs)

1. Open `tmp/tui-parity/logs/run-20260416-101433/status.tsv`. Count
   batches. Note the last successful and the first silent batch.
2. Read the batch's wrapper logs (look for `*.log` siblings in the run
   directory).
3. Trace the shell: `set -x` and re-run `run-tui-parity.sh --dry-run` on
   the same prompt set; observe where the wrapper would have exited.
4. Identify root cause. Common suspects:
   - `set -e` propagating from a subshell with no error message.
   - Hardcoded `for i in {1..14}` cap.
   - SSH timeout on a remote runner.
   - Disk full (logs partition) — check disk usage at the time.
5. Write the post-mortem section. Be concrete.

### Step 2 — Env knobs (1 hr)

Apply the diff in `lib/common.sh` per Deliverable above. Test with:

```bash
TUI_PARITY_MAX_BATCHES=2 bash run-tui-parity.sh --dry-run
# Expect: log "hit cap" after 2 batches.
```

### Step 3 — Trailer-on-exit (1 hr)

Apply the trap. Test by deliberately killing the runner:

```bash
bash run-tui-parity.sh --dry-run &
sleep 2; kill -TERM $!
tail -2 tmp/tui-parity/logs/run-*/status.tsv
# Expect a "trailer\t<ts>\tsignal-or-eof" line.
```

### Step 4 — Log retention (30 min)

Add the gitignore entry + cleanup script. Validate:

```bash
ls tmp/tui-parity/logs/
# Should always show ≤ 5 directories after a run.
```

### Step 5 — Sister runner (30 min)

Copy applicable changes to `tmp/ux-followup-runner/`. Verify by running
its dry-run.

### Step 6 — Followup closure (5 min)

Mark items 27, 27a, 28 DONE in
`tmp/ux/ux-followup/04-t9-t19-residuals.md`. Bump
`tmp/ux/ux-followup/00-INDEX.md` totals.

---

## Anti-patterns to avoid

- **Don't ship the env knobs without testing the cap behaviour.** The
  exact bug the runner had was a silent cap; reproducing it is the
  guard.
- **Don't write the post-mortem from speculation.** It costs more
  later to walk an unverified theory.
- **Don't gitignore the logs without `git rm --cached`** if any are
  already committed. The history will keep them; the working tree
  becomes consistent.
- **Don't make `TUI_PARITY_MAX_BATCHES` smaller than the typical
  number of batches you queue.** 25 is generous. If batches exceed
  25, the operator should be intentional about raising the cap.
- **Don't use `trap '...' ERR` instead of `trap '...' EXIT`.** ERR
  doesn't fire on signals or normal completion; EXIT covers both.
- **Don't write the trailer to a different file than `status.tsv`.**
  Existing consumers (the audit catalogue) read `status.tsv`. Adding
  a parallel file fragments the parsing.
- **Don't keep the cleanup script silent.** Print "[runner] pruned 3
  old runs" so operators see when their logs vanished.

## Done when

1. `tmp/tui-parity/POSTMORTEM-20260416.md` exists with a filled-in root
   cause section.
2. `TUI_PARITY_MAX_BATCHES=2 bash tmp/tui-parity/run-tui-parity.sh
   --dry-run` stops after 2 batches with a logged cap message.
3. Killing the runner with SIGTERM produces a `trailer` row in
   `status.tsv`.
4. `.gitignore` includes the runner log directories.
5. `tmp/tui-parity/logs/` contains at most 5 run subdirectories after
   the cleanup script has fired.
6. `tmp/ux-followup-runner/` (if present) has the same fixes.
7. `tmp/ux/ux-followup/04-t9-t19-residuals.md` items 27, 27a, 28
   marked DONE.
