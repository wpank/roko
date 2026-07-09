# May 6 Demo Build Checklist

> **Context**: Nunchi is pitching a16z on May 6, 2026 for a Series A. The demo is the centerpiece — a 5-minute live terminal session showing an agent runtime that handles identity, routing, gates, knowledge-sharing, and crash recovery. This checklist covers everything that must be built or wired to make the demo runnable.
>
> **What Nunchi is**: An open-source Rust agent runtime (18 crates, "Roko") + a sovereign EVM blockchain for agent identity and knowledge. The demo shows the RUNTIME, not the chain. The chain is Phase 4.
>
> **Codebase location**: `/Users/will/dev/nunchi/roko/roko/`
> **Key crate**: `crates/roko-cli/` (the CLI binary)
> **Orchestrator**: `crates/roko-cli/src/orchestrate.rs` (the main agent dispatch loop)
> **Agent dispatcher**: `crates/roko-agent/src/dispatcher/mod.rs`

---

## Priority 0: Demo-Critical (Must work May 5)

### P0-1: `nunchi` CLI wrapper binary

**What**: A thin CLI binary (or shell script) that wraps `cargo run -p roko-cli --` with a cleaner interface. The demo uses `nunchi` not `cargo run`.

**Checklist**:
- [ ] Create `bin/nunchi` shell script that forwards args to `cargo run -p roko-cli --release --`
- [ ] OR build a release binary and symlink to `~/.local/bin/nunchi`
- [ ] Verify: `nunchi --help` shows subcommands
- [ ] Verify: `nunchi status` works
- [ ] Verify: `nunchi plan run` works with the existing orchestrator

**Files to check**: `crates/roko-cli/src/main.rs` for existing CLI entry point, `Cargo.toml` for binary name

---

### P0-2: `nunchi agents list` command with identity display

**What**: Show registered agents with their identity, model, and status. This is the first thing shown in the demo (Minute 1).

**Target output format** (Clack-style, NO emoji):
```
$ nunchi agents list --env=prod
◆ Agents (prod)
│
│  NAME           IDENTITY                        MODEL           STATUS
│  researcher     nhi://acme/researcher.v2        claude-haiku    ● active
│  auditor        nhi://acme/auditor.v1           claude-sonnet   ● active
│  reviewer       nhi://acme/reviewer.v3          gpt-4o-mini     ○ idle
│
└  3 agents registered · 2 active · 1 idle
```

**Checklist**:
- [ ] Check if `roko agent list` already exists in CLI (`crates/roko-cli/src/main.rs` or agent subcommand)
- [ ] If exists, modify output formatting to use Clack-style symbols (◆ ◇ │ └ ● ○)
- [ ] If not exists, add `agents list` subcommand that reads from `.roko/` config or agent registry
- [ ] Add `--env` flag (can be cosmetic for demo — filter display only)
- [ ] Add identity column showing `nhi://` URI format
- [ ] Add model column from agent config
- [ ] Add status column (active/idle based on PID existence or heartbeat)
- [ ] Verify: runs in <1 second with pre-configured agents

**Files**: `crates/roko-cli/src/main.rs`, `crates/roko-agent/`

---

### P0-3: `nunchi audit` command (the demo centerpiece)

**What**: Run an audit workflow that demonstrates all four primitives: identity verification, cost prediction, gate checks, and knowledge loading. This is Minutes 2-3 of the demo.

**Target output format**:
```
$ nunchi audit deployment payments-svc --rev=abc123 --policy=prod-sec
◆ audit · payments-svc@abc123 · policy: prod-sec
│
├─ identity   auditor@v1 · nhi://acme/auditor.v1 (verified)
├─ predict    $0.043 · 12.4s · route: haiku → gpt-4o-mini
├─ gates      pii_scan ✔  cost_ceiling<$0.10 ✔  sox_compliance ✔
├─ knowledge  loaded 7 facts from /security/payments (3 agents, 0.91 avg conf)
│
│  [1/8] scanning dependencies...          ✔ 2.1s
│  [2/8] checking auth endpoints...        ✔ 3.4s
│  [3/8] auditing secrets management...    ⚠ leaked AWS_SECRET_KEY in config.yaml
│  [4/8] reviewing access controls...      ✔ 1.8s
│  [5/8] validating rate limiting...       ✖ PANIC (pre-seeded failure)
│
├─ actual     $0.031 (−28% vs predicted) · routed to haiku
├─ deposited  2 new facts → /security/payments
│
└  audit incomplete · 5/8 steps · 1 finding · 1 failure
```

