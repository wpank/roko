# Product Vision: What tiagent Is and How It Works

---

## 1. What is tiagent?

tiagent is a coding agent --- software that writes code for you.

You have probably seen tools like this before. Claude Code reads your codebase, takes
instructions in plain English, and writes code on your behalf. GitHub Codex does the
same thing inside GitHub. Cursor does it inside a VS Code fork. These tools are useful.
Millions of developers use them daily. But they all share the same limitation:

**Your 1,000th session is exactly as capable as your 1st.**

The agent learns nothing from your usage. It never remembers what worked before, which
approaches failed, or what your codebase actually needs. Every session starts from
zero.

tiagent is different. It is a coding agent that gets measurably better the more you
use it. Not through better prompts or manual tuning --- through structural feedback
loops built into the runtime itself:

- **Self-improving.** After 100 coding sessions, tiagent automatically routes tasks
  to the right model, knows which prompt patterns work for your codebase, and has
  extracted reusable strategies from your successful runs. This happens without any
  manual configuration.

- **Model-agnostic.** Use any LLM --- Claude, GPT, Gemini, Llama, Mistral, or any
  local model via Ollama. Switch providers without changing your workflow. tiagent
  automatically routes different task types to different models based on learned
  performance data.

- **Open source.** Not locked into any vendor's ecosystem. Written in Rust. Runs
  anywhere --- your laptop, a server, a CI pipeline, a cloud deployment.

- **Optionally uses Celestia DA for shared learning.** When enabled, tiagent
  instances can share learning artifacts across the network: routing weights,
  efficiency patterns, and successful strategies. Every agent benefits from every
  other agent's experience. This is optional --- the core product works entirely
  standalone.

In short: tiagent is the first coding agent where the harness itself improves through
use, and the first where those improvements can be shared across deployments via
Celestia's data availability layer.

---

## 2. How Developers Use It

tiagent is a command-line tool. You install it, point it at a task, and it writes
code. Here are five real workflows, from simple to advanced.

### 2.1 Single task: one prompt, one result

The simplest usage. Give tiagent a task in plain English, and it writes the code:

```bash
tiagent run "add JWT authentication to the API"
```

tiagent reads your codebase, generates an implementation plan, writes the code,
and validates the result through automated quality gates (compilation, tests, lint).
If the code compiles and the tests pass, the task is done. If something fails,
tiagent automatically retries with a different approach.

This is comparable to what Claude Code or Cursor does --- but with automatic
validation, model routing, and learning from the outcome.

### 2.2 Multi-step plan: execute a sequence of dependent tasks

Real features are not single tasks. They require coordinated changes across multiple
files, in a specific order. tiagent handles this with plan execution:

```bash
tiagent plan run plans/feature-auth/
```

The plan directory contains a `tasks.toml` file that defines a DAG (directed acyclic
graph) of tasks with dependencies. tiagent executes them in order, validates each
step through quality gates, and parallelizes independent tasks automatically. If a
task fails, the plan pauses and can be resumed later.

### 2.3 PRD-to-code: from spec to working implementation

This is the workflow that no other coding agent supports. You start with a product
requirements document (PRD) and end with validated, working code:

```bash
# 1. Write the spec
tiagent prd draft "user-authentication"

# 2. Generate an implementation plan from the spec
tiagent prd plan "user-authentication"

# 3. Execute the plan --- agents run tasks, gates validate each step
tiagent plan run plans/user-authentication/
```

Step 1 creates a structured PRD. Step 2 uses an LLM to generate a concrete
implementation plan with tasks, dependencies, and acceptance criteria. Step 3
executes that plan autonomously. The result is a validated implementation that
traces back to the original spec.

### 2.4 Long-running tasks with crash recovery

Some tasks take hours --- large refactors, codebase migrations, complex feature
builds. tiagent handles these with snapshot-resume:

```bash
# Start a long-running task
tiagent run "refactor the database layer to use async" --resume
```

The `--resume` flag enables state persistence. If the process is interrupted ---
network failure, laptop sleep, intentional pause --- tiagent saves its progress.
When you run the same command again, it picks up exactly where it left off. No
work is lost.

### 2.5 Celestia-specific development

For developers building on Celestia, tiagent provides native tooling:

