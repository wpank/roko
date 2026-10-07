# R3: a capped live run on real cheap models

> Runtime note · 2026-10-02 10:31–10:48 CEST · run by the coordinating session · binary `roko-a43288b5f` (built at
> `a43288b5f`) · spend cap $5 (Will, 2026-10-02) · real spend about $1.2–1.4 (recorded $0.99, see "Cost").
> Evidence root `$L` = `<session-scratchpad>/runtime/live/`.
> Root causes of the defects found here: [R4](2026-10-02-live-defect-root-causes.md). Exercise level for everything below: LIVE.

## TL;DR

- **The golden path ran on real models and delivered twice.** Two 5-task plans, 10 tasks, all 10 verified, both plans
  merged into batch branches by the whole-plan check, and an independent re-run of every test on each delivered
  branch passed (0 false greens). Seven of the ten passes came from gpt-oss-120b on the cheap rung, two from
  gpt-5.4-mini, one from Sonnet.
- **It did not run unattended.** Six interventions (table at the end): four count against "unattended" (three rounds
  of moving `.roko/immune/` files aside, the last with a ladder change, then a second ladder change); one fixed the
  test harness itself; one was the designed fast-forward of `main` between plans. 48 attempt verdicts for 10 tasks.
- **Parallelism, the ladder, escalation, per-task commits, delivery and the run-to-run knowledge flow all worked
  live.** Four tasks ran at once (`max_turn_usd = 0.5` lets four $0.50 reservations fit in the $2.40 cap).
- **The failure paths are where it breaks.** One blank answer from GLM-4.7 got the task isolated for good, across
  providers and runs, and the denials opened the circuit breakers of two healthy providers. Provider errors never
  count towards a climb, so failover moved tasks *down* to the cheap rung instead of up. A substitute's failure was
  counted against the rung it replaced. OpenAI spend is invisible to the budget. Every cost amount in
  `attempts.jsonl` is null.
- **The learning data flows but carries little.** Knowledge written by run 1 reached every prompt of run 2, but the
  entries are generic ("Successful runtime episode for mechanical with gpt-oss-120b passed verify[0:test]"). Error
  patterns were learned from turn caps, not from code. No verify step ever failed, so the retry-diagnosis,
  reflection, judge and error-pattern-from-verify paths had no live exercise.

## Setup

- **Fixture** (`$L/repo`): a git repo with a small Python package, `textkit`, and planner-written unittest files that
  are red on the seed. Every test file was first checked against reference implementations in a throwaway copy
  (`$L/refcheck`, `$L/refcheck2`: all pass), so each task is satisfiable. Tasks name one file each, read the test in
  `read_files` (placed at task level by mistake: the parser dropped them silently, so no prompt carried them;
  see the correction in `tmp/status-quo/tldr-audit/00-README.md`), have one verify step that runs their test module, and say "do not edit the tests". No
  `[task.accept]` (it would have written to the real `~/.roko/accept/`; pinned tests are covered by R2 and canary C5).
- **Plans:** `live-a`: `slug` (mechanical), `duration`, `roman` (focused; strict canonical Roman numerals, bool
  rejected), `readme` (mechanical), `cli` (integrative, depends on the first three). `live-b`, which reuses the same
  words: `slug-unique`, `duration-human` (mechanical), `roman-range` (focused), `wrap` (focused, the hard one: match
  `textwrap.wrap`/`fill` exactly without importing `textwrap`), `cli-ext` (integrative). Both plans omit
  `max_parallel` and author `[meta] verify = python3 -m unittest discover -s tests`.
- **Config** (`$L/repo/roko.toml`): providers and models copied from the main `roko.toml`; the main ladder (cheap =
  gpt-oss-120b on Cerebras, mid = glm-4.7 on Z.ai, strong = gpt-5.4-mini on OpenAI, top = claude-sonnet-4-6 through the
  Claude CLI); `fallback_models = []`; `max_plan_usd = 2.4`, `max_turn_usd = 0.5`, `max_task_retry_usd = 0.8`; turn
  caps lowered for the audit to 20/25/30 (the main config has 40/60/90); `[learning]` as in the main config. An
  empty `.roko/`, so anything in run 2's prompts came from run 1.
