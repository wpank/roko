+++
id = "gap-a2f86a"
kind = "gap"
title = "F7's in-tree .cargo-target build directory lands in commit_final's archived c_i"
status = "open"
triage = "verified"
severity = "p2"
goal = "proof"
size = "S"
subsystem = ["benchmarks/viabilitybench/families/f7_rustiter", "benchmarks/viabilitybench/driver"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "wave-3 follow-up reports 2026-10-02 (w3-bench gap-46fd19)"
discovered_from = "gap-46fd19 (task 3324 handled CARGO_HOME/RUSTUP_HOME but left the agent-facing visible-verify's in-tree target dir as originally designed)"
anchors = ["benchmarks/viabilitybench/families/f7_rustiter/gen.py::VISIBLE_VERIFY", "benchmarks/viabilitybench/families/f7_rustiter/hidden.py", "benchmarks/viabilitybench/driver/archive.py::commit_final", "benchmarks/viabilitybench/families/common/repo.py::export_tree"]
lane = "bench"
parent = "spec-567e52"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_f7_archive_excludes_the_cargo_target_dir' benchmarks/viabilitybench/families/f7_rustiter/test_f7.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/families/f7_rustiter/test_f7.py -k test_f7_archive_excludes_the_cargo_target_dir -q"
+++

## Problem

In the F7 (`rust-iter`) family, the agent's own visible-verify step builds its crate with an **in-tree** target
directory, which `commit_final` then sweeps into `c_i` (the archived final commit), bloating the archive and the
diff for every real F7 run.

`families/f7_rustiter/gen.py`'s `README_TEMPLATE` (and `VISIBLE_VERIFY`, :71) tell the agent to verify with:

```
CARGO_TARGET_DIR=.cargo-target cargo test --offline --test visible
```

`.cargo-target` is a **relative** path, so it is created at `<cwd>/.cargo-target` — inside the crate's own
directory, i.e. inside the task's workdir. The module docstring (:10-12) explains the original intent: "Each
instance is a standalone Cargo crate... with its own `.cargo-target/` build directory, **never the roko
workspace's `target/`**... `visible_verify` sets `CARGO_TARGET_DIR` itself, so no change to the harness is
needed." That reasoning only solves *host contamination* (not colliding with roko's own `target/`); it does not
address where the build output ends up relative to the exported tree.

`commit_final` (`driver/archive.py:60`) calls `repo.export_tree(workdir, dest)` (`families/common/repo.py:114`),
which copies the working tree's files on disk wholesale with no gitignore-style filtering (its docstring: "Copy
`source`'s files on disk... into `dest`"). So `.cargo-target/` — full build artifacts: compiled binaries, rlibs,
incremental-compilation data, even for an offline, no-crates.io-dependency crate — lands in `c_i`, and therefore in
`archive_task`'s `.tar.gz` and `.diff` (`driver/archive.py`).

Note that the **hidden**-test path already gets this right: `hidden.py:160` runs `cargo test --test hidden` with
`CARGO_TARGET_DIR=str(scratch / "target")`, an out-of-tree absolute scratch path. Only the agent-facing visible
verify (`gen.py`'s `VISIBLE_VERIFY`/`README_TEMPLATE`) uses the in-tree relative form.

## Why it matters

F7 is one of the P1-core families (gap-46fd19's task 3324; "two families alone cannot carry H1's envelope or
LOG1"), so every real F7 instance in the pilot and any production run hits this. The bloated `c_i` inflates
`<stem>.tar.gz` and `<stem>.diff` with build noise unrelated to the agent's actual code change, which is exactly
the signal `archive_task`'s diff exists to show plainly (and whose `diff_sha256` other tooling may key on).

## Where

- `benchmarks/viabilitybench/families/f7_rustiter/gen.py`: `VISIBLE_VERIFY` (:71), `README_TEMPLATE` (:224), the
  module docstring (:10-12) stating the (incomplete) original reasoning.
- `benchmarks/viabilitybench/families/f7_rustiter/hidden.py`: the correct, out-of-tree pattern already in use for
  the audit path (:160), `scratch / "target"`.
- `benchmarks/viabilitybench/driver/archive.py::commit_final` (:60) and `families/common/repo.py::export_tree`
  (:114): where the in-tree directory gets swept up, with no filter that could exclude it even if asked.

## Current state

Unfixed. Confirmed no `CARGO_TARGET_DIR` is set anywhere out-of-tree for the agent's own visible-verify step;
`families/common/toolchain.py` (gap-46fd19's task 3324, done) sets `CARGO_HOME` (registry/cache) and
`RUSTUP_HOME` per run but never touches `CARGO_TARGET_DIR` for the visible path. This is not a correctness gap —
the tests still build and run correctly wherever `target/` lands — purely an archive-hygiene one.

## Plan

1. Change `VISIBLE_VERIFY` / `README_TEMPLATE` to point `CARGO_TARGET_DIR` out-of-tree, mirroring `hidden.py`'s
   pattern: an absolute path under the task's own scratch/home area (not inside the workdir `commit_final` will
   export), e.g. `CARGO_TARGET_DIR=$CARGO_TARGET_DIR_OVERRIDE` resolved by the harness to a per-task scratch dir,
   or simply pick one directory per task up front (in `agent_env.build`, alongside `CARGO_HOME`) and export it as
   an environment variable the README references instead of a literal relative path.
2. Alternatively (simpler, no harness plumbing): keep `.cargo-target` in-tree during the run, but have
   `commit_final` or `export_tree` skip a fixed, documented set of build-cache directory names (`.cargo-target`,
   `target`) the same way it presumably already must skip `.git`. Less clean (relies on a name convention rather
   than an explicit out-of-tree path) but needs no change to `gen.py`'s generated instructions.
3. Either way, add a regression test: run the full F7 visible-verify flow, then `commit_final`, and assert the
   resulting tree/tarball contains no `.cargo-target` (or `target`) directory.

## Done when

- A real F7 run's archived `c_i` (tarball and diff) contains no cargo build-cache directory.
- The `[[verify]]` command passes.

## Notes

- Not covered by gap-46fd19 (done): that package's task 3324 handled `CARGO_HOME`/`RUSTUP_HOME` plumbing and the
  *hidden*-test path's out-of-tree target dir, but left the agent-facing visible-verify's in-tree `.cargo-target`
  as originally designed, for a different, narrower reason (host-workspace contamination, not archive bloat).
- Do not change `hidden.py`'s existing out-of-tree pattern — it is already correct and can be reused.
