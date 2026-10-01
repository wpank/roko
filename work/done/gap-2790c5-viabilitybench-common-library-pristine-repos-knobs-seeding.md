+++
id = "gap-2790c5"
kind = "gap"
title = "ViabilityBench common library: pristine repos, knobs, seeding, AST checks and canaries (S08.T2)"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
size = "M"
subsystem = ["benchmarks/viabilitybench/families"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "ff5a7a1dc"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e12"
discovered_from = "tmp/cybernetic-harness/specs/S08-benchmark-suite.md (§4.2, §4.4, §6 T2; checklist S08.T2)"
anchors = ["benchmarks/viabilitybench/families/common/"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = ["gap-0580f7"], blocks = [], related = ["gap-a8a160"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_pristine_restore_gives_same_tree_hash' benchmarks/viabilitybench/families/common/test_common.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/families/common/test_common.py -k test_pristine_restore_gives_same_tree_hash -q"

[[verify]]
command = "grep -qw 'def test_hidden_cases_depend_on_secret_file' benchmarks/viabilitybench/families/common/test_common.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/families/common/test_common.py -k test_hidden_cases_depend_on_secret_file -q"

[closed]
at = 2026-09-29
commit = "ff5a7a1dc"
by = "commit trailer"
evidence = "ff5a7a1dc adds benchmarks/viabilitybench/families/common/ (S08 T2): repo.py (a deterministic pristine commit saved to a bundle outside the workdir; plumbing-only exports of the disk or of a commit; restores that replace planted symlinks; git's tree hash in Python), knobs.py (ladder.toml, S08 §4.4 types and bands, seeded range draws, instance ids), hmac_seed.py (HMAC-SHA256 streams, public for the visible instance and secret-keyed for hidden cases; the secret only from a 0600 --secret-file, and --secret VALUE is refused), astcheck.py (literal returns, skipped or ignored tests, visible-test edits, raises outside the required base class), canary.py (release canary, marker lines, strip on render, search in text, diffs and trees) and mutate.py (seeded whole-identifier renames). Both [[verify]] commands pass in the pinned venv, and all 35 tests in test_common.py pass under Python 3.11 and 3.12, with a planted example for each AST check and canary."
+++

## Problem

Every task family needs the same machinery, and none of it exists:

- a task repo with a pristine snapshot of its base commit;
- the difficulty knobs for levels ℓ1–ℓ5;
- hidden test cases derived from a secret;
- AST checks that detect gaming;
- canary strings that show when an agent has read a hidden file.

If each family writes its own copy, the verifiers will disagree with each other.

## Why it matters

S08's principles (§4.2) rest on this code: VS-census labels, hidden tests that stay hidden, canaries, and a fresh,
archived workdir per task. So do SC1 (valid verifiers) and SC4 (hidden tests stay hidden). F1 (gap-4723ff), F4
(gap-9e7079) and the driver (gap-28ebea) wait on it, then run in parallel.

## Where

All new, and the path follows D4: `benchmarks/viabilitybench/families/common/`. It holds `repo.py`, `knobs.py`,
`hmac_seed.py`, `astcheck.py`, `mutate.py`, `canary.py` and `test_common.py`.

## Current state

Checked at `41c7ffbd6`: nothing exists.

`scripts/dev_benchmark.py` has helpers worth copying: private directories (`private_mkdir`), captured subprocess
runs (`run_capture`) and worktree creation (`create_worktree`).

## Plan

1. **`repo.py`:** create a task repo from a template at a base commit and record its pristine tree hash; restore
   the visible test files from that base; export a clean worktree of any commit for the census.
2. **`knobs.py`:** load a family's `ladder.toml` and map each level ℓ to the knob values in S08 §4.4.
3. **`hmac_seed.py`:** derive per-instance random streams from HMAC-SHA256 of (secret, family, instance_id). The
   secret arrives only as a file path (`--secret-file`), never on a command line or in the environment. This
   departs from S08 §5.2's `--secret $VB_SECRET`, because agents running at the same time can read command lines
   with `ps`.
4. **`astcheck.py`:** detect literal returns, skipped or ignored tests, changed visible-test hashes, and exceptions
   that do not subclass the required base class.
5. **`canary.py`** inserts a per-release canary GUID and finds it in text, diffs and trees; **`mutate.py`** applies
   surface renames so instances vary.
6. **Fix the public signatures first,** in module docstrings: three items build on them at once.

## Done when

- [ ] Restoring from the pristine snapshot gives the same tree hash.
- [ ] The same secret, family and seed give the same instance. A different secret gives different hidden cases.
- [ ] Each AST check fires on a planted example, and every inserted canary is found.
- [ ] Both `[[verify]]` commands pass.

## Notes

- **Dependencies:** stdlib only, Python ≥ 3.11.
- **Secrets in tests:** use a throwaway secret file, never a real one.
- **Where the real secret lives:** gap-a8a160.