- **Command** (`$L/run.sh`): `env -i PATH=... HOME=$HOME TERM=dumb RUST_LOG=info,roko_cli=debug roko-a43288b5f
  --verbose --json plan run plans/<plan> --workdir $L/repo --no-tui`. Real HOME, so roko loads its own keys; nothing
  here read them. Before/after snapshots of `.roko/`, refs and `git status` per run under `$L/evidence/<run>/`.
- **Pre-flight:** `plan validate` gave 0 diagnostics. `plan validate --spec-quality` graded all five live-a tasks C
  (48.7–50 of 100) although each has an exact spec and a planner-written test. `roko config providers test` gave
  200 OK for zai, cerebras and openai (zai and cerebras returned `content_empty` on the 10-token probe).

## The runs

| Run | What | Wall | Exit | Result |
|---|---|---|---|---|
| r1-live-a | plan A | 37 s | 1 | 4 tasks passed (gpt-oss), `cli` failed: GLM blank answer, immune isolation, 5 instant retries |
| r1b | `--resume-plan` | 3 s | 1 | `cli` denied 6 more times in 1.2 s with no model call: the isolation persists across runs |
| r1c | after moving `quarantine-vault.json` aside | 2 s | 1 | still denied: the block is the isolation control, not the vault |
| r1d | after moving `agent-controls.json` aside | 10 s | 1 | GLM returned a blank answer again (0 tokens, 7 s), re-isolated |
| r1e | ladder: integrative starts on `strong`; controls cleared | 10 s | 0 | `cli` passed first try on gpt-5.4-mini; whole-plan check passed; delivered |
| r2-live-b | plan B (mid rung removed from the ladder) | 140 s | 1 | 3 passed first try (gpt-oss); `wrap` failed after 6 attempts; `cli-ext` skipped |
| r2b | `--resume-plan` | 96 s | 1 | `wrap` climbed to `top`, but the Claude CLI said "Not logged in" (harness env, see below) |
| r2c | `--resume-plan`, harness passes `USER` | 112 s | 0 | `wrap` passed on Sonnet (17 turns, 97 s); `cli-ext` passed first try on gpt-5.4-mini; delivered |

Between the plans the scratch repo's `main` was fast-forwarded to plan A's batch branch, as the run summary suggests.

## What worked, with evidence