```bash
tiagent run "deploy a sovereign rollup on Mocha testnet"
```

tiagent includes Celestia-specific tools for blob submission, namespace
management, DA interaction, and rollup deployment. These are available as MCP
tool servers, so they integrate seamlessly with the agent's tool-calling loop.
When Celestia DA-based shared learning is enabled, tiagent benefits from
strategies other Celestia developers have already discovered.

---

## 3. How It Gets Better (Self-Improvement)

tiagent's self-improvement system is organized into three nested feedback loops.
Each operates at a different timescale and scope. The first two work entirely
standalone --- no external services, no blockchain. The third optionally uses
Celestia DA.

### 3.1 Inner loop: learning within a single task

**Timescale:** seconds to minutes
**Scope:** one task execution

When tiagent runs a task, it does not just generate code and hope for the best.
After every agent action, automated quality gates check the result:

1. **Parse** --- does the code have valid syntax?
2. **Compile** --- does it build without errors?
3. **Test** --- do the existing tests still pass?
4. **Lint** --- does it meet code quality standards?

If a gate fails, tiagent does not give up. It reads the error, adjusts its
approach, and retries. A compilation error becomes feedback: the agent sees the
error message, understands what went wrong, and generates a fix. This is the
inner feedback loop --- sense the outcome, compare against the goal (passing
gates), act to close the gap.

The inner loop means tiagent can recover from mistakes within a single task
without human intervention.

### 3.2 Middle loop: learning across tasks

**Timescale:** hours to weeks
**Scope:** all tasks run by one tiagent instance

The middle loop is where real improvement happens. After every task completes,
tiagent records structured data about what happened: which model was used, how
many tokens it consumed, how long it took, whether it succeeded, what tools it
called, and in what order. This data accumulates over time, and tiagent uses it
to make better decisions:

- **Model routing.** The Cascade Router tracks success rates per model per task
  category. After enough data, it stops sending simple test-writing tasks to
  expensive models and routes them to cheaper ones that handle them just as well.
  Complex architectural tasks still go to the most capable model.

- **Playbook extraction.** When a multi-step task succeeds, the sequence of tool
  calls becomes a reusable template. The next time tiagent sees a similar task,
  it starts with a proven strategy instead of figuring it out from scratch.

- **Prompt refinement.** tiagent runs A/B experiments on prompt templates. When
  one variant consistently outperforms another, the better version becomes the
  default.

- **Gate threshold adaptation.** Quality gate thresholds adjust based on
  historical pass rates. If your codebase has 200 lint warnings that existed
  before tiagent touched it, the lint gate learns realistic baselines instead of
  failing on pre-existing issues.

- **Efficiency optimization.** Tokens per task, cost per task, and time per task
  are tracked and minimized over time. The system converges toward the cheapest
  configuration that still produces high-quality results.

- **Knowledge demurrage.** Entries in tiagent's durable knowledge store pay a
  continuous holding tax --- a Gesellian decay on stored information. Unused
  knowledge fades naturally; actively validated knowledge strengthens. Four tiers
  (Transient, Working, Consolidated, Persistent) with half-lives from minutes to
  months govern how aggressively entries decay. Knowledge that is retrieved,
  cited, or gate-validated gets reinforced and promoted up the tiers. Knowledge
  that sits untouched loses balance and eventually expires. The result is a
  memory that self-curates: the agent forgets what no longer matters and
  remembers what keeps proving useful.

The middle loop requires no configuration. It works entirely standalone --- no
external services, no network access, no blockchain. Just local learning from
your own usage.

### 3.3 Outer loop: learning across ALL users via Celestia DA

**Timescale:** hours to days
**Scope:** all tiagent instances in the network

This loop is optional. When enabled, tiagent publishes anonymized learning
artifacts --- routing weights, efficiency patterns, behavioral fingerprints,
successful strategies --- to Celestia's data availability layer. Other tiagent
instances can retrieve these artifacts and incorporate them into their own
learning.

The result: a new tiagent installation does not start from zero. It starts with
the collective knowledge of every other developer who has opted in. Routing
weights reflect thousands of task executions, not just your own. Playbooks
include strategies discovered by developers working on problems you have never
encountered.

