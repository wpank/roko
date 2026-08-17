# Runner 4: `plan-grounding` — Granular Batch Specification

Date: 2026-04-28

Parent: [FULL-WORK-PLAN.md](./FULL-WORK-PLAN.md) Runner 4 section.

---

## Runner Goal (one sentence)

Make PRD and plan generation grounded in the intended repository, reject invalid artifacts
before execution or learning, and separate process success from artifact success.

## Context Pack Files

```text
tmp/runners/plan-grounding/
  README.md
  batches.toml
  context/
    00-RULES.md                     — universal + runner-specific anti-patterns
    ARCHITECTURE-CONTRACT.md        — single-owner map for this runner
    ANTI-PATTERNS.md                — forbidden patterns with repo examples
    ACCEPTANCE.md                   — proof commands including negative proofs
    FILE-OWNERSHIP.md               — batch → write path map
    ISSUE-MAP.md                    — batch → issue id map
    PRD-FLOW-AUDIT.md              — current PRD generation data flow (Group 0 output)
    PLAN-FLOW-AUDIT.md             — current plan generation/validation flow (Group 0 output)
    GROUNDING-CONTRACT.md          — RepoContextPack + ArtifactValidationReport spec
```

---

## Anti-Pattern Rules (00-RULES.md)

Include the universal rules from FULL-WORK-PLAN.md plus:

```markdown
# Plan-Grounding Anti-Patterns

PG-1. **Prompt-only grounding is not grounding.** Telling the model "search the codebase" is
      NOT a grounding mechanism. The grounding mechanism is VALIDATED OUTPUT: does the
      generated artifact name real files, avoid duplicates, and pass schema checks?

      EXISTING ANTI-PATTERN (do not repeat):
      - `crates/roko-cli/src/plan_generate.rs` says the plan generator must "search and read
        files." But no gate checks whether the output cites real files.
      - Three consecutive demo runs generated `roko-prompt` and `roko-orchestrate` crates
        that already exist under different names.

PG-2. **Process success ≠ artifact success.** A subprocess exiting 0 and writing a file
      does NOT mean the artifact is valid. Artifact validation is a separate gate.

      EXISTING ANTI-PATTERN (do not repeat):
      - `crates/roko-cli/src/prd.rs` emits `prd:plan:generated` signal because tasks.toml
        parsed. It does not check whether the plan is grounded.
      - Episodes are marked successful when the agent process exits cleanly, even if the
        plan proposes greenfield crates in an existing workspace.

PG-3. **No positive learning from failed artifacts.** Knowledge seeds, cascade router
      observations, and efficiency metrics should be WITHHELD when artifact validation fails.

      EXISTING ANTI-PATTERN (do not repeat):
      - `knowledge-seeds.jsonl` records "successful strategy" insights from demo runs that
        produced invalid plans (greenfield duplicates).
      - The cascade router gets positive observations from runs where the artifact was wrong.

PG-4. **Context pack is bounded.** The repo context pack must fit within ~8000 tokens.
      Do not dump the entire codebase. Include: workspace members, key files, symbol matches,
      related PRDs/plans, and explicit "do not create" warnings.

PG-5. **Context-root mismatch is an error.** If the user requests a Roko-internal feature
      from a workspace without Roko crates, that is an error or ambiguity — not a signal to
      generate a greenfield plan.

      EXISTING ANTI-PATTERN (do not repeat):
      - Demo ran `prd plan system-prompt-wiring` in `/tmp/roko-demo-*` which had no Rust
        source tree. The plan confidently described "no Rust crates exist yet" and proceeded.
```

---

## Group 0: Contract Guardrails

### Z01 — Audit PRD generation data flow

**Type:** Context-only (no code changes)

**Goal:** Map exactly how PRD drafts are generated, from prompt to persisted artifact.

**Write scope:**
- `tmp/runners/plan-grounding/context/PRD-FLOW-AUDIT.md`

**Read:**
- `crates/roko-cli/src/commands/prd.rs` (PrdCmd::Draft handler)
- `crates/roko-cli/src/prd.rs` (draft generation logic)
- `crates/roko-cli/src/prd_prompt.rs` (system prompt for PRD generation)
- `crates/roko-cli/src/agent_exec.rs` (how agent is invoked for PRD)
- `crates/roko-agent/src/claude_cli_agent.rs` (what the agent receives)

