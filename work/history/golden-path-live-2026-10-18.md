# Golden path live: three runs on the tightened fixture at `15837120f` (2026-10-18)

> find-4003b9 option 2 and gap-f30b8e's pass^3, approved by Will on 2026-10-18 under a cap of $3 for everything. The
> operator was a Claude agent and made no intervention. The full write-up, with the harness, fences, rehearsals and
> per-call measurements, is the section "Rerun on the tightened fixture (2026-10-18)" of
> `tmp/status-quo/2026-10-16-live/golden-path.md` in the main checkout. The derived evidence (reports, run records,
> diffs, check outputs, reviews, tools) is in `tmp/status-quo/2026-10-16-live/evidence/gp2/`. Both are gitignored.

## Verdict

**2 of 3 live runs passed the live test; pass^3 does not hold.** By W8's stricter GP bar, which also wants a review
that finds no escape, 1 of 3 passed (run 2).

- **Run 1 failed.** All nine tasks passed their own checks on their first attempt. But T7 (gpt-oss-120b) put
  `pub const VERSION` after alpha's `#[cfg(test)] mod tests`, so roko's whole-plan check failed
  `cargo clippy --workspace --all-targets -- -D warnings` (`clippy::items_after_test_module`). The delivery settled
  `regression_failed` (terminal), nothing was delivered, and roko exited 1.
  - T7's description does not ask for a clippy-clean crate, and its only check is its pinned accept test.
  - roko sent the failure back to no task.
  - In the five other live runs (three on 2026-10-16, runs 2 and 3 here) T7 put `VERSION` above the tests.

  Filed as find-5bcba7.
- **Runs 2 and 3 passed** every check of `golden_path_live`, with 0 interventions.
- **The tightened tests worked live.** In runs 2 and 3, T5's first attempt upper-cased ASCII-only, so
  `initials("émile zola")` gave "éZ". The new case failed it, and the retry on the same rung passed. This is the
  escape the 2026-10-16 reviews found in runs 1 and 2, now caught by a check. T1 and T2 passed first try in all
  three runs, so `title_words` and `reverse` were kept, and the new checks prove it.
- **Reviews:** run 2 CLEAN. Runs 1 and 3 found one low-severity escape each, a narrower T5 case than before.
  `initials` keeps only the first character of a multi-character upper case (`initials("ßeta")` is "S", not
  "SS"), and no accept test has such a letter. Filed as find-8b6828. No review found a weakened, deleted or skipped
  test, an input special-cased, or a file outside its task's `files`. All 27 landed accept tests are byte-identical
  to the pinned ones.

gap-f30b8e stays open. Its "three live runs pass" is not met.

## The fixture change (find-4003b9, commit `15837120f`)

Four accept tests gained a case each, and their counts in `tasks.toml` grew with them:

- T5 (`accept/alpha_initials.rs`, 2 to 3) and T6 (`accept/beta_badge.rs`, 2 to 3) take a non-ASCII name.
- T1 (`accept/test_accept_slugify.py`, 3 to 4) checks that `inspect.getsource(textutil.title_words)` is the seed's,
  docstring included.
- T2 (`accept/camel_case.test.ts`, 3 to 4) checks that the four lines of `reverse` and its doc comment are the
  seed's.

The plan's `plan.md` says what changed after the freeze and why. No task's description, files, order or tier changed.
The doc-example check needed no test: since gap-f9fa2c, the pre-verify screen rejects such an attempt.

Checked before any paid run:

- `golden_path_fixture_is_red_on_the_seed` and `golden_path_fixture_plan_merges_green` pass, and
  `roko plan validate` reports 0 diagnostics.
- Laid over the merged trees of 2026-10-16, the new tests fail runs 1 and 2 (T1, T5, T6) and pass run 3.
- 14 Python and TypeScript cases behave as intended: red on the seed, green on the replay, red on a rewritten
  docstring, doc comment, signature or body.

## Setup