Celestia DA is well-suited for this because it provides high-throughput,
verifiable data availability without requiring a full execution layer. Learning
artifacts are posted as blobs, namespaced by category, and retrievable by any
participant. The data is public and auditable.

### 3.4 Improvement timeline

Here is what the progression looks like in practice:

```
DAY 1
  Factory defaults. All tasks routed to your default model.
  Generic system prompts. No codebase-specific knowledge.
  No playbooks --- every problem solved from scratch.
  Gate thresholds at factory defaults.

WEEK 2 (~30-50 tasks)
  Cascade Router has learned your preferred models for 2-3 task types.
  3 playbooks extracted from successful multi-step runs.
  Gate thresholds starting to reflect your codebase's baselines.
  Token budgets beginning to calibrate to actual usage.

MONTH 2 (~100-200 tasks)
  15+ playbooks covering common task patterns.
  Routing accuracy above 90% (right model for the right job).
  Cost per task reduced roughly 40% (fewer retries, cheaper models where safe).
  Time per task reduced roughly 30% (better prompts, fewer gate failures).
  Prompt templates evolved through A/B experiments.

MONTH 6 (with Celestia enabled)
  Benefits from 10,000+ other developers' learning data.
  Playbooks from the community augment your local library.
  Routing weights reflect collective experience, not just yours.
  New task categories handled well immediately, using strategies
  discovered by other developers.
```

None of this requires manual intervention. You just use tiagent, and it
improves. The inner and middle loops drive the local progression. The outer
loop, when enabled, accelerates it with network-wide learning.

---

## 4. How It Compares

The following table compares tiagent against the major coding agents available today
across the dimensions that matter most to developers:

| Capability | tiagent | Claude Code | Codex | Cursor | Aider |
|---|---|---|---|---|---|
| **Self-improvement** | Yes --- learns routing, thresholds, playbooks, prompts across sessions | No | No | No | No |
| **Model agnostic** | Yes --- Claude, GPT, Gemini, Llama, Ollama, any OpenAI-compatible API | No (Anthropic only) | No (OpenAI only) | Limited (3-4 providers) | Yes |
| **Plan execution** | Yes --- DAG executor with dependency ordering and parallel dispatch | No | No | No | No |
| **Quality gates** | Yes --- parse, compile, test, lint gates run automatically after every action | No | No | No | No |
| **PRD workflow** | Yes --- draft specs, generate plans, execute to validated code | No | No | No | No |
| **Shared learning** | Yes --- via Celestia DA (optional) | No | No | No | No |
| **Crash recovery** | Yes --- snapshot-resume from any interruption point | No | No | No | Partial |
| **Knowledge lifecycle** | Yes --- demurrage-based decay; four tiers with half-lives from minutes to months; actively used knowledge strengthens, unused knowledge fades | No (stateless) | No (stateless) | No (stateless) | No (stateless) |
| **Episode logging** | Yes --- structured JSONL with turns, tool calls, gate results, costs | No | No | No | Partial |
| **MCP support** | Yes | Yes | No | Yes | Partial |
| **Open source** | Yes | Partial | Yes | No | Yes |
| **Celestia integration** | Native (optional) | No | No | No | No |

The key differentiator is **self-improvement**. Every other tool on this list treats
session 1,000 identically to session 1. tiagent is the only coding agent where the
harness itself learns from usage.

The second differentiator is **plan execution with quality gates**. No other tool can
take a multi-step implementation plan, execute it autonomously with dependency
ordering, validate each step through automated checks, and recover from failures.

The third differentiator, unique to the Celestia ecosystem, is **shared learning**.
No other agent framework allows instances to share improvement data across
deployments through a verifiable data availability layer.

---

## 5. Technical Summary

This section provides a brief overview of how tiagent is built. It is written for
non-engineers --- no programming knowledge required.

### 5.1 Language and performance

tiagent is written in **Rust**, a programming language known for speed, memory safety,
and reliability. Rust is what Firefox, the Linux kernel, and Cloudflare's edge network
use for performance-critical software. For tiagent, this means:

- **Fast startup.** The CLI launches in milliseconds, not seconds.
- **Low memory usage.** Runs comfortably on a laptop alongside your editor and browser.
- **No runtime crashes.** Rust's type system prevents entire categories of bugs at
  compile time.
