# /prd-draft Is Broken, Poorly Connected, and Redundant

## What Happened

`/prd-draft costs` in Zed ACP:

```
📄 Creating PRD: costs
WARNING: Repository context not verified for keywords ["costs"].
Generated PRD may reference nonexistent code.
🏗 Agent working... (15s) [gpt54-mini via openai_compat]
🏗 Agent working... (30s) [gpt54-mini via openai_compat]
✓ Agent completed (35s, 22740 bytes) [gpt54-mini]
✓ Draft generated: costs
📄 Draft written to /Users/will/.roko/prd/drafts/costs.md
[ERROR] duplicate_crate: PRD proposes creating crate 'roko-learn' which already exists
Validation sidecar: costs.validation.json
Artifact validation: FAILED (1 errors, 0 warnings)
Timing: init=13ms prompt=3ms context=9215ms agent=35046ms post=5ms learn=60ms total=44345ms
```

Issues:
1. **Validation fails** — agent proposes creating crate that already exists
2. **"Repository context not verified"** — keyword extraction found "costs" but couldn't
   ground it against the codebase
3. **No tools dispatched** — `allowed_tools: Some("none")` (line 468 of prd.rs) means the
   agent has zero tools. It generates markdown from thin air based on the slug alone.
4. **No connection to ideas** — `/prd-draft costs` ignores the ideas list entirely. The
   slash command description says "Draft a new PRD from an idea" but the `title` parameter
   is just free text. It doesn't look up or reference the idea that was captured.
5. **Validation failure is non-blocking** — the draft is written even though validation fails.
   The error is printed but there's no retry or cleanup.

## Problems in Detail

### A. Agent has zero tools (`allowed_tools: Some("none")`)

`crates/roko-cli/src/commands/prd.rs:468`:
```rust
allowed_tools: Some("none"),
```

The agent generates the PRD purely from the system prompt + the task prompt. It cannot:
- Read the codebase to understand existing architecture
- Read the ideas file to find the captured idea
- Read existing PRDs to avoid duplication
- Search for existing implementations

The "Repository context" section (9.2s of context scanning) is injected into the prompt,
but it's keyword-based guessing — it finds files matching "costs" and dumps them. The agent
then hallucinates an architecture because it can't actually inspect the code.

### B. No connection between ideas and drafts

The flow is supposed to be: `idea → draft → plan → execute`. But:

- `/prd-idea "better cost analysis"` captures text to `.roko/prd/ideas.md`
- `/prd-draft costs` creates a new draft with slug "costs" — **it does not look up the idea**
- The slug is just slugified from whatever text you type after `/prd-draft`
- There's no link between the idea and the draft

The actual idea text ("better cost analysis, summaries, aggregations for agents") is never
fed to the drafting agent. The agent just sees the slug "costs" and the system prompt.

### C. Validation failure is toothless

The grounding check correctly detects that the agent proposed creating `roko-learn` (which
exists). But:
- The draft is still written to disk
- The validation report is a sidecar JSON file nobody reads
- There's no retry or correction loop
- The next command (`/prd-plan costs`) would generate a plan from the broken PRD

### D. The slug-based keyword extraction is broken

`extract_keywords_from_slug_and_description("costs", "costs")` produces `["costs"]`.
The repo context scanner then searches for files containing "costs" — but that's too
generic and matches everything with a cost field. The warning "not verified" means
even the keyword heuristic couldn't find a good match.

## Better Alternatives That Already Exist

### 1. `roko do` (complex path) — does idea+draft+plan+execute automatically

`do_cmd.rs:273-400` has a `run_complex_path` that does all four steps in sequence:
```
Step 1/4: Creating PRD...      (captures idea)
Step 2/4: Drafting PRD...      (generates draft)
Step 3/4: Generating plan...   (creates tasks.toml)
Step 4/4: Executing plan...    (runs agents per task)
```

This is already wired as `/do <prompt>` in the ACP. For complex prompts, the scope resolver
auto-classifies them as complex and runs this path. Users should use this instead of the
manual idea → draft → plan chain.

### 2. `roko develop` — plan-first with approval

`develop.rs` wraps `roko do --plan` with an interactive approval step. It:
- Forces plan generation before execution
- Shows the plan for approval (in TTY mode)
- Auto-launches the TUI dashboard

**Not yet wired as an ACP slash command.** Should be `/develop <prompt>`.

### 3. `roko plan generate` — skips PRD entirely

`/plan-generate <description>` generates a tasks.toml directly from a prompt.
No PRD, no draft, no ideas file. Just prompt → plan → execute.
For most use cases this is more direct and useful.

## Recommendations

### For the ACP slash commands:

1. **Add `/develop <prompt>`** — wire `roko develop "<prompt>" --yes` as an ACP slash command.
   This is the single command that does everything: scope → plan → execute.

2. **Fix or remove `/prd-draft`** — in its current state it produces hallucinated PRDs that
   fail validation. If kept:
   - Give the agent tools (`Read,Glob,Grep`) so it can actually inspect the codebase
   - Feed the idea text into the prompt (look up matching ideas from ideas.md)
   - Make validation failure block the draft (don't write a broken PRD)
   - Fix the tool alias bug (#15) so non-Claude models actually get tools

3. **Simplify the ACP PRD commands** — the current set is:
   ```
   /prd-idea    → captures text to ideas.md
   /prd-draft   → generates draft (broken, disconnected)
   /enhance-prd → research-enhances a draft
   /prd-plan    → generates plan from PRD
   /prd-list    → lists PRDs
   /prd-status  → coverage report
   ```

   This could be collapsed to:
   ```
   /develop <prompt>     → full pipeline (idea → draft → plan → execute)
   /plan <description>   → generate plan from description (skip PRD)
   /prd-list             → list existing PRDs + plans
   ```

4. **The `/prd-draft → /enhance-prd → /prd-plan` chain has 3 blocking bugs:**
   - #15: Tool alias mismatch (zero tools on non-Claude models)
   - No idea ↔ draft link (draft ignores captured ideas)
   - `allowed_tools: Some("none")` in draft (agent can't read codebase)

   All three must be fixed for the chain to be useful. Until then, `/do` and
   `/plan-generate` are the working alternatives.

## Files Involved

| File | Issue |
|------|-------|
| `crates/roko-cli/src/commands/prd.rs:324-560` | `PrdDraftCmd::New` — the full draft flow |
| `crates/roko-cli/src/commands/prd.rs:468` | `allowed_tools: Some("none")` — agent has no tools |
| `crates/roko-cli/src/commands/prd.rs:418-423` | Keyword verification warning |
| `crates/roko-cli/src/commands/prd.rs:549-560` | Validation that fails but doesn't block |
| `crates/roko-cli/src/commands/do_cmd.rs:273-400` | `run_complex_path` — the working alternative |
| `crates/roko-cli/src/commands/develop.rs` | `roko develop` — not wired to ACP |
| `crates/roko-acp/src/bridge_events.rs:3427-3436` | `/prd-draft` dispatch |
| `crates/roko-acp/src/session.rs:1542-1547` | `/prd-draft` slash command definition |

## Priority

**P2** — The feature is broken but has working alternatives (`/do`, `/plan-generate`).
The real fix is wiring `/develop` to the ACP and making it the primary workflow command.
The manual PRD pipeline (`/prd-idea → /prd-draft → /enhance-prd → /prd-plan`) is an
expert workflow that needs the tool alias fix (#15) and idea linking before it's usable.
