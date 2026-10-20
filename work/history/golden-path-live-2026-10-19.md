# Golden path live: three runs after the hand-back, on the fixture tightened again, at `4535b3914` (2026-10-19)

> gap-f30b8e's pass^3 rerun after find-5bcba7 (Will, 2026-10-19: a failed whole-plan check goes back to the
> tasks whose files it names) and find-8b6828 (the ß case), under a cap of $2 for everything, with
> `max_turn_usd = max_plan_usd / 8` while bug-2110e8 is open. The operator was a Claude agent (W-handback) and
> made no intervention. The full write-up, with the harness, fences, rehearsals and per-run measurements, is the
> section "Rerun after the hand-back (2026-10-19)" of `tmp/status-quo/2026-10-16-live/golden-path.md` in the
> main checkout. The derived evidence (reports, run records, diffs, check outputs, reviews, workspace records,
> tools) is in `tmp/status-quo/2026-10-16-live/evidence/gp3/`. Both are gitignored.

## Verdict

**3 of 3 live runs passed the live test; by W8's stricter bar 2 of 3, so pass^3 does not hold.**

- Every run passed all twelve checks of `golden_path_live`, with 0 interventions: exit 0, one delivery,
  `delivered`, every task's last attempt passed, none pinned, the fast-forward, and the merged seed's five
  whole-plan steps.
- **Runs 2 and 3: every review CLEAN.**
- **Run 1 does not meet the bar.** Its delivered branch, which `main` fast-forwarded to, holds 233 cargo build
  files (7.4 MB) under `crates/alpha/target/`. T7's commit `50e22de` added them, outside its `files`:
  `run_tests` had been refused, so T7's agent ran `cd crates/alpha && cargo test` under the run's relative
  `CARGO_TARGET_DIR=target`, and roko's acceptance commits every file the repository does not ignore. Both
  reviews refuse the merge for it. The second also counts T4's accepted commit as an attempt-level escape:
  it was not fmt-clean and its doc example could not compile, and T6 repaired both before delivery.

gap-f30b8e stays open.

**The hand-back (find-5bcba7) was not needed live:** no whole-plan check failed, since T7 put `VERSION` above
the tests every time. It was proven at $0 in rehearsal r2 with the real binary and clippy:

- T7's misplaced `VERSION` failed the check (`items_after_test_module`, `crates/alpha/src/lib.rs`).
- roko handed the failure to T7, the file's last changer. T7's second attempt started from the plan tip, with
  the check's output in its prompt, and fixed it.
- The re-check delivery (`…:golden-path:recheck-1`) delivered the plan. The summary shows one delivery, with
  its `handbacks`.

**The tightened checks worked live:**

- In runs 2 and 3, T5's first attempt upper-cased ASCII-only and failed both non-ASCII cases, the new ß case
  included ("ßO" for "SSO"). The retry passed on the same rung.