**Checklist**:
- [ ] Create `audit` subcommand in the CLI
- [ ] The command can be a SCRIPTED DEMO — it doesn't need real audit logic. What matters is the OUTPUT FORMAT showing all four primitives.
- [ ] Option A (recommended for demo): Pre-recorded output played back via `demo-magic` or a Rust script that prints lines with realistic delays
- [ ] Option B (ideal but more work): Wire into actual `plan run` with a pre-authored `tasks.toml` that defines 8 audit steps
- [ ] Include pre-seeded failure on step 5 (the demo shows crash recovery)
- [ ] Show identity line with `nhi://` URI
- [ ] Show prediction line with cost estimate, latency estimate, model route
- [ ] Show gates passing
- [ ] Show knowledge loaded from store
- [ ] Show actual-vs-predicted cost delta
- [ ] Show "deposited N new facts" line
- [ ] Use Clack-style formatting throughout (◆ ├─ └ ✔ ✖ ⚠)

**Files**: New file `crates/roko-cli/src/demo.rs` or `crates/roko-cli/src/audit.rs`

---

### P0-4: `nunchi resume` command (crash recovery)

**What**: After the Ctrl+C kill in Minute 3, resume the audit from where it left off. This is Minute 4 — the "Temporal moment."

**Target output**:
```
$ nunchi resume run_4823
◆ resuming run_4823 from event 47/52
│
│  [5/8] validating rate limiting...       ✔ 1.2s (retried)
│  [6/8] checking TLS configuration...     ✔ 0.9s
│  [7/8] credential rotation...            ✔ vault rotated AWS_SECRET_KEY
│  [8/8] opening remediation PR...         ✔ PR #847 opened
│
├─ attestation  bundle hash: 0x7a3f...9e2b
│
└  audit complete · 8/8 steps · 1 finding remediated
```

**Checklist**:
- [ ] Check if `roko plan run --resume` already works (it should — session persistence is wired)
- [ ] If yes, create `nunchi resume <run_id>` as alias for `roko plan run --resume .roko/state/executor.json`
- [ ] If the existing resume doesn't produce clean output, create a scripted version for demo
- [ ] The resume must visually show it's starting from a CHECKPOINT, not from scratch
- [ ] Show the event count (47/52) to prove state was preserved
- [ ] Show remaining steps completing
- [ ] End with attestation hash (cosmetic — signals auditability)

**Files**: `crates/roko-cli/src/main.rs`, `.roko/state/executor.json`

---

### P0-5: `nunchi replay` command (audit trail)

**What**: Stream the JSON audit trail for a completed run. This is Minute 5 — proves every decision is recorded and replayable.

**Target output**:
```
$ nunchi replay run_4823 --as-of="step 05" | head
{"event":47,"agent":"auditor@v1","identity":"nhi://acme/auditor.v1","tool":"rate_limit_check","args":{"endpoint":"/api/v2/transfer"},"policy":"prod-sec","decision":"allow","attestation":"0x7a3f..."}
{"event":48,"agent":"auditor@v1","identity":"nhi://acme/auditor.v1","tool":"vault_rotate","args":{"key":"AWS_SECRET_KEY"},"policy":"prod-sec","decision":"allow","attestation":"0xa1b2..."}
...
```

