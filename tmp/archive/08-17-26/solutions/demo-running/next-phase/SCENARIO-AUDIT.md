# Demo Scenario Audit

**Date**: 2026-05-04
**Purpose**: Full inventory of all 14 demo scenarios — what they do, how they fail, and why they're unsuitable for live demo.

---

## Current Scenario Inventory

### 14 Scenarios, 5 Categories

| # | ID | Title | Category | Panes | Commands | Duration Claim | Actual Duration |
|---|---|---|---|---|---|---|---|
| 1 | prd-pipeline | PRD Pipeline | pipeline | 1 | 8 | 2-5 min | 5-30 min |
| 2 | prd-research-loop | Research Loop | pipeline | 1 | 9 | ~90s | 10-30 min |
| 3 | race | Cost Race | comparison | 2 | 3 | ~60s | 5-15 min |
| 4 | gate-retry | Gate Retry | pipeline | 2 | 6 | ~75s | 5-15 min |
| 5 | providers | Providers | comparison | 4 | 4 | ~45s | 3-10 min |
| 6 | provider-race | Provider Race | comparison | 4 | 5 | ~60s | 5-15 min |
| 7 | explore | Explore | exploration | 4 | 12 | ~120s | 2-5 min |
| 8 | knowledge-accumulation | Knowledge Growth | learning | 2 | 10 | ~90s | 10-20 min |
| 9 | knowledge-transfer | Knowledge Transfer | learning | 2 | 6 | ~90s | 10-30 min |
| 10 | dream-consolidation | Dream Cycle | learning | 2 | 6 | ~60s | 5-15 min |
| 11 | chat | Chat | exploration | 1 | 4 | ~30s | 2-5 min |
| 12 | chain-intelligence | Chain Intelligence | chain | 2 | 12 | ~120s | 10-30 min |
| 13 | mirage | Mirage | chain | 1 | 4 | ~30s | 1-2 min |
| 14 | isfr-agents | ISFR Agents | chain | 8 | 9 | ~120s | 20-60 min |

---

## Systemic Problems (Every Scenario)

### 1. Speed: Everything Takes Too Long
Every scenario that calls `roko run` or `roko prd draft/plan` invokes a real LLM agent round-trip. These take 30s-5min each. Scenarios with multiple LLM calls chain 3-5 of these, creating 5-30 minute wait times. This is **completely unworkable for live demo**.

Duration hints in the scenario metadata are wildly optimistic. "~60s" scenarios routinely take 10+ minutes.

### 2. Fragility: Too Many Failure Modes
- **Slug matching**: PRD scenarios hardcode slugs (`btc-funding-alert-cli`). If the LLM generates a different slug, all downstream commands fail.
- **Provider availability**: Provider/race scenarios require 3-4 API keys active. One missing key = one failed pane.
- **External deps**: Chain scenarios need mirage-rs + foundry + specific fork state. Any missing piece = total failure.
- **Workspace conflicts**: 6/14 scenarios run multiple `roko run` in the same workspace directory, causing file write conflicts.
- **Module-level state**: gate-retry, provider-race, knowledge-transfer use singleton state that persists across resets.

### 3. Friction: Too Many Clicks
The "ClickableScenario" pattern means each step is manually triggered. Scenarios with 8-12 commands require clicking through each one, watching it execute, then clicking the next. For a live demo, the presenter is a button monkey.

### 4. Opacity: Results Don't Tell a Story
After 10+ minutes of waiting, what does the audience see?
- Terminal output scrolled past the viewport
- A `roko learn efficiency` dump that's just raw JSONL stats
- A `roko status` that shows signal counts
- No visual narrative of "here's what roko did, here's why it's impressive"

### 5. Redundancy: Too Many Similar Scenarios
- `providers` and `provider-race` do nearly the same thing
- `prd-pipeline` and `prd-research-loop` differ by one step (research)
- `race` is a simpler `provider-race`
- `knowledge-accumulation` and `knowledge-transfer` test overlapping concepts

---

## Per-Scenario Problems

### 1. prd-pipeline (8 commands)
- Three 10-min timeout steps make this a 10-30 min wait
- `prd draft new "BTC Funding Alert CLI"` must produce slug `btc-funding-alert-cli` or `promote` + `plan` fail
- The `PRD_IDEA` is a long embedded string visible in the terminal — looks awkward
- After `plan run`, user sees a wall of agent output but no clear "what was built"