| Mechanism (matrix row) | Live evidence |
|---|---|
| Ladder routing (EX7, RC2 shadow) | Every attempt logs `model routed by the ladder role=... tier=... rung=... model=...`; mechanical and focused tasks went to gpt-oss-120b, integrative to the start rung. The router's pick (`gemini-2.5-flash`, a model this workspace doesn't configure; later `claude-haiku-4-5`) is a log field only |
| Escalation (EX7) | r2: `wrap` failed twice on `cheap` (turn caps 25 and 38), then `the task failed twice on its rung; its next attempt runs one rung up the ladder ... escalations=1`, and a3 ran on gpt-5.4-mini with `ladder.reason = escalated`, `step = 1`. The standing survived both resumes (`retry-feedback.json` `ladder.wrap = {escalations, failures_on_rung}`) |
| Turn-cap retry (EX6, EX9) | `previous attempt hit its turn cap; resuming with a raised cap previous_cap=25 max_turns=38` (and 38 → 57) |
| Parallel scheduling (EX3) | `width=5`, and attempt intervals overlap: 4 attempts in flight at once in both plans (computed from `attempts.jsonl`) |
| Per-task worktrees, plan branch, delivery (IS1–IS3) | `acquired isolated worktree`, `accepted the attempt onto its plan branch`, then `delivery regression passed` and `plan delivered into the run's batch branch` for both plans. Plan branch history has one commit plus one fold per task. The operator checkout was untouched except for an untracked `plans/INDEX.md` written by every run (R2 defect 7) |
| Whole-plan check (IS3) | The delivery regression runs the plan's `[meta] verify` steps (`graph_execution/delivery.rs:157-162`, `with_regression_steps`); the receipt records `state = delivered` and `verified_commit`, not the command or its output |
| Honest verdicts (QA1, QA2) | 10 of 10 `passed` verdicts were real: re-running the full suite on each delivered branch passed (`$L/audit-a`, `$L/audit-b`: 28 and 51 tests OK), and `wrap.py` does not mention `textwrap` |
| Served-model record (LM1) | `executed.model_reported` filled for Cerebras and OpenAI (`gpt-5.4-mini-2026-03-17`); null for the Claude CLI attempts; `failover_chain` names the replaced model on every substitution |
| Knowledge write-back and injection (LM3) | 8 entries written from verified passes (`verified attempt ingested into durable knowledge`). Each later attempt's episode lists 3 `knowledge_ids_injected`; every run-2 task got entries written in run 1 or r1e. Run 1 started with an empty store, so the flow is run-to-run |
| Gate history (QA7) | `learn/gate-thresholds.json` rung 2 moved to 8 of 8 passes |
| Router learning (RC2) | `cascade-router.json` reached 12 observations; the shadow pick moved from `gemini-2.5-flash` to `claude-haiku-4-5`. It never chose a model |
| Run metrics (RG1) | `learn/run-metrics.jsonl`, one row per run, cumulative cost per run id |

## What broke, with evidence

1. **One blank answer permanently isolates a task, and the denials break two healthy providers** (R4 §1–2, p1).
   GLM-4.7 returned `iterations=0 final_text_len=0 input_tokens=0 output_tokens=0` after 8.5 s (r1) and 7 s (r1d).
   The immune boundary scored `blank_primary_output_text` 0.9 (threshold 0.8), wrote a quarantine entry, and wrote an
   isolation control `agent_id = "live-a/cli"`, `state = isolated`, decay `none` (`$L/evidence/agent-controls.after-r1c.json`).
   Every later attempt of `cli`, on any model and in any later run, was denied about 1 ms after the agent was
   created, with no model call. 24 provider errors, retried 50–150 ms apart. Each denial was counted as a provider
   failure (`error_class=Unknown`), which opened the circuit for zai and then for cerebras. No CLI or API releases an
   isolation; only `GET /api/safety/quarantine` reads the vault. The operator has to find and delete
   `.roko/immune/agent-controls.json` by hand.
2. **The mid rung is unusable for agent work in this workspace** (R4 §1). GLM-4.7 blanked on both real attempts. The
   OpenAI-compatible stream parser keeps one field per chunk and makes up a `stop` finish reason, so the failure looks
   like an empty success. The ladder skips rungs that have no key, but nothing checks that a rung can actually do
   agent work.
3. **Failover goes down the ladder, and provider errors never climb it** (R4 §4c, p2). After gpt-5.4-mini's circuit
   opened, `wrap` was handed sideways to gpt-oss-120b on the cheap rung (`model substitution ... running
   cerebras-gptoss`) three times, and after Sonnet's circuit opened, once more. Provider errors do not count towards a
   climb, so a task on a broken rung can only fail or fall back to a cheaper model.
4. **A substitute's failure counts against the routed rung.** In r2b the `strong` rung's "second failure" was
   gpt-oss's turn cap after failover (`failover_chain = ['gpt-5.4-mini']`), and it triggered the climb to `top`.
5. **An immune denial of a finished answer, with no reason logged** (R4 §2, p1). r2 a3: gpt-5.4-mini finished
   normally (`iterations=23 final_text_len=459 finish_reason=stop`), and the result was "denied by immune boundary".
   No vault entry, control or log line names the reason. R4 infers the provider stream cap (4,096 events counted over
   the whole 23-turn tool loop).
