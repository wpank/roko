# CLI Parity with Familiar Workflows

> **TL;DR**: Users coming from Claude Code, OpenClaw, Aider,
> Cursor agent mode, and similar tools bring specific muscle
> memory. Adopting these idioms where they don't conflict with
> Roko's model lowers the onboarding cost dramatically — hours
> instead of days. This doc maps the expectations, flags the
> genuine conflicts, and proposes a CLI shape that is "Claude
> Code-shaped where possible, Roko-shaped where necessary."

> **For first-time readers**: Users coming from Claude Code, Aider,
> Cursor, Codex CLI, and similar tools have built muscle memory for
> slash commands, per-hunk diffs, workspace detection, resumption,
> and transcripts. Roko should honor that muscle memory where
> possible, then layer its own distinct capabilities (plan workflow,
> heuristic provenance, c-factor visibility) on top. Read 23 (user
> UX) first; this is the CLI-specific slice.

## 1. What users expect

The Claude Code / OpenClaw / Aider lineage has converged on a
loose pattern:

1. You invoke the tool in a project directory.
2. It indexes / probes the workspace.
3. It opens an interactive REPL (sometimes with TUI chrome).
4. You type natural-language requests.
5. It proposes changes; you approve, edit, or reject.
6. It can apply changes, run tests, and commit.
7. Slash commands extend functionality (`/edit`, `/compact`,
   `/run`, `/undo`, etc.).
8. A transcript is saved for later review.

Users have this loop in their fingers. They don't want to learn a
new loop; they want the same loop with Roko's capabilities behind
it.

## 2. Where Roko differs (deliberately)

Some Roko differences are genuine and worth preserving:

- **Plan-first workflow**: Roko prefers to propose a plan, then
  execute tasks, with gates between. Not all users want this for
  every interaction.
- **Persistent learning**: episodes, heuristics, and playbooks
  persist across sessions. Users accustomed to stateless tools
  need to opt in (or opt out, depending on how we default).
- **Multi-agent**: Roko can orchestrate multiple agents in one
  session; Claude Code is single-agent.

The CLI should accommodate both the "stateless chat" muscle memory
and the "plan-driven workflow" that makes Roko distinct. It should
*not* force users into the plan-driven model on turn one.

## 3. The unified `roko` entry

Today's CLI has many subcommands (`roko run`, `roko plan`, `roko
chat`, `roko prd`, ...). Proposal: keep them, but make `roko`
with no subcommand the default interactive mode, detecting intent
from the first message.

```
$ roko
[roko 1.0 — /Users/will/myproject]
> fix the failing test

Analyzing workspace...
Found: 2 failing tests in `tests/core.rs`.
Propose to: read source, reproduce failure, apply fix, rerun tests.

Proceed? [Y/n]
```

No mode selection up front. The agent observes the prompt and
picks: "this is a simple ask" vs "this requires a plan" vs "this
is a chat question." The user sees the choice and can approve,
tweak, or cancel.

## 4. Slash commands

Adopt the Claude Code set where meaning matches, add Roko-specific
where it doesn't:

| Command | Meaning | Source of inspiration |
|---|---|---|
| `/edit <file>` | Edit a specific file with the agent | Claude Code |
| `/run <cmd>` | Run a shell command (with safety layer) | Claude Code / Aider |
| `/undo` | Undo last action | Aider |
| `/compact` | Compact the conversation context | Claude Code |
| `/plan` | Convert current thread into a plan | Roko-specific |
| `/execute` | Execute the current plan | Roko-specific |
| `/watch` | Open live dashboard | Roko-specific |
| `/inspect <hash>` | Drill into an engram/episode/heuristic | Roko-specific |
| `/explain` | Show why the agent did what it did | Roko-specific |
| `/heuristics` | Browse active heuristics | Roko-specific |
| `/learn` | Turn this exchange into a heuristic/playbook | Roko-specific |
| `/replay <episode>` | Re-run an episode | Roko-specific |
| `/tune <param>` | Adjust configuration on the fly | Roko-specific |
| `/help` | Show help | Universal |
| `/exit` | Exit | Universal |

Every slash command has a direct CLI equivalent so the same thing
can be scripted non-interactively. `/plan` ↔ `roko plan create`.

## 5. Natural-language shortcuts

Some requests should route automatically:

- "show me last failure" → `/inspect <last failed episode>`
- "what changed?" → `/diff HEAD~1`
- "explain that" → `/explain <most recent action>`
- "try again with..." → `/replay <episode> --modify ...`

These aren't hardcoded patterns; they're heuristic matches the
Composer is allowed to suggest. Accept loose phrasing; fall back
to asking if ambiguous.

## 6. Workspace awareness

Claude Code's biggest UX win is feeling like it *knows your
project*. Roko can match and exceed:

- On `roko` startup, scan the workspace:
  - Detect language (Cargo.toml, package.json, pyproject.toml, ...)
  - Detect frameworks from dependencies
  - Detect test runners
  - Detect VCS state (branch, dirty files)
  - Surface the detection in the opening banner
- Offer to import prior context:
  - "I see this project has a `.roko/` directory with 42 episodes.
    Import?" [Y/n]
  - "No roko state yet. Initialize?" [Y/n]
- On re-entry, summarize what changed since last session:
  - "Since you left 3h ago: 17 commits, 1 new branch, 2 failing
    tests added."

This meets the user where they are. Users feel seen, not dropped
into a generic shell.

## 7. Diff-first output

When agents propose changes, show them as diffs, not as rewritten
files:

```
Proposed change to src/core.rs:

  @@ -42,7 +42,9 @@
       fn process(&self, input: &str) -> Result<String> {
  -        let normalized = input.trim();
  +        let normalized = input.trim().to_lowercase();
  +        if normalized.is_empty() { return Err(Error::Empty); }
           Ok(normalized.to_string())
       }