**Checklist**:
- [ ] Create `replay` subcommand
- [ ] Read from `.roko/episodes.jsonl` or the executor state
- [ ] Filter by `--as-of` (step number or event number)
- [ ] Output as JSON lines showing: event number, agent identity, tool called, args, policy decision, attestation hash
- [ ] Pipe to `head` for demo (don't flood the screen)

**Files**: `.roko/episodes.jsonl`, `crates/roko-cli/src/main.rs`

---

### P0-6: Pre-warm LLM cache for demo prompts

**What**: The demo must run against cached LLM responses to eliminate network dependency, non-determinism, and cold starts.

**Checklist**:
- [ ] Identify all LLM calls the demo will make (audit steps, enrichment, etc.)
- [ ] Run the full demo flow 3x against real APIs and capture responses
- [ ] Store responses in a local cache directory (`.roko/demo-cache/` or similar)
- [ ] Modify the agent dispatcher to check cache first, fall through to real API on miss
- [ ] OR use a local HTTP proxy (e.g., `mitmproxy` or a custom Rust proxy) that serves cached responses
- [ ] OR use `demo-magic` to script the entire output (simplest, most reliable)
- [ ] Verify: demo runs identically with no internet connection

---

### P0-7: Demo backup tiers

**What**: Three backup levels in case the live demo fails.

**Checklist**:
- [ ] Record the demo using `asciinema rec demo.cast` (exact same flow as live)
- [ ] Convert to `vhs`-rendered MP4 for Keynote embedding
- [ ] Also keep the `.cast` file for `asciinema play` fallback
- [ ] Take 6-10 annotated screenshots of key moments
- [ ] Embed the MP4 as a "slide 7.5" in Keynote (between "Let me show you" and traction)
- [ ] Practice switching to backup mid-demo (establish a kill word with any co-presenter)
- [ ] Load Loom/QuickTime backup locally (no internet required)

---

## Priority 1: Strong-to-Have for Meeting

### P1-1: Fix TUI streaming (shows agent activity live)

**What**: Currently the TUI is blind during agent execution — no streaming output, no live token counts. If Casado opens the TUI dashboard, it should show activity.

**Checklist**:
- [ ] Call `emit_server_event(ServerEvent::AgentOutput { ... })` after each dispatch in `orchestrate.rs`
- [ ] Add `--output-format stream-json` flag to Claude CLI spawns
- [ ] Parse per-line JSON from agent stdout during execution
- [ ] Emit `DashboardEvent::EfficiencyUpdate` after each dispatch with tokens/cost
- [ ] Embed model name in `AgentSpawned` event so TUI shows it immediately

**Files**: `crates/roko-cli/src/orchestrate.rs` (search for `dispatch_agent_with`), `crates/roko-agent/src/dispatcher/mod.rs`
**Reference**: `tmp/dogfood/05-mori-vs-roko-agent-wiring.md` sections 1-3

---

### P1-2: Fix TOML markdown fence stripping

**What**: LLMs wrap TOML output in markdown fences (```toml ... ```). Parser chokes. Simple fix.

**Checklist**:
- [ ] Add `strip_code_fences(input: &str) -> String` function
- [ ] Call it before any TOML parse in the enrichment path
- [ ] Test: `"```toml\n[task]\nname = \"test\"\n```"` → `"[task]\nname = \"test\""`

**Files**: Search for `toml::from_str` in `orchestrate.rs`

---

### P1-3: Memory leak investigation

**What**: 9.5GB RSS after 17 minutes with only 3 enrichment dispatches. Likely enrichment artifact strings held in TaskTracker with no GC.

**Checklist**:
- [ ] Run with DHAT profiler: `cargo run --features dhat-heap`
- [ ] Identify largest allocations
- [ ] Likely fix: clear enrichment artifacts after they're applied to tasks
- [ ] Verify: RSS stays under 500MB for a 30-minute run

---

## Priority 2: Polish for Meeting Week

### P2-1: Deck as PDF

**Checklist**:
- [ ] Build 13 slides in Figma or Keynote
- [ ] Export to PDF
- [ ] Send to meeting contacts 24-48h ahead (Friday May 1 or Saturday May 3)
- [ ] Include 1-page exec summary as separate attachment
- [ ] Never put behind DocSend

### P2-2: Pre-read memo (2,000 words)

**Checklist**:
- [ ] Write 10-section memo per R15 Kirwin template
- [ ] Send as Google Doc link Friday May 1, 6pm PT
- [ ] Include PDF backup and deck as separate attachment

### P2-3: Design partner outreach

**Checklist**:
- [ ] Contact Hebbia (a16z portfolio, Casado warm intro path via Immerman)
- [ ] Contact Harvey (Pereyra direct, LinkedIn)
- [ ] Contact Decagon (a16z portfolio, Casado warm intro)
- [ ] Get at MINIMUM verbal confirmation of "in conversation" for each
- [ ] Ideal: signed LOI or DPA from one

### P2-4: Landing page updates

**Checklist**:
- [ ] Replace mock data (84,213 / 12,425 / 3,240) with real or remove
- [ ] Add `/changelog` page with 3-4 recent shipped items
- [ ] Add `/docs` page (even if minimal)
- [ ] Update hero to reflect Agent Coordination Plane framing
- [ ] Ensure no "trust layer" language remains
- [ ] Add the `nunchi run` CLI output somewhere visible

---

## Pre-Flight Checklist (May 4-5)

- [ ] Confirm meeting location with EA (Sand Hill or Townsend)
- [ ] Get written attendee list
- [ ] Pack: MacBook, HDMI dongle, USB-C dongle, USB-C drive with deck PDF, LTE hotspot
- [ ] Second laptop with demo mirrored
- [ ] Run demo 5x on actual hardware + adapter
- [ ] Disable: display sleep, notifications, auto-updates, Spotlight indexing
- [ ] Pre-warm LLM cache
- [ ] Check Casado's X feed morning of
- [ ] Check for competitor funding announcements
- [ ] Prepare one-liner for DeepSeek promo expiry (May 5) and OpenAI Workspace Agents (May 6)
- [ ] Thank-you email drafted (fill in specifics after meeting)
- [ ] 8-12 customer references ready to attach
- [ ] ARR definition documented, cohort table prepared