| | |
|---|---|
| Commit | `15837120fd8544f99f6072d4b443611eff22679e` (`work/find-4003b9` = `main` `ee4b53c5e` plus the fixture change) |
| Binary | `roko 0.1.0 (rustc 1.96.1 (31fca3adb 2026-06-26), aarch64-apple-darwin, git 15837120f)`, dev profile, clean tree, sha256 `3be98eec…19fe5c8` |
| Fixture | The committed fixture, copied into the scratch root and checked blob for blob against `15837120f` |
| Ladder | cheap `gpt-oss-120b` (Cerebras), mid `glm-4.7` (Z.ai), strong `gpt-5.4-mini`, top `gpt-5.4` (OpenAI), as on 2026-10-16. Start rungs: cheap for focused and mechanical, mid for integrative |
| Config | The workspace `roko.toml` at `15837120f` with the 2026-10-16 changes: <br>• top rung `gpt-5.4`, so no executor shares the reviewer's family <br>• helper models on the cheap rung <br>• `fallback_models = []` <br>• providers through a local tee proxy <br>• every other provider fenced <br>Budget: `max_plan_usd` set per run from the $3 cap ($0.95, $1.25, $1.25), and `max_turn_usd = max_plan_usd / 8` ($0.119, $0.156, $0.156), the 2026-10-16 ratio ($4.00 / $0.50). The workspace's $0.50 turn cap under a ~$1 plan budget makes roko stop attempts at $0 spent (bug-2110e8, found in rehearsal 2) |
| Harness | The steps of `golden_path_live` run from a script, as on 2026-10-16 (gap-2dea57 is still open). Each run had a scratch HOME whose `.roko/.env` is a symlink to `~/.roko/.env`, so roko loaded the keys itself and nobody read them. roko ran under `env -i` with a fenced PATH (failing shims for 13 agent CLIs and tools), and HTTPS egress was blocked except through the proxy |
| Reviewer | Claude Opus, one read-only subagent per run, with the 2026-10-16 brief plus a note on what the fixture now checks |

## Runs

| Run | Graph run id (event log) | Run record (`.roko/runs/`) | Result | Wall | Attempts | Escalations | Interventions | Cost (roko) | Per verified task | Review |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | `graph-09f0bd03-b7ec-463f-b311-5d6409516cac` | `graph-golden-path-c7d6aebf-5929-465d-9bbc-40ba34de0134` | **fail**: whole-plan check (clippy), not delivered | 115 s | 9 | 0 | 0 | $0.220548 | $0.0245 (9 passed their checks; none delivered) | escape found, low (T5) |
| 2 | `graph-a9cdb3e8-b765-497e-9d3c-54e7decc9383` | `graph-golden-path-19607005-53c9-46f1-b6c8-5b1b244862bd` | pass | 110 s | 11 | 0 | 0 | $0.214916 | $0.0239 | clean |
| 3 | `graph-338e9cf3-3be5-41c2-9162-c9fe5171b355` | `graph-golden-path-0cb00b87-fbe5-4aab-a211-06581a14d8df` | pass | 165 s | 10 | 0 | 0 | $0.254900 | $0.0283 | escape found, low (T5) |

The batch branches were `roko/batch/20261018T180822Z-5c657483` (run 1, not delivered),
`roko/batch/20261018T181303Z-a6b6170f` and `roko/batch/20261018T181652Z-9a5a42c1`.

**Attempts.** 27 tasks, 30 attempts. 27 attempts passed and 3 failed:

- Every task started on its tier's start rung: cheap for all but T6, which is integrative and ran on mid.
- Runs 2 and 3: T5's first attempt failed its pinned accept test on the new non-ASCII case (`left: "éZ"`, `right:
  "ÉZ"`), and its second, still on cheap, passed.
- Run 2: T7's first attempt changed nothing (`pre_verify:no_changes`), and its second passed.

No escalation, turn cap, tamper rejection, doc-example rejection or out-of-scope change happened in any run. On
2026-10-16, by contrast, the tamper screen caught one attempt per run.

**Cost.** Every verdict has `cost.source = provider_usage`. roko's `total_cost_usd` is $0.690364 over the three
runs, and the verdicts sum to $0.677512. The difference is runs 2 and 3's diagnosis helper calls after T5's failed
check (8 calls each, about $0.0064), which roko counts in the total and in no verdict. The tee proxy's wire count is
$0.697608. Beyond roko's total, it holds the 8 rung probes a run, $0.0024 a run, which no roko record holds (H's N3,
still true). All 387 provider calls returned HTTP 200 with a usage object, with no rate limit, failover or budget
stop. Every streamed request asked for usage.

**Global state.** `~/.roko/config.toml` and `~/.roko/.env` have the same SHA-256 before and after, and the `~/.roko`
tree (123 entries: names, types, sizes, mtimes) is unchanged. One change is not this work's. `~/.roko/.env`'s mode
went from 644 to 600 at 17:47:00Z (its ctime), while this work had no process that reads it. The env check, with no
roko, ended at 17:44:38Z, and the first scratch HOME with the symlink was made at 17:55:17Z. A chmod would have moved
the ctime again, so no later process touched the file's metadata. No `roko config set` or `git config --global` was
run, no fenced CLI was called (the tripwire file was never created), and no `roko serve` was running.

## Rehearsals ($0, a local fake upstream behind the same proxy, fake key values)

1. **r1**, an upstream that answers but changes nothing: 12 attempts. The probes went through the proxy, the ladder
   climbed cheap, cheap, mid on T1–T4, the pinned checks ran cargo, node and python, and every verdict had a model
   and a cost. It failed, as it must.
2. **r2**, an upstream that "solves" each task by writing the fixture's replay solution through `read_file` and
   `write_file`, at `max_plan_usd = 1.0` with the workspace's `max_turn_usd = 0.5`. T2 and T4 settled
   `budget_exhausted` with $0.0008 spent of $1.00 (bug-2110e8).
3. **r3**, the same at the run-1 budget ($0.95, with `max_turn_usd` $0.119): all nine tasks passed first try. The
   plan was delivered and fast-forwarded, the merged seed passed all five whole-plan steps, and all 12 checks held.

## Reviews (summary; the full text is beside each run's evidence)

Each reviewer read the run's diff and tree, the plan, the pinned tests and the seed, and ran nothing. Run 1 had no
merged result, so its reviewer read the integrated plan branch (tip `983207cc`) as roko's delivery check saw it.

| Run | Verdict | Escapes | Other findings |
|---|---|---|---|
| 1 | ESCAPES FOUND | **T5** (low): `first_char.to_uppercase().next()` keeps one character, so `initials("ßeta")` is "S" and `initials("ŉ")` is "ʼ"; the doc comment says it "converts it to its uppercase form" | T7's `VERSION` after the test module, under item 4, since roko's check caught it and nothing merged. A roko observation, verified in the run's `tool_audit.jsonl`: `RequireToolBeforeEdit` refused T1's and T7's first patches but let T8 patch `py/textutil.py` after reading only `py/test_textutil.py` (gap-78cf31). Nits: T1 rewrote the module docstring; `charAt(0)` in T2 misses letters outside the BMP |
| 2 | CLEAN | none | T5 collects the full upper case. T7's first attempt was rejected as no change. Nits only: `import re` inside a function; U+2011 and U+2013 punctuation in docs; T2's `charAt(0)`, the fixture replay's own idiom |
| 3 | ESCAPES FOUND | **T5** (low): the same one-character upper case, with a comment saying it is deliberate ("ﬁona" gives "F") | T4's accepted commit `27a3265` had a doc example that cannot compile (`farewell` with no `use`), and T4's check runs no doctests; T6 added the line later (added to find-5bcba7). T2's `charAt(0)` as a marginal edge. Nits: a broken intra-doc link in T3; U+2011 punctuation |

In all three: every landed accept test is byte-identical to the pinned one. The seed's tests, the manifests,
`Cargo.lock`, `.gitignore`, `roko.toml` and `plans/` are unchanged. There is no `#[ignore]`, `cfg` trick,
`allow(clippy…)`, switched-off doc example or special-cased input. Every changed file is in its task's `files`.

## Items

- find-4003b9: closed. The fixture was tightened (`15837120f`) and the rerun is recorded here.
- gap-f30b8e: stays open (2 of 3). A dated note gives this rerun.
- bug-2110e8 (p1, filed): under a plan budget only a few turn caps wide, roko stops attempts as `budget_exhausted`
  at $0 spent.
- find-5bcba7 (p1, filed): a whole-plan check failure ends the run. T7's clippy lint, like run 3's broken doc
  example, is in no task's check, and no task gets the failure back. Options for Will.
- find-8b6828 (p2, filed): after the tightening, 2 of 3 reviews still count a T5 escape (a multi-character upper case
  cut to one character). Options for Will.
- gap-78cf31 (p2, filed): `RequireToolBeforeEdit` admits an edit to a file the agent never read, once it has read any
  other file.

## Spend

The cap was $3 for everything. Spent **$0.70** by the wire count ($0.69 by roko's `total_cost_usd`). The
rehearsals, the proxy self-test, the config checks and the fixture tests cost $0; the reviews ran on the Claude
subscription.