### 2. prd-research-loop (9 commands)
- Claims "~90s" but has 5 LLM calls (draft, research, plan, run) each 30s-5min
- Requires `PERPLEXITY_API_KEY` for research step
- Slug `cli-config-validation` is hardcoded

### 3. race (3 commands)
- Both panes share same workspace — concurrent `roko run` conflicts
- `--no-replan` difference isn't visible or explained
- `roko learn efficiency` in both panes shows identical output

### 4. gate-retry (6 commands)
- 6-minute timeout on main step
- Gate failures are non-deterministic — may or may not actually fail
- Module-level `runOutcome` singleton persists across resets

### 5. providers (4 commands)
- All 4 panes share same workspace
- No `workspaceDir` passed to `showCmd` — commands may run in wrong directory
- No summary/compare step
- Always returns `{ ok: true }` regardless of failure

### 6. provider-race (5 commands)
- Same workspace conflict as providers
- Module-level `state` singleton
- Race detection logic fragile (relies on regex parsing of output)

### 7. explore (12 commands)
- 12 sequential clicks is tedious
- Command display mismatch: UI shows `roko status`, executes `./target/release/roko --model glm51 status`
- Most commands are read-only and fast, but there are still 12 of them

### 8. knowledge-accumulation (10 commands)
- 10 commands with 2 full LLM runs
- Knowledge store growth depends on knowledge actually being written — not guaranteed
- Run-2 finds run-1's files, making agent behavior unpredictable

### 9. knowledge-transfer (6 commands)
- Two 5-minute LLM runs = 10+ minute minimum
- `betaWorkspaceDir` module-level singleton
- `sync-knowledge` is invisible (uses `execCmd` not `showCmd`)
- The "knowledge helps" narrative is impossible to prove in a demo

### 10. dream-consolidation (6 commands)
- `dream run` has 5-minute timeout
- Gate detection uses fragile regexes (`/hypnagog|replay|select/i`)
- Dream pipeline may not produce visible output that proves it worked

### 11. chat (4 commands)
- Interactive TUI mode — can't be reset
- `typeCmd` into chat TUI has different keyboard handling than shell
- All commands return `{ ok: true }` unconditionally — no error detection
- Subsequent scenarios can't use the terminal after chat TUI starts

### 12. chain-intelligence (12 commands)
- Requires mirage-rs + foundry + correct fork state
- 12 commands across 2 panes
- Agent prompts are 200+ chars with quoting issues
- Both panes share workspace

### 13. mirage (4 commands)
- Redundantly calls `enterWorkspace` on every command click
- Requires external mirage-rs + foundry
- Useful only as chain prerequisite, not standalone demo

### 14. isfr-agents (9 commands)
- **8 panes** — UI is extremely cramped
- All 8 agents share one workspace
- Requires mirage-rs + foundry + specific contract state at fork block
- Agent prompts contain hardcoded contract addresses that go stale
- 20-60 minute runtime

---

## Infrastructure Problems

### terminal-session.ts
- `resolveRoko()` does 3-step binary detection with invisible sideband markers — fragile
- `enterWorkspace()` takes ~1-2s per call (WS ready + prompt wait + resolve + cd)
- `showCmd()` types commands character-by-character with jitter — adds seconds of dead time per command
- Speed multiplier only affects `adjustedSleep`, not actual command execution time

### useTerminal.ts
- 60KB output buffer — long-running agents fill this and lose early output
- `waitForPrompt` polls with regex that may not match all shell prompts
- WebGL renderer fallback adds complexity without clear benefit

### scenario-utils.ts
- `executeCommand` runs parallel across panes but `showCmd` is still serial within each pane
- `cmdForPane/cmdForPanes/cmdForAll` helpers add a layer of indirection

### Module-level State
Three scenarios use module-level singletons that persist across resets:
- `gate-retry.ts`: `runOutcome`
- `provider-race.ts`: `state`
- `knowledge-transfer.ts`: `betaWorkspaceDir`

---

## What Works (Keep/Learn From)

1. **Explore scenario** — fast, read-only, shows breadth. Closest to a viable demo.
2. **ClickableScenario pattern** — good for presenter control, just needs fewer clicks.
3. **CommandList sidebar** — visual progress tracking is useful.
4. **Gate detection** — seeing gates pass/fail is the kind of visual feedback demos need more of.
5. **Multi-pane layout** — showing parallel agents is powerful when it works.
6. **Speed control** — the 0.5x/1x/2x/4x multiplier is a good idea.
7. **ConfigWidget** — model selection pill is clean.
