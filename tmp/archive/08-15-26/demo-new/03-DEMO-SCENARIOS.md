# Demo Scenarios

Each scenario is a tab in the unified demo page. Scenarios range from
simple (showcase a single command) to complex (multi-pane comparisons).

All scenarios use a clean `/tmp/roko-demo` workspace and auto-resolve the
roko binary path at runtime.

---

## Scenario 1: Self-Hosting Workflow

**Tab label:** Self-Hosting
**Panes:** 1
**Side panel:** metrics + event log
**Purpose:** Show the complete PRD → plan → execute → validate loop

### Commands

```bash
# 1. Initialize workspace
roko init

# 2. Capture an idea
roko prd idea "Add retry logic to the HTTP client with exponential backoff"

# 3. Draft a PRD from the idea
roko prd draft new "http-retry"

# 4. Show PRD status
roko prd list

# 5. Generate plan from PRD
roko prd plan http-retry

# 6. Show the generated plan
roko plan list

# 7. Check workspace status
roko status
```

### What it demonstrates
- Zero-config startup (`roko init` creates everything)
- Idea capture → PRD → plan pipeline
- Agent-driven drafting
- The plan DAG that roko generates

### Side panel shows
- Each command's cost (from status output)
- PRD count, plan count
- "What just happened" event log

---

## Scenario 2: Builder

**Tab label:** Builder
**Panes:** 1
**Side panel:** file tree + gates + metrics
**Prompt bar:** visible (user types what to build)
**Purpose:** Type a request, watch roko build it

### Preset Quick-Start Cards

| Card | Prompt |
|---|---|
| calculator | "Build a CLI calculator in Rust" |
| REST API | "Create a REST API with health check and CRUD endpoints" |
| md-to-html | "Write a markdown to HTML converter with syntax highlighting" |
| dedup | "Build a file deduplication tool using content hashing" |
| commitgen | "Create a git commit message generator that reads diffs" |

### Commands (dynamic from prompt)

```bash
roko init
roko run "<user prompt>" --share
find . -type f -not -path './.roko/*' -not -path './target/*' | head -20
```

### What it demonstrates
- Single-command code generation (`roko run`)
- Gate pipeline (compile/test/clippy results)
- File creation detection
- Cost tracking per run

### Side panel shows
- **File tree:** Files created by the agent (detected from Write tool calls)
- **Gates:** compile ✔/✖, test ✔/✖, clippy ✔/✖
- **Metrics:** cost, tokens, model, elapsed time

---

## Scenario 3: The Race (side-by-side comparison)

**Tab label:** The Race
**Panes:** 2 (left = stock, right = roko)
**Side panel:** comparison metrics
**Purpose:** Show roko vs stock LLM on the same task

### Setup

| Pane | Label | What runs |
|---|---|---|
| Left | "stock LLM" | `roko run "Build a URL shortener" --no-replan` (no learning, no gates) |
| Right | "roko (full)" | `roko run "Build a URL shortener"` (full pipeline) |

### Commands

**Left pane (stock):**
```bash
roko init
roko run "Build a URL shortener in Rust" --no-replan
```

**Right pane (roko full):**
```bash
roko init
roko run "Build a URL shortener in Rust"
```

