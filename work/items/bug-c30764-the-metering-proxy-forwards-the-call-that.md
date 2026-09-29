+++
id = "bug-c30764"
kind = "bug"
title = "The metering proxy forwards the call that crosses input_token_cap, so a task can overshoot its input cap by one call"
status = "done"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "6c44ef04a"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-fix1's report on gap-e003ec)"
anchors = ["benchmarks/viabilitybench/driver/faultproxy.py", "benchmarks/viabilitybench/driver/vb.py"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-e003ec", "gap-e90ebd", "gap-33d54b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_the_input_cap_bounds_a_tasks_input' benchmarks/viabilitybench/driver/test_faultproxy.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_faultproxy.py -k test_the_input_cap_bounds_a_tasks_input -q"

[closed]
at = 2026-09-29
commit = "6c44ef04a"
by = "wk-bench-fix1"
evidence = "6c44ef04a: faultproxy refuses a call when counted input + in-flight reservations + the call's input bound would pass input_token_cap; the bound is the request's bytes + PREAMBLE_TOKENS, or an extended prompt's earlier reported input + the added messages' bytes; missing usage counts the bound; rows log input_bound. Verify passes (test_the_input_cap_bounds_a_tasks_input: metered input stays under the cap, every bound covers the billed input, a held call's reservation refuses a second); full viabilitybench suite 329 passed, 2 skipped with the prebuilt roko. Real roko keeps its prefixes, so its later calls' bounds were 1,332 and 1,469 against a 24,036-byte first call."
+++

## Problem

The metering proxy refuses a task's calls only once its counted input has reached `input_token_cap` (`driver/faultproxy.py`:29-30). The check runs before a call is forwarded and uses the input counted so far. The call that crosses the cap is therefore forwarded in full. A task can overshoot `input_tokens_per_task`, and the `worst_task_usd` bound that the driver's admission and budget lines are built on, by up to one call's input.

## Why it matters

Pilot benchmark (epic spec-567e52): the caps are the guarantee that a run can't exceed its budget line (S09 §4.6). A bound that one call can break isn't a bound. The overshoot is largest for the long-context calls at the end of a task.

## Where

The cap check in `faultproxy.py`, and how `vb.py` computes `worst_task_usd` from the arm's caps.

## Plan

There are two ways to make the bound hold. Pick one:

- **(a)** Before forwarding, estimate the call's input from the request body (a conservative bytes-to-tokens ratio) and refuse the call if it would cross the cap.
- **(b)** Keep the check, and include one maximum-size call in `worst_task_usd` and the caps.

(a) keeps the bound tight. Then add `test_the_input_cap_bounds_a_tasks_input`.

## Done when

- [ ] No task's metered input exceeds its cap, or the cap arithmetic includes the overshoot.
- [ ] The `[[verify]]` command passes.
