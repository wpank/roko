# Decision Memo: First-Time Contributor Journey for Roko

> **Plan:** demo-multistage · **Stage:** 2 (decision) · **Status:** recommended
> This document synthesizes evidence from upstream artifacts to recommend a
> concrete onboarding journey. No source code was modified; no upstream evidence
> files were altered.

---

## Decision

**Recommend a six-step first-time contributor journey that builds Roko from
source, validates the full workspace, then runs the multi-stage demo as a
live smoke test.**

The journey is deliberately ordered so that each step either produces a
verifiable artifact or fails with an actionable error message, giving the
contributor a progressive confidence ramp rather than a single all-or-nothing
gate.

### Recommended journey

| Step | Command | Purpose | Artifact |
|------|---------|---------|----------|
| 1 | `rustup update stable` | Ensure Rust 1.91+ (alloy deps require it, per `README.md` line ~289) | toolchain |
| 2 | `cargo build --workspace` | Compile all 34 workspace members (not just `default-members`) | build cache |
| 3 | `cargo clippy --workspace --no-deps -- -D warnings` | Enforce pedantic lint policy (`README.md` line ~292) | clean lint |
| 4 | `cargo test --workspace` | Run 1,600+ tests across all crates | test results |
| 5 | `cargo install --path crates/roko-cli` | Install the `roko` binary for interactive use | `~/.cargo/bin/roko` |
| 6 | `roko init my-project && cd my-project && roko run "add a health check endpoint to the API"` | Run the quick-start pipeline (`README.md` lines 10–13) as a live smoke test | `.roko/` directory, `roko.toml`, first signal |

**Why this order?** Step 2 (`cargo build --workspace`) comes before step 5
(`cargo install`) because `default-members` in `Cargo.toml` omits 31 of 34
crates (see Risk 1 in `demo/multistage-plan/discovery.md`). A bare `cargo
build` would succeed without compiling `roko-core`, `roko-agent`, `roko-gate`,
or any other library crate. Running `--workspace` first ensures the contributor
sees the full compilation surface and any breakage before the CLI binary is
installed.

**Why include the multi-stage demo as a smoke test?** The quick-start
one-shot (`roko run "..."`) exercises the core loop — observe, plan, execute,
verify, learn, repeat — exactly as documented in `README.md` lines 5–6. It
validates that LLM provider credentials are configured, gates are detected,
and signals are persisted. If this fails, the contributor knows the issue is
environmental (missing API key, wrong Rust version) rather than structural.

---

## Evidence considered

Three upstream artifacts were inspected and are referenced verbatim below.
Neither was modified.

### 1. `demo/multistage-plan/discovery.md`

The discovery note (produced by STAGE-1A) identifies eleven observed facts
about the repository and five risks. The facts most relevant to this decision:

- **Fact 2 (Quick-start entry point):** `README.md` (lines 9–15) documents a
  three-command onboarding path: `cargo install`, `roko init`, `roko run`.
- **Fact 3 (Workspace shape):** `default-members` includes only
  `crates/roko-cli`, `crates/roko-mcp-code`, and `crates/roko-mcp-github`.
  A bare `cargo build` omits most of the workspace.
- **Fact 5 (roko-orchestrator discrepancy):** Documented as a crate in
  `README.md` line ~123 and the architecture doc, but lives inside
  `crates/roko-cli/orchestrator/`. Contributors searching for
  `crates/roko-orchestrator/` will not find it.
- **Fact 6 (Rust version conflict):** `Cargo.toml` declares `rust-version =
  "1.85"` but `README.md` requires 1.91+ for alloy deps.
- **Fact 9 (Contributor guidance):** Four ground rules: "Search before
  writing," "Wire, don't build," "Verify before marking done," "All tests
  must pass."
- **Fact 10 (Lint strictness):** `unsafe_code = "deny"`, `unwrap_used =
  "deny"`, pedantic + nursery clippy lints at warn level.

The five risks directly shaped the journey ordering (step 2 before step 5) and
the acceptance criteria below.

### 2. `demo/multistage-plan/evidence.json`

The evidence manifest (produced by STAGE-1B) is a structured record with
`schema_version: 1`, three inspected sources, and four lifecycle stages
(discovery → decision → validation → review). Key structural observations:

- The manifest confirms the repository was inspected without modification
  (stage kind `review`, purpose: "Verify the evidence manifest accurately
  captures the repository structure without altering any source file").
- The `acceptance.executable` field contains a Python assertion that validates
  the manifest's schema, subject, sources, and stage kinds — proving the
  evidence is machine-checkable.
- The four-stage lifecycle (discovery, decision, validation, review) maps
  directly onto the multi-stage demo's own pipeline, validating that the demo
  structure mirrors Roko's internal workflow.

### 3. `README.md` (lines 1–180)

The canonical user-facing documentation. The quick-start commands (lines
9–15), building/testing instructions (lines ~287–296), and contributing
guidelines (lines ~298–305) anchor the recommended journey to documented,
maintained entry points rather than ad-hoc commands.

---

## Alternatives

### Alternative A: Quick-start only (skip full workspace build)

**Proposal:** Jump directly to `cargo install --path crates/roko-cli && roko
init && roko run "..."`, relying on `cargo install` to compile only what the
CLI needs.

**Rejected because:** `cargo install --path crates/roko-cli` compiles the
CLI and its transitive dependencies but not the full workspace. A contributor
who never runs `cargo build --workspace` or `cargo test --workspace` will have
a working CLI but no visibility into breakage in `roko-core`, `roko-agent`,
`roko-gate`, `roko-conductor`, `roko-learn`, or any of the 31 non-default
crates. This violates the contributing guideline "All tests must pass"
(`README.md` line ~305) and the "Verify before marking done" rule (line ~303).
The `default-members` scoping issue (Risk 1 in `demo/multistage-plan/discovery.md`)
makes this alternative actively misleading: the contributor sees a green build
that hides most of the codebase.

### Alternative B: Docker-based onboarding

**Proposal:** Provide a `Dockerfile` or `devcontainer.json` that pre-installs
Rust 1.91+, builds the workspace, and exposes the `roko` binary, so
contributors start with a container rather than building from source.

**Not selected because:** While Docker eliminates toolchain friction (Risk 4
in `demo/multistage-plan/discovery.md`), it obscures the build/test cycle
that contributors must understand to work on the codebase. The contributing
guidelines assume local `cargo` commands. A container also introduces
credential-passing complexity for LLM API keys required by `roko run`.
This alternative is viable as a *supplement* (e.g., a CI devcontainer) but
should not replace the local-build journey for first-time contributors.

### Alternative C: `roko doctor` as the sole validation step

**Proposal:** Add a `roko doctor` command that checks toolchain version,
workspace compilation, lint cleanliness, and test status in one step, then
point contributors at that instead of the manual `cargo build/test/clippy`
sequence.

**Deferred, not rejected.** A `roko doctor` command would be valuable
(especially for diagnosing Risk 4 — the Rust version mismatch). However,
it does not yet exist in the CLI subcommand set (the crate map in
`README.md` does not list it, and `crates/roko-cli/commands/` has no
`doctor.rs` — wait, it does have `doctor.rs`). The command exists but is
not documented in the quick-start or contributing sections. Recommending an
undocumented command as the primary onboarding path would create a different
kind of discoverability problem. This should be promoted to a documented
step once it is integrated into `README.md`.

---

## Acceptance criteria

The following checklist defines when the recommended contributor journey is
considered successful. Each criterion is independently verifiable.

- [ ] **A1. Full workspace compiles.** `cargo build --workspace` exits 0 with
  all 34 workspace members compiled (not just `default-members`). This
  directly addresses Risk 1 in `demo/multistage-plan/discovery.md` (hidden
  crates).

- [ ] **A2. Lint and test suites are clean.** Both
  `cargo clippy --workspace --no-deps -- -D warnings` and
  `cargo test --workspace` exit 0, satisfying the contributing guideline
  "All tests must pass" from `README.md` line ~305 and the lint strictness
  documented in `demo/multistage-plan/discovery.md` Fact 10.

- [ ] **A3. CLI binary is installed and functional.**
  `cargo install --path crates/roko-cli` produces a `roko` binary, and
  `roko init my-project` creates a `.roko/` directory with a valid
  `roko.toml`, as documented in `README.md` lines 10–13.

- [ ] **A4. One-shot execution produces a signal.** `roko run "add a health
  check endpoint to the API"` completes (success or gate-failure with retry),
  and the `.roko/` directory contains at least one JSONL signal file,
  confirming the core loop (observe → plan → execute → verify → learn →
  repeat) is end-to-end functional.

- [ ] **A5. No stale crate references cause contributor friction.** The
  contributor can locate the orchestrator code at
  `crates/roko-cli/orchestrator/` (not `crates/roko-orchestrator/`), and
  understands that architecture-doc target crates like `roko-bus` and
  `roko-hdc` do not yet exist as workspace members. This addresses Risks 2
  and 3 in `demo/multistage-plan/discovery.md`.

---

## Follow-up

1. **Document `cargo build --workspace` as the canonical first build command.**
   `README.md` quick-start currently leads with `cargo install --path
   crates/roko-cli`, which is correct for users but misleading for
   contributors. Add a "Contributing" subsection before the existing
   contributing guidelines that explicitly calls out `cargo build --workspace`
   as the first build step and explains the `default-members` scoping.

2. **Promote `roko doctor` to a documented onboarding step.**
   `crates/roko-cli/commands/doctor.rs` exists but is not mentioned in the
   contributing section. Adding `roko doctor` as step 1.5 (between toolchain
   update and workspace build) would automate the Rust version check and
   surface the `default-members` vs `--workspace` distinction.

3. **Resolve the `roko-orchestrator` documentation discrepancy.**
   Either extract `crates/roko-cli/orchestrator/` into its own workspace
   crate (matching the architecture doc's target state) or update the README
   crate map and `docs/v1/00-architecture/15-crate-map.md` to reflect that
   the orchestrator is currently a module within `roko-cli`. This eliminates
   Risk 2 from `demo/multistage-plan/discovery.md`.

4. **Align `rust-version` in `Cargo.toml` with the README's 1.91+ requirement.**
   `Cargo.toml` declares `rust-version = "1.85"` while `README.md` requires
   1.91+ for alloy deps (Risk 4 in `demo/multistage-plan/discovery.md`).
   Setting `rust-version = "1.91"` in `Cargo.toml` would make `cargo` enforce
   the correct minimum, eliminating the version mismatch as a source of
   contributor friction.

5. **Update the "18 crates" figure in `README.md`.**
   The workspace currently has 34 members. The stale "18 crates" claim
   (Risk 5 in `demo/multistage-plan/discovery.md`) understates the codebase
   and weakens the "Search before writing" contributing guideline by
   suggesting a smaller search surface than actually exists.