Apply? [y/N/edit/explain]
```

The `explain` option is Roko-specific: it reveals the
heuristic/claim citations that led the agent here. This is
`14-worldview-validation.md` §9 made visible.

## 8. Inline apply, copy, git-hunk granularity

Cursor and Claude Code have taught users to expect *per-hunk*
control:

```
Proposed 3 hunks:
  [1/3] src/core.rs: add lowercase normalization
  [2/3] src/core.rs: add empty-check
  [3/3] tests/core.rs: add test for empty input

Apply: [a]ll, [1,2] subset, [n]one, [e]dit >
```

Users accept/reject at the hunk level. Rejected hunks feed back
as negative signal.

## 9. Interactive budgeting

For users worried about LLM costs (most of them):

```
Budget: $0.50/turn, $5 session
Current: $0.12 / $5.00 (2% spent)
```

Visible always in the prompt line. `--budget` CLI flag sets per
invocation. The cascade router respects the budget.

## 10. Transcripts and resumption

Every session produces a transcript. Resumption:

```
$ roko
[3 prior sessions found]
  1. Today 14:22 — "fix the failing test" (completed)
  2. Today 10:07 — "implement rate limiter" (paused)
  3. Yesterday — "refactor auth flow" (completed)

Resume, new, or quit? [R/n/q] r 2
```

Resuming loads the session's episode chain, heuristic choices,
and tool permissions. Identical to the Claude Code "continue"
experience but with richer state.

## 11. Piped mode

The CLI should work in scripts, not just interactively:

```bash
echo "summarize README.md" | roko --format json | jq ...
roko --prompt "lint this" < file.rs
```

In piped mode:

- No TUI, no prompts (unless `--interactive` forced).
- JSON output by default with `--format json`.
- Exit codes semantic (0 success, 1 agent refusal, 2 gate failure,
  3 budget exhausted, 4 config error).
- All feedback to stderr; data to stdout.

This makes Roko composable with shell pipelines and CI.

## 12. Tab completion

`roko` should offer completions:

- Subcommand names
- Plan IDs, PRD slugs, episode hashes (from `.roko/`)
- Model names, role names
- Flag values where enumerable

Ship completion scripts for bash, zsh, fish. Generate via `clap`'s
completion generator. One-time setup during `roko init`.

## 13. What *not* to copy

Some conventions look good in marketing but are actually weak:

- **Massive system prompts visible on the screen**: no.
  Our system prompt layers are interesting *on demand* via
  `/explain`, not always on.
- **"Hallucination avoidance" banners**: no. Our heuristic
  provenance is more honest and less scolding.
- **Emoji-heavy output**: no. Emoji per important state is fine
  (✓/✗/⚠), emoji as decoration is not.
- **Forced full-screen TUI**: no. Interactive CLI first, TUI is
  `/watch` or `roko dashboard`.

## 14. Error messages that teach

Borrow from Rust's error discipline:

```
error: tool 'cargo.test' not allowed for role 'reader'

  This role has the reader capability set, which excludes
  running tests.

  Try one of:
    - Switch role: roko ... --role implementer
    - Add capability: roko config set roles.reader.tools +=cargo.test
    - Use a different profile: roko use profile coding-full

  See: https://roko.dev/docs/roles
