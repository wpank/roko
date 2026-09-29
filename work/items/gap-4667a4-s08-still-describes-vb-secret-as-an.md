+++
id = "gap-4667a4"
kind = "gap"
title = "S08 still describes VB_SECRET as an environment variable at lines 106 and 302"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["cybernetic-harness/specs"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "42319587f"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-bench-fix3's report)"
anchors = ["tmp/cybernetic-harness/specs/S08-benchmark-suite.md"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = ["gap-6e1381", "gap-a8a160"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'is scrubbed from agent envs' tmp/cybernetic-harness/specs/S08-benchmark-suite.md && ! grep -q -- '--secret \\$VB_SECRET' tmp/cybernetic-harness/specs/S08-benchmark-suite.md"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "S08 (tmp, wk-bench-fix3): line 106 describes the driver-only secret file and key file, the tripwire and the canaries; line 302 shows hidden.py --secret-file and how the path travels; SC4 (line 44) and principle 2 (line 102) fixed under the same no-env-var rule; the only VB_SECRET left is $VB_SECRET_FILE in decision 8. Verify passes in MAIN."
+++

## Problem

gap-6e1381 fixed S08 §9 decision 8. Two other lines of `tmp/cybernetic-harness/specs/S08-benchmark-suite.md` still describe the secret as an environment variable:

- :106, "`VB_SECRET` is scrubbed from agent envs";
- :302, `hidden.py --task … --workdir <clean worktree> --secret $VB_SECRET`.

Since gap-a8a160 the secret is a driver-only file (`--secret-file`, `$VB_SECRET_FILE` or `~/.config/viabilitybench/secret`), passed to `hidden.py` as `--secret-file PATH`, and never set in any environment.

## Why it matters

Pilot benchmark (epic spec-567e52): S08 is the spec that implementers and the paper cite. A reader following it would put the secret where agents can read it. p3, because the code is right.

## Where

The two lines. S08 is untracked (`tmp/`), so edit it in place.

## Plan

1. Rewrite both lines for the driver-only file and `--secret-file`, citing gap-a8a160 and the tripwire (gap-308373).

## Done when

- [ ] S08 never describes the secret as an environment variable.
- [ ] The `[[verify]]` command passes.