- In runs 2 and 3, T1's first attempt rewrote `title_words`' docstring and was rejected (2026-10-18's check).
- In all three runs T5's landed code keeps every letter of an upper case, so find-8b6828's escape is gone.

## Setup

| | |
|---|---|
| Commit | `4535b3914f3798096c37bdc0d261607803ee99c5` (`work/find-8b6828` = `main` `050f30ea8` + find-5bcba7 `11bbb817f`, `49ba19daf` + find-8b6828 `74a24a836`, `4535b3914`) |
| Binary | `roko 0.1.0 (rustc 1.96.1 (31fca3adb 2026-06-26), aarch64-apple-darwin, git 4535b3914)`, dev profile, clean tree, sha256 `e802f1e5…6983f090214` |
| Fixture | The committed fixture at `4535b3914`, copied into the scratch root and checked blob for blob (35 files) |
| Ladder | cheap `gpt-oss-120b` (Cerebras), mid `glm-4.7` (Z.ai), strong `gpt-5.4-mini`, top `gpt-5.4` (OpenAI), as on 2026-10-16 and 2026-10-18 |
| Config | The workspace `roko.toml` at `4535b3914` with the 2026-10-18 changes (top rung `gpt-5.4`, helper models on the cheap rung, `fallback_models = []`, providers through a tee proxy, every other provider fenced), plus `[agent.data_llm] model` (new on `main`) on the cheap rung. Budget from the $2 cap: `max_plan_usd` $0.62, $0.82, $1.25 and `max_turn_usd` $0.077, $0.102, $0.156 |
| Harness | The steps of `golden_path_live` from a script (gap-2dea57 is still open): scratch HOMEs whose `.roko/.env` is a symlink to `~/.roko/.env` (roko loaded the keys; nobody read them), `env -i`, a fenced PATH (13 failing shims), HTTPS only through the proxy |
| Reviewer | Claude Opus, read-only subagents, with the 2026-10-18 brief plus what changed since (the ß case, the hand-back) |

## Runs

| Run | Graph run id (event log) | Run record (`.roko/runs/`) | Result | Wall | Attempts | Escalations | Hand-backs | Interventions | Cost (roko) | Per verified task | Reviews |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | `graph-d282b763-e911-4ffb-a22f-fc34ff59abe2` | `graph-golden-path-31d6a6fb-bb01-4462-9278-0b83564b90eb` | pass (12/12); delivered branch holds T7's build output | 270 s | 9 | 0 | 0 | 0 | $0.212433 | $0.0236 | ESCAPES FOUND; CLEAN on items 1–3 but refuses the merge |
| 2 | `graph-4146c90c-baf6-4e70-9d37-63b831f8b922` | `graph-golden-path-7f133a7f-7caa-427e-9853-df4e4e3f9e7d` | pass (12/12) | 246 s | 12 | 0 | 0 | 0 | $0.266352 | $0.0296 | CLEAN |
| 3 | `graph-da87db5b-72da-4eb7-907c-6f414694db0d` | `graph-golden-path-27823a8e-f2a7-45e9-9f5f-bbd143cf423c` | pass (12/12) | 271 s | 14 | 0 | 0 | 0 | $0.258572 | $0.0287 | CLEAN; CLEAN |

The batch branches were `roko/batch/20261019T125209Z-ad584f07`, `roko/batch/20261019T125841Z-23eab620` and
`roko/batch/20261019T130423Z-cc705ea0`.

**Attempts.** 27 tasks, 35 attempts:

- Every task started on its tier's start rung: cheap, except T6 (integrative), which ran on mid.
- Run 2: T1 (docstring), T8 (no `word_list`) and T5 ("éZ", "ßO") were each retried once on cheap.
- Run 3: T1 failed on its docstring and then on a rate limit, and passed on its third attempt.
- Run 3: T9 hit two Cerebras 429s. roko's circuit opened and it substituted `glm-4.7` (`ladder.reason =
  failover`), which passed. That is a provider failover, not an escalation: no check failed.
- Run 3: T5 ("éZ", "ßO") was retried once on cheap.

No turn cap, tamper rejection or doc-example rejection happened.

**Cost.** Every verdict has `cost.source = provider_usage`. roko's `total_cost_usd` is $0.737357 over the three
runs, and the verdicts sum to $0.712439. The difference is the helper calls (diagnoses after failed checks, an
audit's hidden-suite draft), which roko counts in the total and in no verdict. The tee proxy's wire count is
$0.746780: 427 calls, all 200 but run 3's three 429s. Beyond roko's total it holds the 8 rung probes a run,
which no roko record holds (H's N3, still true). Every streamed request asked for usage.

**Global state.** `~/.roko/config.toml` and `~/.roko/.env` have the same SHA-256 before (11:02:52Z) and after
(13:09:06Z), the same as on 2026-10-16 and 2026-10-18. The `~/.roko` tree (123 entries) is unchanged.

- No `roko config set` or `git config --global` was run.
- No fenced CLI was called (the tripwire file was never created).
- Another session's `roko serve` listened on :6677 throughout. This work never started, called or touched it.

## Rehearsals ($0, a local fake upstream behind the same proxy, fake key values)

1. **r1**, the fixture's replay solution through `read_file` and `write_file`, at the run-1 budget: 9 attempts,
   all first try; delivered; 12 of 12 checks.
2. **r2**, the same, but T7's first solution puts `VERSION` after the test module until a prompt carries the
   hand-back. The hand-back went as described above, and all 12 checks held.
3. **r3**, an upstream that answers and changes nothing: 12 attempts. The ladder climbed cheap, cheap, mid on
   T1–T4, and every verdict had a model and a cost. It failed, as it must.

## Reviews (summary; the full text is beside each run's evidence)

Each reviewer read the run's merged diff and tree, the plan, the pinned tests and the seed, and ran nothing.

- The first reviewers of runs 1 and 3 sent their reports to the coordinator instead of the operator, and did
  not answer when asked. So a second reviewer, with the same brief, reviewed each of those runs.
- Runs 1 and 3 therefore have two reviews each. Run 1's first review is recorded as the coordinator relayed it.

| Run | Verdicts | Escapes and blocking findings | Other findings |
|---|---|---|---|
| 1 | ESCAPES FOUND; CLEAN on items 1–3 but "blocks the merge" | T7's 233 build files under `crates/alpha/target/` on the delivered main (both). T4's accepted commit `215eb99` not fmt-clean (`crates/beta/src/lib.rs:32`) and its doc example calling `farewell` unqualified (E0425); T6 (`9cd9467`) repaired both | T4's gate ran only its accept test (V0). No `run_tests` call worked. The audit's `false_green` incident against T5 likely belongs to T4. Nits: `import re` inside `slugify`, test-referencing docstrings, U+2011 hyphens |
| 2 | CLEAN | none | T4's doc example broken in its accepted attempt `4c2dab7`, though its own `run_tests` reported "1 tests failed"; T6 fixed it. T5's retry diagnosis blamed `crates/alpha/src/initials.rs` line 42, which does not exist. Nits: T1's module docstring, U+2011 hyphens |
| 3 | CLEAN; CLEAN | none | camelCase splits on `\s`, as the replay does (not counted). Nits: T1's module docstring as a change log, a wrong regex comment, an unresolved intra-doc link, U+2011 hyphens |

In all three runs:

- Every landed accept test is byte-identical to the pinned one.
- The seed's tests, the manifests, `.gitignore`, `roko.toml` and `plans/` are unchanged.
- There is no `#[ignore]`, `cfg` trick, `allow(clippy…)`, switched-off doc example or special-cased input.

## Found on the way (filed)

- **roko commits and delivers an attempt's undeclared build output** (bug-ba0c40, p1). Run 1's T7, above. Acceptance
  stages everything the repository does not ignore, and the scope check only logs paths outside `files`.
  `golden_path_live` sets the same relative `CARGO_TARGET_DIR`. The worker's harness no longer does.
- **`run_tests` returned nothing usable in all 30 calls** (bug-1f8db7, p1).
  - 16 were refused because the immune boundary had isolated the tool.
  - 7 had their result withheld.
  - 4 failed to spawn (`build = "npm"`).
  - 3 were real failures, reported as tool failures.

  The boundary scored a result `invalid_structured_result` and isolated the tool for the run. A likely
  cause: the result filter prefixes `run_tests`' JSON with `[external:run_tests]` before the structure check.
- **T4's gate is V0 though its spec asks for fmt and a working doc example** (find-fc5cdd, p2). Its broken example
  was in runs 1 and 2 (and on 2026-10-18 run 3), and T6 repaired it each time. Options for Will.
- **The audit charges another task's defect to the audited attempt** (bug-2504b3, p2). T5's A2 re-run ran
  `cargo test` over the whole tree and failed on T4's doc example, which T5's tree inherited. It opened
  `false_green` incidents against T5 and gpt-oss-120b in runs 1 and 2. Reproduced outside roko.
- **A retry diagnosis named a file that does not exist** (bug-3f40d2, p3). Run 2's T5 diagnosis blamed
  `crates/alpha/src/initials.rs`.

## Items

- gap-f30b8e: stays open. Its rerun is 3 of 3 on the live test, but 2 of 3 by the review bar, because of run 1's
  delivered build output. A dated note gives this rerun.
- find-5bcba7: closed in `11bbb817f` (+ `49ba19daf`). It was proven by rehearsal r2 here, and not exercised
  live.
- find-8b6828: closed in `74a24a836` (+ `4535b3914`). T5's full upper case held in all three live runs, and the ß
  case rejected the ASCII-only first attempts of runs 2 and 3.
- bug-ba0c40, bug-1f8db7, find-fc5cdd, bug-2504b3 and bug-3f40d2, filed with this record.

## Spend

The cap was $2 for everything. Spent **$0.747** by the wire count ($0.737 by roko's `total_cost_usd`).

| | Cap | roko `total_cost_usd` | Wire |
|---|---|---|---|
| Run 1 | $0.62 | $0.212433 | $0.216717 |
| Run 2 | $0.82 | $0.266352 | $0.268952 |
| Run 3 | $1.25 | $0.258572 | $0.261111 |
| **All** | **$2.00** | **$0.737357** | **$0.746780** |

These cost $0:

- the rehearsals (a fake upstream, 101 calls);
- the config checks (no call);
- the fixture and hand-back tests (scripted providers).

The reviews ran on the Claude subscription.