```

Every error points at a fix. Users learn the system through
recovery.

## 15. Migration aids

Users moving from Claude Code:

```bash
roko import --from claude-code ~/.claude_code_history
```

Reads their prior transcripts, extracts prompts/responses, stores
as Roko episodes, applies starting demurrage. Gives them a
running start rather than an empty substrate.

Same pattern for Aider logs, Cursor chat exports, etc. Writing
one importer per tool takes a few hours and pays off dramatically
in onboarding.

## 16. The one-minute pitch

"If you know Claude Code, you know Roko's CLI. Slash commands,
workspace detection, per-hunk diffs, transcripts, budgets — it's
all where you expect. Roko adds persistent learning, multi-agent
orchestration, live dashboards, and explainable decisions on top
— but the first hour feels familiar."

This framing makes Roko *additive* rather than alternative, and
reduces the cognitive cost of adoption.

## 17. Priority for familiar-workflow CLI

1. **Interactive `roko` entry with intent detection**. One week.
2. **Slash commands for the Claude Code-aligned set**. One week.
3. **Diff-first output with per-hunk control**. One week.
4. **Workspace detection banner + resumption prompt**. Three days.
5. **Budget display and enforcement**. Three days.
6. **Tab completion**. Two days.
7. **Claude Code / Aider transcript importers**. One week.

A month of focused UX work lands all of it. Adoption trajectory
after this should be noticeably different.

## 18. Slash command dispatch model

Slash commands route through a unified dispatcher so extending them
is a small amount of code:

```rust
pub trait SlashCommand {
    fn name(&self) -> &str;
    fn aliases(&self) -> &[&str] { &[] }
    async fn execute(&self, ctx: &ChatCtx, args: &[&str]) -> SlashOutcome;
    fn help(&self) -> SlashHelp;
}

pub enum SlashOutcome {
    Message(String),          // post as agent response
    Diff(DiffView),           // render per-hunk
    OpenPanel(PanelRef),      // open TUI pane / web modal
    SilentOk,                 // logged but not rendered
    Error(String),            // render as error
}
```

Plugins register new slash commands via tier-1 (pure-data) manifest
or tier-4 (native) trait impl (17). The core set from §4 is just
the default registry.

## 19. Tab-completion spec

Completion expectations, per cursor position:

- After `roko <TAB>` → subcommand list.
- After `roko plan <TAB>` → `list`, `show`, `create`, `run`, `pause`.
- After `roko plan show <TAB>` → current plan IDs (discovered from `.roko/plans/`).
- After `roko inspect <TAB>` → all Engram hashes with brief descriptions,
  prefix-filtered.
- After `roko ask --role <TAB>` → role names from current profile.
- After `roko config set <TAB>` → dotted keys from current roko.toml.
- Flag values enumerable wherever `clap` knows them.

Generate via `clap_complete` at build time. Re-run on `roko init`
to refresh per-workspace completions (plan IDs change).

## 20. Interactive first-turn flow

When the user types their first message, Roko doesn't immediately
run. It classifies the intent:

1. **Question** ("what does this repo do?") — runs as
   `/ask` in researcher role. Non-destructive. No plan.
2. **Small change** ("fix the typo in README") — runs as
   `/edit`. Single diff, one approval step.
3. **Multi-step change** ("add rate limiting to the API") —
   runs as `/plan`. Proposes a plan; user approves plan; executes
   with gates.
4. **Ambiguous** — asks a clarifying question with 2–3 pre-filled
   choices.

Classification uses a small local heuristic (keyword matching + HDC
similarity to past prompts) plus a soft LLM check. The user
always sees the chosen classification and can override (`/ask`,
`/edit`, `/plan` redirect).

This removes "which mode am I in?" anxiety for new users while
preserving power-user control.

## 21. Non-interactive CI mode

Every interactive affordance has a CI equivalent:

```bash
# run a plan, fail if any gate fails
roko plan run plans/foo --non-interactive --fail-on-gate-violation

# ask and print JSON result (no TUI)
roko ask --format json "list current heuristics" | jq '.heuristics[]'

# approve all safe operations without prompting (dangerous, explicit)
roko plan run plans/foo --non-interactive --auto-approve safe

# record every action to a replay-able session
roko --record session.jsonl ask "summarize"

# replay a session, verifying outputs match
roko replay session.jsonl --assert
```

`--assert` turns a replay into a regression test. CI pipelines
that need stable agent behavior can lock it down.

## 22. Cross-references

- The broader user UX story (all surfaces): `23-user-ux-running-agents.md`.
- Web UI equivalents: `29-web-ui-architecture.md`.
- Rich UX primitives (diffs, banners, footnotes) are defined in
  `30-rich-ux-primitives.md`.
- Permission model behind `/run`, `/edit`, and approval flows:
  `32-safety-sandbox-provenance.md` §4.
- Plugin-contributed slash commands:
  `17-plugin-extension-architecture.md` §2.1 and §4.
- Cost visibility that `--budget` enforces:
  `24-deployment-ux.md` §10.
