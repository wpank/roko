Status: reviewed · budget 550 words · owner gap-2aad7d

# 8 Evaluation plan

## 8.1 The claim

Roko is designed to match Claude Code on Opus 5.5 in quality at a lower cost per verified task, on
decomposable, checkable work (spec-567e52). The claim is UNPROVEN@a43288b5f (appendix row V7). A task is verified
only when its final output passes hidden tests the agent never sees. Its cost counts every attempt,
retry, escalation, planner call and model-based check at API list price from one dated snapshot (gap-0580f7),
with subscription cash reported separately.

## 8.2 Arms

The arms run on ViabilityBench, a hidden-test benchmark at `benchmarks/viabilitybench/` (dec-b78874), under its
arm ids (designed): `cheap_direct`, `roko_fixed`, `roko_full`, `fd_claude` and `fr_claude`.

| Stage | Arm | What runs | Answers |
|---|---|---|---|
| Pilot, full | `cheap_direct` | gpt-oss-120b in a bash-only loop | A cheap model unaided |
| Pilot | `roko_fixed` | gpt-oss-120b in Roko: one pinned model, gates and retries | What Roko adds |
| Pilot, full | `fd_claude` | Claude Code on Opus 5.5, on the subscription | The baseline |
| Full | `roko_full` | Cheap models in Roko; the tier ladder, on by default, escalates among them | The claim |
| Full | `fr_claude` | Opus 5.5 in Roko, a 48-task probe | What Roko adds to a frontier model |

The pilot runs 20 tasks from two Python families at five difficulty levels, 3 seeds each, for at most $15
billed; only its cost ratio is likely to resolve.[^8-pilot] The full comparison, to be pre-registered before it
spends money, has three arms plus a probe. An exploratory plan slice runs 6–10 generated features of 4–8 tasks,
one seed first: Roko, with a frontier planner, the ladder and a whole-plan gate, against Claude Code and its
subagents (gap-89f393, gap-1cd676).

## 8.3 Metrics and safeguards

Metrics are reported by task family and difficulty level, with 95% paired-bootstrap intervals and run
ids:[^8-metrics] the verified success rate; cost per verified task, or per verified feature; pass^k, the chance
that all k seeds succeed; false greens, accepted runs that hidden tests reject; and wall-clock time, or a plan's
makespan.

Safeguards: the hidden-test secret sits in a driver-only file, read only after the agent exits (gap-a8a160); Claude
Code runs with an isolated config, free of the user's memory, hooks and plugins (gap-c4f364); and each run starts
fresh, with every attempt's model checked, because failover can switch it; Roko has recorded a switch since
`6f8286d48` (gap-b7ab99), though failover itself, which moves a task sideways or down the ladder, is PARTIAL@a43288b5f
(row RC4).

## 8.4 Falsifiers

The full comparison tests difficulty levels cumulatively. Level j holds if, on tasks up to level j,
`roko_full`'s verified success rate is at least 0.90 of `fd_claude`'s and its cost per verified task at most
0.30 of `fd_claude`'s, both at 95% confidence or stricter; testing stops at the first failure.[^8-bar] These
results would count against the thesis:

- level 1 fails: the claim does not hold on this benchmark;
- the last level that holds is below 3, the pre-registered expectation (which also has Claude Code ahead at
  level 5);
- `roko_full` is no better than `cheap_direct` in verified success: the savings are the model's, not Roko's;
- `roko_full`'s pass^3 is below `fd_claude`'s, with an interval that excludes zero;
- in the plan slice, Roko verifies fewer features, or pays more per verified feature, than Claude Code.

## 8.5 Scope and prerequisites

The pilot and the full comparison measure single tasks, where every arm gets the same written spec; the
planner's cost enters only in the plan slice, charged to Roko. The families are mostly Python, and the baseline
is Claude Code as shipped, not a tuned frontier harness.

Later Roko arms need honest verdicts end to end, WIRED@a43288b5f since spec-e9d7ec (row QA2); a ladder that escalates,
WIRED@a43288b5f and driven end to end by canary C8 (row EX7, gap-e21595); and failure paths that are designed to climb
the ladder rather than fall down it and to read every provider's stream, both PARTIAL@a43288b5f after the first live
run (rows RC1, RC4; gap-625195, gap-e00238).[^8-live] The plan slice's whole-plan gate is WIRED@a43288b5f (row IS3).
The benchmark's driver replaces `roko bench`, which leaks the SWE-bench gold patch (bug-28becc).

No arm has run: the pilot's results are pending gap-d9e9fe (`vb report --pilot`), and the plan slice's
gap-1cd676.

[^8-pilot]: spec-567e52. The direct arms run in gap-c33709, the Roko and Claude Code arms in gap-327242. The full
    comparison's arms and the 48-task probe: programme spec S09 v1.1, frozen as `evidence/2026-09-28-s09-h1-bar.md`
    (sha256 `029756b064eb`), "Design (D1)".

[^8-metrics]: gap-d9e9fe, which labels the pilot's page "pilot, descriptive".

[^8-bar]: The author's bar (D3), decided on 2026-09-28: S09 v1.1, frozen as `evidence/2026-09-28-s09-h1-bar.md`
    (sha256 `029756b064eb`), "Envelope procedure" and H1's "Primary", which also give the expectation. gap-c4f364
    and gap-d9e9fe refer to the bar, and spec-567e52 defers `roko_full` until its mechanisms are live.

[^8-live]: The live run of 2026-10-02, frozen as `evidence/2026-10-02-live-cheap-model-run.md` (sha256
    `813172c96b88`), "What broke" 2 and 3: GLM-4.7 returned blank answers, and failover moved a task down to the cheap
    rung four times.