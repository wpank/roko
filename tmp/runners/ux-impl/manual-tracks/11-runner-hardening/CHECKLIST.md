# RH — Runner hardening (manual)

> Source plan: `tmp/ux/implementation-plans/11-runner-hardening.md`.
> Tracker rows: ISSUE-TRACKER.md Wave RH.

Bash-only fixes for the legacy TUI-parity runner under `tmp/tui-parity/`.
Outside the Rust runner's scope.

---

## RH01 — Postmortem of the 2026-04-16 silent stop

```bash
cd /Users/will/dev/nunchi/roko/roko
ls tmp/tui-parity/logs/run-20260416-101433/
cat tmp/tui-parity/logs/run-20260416-101433/status.tsv
```

Investigate:

- [ ] Read `status.tsv` end to end. Note the last successful batch and
  the first silent batch.
- [ ] Read every `*.log` file in the run directory.
- [ ] `set -x` and re-run the wrapper in dry-run mode to trace the shell
  flow that ended at T14.
- [ ] Identify the root cause. Common suspects:
  - `set -e` propagating from a subshell with no error message.
  - Hardcoded `for i in {1..14}` cap.
  - SSH / network timeout in the wrapper.
  - Disk full at the time (check `df` history if logged).

Write `tmp/tui-parity/POSTMORTEM-20260416.md`:

```markdown
# Runner Stop Post-Mortem — 2026-04-16

## Context
The `run-20260416-101433` TUI-parity run advanced T1-T13 successfully,
recorded `attempt_started` for T14, then never wrote a follow-on event
or `.result` file.

## Root cause
<filled-in>

## Mitigations landed under Wave RH
- TUI_PARITY_MAX_BATCHES + TUI_PARITY_MAX_RETRIES env knobs (RH02).
- status.tsv trailer-on-exit trap (RH03).
- Log retention via gitignore + cleanup script (RH04).

## Open follow-ups
- (e.g. richer per-batch failure breadcrumbs)
```

- [ ] Tick `[ ] RH01`.

---

## RH02 — Env knobs for max-batch / max-retry

Edit `tmp/tui-parity/lib/common.sh`. Append near the top:

```bash
: "${TUI_PARITY_MAX_BATCHES:=25}"
: "${TUI_PARITY_MAX_RETRIES:=3}"

if [[ -z "${TUI_PARITY_BANNER_PRINTED:-}" ]]; then
    echo "[runner] TUI_PARITY_MAX_BATCHES=${TUI_PARITY_MAX_BATCHES}"
    echo "[runner] TUI_PARITY_MAX_RETRIES=${TUI_PARITY_MAX_RETRIES}"
    export TUI_PARITY_BANNER_PRINTED=1
fi
```

Edit `tmp/tui-parity/run-tui-parity.sh` so the batch loop honours the
cap:

```bash
if (( batches_run >= TUI_PARITY_MAX_BATCHES )); then
    echo "[runner] hit TUI_PARITY_MAX_BATCHES=${TUI_PARITY_MAX_BATCHES}; stopping"
    write_trailer "max-batches-reached"
    exit 0
fi
```

Test:

```bash
TUI_PARITY_MAX_BATCHES=2 bash tmp/tui-parity/run-tui-parity.sh --dry-run
# Expect: "hit cap" log after 2 batches.
```

- [ ] Tick `[ ] RH02`.

---

## RH03 — Trailer-on-exit trap

Add to `tmp/tui-parity/lib/common.sh`:

```bash
write_trailer() {
    local reason="$1"
    {
        printf 'trailer\t%s\t%s\n' "$(date +%s)" "$reason"
    } >> "$STATUS_TSV"
}

trap 'write_trailer "${LAST_REASON:-signal-or-eof}"' EXIT
```

Document trailer rows in `tmp/tui-parity/BATCHES.md`:

```markdown
## status.tsv schema
- `attempt_started   <ts>   <batch>   <attempt>` — batch attempt began.
- `attempt_done      <ts>   <batch>   <attempt>` — batch attempt finished.
- `trailer           <ts>   <reason>` — runner exited (signal, max-batches, error).
```

Test:

```bash
bash tmp/tui-parity/run-tui-parity.sh --dry-run &
sleep 2; kill -TERM $!
tail -2 tmp/tui-parity/logs/run-*/status.tsv
# Expect a "trailer ..." row.
```

- [ ] Tick `[ ] RH03`.

---

## RH04 — Log retention + gitignore + cleanup script

Add to `.gitignore`:

```
tmp/tui-parity/logs/
tmp/ux-followup-runner/logs/
```

If any logs are already tracked: `git rm -r --cached tmp/tui-parity/logs/`.

Create `tmp/tui-parity/lib/cleanup-logs.sh`:

```bash
#!/usr/bin/env bash
set -euo pipefail
logs_dir="$(dirname "$0")/../logs"
[[ -d "$logs_dir" ]] || exit 0
cd "$logs_dir"
victim_count=$(ls -1dt run-* 2>/dev/null | tail -n +6 | wc -l | xargs)
if (( victim_count > 0 )); then
  ls -1dt run-* 2>/dev/null | tail -n +6 | xargs -I {} rm -rf {}
  echo "[runner] pruned ${victim_count} old run(s)"
fi
```

`chmod +x tmp/tui-parity/lib/cleanup-logs.sh`.

Edit `tmp/tui-parity/run-tui-parity.sh` to call it on startup:

```bash
bash "$(dirname "$0")/lib/cleanup-logs.sh" || true
```

Document in `BATCHES.md`:

> Log retention: the last 5 runs. Older runs are pruned on startup.

- [ ] Tick `[ ] RH04`.

---

## RH05 — Apply same fixes to sister runner if present

Check:

```bash
ls tmp/ux-followup-runner/ 2>/dev/null
```

If present, repeat RH02 / RH03 / RH04 there.

- [ ] Tick `[ ] RH05`.

---

## Close followup items

- [ ] Mark items 27, 27a, 28 in
  `tmp/ux/ux-followup/04-t9-t19-residuals.md` as DONE.
- [ ] Bump `tmp/ux/ux-followup/00-INDEX.md` totals.

---

## Done when

- [ ] `tmp/tui-parity/POSTMORTEM-20260416.md` exists with a non-empty
  Root cause section.
- [ ] `TUI_PARITY_MAX_BATCHES=2 bash tmp/tui-parity/run-tui-parity.sh --dry-run`
  stops after 2 batches with a logged cap message.
- [ ] Killing the runner with SIGTERM produces a `trailer` row.
- [ ] `.gitignore` excludes runner log dirs.
- [ ] `tmp/tui-parity/logs/` has at most 5 run subdirectories after
  cleanup runs.
- [ ] All ISSUE-TRACKER.md Wave RH rows ticked.
