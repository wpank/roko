# Verification policy

## Principle

Verification should be risk-weighted, impact-selected, and owned by one layer. Broad tests are
valuable release evidence, but they are poor default feedback when compilation dominates and the
changed behavior can be proved directly.

## T0–T3 matrix

| Tier | Examples | Interactive evidence | Pre-merge evidence |
|---|---|---|---|
| T0 | Docs, comments, trivial enum/string/config wiring | Diff check, format/parse, structural assertion, actual command if applicable | Async fast CI |
| T1 | Local pure Rust logic, isolated parser/renderer | T0 + one target-aware cargo check + one exact existing/new unit filter | Impacted package check/test |
| T2 | Cross-crate CLI, API, process, UI behavior | T1 + impacted tests + one real CLI/API/TUI/browser smoke and run bundle | Reverse dependents + focused CI |
| T3 | Safety, auth, persistence, migration, scheduler/concurrency, payments | Focused invariant/regression test + real integration scenario | Full blocking risk suite and review |

Uncertainty upgrades the tier. T3 is not guaranteed to complete in five minutes.

## One semantic gate

The target design builds a semantic set of required evidence:

- format
- parse/structure
- compile target
- test target/filter
- runtime surface

It then chooses one command per requirement. The expanded integration maps the actual Git diff
through Cargo metadata to exact lib/bin/test/example/bench targets and required features, widens
shared modules, and compiles bounded transitive reverse dependents for likely public contracts.
Broad commands, optional commands, deliberate repeats, and arbitrary shell wrappers are retained.
Equivalence hidden behind arbitrary wrappers remains deferred so the runner cannot accidentally
erase authored verification intent.

Examples:

- crates/roko-cli/src/main.rs maps to cargo check -p roko-cli --bin roko.
- A roko-cli library module maps to cargo check -p roko-cli --lib.
- crates/roko-cli/tests/foo.rs maps to cargo check/test --test foo.
- A changed API route maps to the impacted crate plus one exact HTTP request.
- A TUI rendering change maps to a text snapshot/screenshot, not a workspace test.

The current canonical --lib gate for a main.rs edit is both slow and insufficient.

## Suggested lanes

### Edit lane

Target: under ten seconds.

- git diff --check
- cargo fmt --all --check, or format only selected Rust files through a supported formatter path
- exact grep/parser/schema assertions
- no compilation owned by the coding agent

### Task lane

Target: warm 15–90 seconds.

- exactly one cargo check for the changed target
- only tests added/changed or one same-module filter
- one direct behavior probe when runtime output changed
- fail fast

Examples:

    cargo check -p roko-cli --bin roko
    cargo nextest run -p roko-learn --lib -E 'test(/reflex_store::/)'
    cargo nextest run -p roko-cli --test reflexes_cli

Nextest supports filters, fail-fast behavior, retries, timing output, and JUnit reports:
[nextest running](https://nexte.st/docs/running/) and
[nextest configuration](https://nexte.st/docs/configuration/).

### Pre-push lane

Target: under five minutes when warm, asynchronous if longer.

- impacted package and direct reverse-dependent cargo checks
- focused nextest suites
- impacted-package clippy once
- one real CLI/API/UI smoke
- evidence-bundle validation

### Nightly/release lane

- workspace/all-feature compilation
- workspace clippy/tests
- property/fuzz tests
- live chain/provider/Ollama/browser suites
- self-host/resume/crash/recovery matrices
- release profile and packaging

## Test inventory redesign

Runtime ignore flags do not prevent Rust from compiling/linking large test harnesses. Separate
slow tests structurally:

- Cargo required-features for live/provider/chain tests.
- A slow-tests feature for expensive property/scenario modules.
- Separate E2E packages where appropriate.
- Nextest profiles/tags for fast, integration, live, flaky, and release lanes.
- Explicit owner, reason, and expiry for every quarantine.

Do not permanently delete tests merely because the current harness cannot observe them. First
classify whether each test protects a real invariant, duplicates stronger runtime evidence, or has
no demonstrated value.

## Impact selection

Selection should use:

1. Changed files and Cargo targets.
2. cargo metadata dependency and reverse-dependency graph.
3. Changed public symbols, re-exports, trait implementations, and serialized schemas.
4. Existing test modules and integration targets referencing those symbols.
5. Risk policy.

Filename-only selection is insufficient for public type changes. The existing
tmp/backlog/231-cross-crate-change-impact-scoping.md captures this need.

## Merge rules

Never auto-merge when any of these is true:

- Compile error.
- Missing or multiple terminal events.
- More than one dispatch for one attempt without an explicit retry ID.
- Malformed or incomplete JSONL evidence.
- Safety/settlement violation.
- Secret detected in the bundle.
- Unreviewed schema/migration.
- Runtime/UI change with no actual-path evidence.
- Changed file outside the authorized scope.

T0/T1 auto-merge can be considered only after the scorecard proves stable reliability. T2/T3
should initially require human or release-lane approval.

## Implemented FAST controls

`./dev.sh fast` now sets the controlled FAST environment, including:

    ROKO_TASK_VERIFY_ONLY=1
    ROKO_SKIP_PREFLIGHT=1
    SKIP_FRONTEND_BUILD=1

The wrapper also selects headless execution, zero retries, one task at a time, patch-only agent
instructions, no startup warm/critical cleanup, focused gate breadth, one compiler owner, and an
existing debug Roko binary. It requires exactly one authored task verify and remains opt-in; these
settings are not production defaults. Normal mode still fails closed to full gate breadth.

## Regression accounting

The speed scorecard must record escaped regressions by tier. A lane is not “faster” if it simply
discovers failures later. Promotion criteria:

- At least 20 representative runs.
- Cold and warm measurements.
- 100% evidence completeness.
- No increase in escaped regressions relative to the current lane.
- Broad CI remains green at or above baseline.
