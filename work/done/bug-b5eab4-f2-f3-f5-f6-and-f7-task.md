+++
id = "bug-b5eab4"
kind = "bug"
title = "F2, F3, F5, F6 and F7 task repos head README.md with their S08 family name ('# api-pager', '# ts-result'), F6's package is 'ts-result-task', and F8's inherit the heading"
status = "done"
triage = "verified"
severity = "p3"
goal = "proof"
size = "S"
subsystem = ["viabilitybench/families"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "b17c3e49f"
source = "bug-d7ff47 worker (claude-agent), 2026-10-17 (every family's gen.py at levels 1-5 x seeds 1-3 with every latent: the workdir files that carry an S08 family name)"
discovered_from = "bug-d7ff47"
anchors = ["benchmarks/viabilitybench/families/f2_apipager/gen.py::README_TEMPLATE", "benchmarks/viabilitybench/families/f3_moneyround/gen.py::README_TEMPLATE", "benchmarks/viabilitybench/families/f5_sqlmigrate/gen.py::README_TEMPLATE", "benchmarks/viabilitybench/families/f6_tsresult/gen.py::README_TEMPLATE", "benchmarks/viabilitybench/families/f6_tsresult/gen.py::PACKAGE_JSON", "benchmarks/viabilitybench/families/f7_rustiter/gen.py::README_TEMPLATE"]
lane = "bench"
links = { depends_on = [], blocks = [], related = ["bug-d7ff47", "bug-9f6252", "gap-394f28"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_no_task_repo_names_its_family' benchmarks/viabilitybench/ci/test_ci.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/ci/test_ci.py -k test_no_task_repo_names_its_family -q"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T07:08:38Z"
commit = "b17c3e49f"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-17T05:23:24Z"
forced = false
evidence = "b17c3e49f: every family's task repo carries a product name, one per family, as F1's (ledgerline, ...) and F4's (tallybook, ...) always did: F2 README '# chargebridge' (was '# api-pager'), F3 '# billfold' (was '# money-round'), F5 '# stockroom' (was '# sql-migrate'), F6 '# shelfwise' and package.json \"shelfwise\" (were '# ts-result' and \"ts-result-task\"), F7 the crate cinder and README '# cinder: <stub>' (were f7_task, the family id in lowercase, which bug-d7ff47's \\bF[0-9]\\b missed, and '# rust-iter: <stub>'); F8 inherits its base's new heading. F7's visible tests and precise spec name the new crate and change in nothing else, and F7's hidden.py reads the crate from the instance's restored Cargo.toml (hidden.crate_name), so an instance of any generator version gets hidden tests of its own crate. Generators f2-1.3.0, f3-1.3.0, f5-1.3.0, f6-1.2.0, f7-1.3.0 (truth suite f7-truth 1.2.0), f8-1.4.0. Decided under Will's decision 9 (## Decision below): no benchmark-internal identifier reaches an agent, for benchmark validity and against contamination through the public repository. The verify passes: test_no_task_repo_names_its_family (ci/test_ci.py), 1 passed: every task family rendered as the verifier CI renders it (levels 1-5, every latent, F8 over F2, F3 and F5, and the plan slice's six features; the rendering, agent_views, is now shared with test_no_task_repo_names_a_family_id); no workdir file (.git aside), in its path or its text, and no spec the agent is handed names a family by its S08 name (money-rounding too), its directory's name (apipager, ...) or its id inside an identifier (f7_task). On the previous generators it fails on 35 renders (every F2, F3, F5, F7 and F8 render, F6's at both latents). Agent-facing trees across 625 instances (all 216 F2, F3, F5, F7 and F8 instances of the streams, LOG1's included, plus levels 1-5 x seeds 1-12 of each family, F6 at v1 and v2; 4,190 files), previous vs new gen.py: the same files and modes; every file byte-identical except README.md (all 625), package.json (F6, 120) and Cargo.toml and tests/visible.rs (F7, 96), each the previous text with the renames applied and nothing else; no family name, id or id token left; spec.precise.md identical but on F7 (the rename only); the vague spec materialize builds (F2, F3, F5, F7: 435) identical; task.json identical but for generator_version (on F7 also the truth suite's version, the spec's sha256 and tests/visible.rs's hash, all matching the new files); the new workdir at the new pristine commit and tree, whose blobs differ exactly for the changed files; F8's wrapped-base record changes the same way. The comparison flags all 35 renders when comparing the previous tree with itself, and the 20 it should when one extra change is planted. verify_verifiers --levels 1-5 --seeds 2: f2, f3, f5, f6, f8 60/60 green; f7 10/10 green under the cargo limiter (verifier f7-truth-1.2.0). Bench pytest: 800 passed, 16 skipped (13 with no roko binary in the worktree, the analysis stack, two opt-in sweeps)."
+++

## Problem

Five of the seven base families head their task repo's `README.md` with the family's own S08 name, the one the specs
and papers use beside its id ("F2 `api-pager`"): F2 `# api-pager`, F3 `# money-round`, F5 `# sql-migrate`, F6
`# ts-result` and F7 `# rust-iter: add` (each family's `gen.py::README_TEMPLATE`). F6's `package.json` also names
the package `"ts-result-task"`. F8 keeps its wrapped base's README heading (bug-9f6252), so every F8 task repo shows
`# api-pager`, `# money-round` or `# sql-migrate`. F1 and F4 do not: F1's README is `# ledgerline` and F4's is
`# tallybook`, product names (F4's CLI is called `kvtool` throughout, as the product it is; its family is
`kvtool-cli`). To see it: render each family with `gen.py --level 1 --seed 1 --out DIR` and read `DIR/repo/README.md`'s
first line. Rendered at levels 1-5 x seeds 1-3 with every latent (165 workdirs), the names appear in README.md in
every F2, F3, F5, F6, F7 and F8 workdir (105) and in every F6 `package.json` (30), and nowhere else once bug-d7ff47's
docstrings are gone; F7's `Cargo.toml` does not name it.

## Why it matters

Goal `proof`. A weaker form of bug-d7ff47 (family ids in module docstrings): these names read as plausible project
names, but they are the benchmark's family names, so an agent that knows the benchmark can tell which family it
faces, and `ts-result-task` calls the repository a task outright. F2, F3, F5, F6, F7 and F8 instances are marked
this way and F1 and F4 instances are not, so the families are not run under the same conditions. F8's honeypot is
not revealed by this (its heading is its base's, the same as an instance of the base family). Changing the headings
changes six families' task repos, so it has to be decided before the pre-registration lock (gap-394f28).

## Where

- `benchmarks/viabilitybench/families/f2_apipager/gen.py::README_TEMPLATE`, and the same in `f3_moneyround`,
  `f5_sqlmigrate`, `f6_tsresult` and `f7_rustiter`.
- `benchmarks/viabilitybench/families/f6_tsresult/gen.py::PACKAGE_JSON`: `"name": "ts-result-task"`.
- `benchmarks/viabilitybench/families/f8_honeypot/gen.py::render_readme` keeps the base's heading.

## Current state

The headings came with the families (gap-46fd19, gap-eb1aa3). bug-d7ff47 removed the family ids ("(F2 api-pager)")
from F2, F3 and F5's module docstrings and added `ci/test_ci.py::test_no_task_repo_names_a_family_id`, which checks
`\bF[0-9]\b` only. No precise spec names a family (rendered at level 1, seed 1).

## Plan

1. Decide whether a task repo may carry its family's S08 name. If it may not:
2. Give each README a product name, as F1 and F4 have (and F6's package one), and bump `GENERATOR_VERSION` in F2,
   F3, F5, F6, F7 and F8.
3. Add `test_no_task_repo_names_its_family` to `ci/test_ci.py`, beside `test_no_task_repo_names_a_family_id`: no
   workdir file of any task family names a family's S08 name.
4. Re-run the verifier CI for the six families, and show that only the renamed text changes in their task repos.

## Done when

The `[[verify]]` command passes, and `ci/verify_verifiers.py --families f2,f3,f5,f6,f7,f8 --levels 1-5 --seeds 2`
stays green.

## Notes

- Found by the bug-d7ff47 worker, 2026-10-17, while checking what else in a task repo names its family.
- Change nothing else in the task repos: the visible tests, the files in scope and the precise specs stay as they
  are. None of the 165 rendered precise specs names a family.

## Decision (2026-10-17)

Plan step 1, whether a task repo may carry its family's S08 name, had no recorded default. Under Will's decision 9 of
2026-10-15 (technical questions with no recorded default take the coordinator's recommendation, the conservative
option, recorded on the item with the reasoning; Will can veto it later), the coordinator decided that it may not: no
benchmark-internal identifier reaches an agent. Every family's task repo carries a product name instead, in its README
heading, its package names and anything else agent-facing, settled before the pre-registration lock (gap-394f28).

Why: benchmark validity, and contamination through the public repository. The S08 names are the ones the specs and
papers use beside the family ids, so each is a search key that leads an agent to its family and its truth suite.
`ts-result-task` calls the repository a task outright, and F7's crate `f7_task` was the family id itself. The names
marked F2, F3, F5, F6, F7 and F8 instances and not F1 or F4 ones, so the families did not run under the same
conditions. Product names are the conservative option: they change no check, no visible test's assertions and nothing
an agent is graded on (F7's visible tests and spec change only in the crate's name, which the hidden tests now read
from the instance), and the change is cheap before the lock and expensive after it. One fixed name per family, rather
than one drawn per instance as F1 and F4 do, keeps the change a pure rename that can be checked byte for byte.
