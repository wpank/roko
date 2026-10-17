+++
id = "bug-9b65dc"
kind = "bug"
title = "A rung whose model fails the same infra way on every try (the boundary denying its stream, a stream that stalls or breaks) never climbs: such a failure is blamed on infra, so glm-4.7 burned four tries and strong and top were never tried"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "04e46172b"
source = "tmp/hermes-roko-v6/findings/dogfood-will.md R3-2 (Run 3, 2026-10-17); seen again in roko-caps' live pocket-synth run"
anchors = ["crates/roko-cli/src/graph_task_dispatch/ladder.rs::note_ladder_outcome"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-3e770b", "gap-618a75"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_repeated_infra_failure_on_a_rung_climbs' crates/roko-cli/src && cargo test -p roko-cli --lib a_repeated_infra_failure_on_a_rung_climbs"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T13:31:14Z"
commit = "9442978eb"
forced = false
evidence = "9442978eb: note_ladder_outcome keeps, per task, a run of infra failures of one class on one rung: an immune denial by its reason code, any other failure by its outcome and the shared classifier's kind (provider_error:timeout for a stalled stream, provider_error:server_error for a broken one). The second of a class in a row climbs the task within MAX_ESCALATIONS and M1's cap; it is still not a failed try and never exhausts the ladder. Another class, a pass or an agent failure breaks the run; rate limits, exhausted quotas, credits and credentials never count (failover and the breaker answer them), nor do pinned or failover-substitute tries. classify_failure_text names a stream broken mid-response (read chunk failed, error decoding response body, a reset, a broken pipe) as a transient server_error, so provider health stops filing it as unknown. a_repeated_infra_failure_on_a_rung_climbs and a_broken_stream_is_a_transient_server_failure pass. Zero-cost run (roko-caps/e2e-climb.sh: plans/long-think, rungs mid on a stand-in that streams reasoning then an overload error object, as z.ai did, and strong on one that writes the file): with 04e46172b, tries 1 and 2 failed on mid (provider_error, infra), attempt_settled climbed true, try 3 ran on strong (route_decided ladder_reason climb) and passed; with main 47b3cdd20 and with the denial-only build, three tries failed on mid and the fourth reached strong only by failover once the breaker opened. Checks at 04e46172b (on main 7a5dd6046): fmt --check, clippy -D warnings (roko-agent, roko-core, roko-cli, roko-serve), lib tests roko-agent 2371 passed, roko-cli 3774 passed with the known load flake plan_check_publishes_start_heartbeat_and_finish (bug-8d8f98: an extra heartbeat while the slot's tests ran; plan_verify.rs is untouched); the [[verify]] passes."
+++

## Problem

A rung whose model the immune boundary denies the same way on every try never climbs. In dogfood-will's Run 3
(`tmp/hermes-roko-v6/findings/dogfood-will.md`, R3-2), SVF climbed to mid glm-4.7, and roko's boundary denied three of
its four tries there as `provider_stream_limit_exceeded` (R3-1). The fourth broke with a read error. A denial is
blamed on infrastructure, so `note_ladder_outcome` does not count it against the rung. The task used up its retries
on glm-4.7, the plan failed, and strong and top were never tried. The provider's circuit breaker does not count a
denial either (`dispatch_v2.rs::ProviderHealthOutcome::ImmuneDenied`), so nothing moved the task off that model.

roko-caps saw the same thing live on 2026-10-17 (pocket-synth, binary 69361cb50). After two bounded cheap tries, SVF
climbed to glm-4.7, and its first try there was denied after 136 s.

## Why it matters

Cheap first only pays if a rung that cannot run the task is left behind. A rung stuck on the same denial burns the
task's retries and its wall-clock time, and the plan fails without trying the models above it.

## Where

- `crates/roko-cli/src/graph_task_dispatch/ladder.rs::note_ladder_outcome` counts a settled try toward the task's
  ladder standing. It counts only agent-blamed failures.
- A denial reaches it as a `provider_error` verdict, blamed on infra, whose failure reason reads
  `provider: provider result denied by immune boundary (reason: <code>)` (the boundary's `denied_result`).

## Plan

1. A try's infra failure is classed: an immune denial by its reason code, any other failure by its outcome and the
   shared failure classifier's kind (`provider_error:timeout` for a stream that stalled, `provider_error:server_error`
   for one that broke). Since R3-1 (bug-91796d, merged as aef71f97d), glm-4.7's tries on SVF fail by such timeouts
   and broken streams far more often than by the stream-limit denial.
2. Two infra failures of the same class in a row on one rung count that rung's model as unable to run the task. The
   second one climbs the task, as two failed checks do, within `MAX_ESCALATIONS` and M1's cap.
3. The run is kept per task and per rung. A failure of another class breaks it, and so do a pass and an agent-blamed
   failure. A failure of the provider account (rate limit, exhausted quota, credits, credentials) is left to failover
   and the circuit breaker and never counts. A pinned try and a failover substitute's try never count either.
4. Such a failure stays an infra outcome: it is not a failed try, and it never exhausts the ladder.
5. On the breaker side, the shared classifier now names a stream broken mid-response ("read chunk failed: error
   decoding response body", a reset, a broken pipe) as a transient `server_error`. Provider health used to file it as
   `unknown`.

## Done when

`a_repeated_infra_failure_on_a_rung_climbs` passes:
- two identical denials climb, and so do two stalled streams and two broken streams;
- failures of different classes in between do not climb;
- a rate limit, a pinned try and a substitute never count.

The `[[verify]]` command passes.

## Notes

- roko-recovery fixed the stream limit itself (R3-1, bug-91796d) and the spend that cut calls leave unrecorded (R3-3,
  bug-72d775) in `roko-agent`. The denial's text and its `immune_denied`/`immune_reason` tags are unchanged.
- The circuit breaker still ignores immune denials, as backlog 1114 decided: most denials are host policy.