**Required output:**
- Exact prompt chain: what system prompt? what user prompt? what context?
- How much repo context is currently included? (Answer: almost none — ~365 chars of user prompt)
- Where is the acceptance gate? (Answer: "does markdown exist?")
- Where is the signal emitted?
- Where are learning/knowledge records created?
- Identify insertion points for: context pack injection, validation gate, learning gate

**DO NOT:** Change any source code.

---

### Z02 — Audit plan generation and validation flow

**Type:** Context-only (no code changes)

**Goal:** Map exactly how plans are generated, validated, and accepted.

**Write scope:**
- `tmp/runners/plan-grounding/context/PLAN-FLOW-AUDIT.md`

**Read:**
- `crates/roko-cli/src/commands/prd.rs` (PrdCmd::Plan handler)
- `crates/roko-cli/src/plan_generate.rs` (plan generator prompt + logic)
- `crates/roko-cli/src/prd.rs` (plan validation logic, signal emission)
- `crates/roko-cli/src/commands/plan.rs` (PlanCmd::Validate, PlanCmd::Regenerate)

**Required output:**
- What validation checks currently exist? (schema, modern fields, task count)
- What is NOT validated? (file existence, crate duplication, model aliases, role field)
- How does `plan regenerate` get its context? Does it see validation errors?
- Where does `prd plan` write the output?
- What learning records are created on plan generation?

**DO NOT:** Change any source code.

---

### Z03 — Define grounding contract

**Type:** Context-only (no code changes)

**Goal:** Write the precise RepoContextPack and ArtifactValidationReport specs.

**Write scope:**
- `tmp/runners/plan-grounding/context/GROUNDING-CONTRACT.md`

**Read:**
- Z01 and Z02 outputs
- FULL-WORK-PLAN.md Runner 4 section
- DEMO-RUN-AUDIT.md (F0-F9 findings)
- REVISED-BEST-SOLUTION-AFTER-DEMO.md

**Required output:**
- Exact `RepoContextPack` struct with field types and size budget
- Exact `ArtifactValidationReport` struct with field types
- Validation rules enumerated (file exists, crate not duplicate, no banned phrases, etc.)
- Where these structs should live (propose crate + file)
- How they integrate into the existing PRD/plan flow (injection points from Z01/Z02)

**DO NOT:** Change any source code.

---

## Group A: Repo Context Pack

### A01 — Define `RepoContextPack` struct

**Goal:** One struct carries bounded repository context for generation prompts.

**Write scope:**
- `crates/roko-cli/src/repo_context.rs` (NEW FILE)
- `crates/roko-cli/src/lib.rs` (module declaration)

**Required behavior:**
```rust
pub struct RepoContextPack {
    pub root: PathBuf,
    pub project_kind: ProjectKind,          // Rust, TypeScript, Go, Mixed, Unknown
    pub workspace_members: Vec<String>,     // from Cargo.toml, package.json workspaces, etc.
    pub key_files: Vec<PathBuf>,            // up to 20 important files
    pub matching_symbols: Vec<SymbolHit>,   // up to 30 symbol matches
    pub related_prds: Vec<PathBuf>,         // existing PRDs in this area
    pub related_plans: Vec<PathBuf>,        // existing plans in this area
    pub do_not_create: Vec<String>,         // crates/packages that already exist
    pub context_root_verified: bool,        // true if root matches intended target
}

pub struct SymbolHit {
    pub file: PathBuf,
    pub line: u32,
    pub text: String,
}

pub enum ProjectKind {
    Rust,
    TypeScript,
    Go,
    Python,
    Mixed,
    Unknown,
}
```
- Include `fn to_prompt_section(&self) -> String` that formats as bounded markdown
- Prompt section MUST fit within 8000 tokens (~32000 chars)
- If repo has more than 50 workspace members: truncate with `... and N more`

**DO NOT:**
- Make this depend on the code-intelligence index (optional enhancement later)
- Include file contents — only paths and symbols
- Make the struct generic over multiple project types simultaneously