- **Single binary.** One executable, no Python environment, no Node.js dependencies,
  no Docker required.

### 5.2 Modular architecture

tiagent is organized into roughly **14 crates** (Rust's term for libraries). Each
crate does one thing:

| Crate | Purpose |
|---|---|
| `tiagent-core` | Core types and abstractions (the Signal type, trait definitions) |
| `tiagent-agent` | LLM provider integrations --- talks to Claude, GPT, Gemini, Ollama, etc. |
| `tiagent-cli` | Command-line interface --- what you interact with |
| `tiagent-orchestrator` | Plan execution --- DAG scheduling, parallel dispatch, dependency ordering |
| `tiagent-gate` | Quality gates --- compile, test, lint checks after every action |
| `tiagent-compose` | Prompt assembly --- builds system prompts from templates and context |
| `tiagent-learn` | Learning system --- episodes, playbooks, routing, experiments |
| `tiagent-fs` | Storage --- persists signals, episodes, and state to disk |
| `tiagent-runtime` | Process management --- starts, monitors, and stops agent processes |
| `tiagent-serve` | HTTP API --- exposes agent functionality to dashboards and external tools |
| `tiagent-neuro` | Knowledge store --- durable memory for the agent |
| `tiagent-plugin` | Plugin system --- extend tiagent with custom tools and integrations |
| `tiagent-mcp` | MCP integration --- connects to the tool ecosystem |
| `tiagent-celestia` | Celestia DA integration --- blob posting, namespace management, shared learning |

This modularity means each piece can be developed, tested, and replaced independently.
The Celestia crate is entirely optional --- removing it does not affect any other
functionality.

### 5.3 Model support

tiagent works with any LLM through a pluggable backend system:

- **Cloud APIs:** Anthropic (Claude), OpenAI (GPT), Google (Gemini), Cerebras, Perplexity
- **Local models:** Ollama (Llama, Mistral, Phi, Qwen, and any GGUF model)
- **Any OpenAI-compatible API:** vLLM, LiteLLM, LocalAI, LM Studio, and hundreds of others

You configure your preferred providers in a configuration file. tiagent's Cascade
Router automatically distributes tasks across them based on learned performance data.
You can also specify a model manually for any task.

### 5.4 Tool ecosystem

tiagent supports **MCP** (Model Context Protocol), the open standard for connecting AI
agents to external tools. The MCP ecosystem has tools for file operations, web search,
database access, API integration, and thousands of other capabilities. Any MCP-compatible
tool server works with tiagent out of the box.

### 5.5 Celestia integration

Celestia integration is **feature-flagged** --- it is compiled in only when explicitly
enabled. The core product works entirely without it. When enabled, tiagent can:

- Post learning artifacts (routing weights, efficiency data, playbooks) as blobs to
  Celestia DA.
- Retrieve learning artifacts posted by other tiagent instances.
- Use Celestia namespaces to organize and discover shared data.
- Verify the integrity and provenance of retrieved artifacts.

This integration is what makes tiagent relevant to the Celestia ecosystem
specifically, but the product stands on its own without it.

---

## 6. What "Self-Improving" Actually Means

"Self-improving" is a strong claim. This section explains the concrete mechanisms that
back it up. Each mechanism is implemented, measurable, and auditable.

### 6.1 Cascade Router: automatic model selection

**What it does:** Tracks success rates, token usage, and latency per model per task
category. Routes each new task to the cheapest model that is likely to succeed.

**How it works:**
1. A task arrives (e.g., "write a unit test for the auth module").
2. tiagent classifies the task by category (test writing, refactoring, bug fix, etc.).
3. The Cascade Router checks its history: which models have handled this category
   before? What were the success rates? What did they cost?
4. It picks the cheapest model whose historical success rate exceeds a configurable
   threshold (default: 85%).
5. After the task completes, the outcome is recorded and the router's weights update.

**Concrete example:** On day 1, all tasks go to Claude Sonnet (your default). By week
3, the router has learned that simple test-writing tasks succeed 95% of the time with
Haiku (which costs 10x less than Sonnet). It starts routing test tasks to Haiku
automatically. Sonnet is reserved for complex refactoring where Haiku's success rate
is only 60%.

**Where the data lives:** `.tiagent/learn/cascade-router.json` --- a JSON file you can
inspect and audit at any time.

### 6.2 Adaptive Gates: quality thresholds that learn

**What it does:** Adjusts pass/fail thresholds for quality gates based on historical
outcomes. Prevents gates from being too strict (blocking good work) or too lenient
(accepting bad work).

**How it works:**
1. Every task result passes through a gate pipeline: parse, compile, test, lint, diff
   review.
2. Each gate produces a score (pass/fail, warning count, test coverage percentage, etc.).
3. An exponential moving average (EMA) tracks the historical score for each gate.
4. Thresholds adjust toward realistic baselines. If your codebase already has 150 lint
   warnings before tiagent touches it, the lint gate will not fail on those pre-existing
   warnings.

**Concrete example:** A codebase with 80% test coverage. On day 1, the test coverage
gate uses the factory default threshold (70%). After 20 tasks, the adaptive threshold
has learned that the realistic baseline is 80%, and it starts flagging any change that
drops coverage below 78%.

**Where the data lives:** `.tiagent/learn/gate-thresholds.json`.

### 6.3 Playbook Extraction: reusable strategies from success

**What it does:** When a multi-step task succeeds, the sequence of tool calls ---
what the agent did and in what order --- becomes a reusable template called a
playbook. The next time tiagent encounters a similar task, it consults matching
playbooks for a proven starting strategy.

**How it works:**
1. A task completes successfully after, say, 8 tool calls: read file, analyze
   structure, write new function, run tests, fix error, run tests again, lint, commit.
2. tiagent records this sequence as a playbook, tagged with the task category and
   relevant metadata.
3. On future tasks in the same category, tiagent retrieves matching playbooks and
   uses them to inform its approach. Instead of exploring blindly, it starts with
   a strategy that has worked before.

**Concrete example:** The first time tiagent adds a REST endpoint to your API, it
takes 12 tool calls and 2 retries. After 5 similar tasks, it has a playbook:
"read the existing route handler, create a new handler following the same pattern,
add the route, write a test, verify." Future endpoint additions start from this
template and typically complete in 6 tool calls with 0 retries.

**Where the data lives:** `.tiagent/learn/playbooks/` directory.

### 6.4 Efficiency Tracking: measuring and reducing cost

**What it does:** Measures tokens consumed, wall-clock time, and estimated cost for
every agent turn. Tracks these metrics over time and surfaces trends.

**How it works:**
1. Every agent turn records: model used, input tokens, output tokens, time elapsed,
   tools called, and the task outcome.
2. These records are written to a structured log.
3. Aggregate statistics are computed per task category, per model, and per time period.
4. The Cascade Router uses efficiency data to break ties --- when two models have
   similar success rates, it picks the one that uses fewer tokens.

**Concrete example:** Over the first month, efficiency tracking reveals that
refactoring tasks average 15,000 tokens with Claude Sonnet but 22,000 tokens with
GPT-4o (because GPT-4o tends to produce more verbose explanations). The router
factors this into its cost calculations and prefers Sonnet for refactoring tasks.

**Where the data lives:** `.tiagent/learn/efficiency.jsonl`.

### 6.5 Knowledge Demurrage: memory that self-curates

**What it does:** Applies a continuous holding cost --- a Gesellian demurrage tax ---
to every entry in tiagent's durable knowledge store. Knowledge that is not actively
used decays and eventually expires. Knowledge that is retrieved, cited, or validated
by quality gates is reinforced and promoted.

**How it works:**
1. Every knowledge entry carries a balance that decays continuously over time,
   governed by its tier.
2. Four tiers define the decay schedule:
   - **Transient** --- half-life of minutes. Scratchpad facts: function signatures
     discovered mid-task, error messages from the last compile. Useful right now,
     irrelevant tomorrow.
   - **Working** --- half-life of hours to days. Context for the current feature
     branch: which files were changed, what approach was chosen, what the PR
     reviewer said.
   - **Consolidated** --- half-life of weeks. Patterns that have proven useful
     across multiple tasks: "this codebase uses the repository pattern," "integration
     tests live in `tests/integration/`."
   - **Persistent** --- half-life of months. Durable facts validated by repeated
     gate success: "the auth module requires a JWT secret in the environment,"
     "deploy scripts assume PostgreSQL 15."
3. Reinforcement events --- retrieval, citation in a successful task, gate-backed
   validation --- add balance and can promote an entry to a higher tier.
4. Entries whose balance reaches zero are pruned automatically.

**Why it matters:** Every other coding agent treats memory as either permanent or
absent. Permanent memory accumulates noise --- outdated facts, stale patterns,
context from deleted code. Absent memory means starting from scratch every session.
Demurrage solves both problems: the agent remembers what keeps proving useful and
forgets what stops being relevant. The knowledge store converges toward a compact,
high-signal representation of your codebase's actual state.

**Concrete example:** During a refactoring sprint, tiagent learns that your API uses
Express middleware in a specific pattern. This entry starts at Working tier. Over the
next two weeks, it is retrieved and validated during 8 related tasks, promoting it to
Consolidated. Six months later, you migrate to Fastify. The Express knowledge is never
retrieved again, its balance decays, and it quietly expires --- replaced by Fastify
patterns that are now being actively reinforced.

**Where the data lives:** `.tiagent/neuro/` --- the durable knowledge store, with
tier metadata and balance per entry.

### 6.6 Episode Logging: complete audit trail

**What it does:** Records every agent turn in structured, machine-readable format.
Every tool call, every gate result, every model response, every retry --- all
captured in a JSONL log.

**How it works:**
1. Each agent turn produces an episode record containing: timestamp, task ID, model
   used, prompt sent, response received, tools called, gate results, token counts,
   and outcome.
2. Episodes are appended to a JSONL (JSON Lines) file --- one JSON object per line,
   easy to parse with standard tools.
3. Episodes feed the middle loop: they are the raw data from which playbooks are
   extracted, routing weights are updated, and efficiency trends are computed.

**Why it matters:** Episode logging makes tiagent's behavior fully auditable. You
can inspect exactly what the agent did, why it made each decision, and how the
learning system used the outcome. For teams deploying agents in production, this
audit trail is essential. For researchers studying agent behavior, it provides
structured data for analysis.

**Where the data lives:** `.tiagent/episodes.jsonl`.

### 6.7 Shared Learning via Celestia DA

**What it does:** When enabled, publishes anonymized learning artifacts to Celestia's
data availability layer. Other tiagent instances retrieve and incorporate these
artifacts into their own learning.

**How it works:**
1. After a configurable number of tasks, tiagent aggregates its learning data:
   routing weights, efficiency statistics, playbook summaries, gate threshold
   baselines.
2. These artifacts are serialized, compressed, and posted as blobs to a Celestia
   namespace dedicated to tiagent learning data.
3. Other tiagent instances periodically query the namespace, retrieve new artifacts,
   validate their integrity, and merge them with their local learning state.
4. Merge uses weighted averaging: local data is weighted more heavily than
   network data, so your agent's behavior reflects your usage patterns first and
   collective knowledge second.

**Why Celestia specifically:** Celestia's DA layer is designed for exactly this
kind of high-throughput, verifiable data publication. It provides data availability
guarantees without requiring a full execution layer, which means low cost and
high throughput. The data is public and verifiable --- anyone can audit what
learning artifacts are being shared and confirm their integrity.

**What is NOT shared:** Raw code, proprietary information, API keys, or anything
that could identify you or your codebase. Only aggregate statistics, anonymized
routing weights, and generalized strategy templates are published.

---

## Summary

tiagent is a coding agent that gets better the more you use it. It works with any
LLM, validates every output through automated quality gates, executes multi-step
plans from specs to working code, and optionally shares learning across all
instances via Celestia DA.

For developers, it replaces Claude Code, Codex, Cursor, or Aider with a tool
that learns your preferences, routes tasks to the right model, extracts reusable
strategies, and reduces cost over time --- automatically, without manual tuning.

For the Celestia ecosystem, it is the first native agent framework that uses DA
for shared learning, turning Celestia into infrastructure for collective AI
improvement.

The code is open source. The Celestia integration is optional. The self-improvement
is real, measurable, and auditable.