### What it demonstrates
- Side-by-side cost comparison
- Gate pipeline catches errors that stock misses
- Replan on gate failure (roko iterates, stock doesn't)
- Final cost: roko may spend more on gates but produces working code

### Side panel shows
- Two-column comparison: cost, tokens, gates passed, time
- Winner highlight (bone color on the better one)

---

## Scenario 4: Multi-Provider Showcase

**Tab label:** Providers
**Panes:** 4
**Side panel:** hidden (metrics per pane)
**Purpose:** Same prompt across different LLM providers

### Setup

Each pane runs the same task with a different provider override.
Uses the providers from `roko.toml`:

| Pane | Provider | Model | Env var |
|---|---|---|---|
| 1 | Zhipu | glm-5.1 | `ZAI_API_KEY` |
| 2 | OpenAI | gpt-4.1-mini | `OPENAI_API_KEY` |
| 3 | Anthropic | claude-sonnet-4-6 | `ANTHROPIC_API_KEY` |
| 4 | Moonshot | kimi-k2.6 | `MOONSHOT_API_KEY` |

### Commands (per pane)

```bash
roko init
# Force specific backend via env override
ROKO_FORCE_BACKEND=zhipu roko run "Write a function that finds prime numbers"
```

Note: If a provider key isn't set, that pane shows a message:
"provider not configured — set $KEY to enable"

### What it demonstrates
- Multi-provider support (all via OpenAI-compat)
- Cost and speed comparison across providers
- Model routing in action

---

## Scenario 5: Knowledge Compounding

**Tab label:** Compounding
**Panes:** 1
**Side panel:** cost chart + knowledge stats
**Purpose:** Same task 3 times, cost decreasing via knowledge carryover

### Commands

```bash
roko init

# Run 1 — cold start
roko run "Add input validation to the user registration form"
roko learn all

# Run 2 — knowledge from run 1
roko run "Add input validation to the payment form"
roko learn all

# Run 3 — knowledge from runs 1+2
roko run "Add input validation to the settings form"
roko learn all

# Show efficiency trend
roko learn efficiency
```

### What it demonstrates
- Episode logging captures what the agent learned
- Knowledge store grows across runs
- Later runs are cheaper (fewer tokens, better routing)
- Adaptive gate thresholds improve over time

### Side panel shows
- Cost per run: $X.XX → $Y.YY → $Z.ZZ (declining)
- Knowledge entries: 0 → N → M
- Efficiency ratio: improving
- Projected cost at run 1000

---

## Scenario 6: Command Showcase

**Tab label:** Commands
**Panes:** 4
**Side panel:** hidden
**Purpose:** Show breadth of roko CLI commands

### Setup

4 panes running different command families:

| Pane | Label | Commands |
|---|---|---|
| 1 | workspace | `roko init`, `roko status`, `roko doctor` |
| 2 | learning | `roko learn all`, `roko learn efficiency`, `roko learn router` |
| 3 | agents | `roko agent list`, `roko config providers list`, `roko config models list` |
| 4 | knowledge | `roko knowledge stats`, `roko knowledge query "testing"`, `roko explain "gates"` |

### What it demonstrates
- Breadth of CLI surface
- Zero-config commands that work instantly
- Introspection capabilities (learn, status, doctor, explain)

---

## Scenario 7: Research & PRD Pipeline

**Tab label:** Research
**Panes:** 1
**Side panel:** metrics + PRD status
**Purpose:** Show the research-enhanced PRD workflow

### Commands

```bash
roko init

# Capture idea
roko prd idea "Implement WebSocket support for real-time agent streaming"

# Create draft
roko prd draft new "websocket-streaming"

# Enhance with research
roko research enhance-prd websocket-streaming

# Check status
roko prd status

# Generate plan
roko prd plan websocket-streaming

# Show plan
roko plan show plans/
```

### What it demonstrates
- Research agent (Perplexity-backed web search)
- PRD enhancement with citations
- Plan generation from enriched PRD
- Task decomposition

---

## Scenario Metadata

All scenarios share:
- Clean workspace per run (`/tmp/roko-demo` or `/tmp/roko-demo-{ts}`)
- `roko init` as first command
- Prompt-based command sequencing (wait for `$` before next command)
- Side panel updates from terminal output parsing
- Pause/resume/reset controls

### Output Detection Patterns

```javascript
const PATTERNS = {
  gate_pass:    /(?:compile|test|clippy).*?✔/,
  gate_fail:    /(?:compile|test|clippy).*?✖/,
  file_create:  /(?:Write|scaffolded)\s+.*?\.(rs|toml|md|json|yaml)/,
  cost:         /\$(\d+\.\d+)/,
  tokens:       /(\d+)\s*(?:in|tokens)/,
  model:        /model:\s*(\S+)/,
  error:        /(?:error|Error|ERROR):\s*(.*)/,
  done:         /(?:done|completed|finished|exit code: 0)/i,
};
```
