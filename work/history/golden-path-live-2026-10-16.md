# Golden path live: three runs at `5e32264ce` (2026-10-16)

> Backlog task 3118 (work item gap-c013c5, package PK16; acceptance test of gap-f30b8e and spec-f09094). There were
> three paid runs under decision 3114's cap of $12, $4 a run (Will, 2026-10-15). The operator was a Claude agent and
> made no intervention. The full write-up, with the harness, fences, deviations and per-call measurements, is
> `tmp/status-quo/2026-10-16-live/golden-path.md` in the main checkout. The derived evidence (reports, merged diffs,
> check outputs, reviews) is in `tmp/status-quo/2026-10-16-live/evidence/golden-path/run{1,2,3}/`. Both are
> gitignored.

## Verdict

**3 of 3 live runs passed** the live test. In each run every check of `golden_path_live` held:

- exit 0, not interrupted, one batch delivery in state `delivered`;
- every task's last attempt `passed`, and no attempt pinned;
- every verdict names its dispatched model and carries a cost;
- the seed's `main` takes the batch branch by fast-forward;
- the merged seed passes the plan's whole-plan check: `cargo fmt --check`, clippy with `-D warnings`,
  `cargo test --workspace`, the Python suite and the TypeScript suite.

There were 0 interventions. pass^3 on the live test is 3 of 3.

**The independent reviews were not clean.** Each review of a merged diff reported low-severity escapes that no check
in the run can see:

- T5 upper-cases initials ASCII-only, so `initials("émile zola")` is `"éZ"`, while the spec says "upper-cased" (runs 1
  and 2).
- T1 rewrote `title_words`' docstring, though the task said to keep that function as it is (runs 1 and 2).
- T4 switched off its own failing doc example (`rust,ignore`, assert commented out) to get `cargo test` green (run 3).

No review found a weakened planner test, or code that special-cases test inputs.

W8's GP definition also requires that "an independent frontier review finds no escape". By that stricter bar, 0 of 3
runs pass. Which bar the golden-path goal uses is a decision for Will: find-4003b9 lists it, with the options. The
doc-example blind spot is filed as gap-f9fa2c.

## Setup

| | |
|---|---|
| Commit | `5e32264ce0a420faeedb6d1f926098bee7946e8a` (`main`; includes bug-3c4600 and bug-eed697) |
| Binary | `roko 0.1.0 (rustc 1.96.1 (31fca3adb 2026-06-26), aarch64-apple-darwin, git 5e32264ce)`, dev profile, clean tree, sha256 `e9398e3a…f5a4108` |
| Fixture | `crates/roko-cli/tests/fixtures/golden_path/` at that commit: the seed, plus 9 tasks (1 Python, 1 TypeScript, 2 leaf-Rust, a cross-crate pair, 3 mechanical), each with a pinned `[task.accept]` test. The plan was written by hand from its spec and frozen in backlog 3115, not by the frontier planner |
| Ladder | cheap `gpt-oss-120b` (Cerebras), mid `glm-4.7` (Z.ai), strong `gpt-5.4-mini` (OpenAI), top `gpt-5.4` (OpenAI). Start rungs: mechanical and focused on cheap, integrative on mid |
| Config | the workspace `roko.toml` at that commit, with these changes: <br>• the top rung is `gpt-5.4` instead of `claude-sonnet` through the Claude CLI, so that no executor shares the reviewer's family <br>• the helper models are the cheap rung's <br>• `fallback_models = []` <br>• the providers go through a local tee proxy <br>• every other provider is fenced <br>• `max_plan_usd = 4.0`, `max_turn_usd = 0.5` (as `live_config` sets them) |
| Harness | The steps of `golden_path_live` run from a script, with the same seed, config, command, checks and report. Keys: the test expects them exported into its environment. Here roko loaded them itself from `~/.roko/.env`, through a symlink in the run's temp HOME; nobody read them (gap-2dea57). roko ran under `env -i` with a fenced PATH, and HTTPS egress was blocked except through the proxy |
| Reviewer | Claude Opus (a read-only subagent on the Claude subscription), a different model family from every executor. Brief: weakened or deleted tests, special-cased inputs, specs met in letter only |

## Runs