**Verify:** `cargo check -p roko-cli`

**Evidence:** DEMO-RUN-AUDIT F0, F1, F2, REVISED-BEST-SOLUTION-AFTER-DEMO.md

---

### A02 — Collect workspace members and project kind

**Goal:** Context pack knows what crates/packages exist.

**Write scope:**
- `crates/roko-cli/src/repo_context.rs`

**Required behavior:**
- For Rust: parse `Cargo.toml` `[workspace] members` array
- For TypeScript: parse `package.json` `workspaces` field
- For Go: parse `go.work` or list go.mod files
- For Mixed/Unknown: check for all of the above
- `project_kind` is determined by which config files exist at root
- If `Cargo.toml` workspace has 18 members: list all 18 as `do_not_create` candidates

**DO NOT:**
- Parse deeply — just read the workspace member list
- Require full compilation or dependency resolution
- Support monorepo tools (nx, lerna, etc.) — just the native workspace files

**Verify:** `cargo check -p roko-cli`

---

### A03 — Collect key files and symbol matches

**Goal:** Context pack names files relevant to the requested feature.

**Write scope:**
- `crates/roko-cli/src/repo_context.rs`

**Required behavior:**
- Accept a `feature_keywords: &[&str]` parameter
- Use bounded `rg` (ripgrep) or in-process glob to find:
  - Files whose names contain keywords (max 20)
  - Lines containing keywords with surrounding context (max 30 hits)
- Exclude: `target/`, `node_modules/`, `.git/`, `tmp/`
- Sort by relevance (exact filename match > content match)
- Each `SymbolHit` includes file, line number, and the matching line text

**DO NOT:**
- Shell out to `rg` if a Rust grep library is available in deps (check first)
- If shelling out: use `--max-count 30 --max-columns 200`
- Include binary files
- Spend more than 5 seconds on this step (timeout and truncate)

**Verify:** `cargo check -p roko-cli`

---

### A04 — Include related PRDs and plans

**Goal:** Context pack references existing PRDs/plans in the same area.

**Write scope:**
- `crates/roko-cli/src/repo_context.rs`

**Required behavior:**
- Scan `.roko/prd/drafts/` for PRDs with matching keywords in filename or first 500 chars
- Scan `.roko/plans/` for plans with matching keywords
- Include up to 5 related PRDs and 5 related plans
- Include only paths, not full contents

**DO NOT:**
- Read full PRD/plan contents into the context pack
- Require the neuro knowledge store
- Fail if .roko/ doesn't exist (return empty lists)

**Verify:** `cargo check -p roko-cli`

---

### A05 — Detect context-root mismatch

**Goal:** If running from a temp workspace while requesting repo-internal features, flag it.

**Write scope:**
- `crates/roko-cli/src/repo_context.rs`

**Required behavior:**
- `context_root_verified = true` when:
  - Feature keywords match workspace member names OR key file paths
  - AND the workspace has real source files (not just .roko/ metadata)
- `context_root_verified = false` when:
  - Feature mentions crate names that don't exist in the workspace
  - OR the workspace has no source files at all
  - OR the workspace is clearly a temp directory with only metadata
- When false: `to_prompt_section()` includes a WARNING:
  `"⚠️ Context root may not match intended repository. Feature keywords reference crates
  not found in this workspace."`