6. **Rate limits are retried at once.** OpenAI returned `rate limited; retry after 1000 ms` and `2000 ms`; the next
   attempt started about 1 s later, and the third 429 opened the circuit.
7. **Auth failures are not classified.** The Claude CLI exited `Not logged in · Please run /login` because this
   harness's `env -i` dropped `USER` (the CLI needs it for the macOS keychain; with `USER` set it answered at once). roko
   classed it `Unknown`, retried it three times, opened the circuit and failed over to gpt-oss. The cause was the
   harness. The handling is roko's (R4 §4d): an auth failure should stop the task with a clear message.
8. **The budget can't see OpenAI spend** (R4 §4a, p1). OpenAI calls record 0 tokens and `cost_source = unknown`, so
   `costs.jsonl` and the plan budget count gpt-5.4-mini as $0. Requests never set `stream_options.include_usage`.
9. **`attempts.jsonl` carries no amounts** (R4 §4b). Every verdict has `usage.tokens_*` and every cost amount null,
   even where `costs.jsonl` has them (gpt-oss, Sonnet).
10. **The cheap model burned its turn budget.** gpt-oss-120b hit the cap 7 times (slug 20, duration 25, wrap 25, 38,
    25, 25, 25). On `slug` and `duration` the second try then passed in 5–8 turns. The caps were set lower than the
    main config's for this audit, so with 40/60 these two might have passed first time.
11. **Learned data of little use.** `error-patterns.json` holds two "patterns", both `agent turn cap reached (turns=N,
    cap=N)`, and every later prompt carries them. The knowledge entries are success notes with no lesson in them. No
    playbooks exist in a new workspace.

## Not exercised live

No attempt reached a failing verify step: every failure was a turn cap or a provider error. So none of these ran live:
the error-enrichment diagnosis in a retry prompt, post-gate reflections, the quality judge, error patterns from a
gate failure, hindsight. Also not exercised: tamper, pinned acceptance tests, pause, approval, plan sets. R2 covers
most of these with a scripted provider.

## Cost

| Source | Amount |
|---|---|
| `learn/costs.jsonl`, gpt-oss-120b (`provider_usage`) | $0.6047 |
| `learn/costs.jsonl`, claude-sonnet-4-6 (`cli_usage`, API-equivalent) | $0.3806 |
| `learn/costs.jsonl`, gpt-5.4-mini and glm-4.7 | recorded as $0.00, source `unknown` |
| Estimate for gpt-5.4-mini (about 6 attempts, a few hundred thousand tokens at $0.40/$1.60 per million) | about $0.2–0.4 |
| Provider probes and one Haiku login check | under $0.01 |
| **Total** | **about $1.2–1.4** of the $5 cap |

Per verified task, everything included: about $0.12–0.14. That includes about $0.38 for one hard task that ended on
Sonnet and the churn around it. This is not a benchmark: 10 small Python tasks, one seed, no comparison arm.

## Operator interventions

| # | Run | Intervention | Why |
|---|---|---|---|
| 1 | r1c | moved `.roko/immune/quarantine-vault.json` aside | the quarantined `cli` task could never run again |
| 2 | r1d | moved `.roko/immune/agent-controls.json` aside | the isolation control, not the vault, was the block |
| 3 | r1e | moved both aside again; set `start.integrative = "strong"` | GLM-4.7 blanked twice; the task was isolated again |
| 4 | r2 | removed the `mid` rung | same |
| 5 | r2c | the harness passed `USER` and `LOGNAME` | the Claude CLI needs them to find its login (a harness artefact) |
| 6 | between plans | `git merge --ff-only` of plan A's batch branch into `main` | delivery leaves merging to the operator, by design |

Interventions 1–4 count against "runs unattended". Intervention 5 does not count against roko. Intervention 6 is the
designed hand-off.