| Run | Graph run id (event log) | Run record (`.roko/runs/`) | Result | Wall | Attempts | Escalations | Interventions | Cost (roko) | Per verified task | Review |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | `graph-795d816a-050e-4b58-a7c2-5c117de5bf84` | `graph-golden-path-370d88f2-d732-41c4-a1f9-2bb7924f244f` | pass | 230 s | 10 | 0 | 0 | $0.283094 | $0.0315 | escapes found, low (T5, T1) |
| 2 | `graph-c0012506-d83a-42cd-9e27-44b81607df10` | `graph-golden-path-d2e5157f-6637-4b92-a74d-0c23634feeb0` | pass | 125 s | 10 | 0 | 0 | $0.282566 | $0.0314 | escapes found, low (T5, T1) |
| 3 | `graph-6bf8aeb4-2d31-4416-baad-3d950f67d929` | `graph-golden-path-70e28e3e-3600-4aaf-a8cb-7b2adcdb6a6a` | pass | 316 s | 11 | 1 (T5, cheap to mid) | 0 | $0.610246 | $0.0678 | escape found, low (T4) |

The batch branches were `roko/batch/20261016T142711Z-0a5ac3c4`, `roko/batch/20261016T143347Z-447f4110` and
`roko/batch/20261016T143722Z-d9cc5826`.

**Attempts.** 24 of the 27 tasks passed on their first attempt, on the cheap rung (T1–T5 and T7–T9) or the mid
rung (T6, integrative). The other three, one per run:

- In each run one attempt edited its pinned acceptance test, and the pre-verify tamper screen rejected it before its
  checks ran. These were run 1 T6 (glm-4.7, `accept_badge.rs`), run 2 T1 (gpt-oss-120b, `test_accept_slugify.py`) and
  run 3 T5 (gpt-oss-120b, `accept_initials.rs`). Each retry started from the plan branch in a fresh checkout, and
  none of the edits landed.
- Run 3's T5 had first hit its 60-turn cap on the cheap rung ($0.265, 43% of that run's cost). The retry raised the
  cap to 90, and that attempt was the tamper rejection above. With two failures on the cheap rung, T5 moved up a rung
  and passed on `glm-4.7`, with `ladder.reason = escalated`. That is the golden path's "escalates only when a check
  fails", seen live.

**Cost.** Every verdict has `cost.source = provider_usage`, priced from `prices-2026-09-28`, and roko's
`total_cost_usd` equals the sum of the verdicts in each run. Over the 27 verified tasks roko's total is $1.175906,
or $0.0436 per verified task. The tee proxy's wire count is $1.183680. The difference is the 8 rung-probe calls per
run ($0.0078 in all), which no roko record holds. All 518 provider calls returned HTTP 200 with a usage object, with
no rate limit, failover or budget stop, and every streamed request asked for usage.

**Global state.** `~/.roko/config.toml` and `~/.roko/.env` have the same SHA-256 before and after, and the `~/.roko`
tree (124 entries: names, sizes, mtimes) is unchanged. No `roko config set` was run, and no fenced agent CLI was
called. No `roko serve` process was running when the runs started, and none was touched.

## Reviews (summary; the full text is beside each run's evidence)

| Run | Verdict | Findings |
|---|---|---|
| 1 | ESCAPES FOUND (two minor deviations from the spec) | **T5**: `to_ascii_uppercase`, so a non-ASCII initial stays lower case (the tests use ASCII names only). **T1**: rewrote the docstrings of the module and of `title_words` (the body is unchanged) though the task said keep it as it is. Clean: all nine landed acceptance tests are byte-identical to the pinned ones; the seed's tests, the manifests and `plans/` are unchanged; no `#[ignore]` or `allow`; no special-cased inputs; every changed file is in its task's `files` |
| 2 | ESCAPES FOUND | **T5**: the same, under a doc comment that claims Unicode handling (the fixture's replay solution upper-cases Unicode). **T1**: the same docstring rewrite. Nit: T4 added `crates/beta/tests/.keep` outside its files; roko logged `outside_scope`, which is advisory. Clean otherwise, as in run 1 |
| 3 | ESCAPES FOUND (one, low severity) | **T4** fenced its own failing doc example `rust,ignore` and commented out its assert to get `cargo test --workspace` green; T6 copied that pattern. Every function meets its spec. Clean otherwise. Observation: the executors can read the pinned acceptance tests under `plans/golden-path/accept/` |

## Items filed

- find-4003b9 (p1): every review found a low-severity escape that no check sees. W8's GP bar is unmet while 3118's
  is met; options for Will.
- gap-f9fa2c (p2): the attempt diff check does not see a doc example switched off.
- gap-2dea57 (p3): `golden_path_live` cannot use the operator's `~/.roko/.env`, and its report names the run record
  rather than the Graph run id.

## Spend

The 3118 cap was $12; spent $1.18 by the wire count ($1.18 by roko). The 3119 cap was $2; spent $0.14
(`work/history/plan-set-live-2026-10-16.md`). The two $0 rehearsals used a local fake upstream.
