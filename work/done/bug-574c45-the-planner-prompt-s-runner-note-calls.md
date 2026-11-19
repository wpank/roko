+++
id = "bug-574c45"
kind = "bug"
title = "The planner prompt's runner note calls any workspace with a tests/ directory a Python project: a C++/CMake repo is told 'pytest is NOT installed here' and nothing about cmake or ctest"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-19
updated = 2026-10-19
last_verified = 2026-10-19
last_verified_rev = "f2ee9a0d3"
source = "round-v7-undertow-spec"
discovered_from = "agent:undertow-spec (round v7, 2026-10-19)"
anchors = ["crates/roko-cli/src/plan_generate/runnable.rs::runners_block"]
links = { depends_on = [], blocks = [], related = ["bug-d758f6", "gap-770cb5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn the_runner_note_names_cmake_not_python' crates/roko-cli/src && cargo test -p roko-cli --lib -- the_runner_note_names_cmake_not_python"

[closed]
at = 2026-10-19
at_ts = "2026-10-19T17:18:06Z"
commit = "35e8996f8"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged 19a072459 (work/bug-3cdc58: 35e8996f8). plan_generate/runnable.rs runners_block: a project is Python by pyproject.toml, requirements.txt, setup.cfg or setup.py, or a .py file within two levels of tests/, no longer by a tests/ directory alone; a CMakeLists.txt at the top or up to two levels down (not under build, target, third_party, vendor or node_modules) adds a CMake line saying whether cmake and ctest run here, with the preset forms (cmake --build --preset, ctest --preset -R) when CMakePresets.json exists. Verify at 19a072459: the_runner_note_names_cmake_not_python passes (a tests/spec/CMakeLists.txt with C++ spec tests gets the CMake line and no Python or pytest; a tests/*.py still gets the Python line); the_runner_note_says_pytest_is_missing passes with a Python test file. Branch: roko-cli lib 3875 passed, 0 failed; clippy and nightly fmt clean."
+++

## Problem

roko's planner prompt tells the model "Python: `pytest` is NOT installed here. Run Python tests with `python3 -m
unittest …`, never `pytest`" in `~/dev/undertow`, a C++/CMake/JUCE repo with no Python at all. Found by
undertow-spec (round v7, 2026-10-19; `tmp/hermes-roko-v7/findings/undertow-spec.md`).

`plan_generate/runnable.rs::runners_block` treats any workspace with a `tests/` directory as a Python project:

```rust
let python = workdir.join("tests").is_dir()
    || workdir.join("pyproject.toml").is_file()
    || workdir.join("setup.py").is_file();
```

Undertow's `tests/spec/` holds `CMakeLists.txt`, `p0/engine_test.cpp` and a C++ support library. The block says
nothing about CMake or ctest, which are what its checks run.

## Why it matters

The note is the planner's only statement of what this machine can run. In a C++ repo it is noise that may steer a
planner toward Python test runners, and it is silent on the runners the repo does use. hermes-pilot saw no harm in the
two phase-0 plans (neither mentions pytest), so this is p3.

## Where

`crates/roko-cli/src/plan_generate/runnable.rs::runners_block`, called from `pipeline.rs::generate_plan`.

## Plan

1. A workspace is a Python project when it has `pyproject.toml`, `setup.py`, `setup.cfg` or `requirements.txt`, or a
   `.py` file in `tests/` (not merely a `tests/` directory).
2. A workspace with `CMakeLists.txt` gets a CMake line: whether `cmake` and `ctest` are installed here, and, with a
   `CMakePresets.json`, that checks name a preset (`cmake --build --preset <name>`, `ctest --preset <name> -R <test>`).

## Done when

The test in `[[verify]]` passes: for a workspace with `CMakeLists.txt` and `tests/spec/p0/engine_test.cpp`, the block
names CMake and says nothing of Python or pytest; a workspace with `tests/test_x.py` still gets the Python line.

## Notes

- Related: bug-d758f6 (implementers without a shell), gap-770cb5 (no CMake build/check tool).
