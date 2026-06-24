# Mega-Parity Runner Monitor

You are monitoring and facilitating a parallel batch runner executing 168 code implementation batches. Your job is to:

1. **Start the runner** in the background
2. **Start the auto-pick monitor** that progressively merges completed batches into the working branch
3. **Check health every 90 seconds** — report detailed status
4. **Resolve any issues** — conflicts, compile failures, stuck batches
5. **Estimate total time** and report progress

## Setup Commands

Run these in order:

```bash
# 1. Start the runner (background) — this takes hours, runs 168 batches at 40 parallel
cd /Users/will/dev/nunchi/roko/roko
nohup bash tmp/runners/mega-parity/run.sh --no-audit --pre-warm > tmp/runners/mega-parity/logs/runner-stdout.log 2>&1 &
echo "Runner PID: $!"

# 2. Wait ~30s for first status to appear, then start auto-pick monitor
sleep 30
bash tmp/runners/parallel-template/lib/auto-pick.sh --interval 90
```

**Important:** The runner uses `--no-audit` for Pass 1 (speed). After Pass 1 completes, you'll run Pass 2 with audit on the successful batches.

## Monitoring Loop

Every 90 seconds, do ALL of these:

### 1. Read the status JSON
```bash
cat tmp/runners/mega-parity/logs/$(ls -1t tmp/runners/mega-parity/logs/ | grep '^run-' | head -1)/status.json
```

### 2. Check auto-pick state
```bash
cat tmp/runners/mega-parity/logs/$(ls -1t tmp/runners/mega-parity/logs/ | grep '^run-' | head -1)/auto-pick-state.env 2>/dev/null | wc -l
```

### 3. Check for compile issues on the working branch
```bash
cd /Users/will/dev/nunchi/roko/roko && cargo check --workspace 2>&1 | tail -5
```

### 4. Report to the user with this format:

```
## Status Update [HH:MM:SS]

**Runner:** XX/168 done (YY active, ZZ pending) | Elapsed: Xh Ym | ETA: Xh Ym
**Picked:** XX batches merged into wp-arch2 | XX conflicts auto-resolved | XX skipped
**Health:** ✓ Compiles clean (or: ✗ N errors — investigating)

### Changes This Cycle
- ✓ R2_A01: Title here
- ✓ R2_B01: Title here

### Failures (if any)
- ✗ R3_C02: compile_failed — [first error line]

### Estimate
- Wave N of 27 | ~XX batches/hour | Projected finish: HH:MM
```

## Conflict Resolution

When auto-pick reports a conflict it couldn't resolve:

1. Read the conflicting files: `git diff --name-only --diff-filter=U`
2. For each file, read the content and resolve manually — prefer the batch's (incoming) changes
3. Stage resolved files: `git add <file>`
4. Complete: `git cherry-pick --continue`
5. Verify: `cargo check --workspace`

## Pass 2 (Audit)

When Pass 1 completes (all 168 batches attempted), run the audit pass on successful batches:

```bash
# Get list of successful batches
SUCCESSES=$(cat tmp/runners/mega-parity/logs/$(ls -1t tmp/runners/mega-parity/logs/ | grep '^run-' | head -1)/*.result | grep -l 'success\|verified' | xargs -I{} basename {} .result | tr '\n' ',')

# Run audit-only pass
AUDIT_ENABLED=1 bash tmp/runners/mega-parity/run.sh --only "$SUCCESSES" --continue
```

## Emergency Commands

```bash
# Stop the runner
kill $(cat tmp/runners/mega-parity/logs/$(ls -1t tmp/runners/mega-parity/logs/ | grep '^run-' | head -1)/runner.pid 2>/dev/null)

# Abort auto-pick (Ctrl-C in that terminal, or):
# The auto-pick script handles SIGINT cleanly

# Undo last pick (if it broke compilation)
git reset --hard HEAD~1

# See all runner branches
bash tmp/runners/mega-parity/run.sh --branches

# Manual pick specific batches
bash tmp/runners/mega-parity/run.sh --pick R2_A01,R3_B01
```

## Key Information

- **Working branch:** wp-arch2
- **Runner branch:** codex/mega-parity-<run-id>-main
- **168 batches** across Runners 2-7 + D8
- **27 waves** in the DAG (wave 1 = 33 batches, all independent)
- **Parallel:** 40 concurrent codex sessions
- **Model:** gpt-5.4-mini (xhigh reasoning) for implementation
- **Logs:** tmp/runners/mega-parity/logs/<run-id>/
- **Per-batch logs:** <run-id>/<BATCH>.log

## What NOT To Do

- Do NOT run `git push` unless explicitly asked
- Do NOT modify files in `crates/` directly — only the runner does that via codex
- Do NOT kill batch worktrees — they contain work that may need inspection
- Do NOT delete runner branches — they're the safety net
- Do NOT run `git reset --hard` on the working branch without asking first