**DO NOT:**
- Make this an error that blocks generation (it's a warning for now)
- Require user confirmation (that's a UX decision for Runner 7)
- Hard-code Roko-specific crate names — use the workspace member list

**Verify:** `cargo check -p roko-cli`

**Evidence:** DEMO-RUN-AUDIT F0

---

### A06 — Build context pack for PRD/plan generation

**Goal:** A single `build_repo_context(workdir, feature_keywords)` entry point.

**Write scope:**
- `crates/roko-cli/src/repo_context.rs`

**Required behavior:**
```rust
pub async fn build_repo_context(
    workdir: &Path,
    feature_keywords: &[&str],
) -> Result<RepoContextPack> {
    // 1. Detect project kind
    // 2. Collect workspace members
    // 3. Collect key files and symbol matches
    // 4. Find related PRDs/plans
    // 5. Build do_not_create list from workspace members
    // 6. Verify context root
    // 7. Return bounded pack
}
```
- Total time budget: 10 seconds max (timeout partial results)
- If workdir doesn't exist or has no project files: return a pack with `Unknown` kind and
  `context_root_verified = false`

**DO NOT:**
- Make this blocking (use async for file I/O if possible)
- Require any external services
- Return errors for missing files — return an incomplete pack with warnings

**Verify:** `cargo check -p roko-cli`

---

## Group B: PRD Grounding

### B01 — Inject context pack into `prd draft new`

**Goal:** PRD generation receives repo context before the model runs.

**Write scope:**
- `crates/roko-cli/src/commands/prd.rs` (PrdCmd::Draft handler)
- `crates/roko-cli/src/prd.rs` (draft generation)

**Read:**
- `tmp/runners/plan-grounding/context/PRD-FLOW-AUDIT.md`
- `crates/roko-cli/src/repo_context.rs`

**Required behavior:**
- Before calling agent_exec for draft: `build_repo_context(workdir, &feature_keywords)`
- Append `context_pack.to_prompt_section()` to the user prompt or system prompt
- The model now receives: workspace members, key files, symbol matches, related PRDs
- If `context_root_verified == false`: include the warning in the prompt

**DO NOT:**
- Change the system prompt in `prd_prompt.rs` (too invasive for this batch)
- Remove existing PRD prompt content
- Make context pack mandatory (generate without it if build fails, but log the failure)

**Verify:** `cargo check -p roko-cli`

**Evidence:** DEMO-RUN-AUDIT F2

---

### B02 — Require Repository Grounding section in PRD output

**Goal:** Generated PRDs must include a machine-checkable grounding section.

**Write scope:**
- `crates/roko-cli/src/prd_prompt.rs` (add requirement to PRD prompt)
- `crates/roko-cli/src/prd.rs` (add validation check)

**Required behavior:**
- Add to PRD system prompt: requirement for `## Repository Grounding` section containing:
  - Existing crates/packages to modify
  - Existing source files referenced
  - New crates needed (should be "none" for most existing-repo features)
  - Explicit non-goals
- After PRD generation: check that `## Repository Grounding` heading exists
- If missing: warn (not error for now) in the output
- The section content is NOT deeply validated yet (that's B03)

**DO NOT:**
- Require the section to be perfect — just require it to exist
- Block PRD generation on section absence (warn, don't fail)
- Remove other PRD prompt requirements (citations, diagrams, etc.)

**Verify:** `cargo check -p roko-cli`

---

### B03 — Validate PRD references existing surfaces

**Goal:** PRD grounding section is checked against real workspace state.

**Write scope:**
- `crates/roko-cli/src/prd.rs` (add validation after generation)

**Required behavior:**
- Parse the `## Repository Grounding` section
- Check "existing crates to modify" against workspace members
- If PRD claims "no existing crates" but workspace has members: flag as WARNING
- If PRD claims "new crate: roko-X" but `roko-X` already exists: flag as ERROR
- Store validation result as sidecar: `.roko/prd/drafts/<slug>.validation.json`
- Print validation summary after generation

**DO NOT:**
- Block PRD generation on warnings (only on errors)
- Parse the full PRD markdown beyond the grounding section
- Require perfect file path matching (crate-level is sufficient)

**Verify:** `cargo check -p roko-cli`

---

### B04 — Persist context pack and validation report sidecars

**Goal:** Every PRD generation has auditable context and validation records.

**Write scope:**
- `crates/roko-cli/src/prd.rs`

**Required behavior:**
- After PRD generation: write `.roko/prd/drafts/<slug>.context.json` (serialized RepoContextPack)
- After validation: write `.roko/prd/drafts/<slug>.validation.json` (serialized validation result)
- Sidecars are informational — they don't block anything
- Include timestamp and feature keywords in both files

**DO NOT:**
- Make sidecars required for PRD to be considered complete
- Store full context pack content in the PRD itself
- Change the PRD file format

**Verify:** `cargo check -p roko-cli`

---

## Group C: Plan Validation and Repair

### C01 — Require `role` field in generated tasks

**Goal:** Plans without role fields are rejected before execution.

**Write scope:**
- `crates/roko-cli/src/plan_generate.rs` (add role requirement to prompt)
- `crates/roko-cli/src/prd.rs` OR plan validation logic (add role check)

**Required behavior:**
- Plan generator prompt explicitly requires `role = "implementer"` (or appropriate role) per task
- Plan validation rejects tasks without `role` field
- Validation error: `"task '<title>' missing required field: role"`
- This check runs in both `plan validate` and the pre-execution gate (Runner 2 D03)

**DO NOT:**
- Add a default role automatically (that hides the generation failure)
- Accept any string as role — validate against known roles if a role registry exists
- Change the TOML schema definition

**Verify:** `cargo check -p roko-cli`

**Evidence:** COMPREHENSIVE-ISSUES 7.1, E2E-DOGFOOD-AUDIT Path 5

---

### C02 — Normalize model aliases before execution

**Goal:** Plans using `sonnet` or `haiku` don't fail on provider resolution.

**Write scope:**
- `crates/roko-cli/src/plan_generate.rs` (teach generator to use full model names)
- `crates/roko-cli/src/prd.rs` OR plan validation (add alias check + normalization)

**Required behavior:**
- Plan generator prompt: "Use configured model names from the project config. Example:
  `claude-sonnet-4-6`, NOT shorthand like `sonnet` or `haiku`."
- Plan validation: if `model_hint` contains a known alias, either:
  - Normalize it (map `sonnet` → `claude-sonnet-4-6` from config) and warn, OR
  - Reject with: `"task '<title>' uses model alias 'sonnet'. Use full name: claude-sonnet-4-6"`
- Include the configured model names in the plan generator's context

**DO NOT:**
- Create a global alias registry — just handle the common cases
- Block plan execution if normalization succeeded (warn only)
- Change the EffectiveModelSelection module (Runner 2 handles runtime selection)

**Verify:** `cargo check -p roko-cli`

**Evidence:** COMPREHENSIVE-ISSUES 7.2, E2E-DOGFOOD-AUDIT Path 5

---

### C03 — Validate referenced files against repo context

**Goal:** Plans referencing non-existent files are flagged.

**Write scope:**
- `crates/roko-cli/src/prd.rs` OR new `crates/roko-cli/src/plan_validation.rs`

**Required behavior:**
- For each task's `files` or target paths: check if file exists on disk
- If file doesn't exist AND task doesn't say "create": flag as WARNING
- For new crate creation: check against workspace `do_not_create` list
- Validation output distinguishes:
  - ERROR: crate already exists (would create duplicate)
  - WARNING: referenced file not found (might be a typo)
  - INFO: new file creation task (acceptable)

**DO NOT:**
- Require every file path to exist (some tasks legitimately create new files)
- Deep-parse task content for file paths (just use explicit `files` fields if present)
- Block execution on WARNINGs (only ERRORs block)

**Verify:** `cargo check -p roko-cli`

**Evidence:** DEMO-RUN-AUDIT F3, F4

---

### C04 — Reject greenfield duplicates

**Goal:** Plans cannot create crates that already exist in the workspace.

**Write scope:**
- `crates/roko-cli/src/prd.rs` OR `crates/roko-cli/src/plan_validation.rs`

**Required behavior:**
- Scan task titles and descriptions for "create crate" / "new crate" / "scaffold" patterns
- Extract proposed crate names
- Check against `workspace_members` from Cargo.toml
- If a task proposes creating `roko-X` and `roko-X` already exists: ERROR
- If a task proposes creating any new crate without `allow_new_crates: true` in PRD frontmatter
  or plan metadata: ERROR
- Banned phrase check in normal repo mode:
  - "greenfield" (when workspace has existing crates)
  - "no Rust crates or source files exist yet" (when they do)
  - "stub Grimoire" / "stub Daimon" (these already exist)

**DO NOT:**
- Ban the word "greenfield" in a genuinely empty workspace
- Require exact string matching — use case-insensitive contains
- Block on this check in blank-project mode (only in existing-workspace mode)

**Verify:** `cargo check -p roko-cli`

**Evidence:** DEMO-RUN-AUDIT F4, all three demo runs

---

### C05 — Feed validation errors into `plan regenerate`

**Goal:** Regeneration gets actionable feedback, not a blank slate.

**Write scope:**
- `crates/roko-cli/src/commands/plan.rs` (PlanCmd::Regenerate handler)
- `crates/roko-cli/src/plan_generate.rs` (regeneration prompt construction)

**Required behavior:**
- Before regeneration: run full validation on the existing plan
- Include validation errors in the regeneration prompt:
  ```
  The previous plan had these validation errors:
  - ERROR: task 3 'Create roko-prompt crate' duplicates existing crate 'roko-compose'
  - ERROR: task 7 missing required field: role
  - WARNING: task 12 references non-existent file: src/orchestrate/mod.rs
  Fix these errors in the regenerated plan.
  ```
- Also include the `RepoContextPack` (same as initial generation)
- After regeneration: validate again and report improvement

**DO NOT:**
- Regenerate without context (the whole point is informed regeneration)
- Remove existing regeneration logic — augment it
- Loop regeneration automatically (one shot; user can re-run if still bad)

**Verify:** `cargo check -p roko-cli`

**Evidence:** COMPREHENSIVE-ISSUES 7.4, E2E-DOGFOOD-AUDIT Path 5

---

### C06 — Separate artifact validation from process status

**Goal:** Episodes and learning distinguish process success from artifact quality.

**Write scope:**
- `crates/roko-cli/src/prd.rs` (where episodes/signals are created)
- `crates/roko-learn/src/runtime_feedback.rs` (if learning gates on this)

**Required behavior:**
- After PRD/plan generation: record TWO outcomes:
  - `process_success: bool` — did the subprocess exit cleanly?
  - `artifact_valid: bool` — did the artifact pass validation?
- Episode metadata includes both fields
- Signal emission: `prd:plan:generated` only when BOTH are true
- If process succeeded but artifact failed:
  - Mark episode as `partial_success` or `artifact_invalid`
  - DO NOT emit positive signals
  - DO NOT feed positive observations to cascade router
  - DO NOT emit knowledge seeds

**DO NOT:**
- Remove process success tracking
- Make artifact validation blocking for all commands (only for PRD/plan generation)
- Change episode schema broadly — add fields with `#[serde(default)]`

**Verify:** `cargo check -p roko-cli -p roko-learn`

**Evidence:** DEMO-RUN-AUDIT F7, F8, PG-2 and PG-3 anti-patterns

---

## Group D: Learning Gate

### D01 — Withhold knowledge seeds on artifact failure

**Goal:** Bad plans don't pollute the knowledge store.

**Write scope:**
- `crates/roko-cli/src/prd.rs` OR `crates/roko-cli/src/orchestrate.rs` (where seeds are emitted)
- `crates/roko-learn/src/runtime_feedback.rs` (if it gates seed emission)

**Required behavior:**
- Before emitting knowledge seeds: check `artifact_valid` from C06
- If `artifact_valid == false`: skip seed emission entirely
- Log: `"Skipping knowledge seeds: artifact validation failed"`
- Cascade router: skip positive observation when artifact invalid

**DO NOT:**
- Delete existing knowledge seed logic
- Add negative seeds for failures (just skip, don't anti-learn)
- Change the knowledge store format

**Verify:** `cargo check -p roko-cli -p roko-learn`

---

### D02 — Withhold cascade router rewards on artifact failure

**Goal:** The model router doesn't learn "this model worked" when the artifact was bad.

**Write scope:**
- `crates/roko-cli/src/orchestrate.rs` (where router observations are fed)
- OR `crates/roko-learn/src/cascade_router.rs` (if it has a gate)

**Required behavior:**
- After agent dispatch: only call `cascade_router.observe(model, true)` when:
  - Process succeeded AND
  - If the task is artifact-generation: artifact validation also passed
- For non-artifact tasks (pure code tasks with real gate results): gate pass is sufficient
- For artifact tasks: require `artifact_valid == true`

**DO NOT:**
- Feed negative observations for skipped validation
- Change the cascade router API
- Remove observation logic for non-artifact tasks

**Verify:** `cargo check -p roko-cli -p roko-learn`

---

## Group E: Proof

### E01 — Unit tests for repo context pack

**Write scope:**
- `crates/roko-cli/src/repo_context.rs` (test module)
- OR `crates/roko-cli/tests/repo_context.rs`

**Required behavior (tests):**
- Rust workspace with 5 crates: all 5 appear in `workspace_members` and `do_not_create`
- Feature keywords "prompt" in Roko repo: `roko-compose` appears in key_files/symbols
- Empty temp directory: `project_kind = Unknown`, `context_root_verified = false`
- `to_prompt_section()` output is under 32000 chars even for large repos

---

### E02 — Unit tests for plan validation

**Write scope:**
- `crates/roko-cli/tests/plan_validation.rs` OR test module

**Required behavior (tests):**
- Plan with missing role → validation error
- Plan with alias `sonnet` → normalized or error
- Plan creating `roko-compose` in Roko workspace → ERROR (duplicate crate)
- Plan creating `my-new-lib` in empty workspace → OK
- Plan referencing `src/nonexistent.rs` → WARNING
- Plan with "greenfield" in existing workspace → ERROR

---

### E03 — Integration proof: grounded PRD generation

**Write scope:**
- `tmp/runners/plan-grounding/context/PROOF-GROUNDING.md` (manual proof script)

**Required proof steps:**
1. In Roko workspace: `roko prd draft new system-prompt-wiring`
   → PRD includes `## Repository Grounding` naming `roko-compose`, `roko-agent`
2. In Roko workspace: plan generation → does NOT create `roko-prompt` or `roko-orchestrate`
3. In empty temp dir: `roko prd draft new add-config-parser`
   → context pack warns about unverified context root
4. Plan with missing roles: `plan validate` → actionable errors
5. Plan with `sonnet` alias: `plan validate` → normalization warning
6. After invalid plan: `learn all` does NOT show positive observations
7. After `plan regenerate`: validation errors fed into prompt, output improves

---

## Batch Summary

| Group | Batches | Main scope |
|---|---:|---|
| 0: Contracts | 3 | PRD flow audit, plan flow audit, grounding contract |
| A: Repo Context Pack | 6 | struct, workspace, symbols, PRDs, context-root, builder |
| B: PRD Grounding | 4 | inject context, require section, validate, persist |
| C: Plan Validation | 6 | role field, aliases, file refs, greenfield, regen, outcomes |
| D: Learning Gate | 2 | withhold seeds, withhold router rewards |
| E: Proof | 3 | unit tests, validation tests, integration proof |
| **Total** | **24** | |

## Suggested Execution Waves

Wave 1: Z01, Z02, Z03 (context-only, parallel)
Wave 2: A01, A02 (struct + workspace members)
Wave 3: A03, A04, A05 (symbols, PRDs, context-root — parallel)
Wave 4: A06 (builder entry point)
Wave 5: B01, B02 (inject into PRD, require section)
Wave 6: B03, B04 (validate PRD, persist sidecars)
Wave 7: C01, C02, C03 (role field, aliases, file refs — parallel)
Wave 8: C04, C05, C06 (greenfield, regenerate, outcomes)
Wave 9: D01, D02 (learning gates — parallel)
Wave 10: E01, E02, E03 (tests and proof)

## Acceptance Criteria

This runner is done when:

**Positive proofs:**
- `roko prd draft new <feature>` in Roko workspace → PRD references existing crates
- Generated PRDs include `## Repository Grounding` section
- `plan validate` catches missing roles, bad aliases, duplicate crates
- `plan regenerate` receives validation errors and produces better output
- Context pack `.context.json` sidecars exist after generation
- Validation `.validation.json` sidecars exist after generation

**Negative proofs:**
- Plan creating `roko-prompt` in Roko workspace → REJECTED (duplicate crate)
- Plan with "greenfield" in existing workspace → REJECTED
- Plan with missing `role` → validation error, cannot execute
- Failed artifact validation → NO knowledge seeds emitted
- Failed artifact validation → NO positive cascade router observations
- Temp workspace PRD generation → context-root warning in output
- Process success with artifact failure → episode NOT marked fully successful
