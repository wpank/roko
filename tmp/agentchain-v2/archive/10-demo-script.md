# Demo Script

The complete playbook for demonstrating Nunchi. Hardware and software setup, the 3-minute general VC demo, the 5-minute a16z-specific R15 variant, the side-by-side @roko vs @cursor demo, the four compounding chains (B, D, A, C), the five-layer recovery stack, and scripted responses to every likely objection. Someone with no prior knowledge should be able to read this document, set up the environment, and deliver the demo.

---

## 1. Pre-Demo Setup

### Hardware

- **Machine:** MacBook Pro. Use the primary development machine, not a secondary laptop. The demo relies on a warm cache and pre-loaded state.
- **Font:** Berkeley Mono ($75 license, variant TX-02), 24–28pt for demo presentations. Says "I read the manual; I have taste." Compare to Geist Mono — fine for web apps, wrong for systems software. Fallback: JetBrains Mono (free, excellent) for benchmark widgets and code displays where licensing is a concern.
- **Theme:** Tokyo Night on `#1A1B26` base background. The specific palette below.
- **Terminal:** Ghostty (Mitchell Hashimoto's GPU-rendered terminal). Zero frame drops during demos. Renders via Metal/Vulkan, so scroll, resize, and high-throughput output all stay at native refresh rate.
- **Symbols:** Clack-style box-drawing — `◆ ◇ │ └ ✔ ✖ ⚠ ℹ ❯ → dots spinner`. **No emoji. Ever.** The Clack symbols (from natemoo-re's Clack CLI framework) communicate structure without being cute.
- **Color accent constraint:** off-limits as primary accents — **green** (Supabase owns it), **orange** (Replit and Hacker News). Nunchi's primary accent is **cyan/blue** (`#7DCFFF` / `#7AA2F7`).

**Tokyo Night palette:**

| Role | Hex | Usage |
|---|---|---|
| Background | `#1A1B26` | Terminal background, all dark surfaces |
| Foreground | `#C0CAF5` | Default text |
| Blue | `#7AA2F7` | Primary accent, links, active states |
| Purple | `#BB9AF7` | Secondary accent, agent identifiers |
| Cyan | `#7DCFFF` | Highlights, success states, Roko brand |
| Red | `#F7768E` | Errors, failures, cost warnings |
| Green | `#9ECE6A` | Pass indicators, completion states |
| Yellow | `#E0AF68` | Warnings, in-progress states |

### Software

Two servers must be running before the demo begins:

1. **Roko control plane:** `roko serve` running on port 6677. ~85 REST routes. Serves the dashboard, SSE streams, WebSocket connections.
2. **Web frontend:** `npm run dev` running on port 5173. Vite dev server for the demo web interface.

**Pre-load three browser tabs:**
- Tab 1: `http://localhost:5173` (demo web UI)
- Tab 2: `http://localhost:6677/api/health` (control plane health check — verify 200 before starting)
- Tab 3: The shareable URL generated during cache warming (pre-generate and keep ready)

### Cache warming

This is critical. The demo depends on fast response times. Cold inference calls take 15–30 seconds. Warm cache hits take 2–5 seconds.

**Procedure.** Run each demo prompt twice before the meeting:

```bash
# Beat 1: Identity and gates
nunchi run agents/researcher.py --task "Summarize Q3 fintech earnings"

# Beat 3: Shared knowledge (run after Beat 1 to populate knowledge store)
nunchi run agents/analyst.py --task "Analyze fintech earnings trends"

# Beat 4: Durability (run, Ctrl+C, resume)
nunchi run agents/writer.py --task "Draft fintech earnings report"
# Ctrl+C after 5 seconds
nunchi run --resume
```

After each pair, verify the second run completes in under 10 seconds. If any run takes longer than 10 seconds on the second attempt, investigate cache or network before proceeding.

### Live benchmark widget

Bloomberg Two-Tape style widget runs in a 400×300 pixel overlay in the corner of the screen during the entire demo. Live-updating cost comparison:

- **Left tape:** Naive execution cost (frontier model, no routing, no cache) — steadily climbing.
- **Right tape:** Nunchi execution cost (CascadeRouter, cached context, shared knowledge) — climbing much slower.
- **Footer:** `COST RATIO 31.6x · p<0.01 · ROKO WINS`.

Runs continuously in the background. Do not draw attention during the 3-minute demo. Reference during Minute 2 (cost prediction) of the 5-minute version. The visual impression of the two tapes diverging is more persuasive than any stated number.

---

## 2. The R15 Casado Demo (5 Minutes)

This supersedes the R12 three-minute script for the Casado meeting specifically. The R12 script remains valid for general VC meetings (see section 3). The R15 concept is tailored to Casado's portfolio, April 2025 skepticism, and Aubakirova's Chronicle Detect + Keycard context.

### Why `nunchi audit`, not the researcher task

R15 analysis eliminated three previously-considered demo topics:

- **Production alert triage** — overlaps with Resolve.ai (GREYLOCK, NOT a16z portfolio). Spotlighting a Greylock company in a Casado pitch is a strategic error.
- **CI failure** — overlaps with Cursor, Casado's $50B bet. Positioning against his own portfolio company signals poor research.
- **Finance summarization** — off-domain for Casado's infrastructure thesis.

`nunchi audit deployment payments-svc` hits both Casado's board adds in security/infrastructure (Kong, Truffle, Pindrop) and Aubakirova's Chronicle Detect + Keycard portfolio (audit trail, identity, policy enforcement).

### Opening line (verbatim)

Deliver as the laptop opens:

> *"Martin, you wrote that we can't yet close the control loop on agents. That's exactly why we built Nunchi as the control plane. Five minutes."*

Addresses his April 2025 skepticism head-on before he raises it. Signals the founder has read his work and frames the demo as a direct response — not a product tour.

### Five-minute flow

#### Minute 1: Identity and attestation

```
$ nunchi agents list --env=prod
```

Output shows SPIFFE / ERC-8004 identities, attestation status, last-seen timestamps, and policy assignments for each agent. Point to the identity column:

> *"Every agent has a verifiable machine identity. Default-off. Nothing runs without attestation."*

(Use "machine identity" — Aubakirova's term — not "non-human identity" or "NHI.")

#### Minute 2: Policy-gated audit run

```
$ nunchi audit deployment payments-svc --rev=abc123 --policy=prod-sec
```

Eight parallel audit steps fire. Step 3 finds a leaked AWS secret in the deployment manifest. The system flags it, records it in the episode log, and does NOT continue past the violation.

> *"The agent didn't close the loop — the coordination plane did."*

#### Minute 3: Pre-seeded failure, Ctrl+C

Step 5 panics (pre-seeded). Visibly Ctrl+C the terminal. Let the room sit with a dead terminal for two seconds.

#### Minute 4: Resume and remediation

```
$ nunchi resume run_4823
```

The workflow recovers from event 47 of 52. The leaked credential is rotated. A PR is opened automatically with the remediation diff.

> *"Zero work lost. The audit trail picked up exactly where it stopped."*

#### Minute 5: Replay and audit trail

```
$ nunchi replay run_4823 --as-of="step 05"
```

Streams the full JSON audit trail from step 5 forward. Every decision, every agent action, every policy evaluation — cryptographically timestamped and replayable.

> *"This is what you hand the compliance officer when they ask what happened on March 15th."*

### Close

Say exactly:

> *"Same primitives — identity, policy, replay — work for triage, migrations, anything. The agent didn't close the loop. We did."*

### Tooling

- **Scripted keystrokes:** Use `demo-magic` for reproducible keystroke playback. No live typing during the critical path.
- **Video backup:** `vhs`-rendered MP4 as Keynote fallback slide. Record the week before the meeting.
- **Terminal:** Ghostty with Tokyo Night palette.

---

## 3. The R12 General VC Demo (3 Minutes)

The default demo for any non-Casado investor meeting. Five beats, tightly scripted. Each beat demonstrates one primitive. Practice until you can deliver in exactly 3 minutes.

### Setup architecture (R12)

`nunchi` CLI binary against local Docker controller. All LLM responses from pre-warmed cache for demo prompts. Cache miss falls through to real APIs. Eliminates non-determinism, rate limits, cold starts. The demo is always fast, always deterministic, always available.

### The CLI output

This is the exact terminal output the investor sees. Every line carries a primitive: identity, prediction, gates, shared knowledge, actual-vs-predicted delta, deposit.

```
$ nunchi run agents/researcher.py --task "Summarize Q3 fintech earnings"
  ▶ agent      researcher@v2  ·  nhi://acme/researcher.v2  (verified)
  ▶ predict    $0.043  ·  12.4s  ·  route: haiku → gpt-4o-mini
  ▶ gates      pii_scan ✔   cost_ceiling<$0.10 ✔   sox_compliance ✔
  ▶ knowledge  loaded 7 facts from /finance/q3 (3 agents, 0.91 avg conf)
  running ━━━━━━━━━━ done in 9.8s
  ▶ actual     $0.031  (-28% vs predicted)  ·  routed to haiku
  ▶ deposited  2 new facts → /finance/q3
```

### Beat 1: Identity and Gates (0:00 – 0:30)

Run the first command. Terminal prints the agent line and gates line immediately.

Point to the output:

> *"Every agent has a verified machine identity — `nhi://acme/researcher.v2`. Before the agent spends a single token, three gates fire: PII scan, cost ceiling, SOX compliance. Default-off. Nothing runs without passing policy."*

**Timing note:** beat must complete in 30 seconds. If gates take longer, keep talking through the identity line and let gates complete in the background.

### Beat 2: Predict-Publish-Correct Delta (0:30 – 1:15)

Point to the predict line:

> *"$0.043 predicted. Now watch."*

The agent runs. Progress bar fills. When complete, point to the actual line:

> *"$0.031 actual. 28% below prediction. The system predicted the cost before execution and published the prediction. After execution, the actual cost is recorded and the delta is computed. Every run improves the predictor."*

> *"This is not monitoring. This is prediction. The system knows what a task will cost before it runs, routes to the cheapest model that can handle it, and self-corrects."*

### Beat 3: Shared Knowledge Across Two Agents (1:15 – 2:15)

Run the second command — different agent, same domain:

```
$ nunchi run agents/analyst.py --task "Draft Q3 fintech earnings brief for CRO"
  ▶ agent      analyst@v1  ·  nhi://acme/analyst.v1  (verified)
  ▶ predict    $0.038  ·  10.1s  ·  route: haiku
  ▶ gates      pii_scan ✔   cost_ceiling<$0.10 ✔   sox_compliance ✔
  ▶ knowledge  loaded 9 facts from /finance/q3 (4 agents, 0.93 avg conf)
  running ━━━━━━━━━━ done in 7.2s
  ▶ actual     $0.022  (-42% vs predicted)  ·  routed to haiku
  ▶ deposited  1 new fact → /finance/q3
```

Point to the knowledge line:

> *"9 facts loaded. The first agent deposited 2 new facts. This agent — a completely different agent — picked them up automatically. It loaded 9 facts from 4 agents with 0.93 average confidence."*

Cost comparison:

> *"First agent: $0.031. Second agent, building on the first agent's knowledge: $0.022. That is a 64% cost reduction versus running both agents naive."*

**Hand the laptop to the partner for this command** if you can. Let them type the task or pick from a card. The pre-warmed cache covers a range of prompts in the /finance/ domain. Always fast.

**The Stripe/Collison pattern.** Patrick Collison closed Stripe's early rounds by handing investors a laptop and letting them process a real payment in under 60 seconds. The equivalent here is handing the investor the laptop and letting them run a 9-line snippet:

```bash
nunchi init
nunchi run agents/analyst.py \
  --task "Draft Q3 fintech earnings brief for CRO" \
  --share
# Output:
#   ▶ agent      analyst@v1  ·  nhi://acme/analyst.v1  (verified)
#   ▶ predict    $0.038  ·  10.1s  ·  route: haiku
#   ▶ gates      pii_scan ✔   cost_ceiling<$0.10 ✔
#   ▶ actual     $0.022  (-42% vs predicted)
#   ▶ share      https://nunchi.dev/runs/abc123
```

The shareable URL is the artifact that leaves the room. It is what they forward internally. The experience must be under 10 seconds from cache.

### Beat 4: Kill and Resume — Temporal's Move (2:15 – 2:35)

Start a third command. Ctrl+C mid-run. Wait two seconds. Re-run with `--resume`:

```
$ nunchi run agents/researcher.py --task "Compare Q3 vs Q2 fintech margins" --resume
  ▶ resuming from checkpoint 3/7  ·  $0.012 spent  ·  4 steps remaining
  running ━━━━━━━━━━ done in 4.1s
  ▶ actual     $0.029  ·  routed to haiku
```

> *"We killed it mid-run. It resumed from the last checkpoint. Zero work lost. Zero tokens wasted."*

This is Temporal's signature move applied to agent workloads. The investor has seen this pattern before — that is the point.

**Theatrical note.** The Ctrl+C must be visible and deliberate. Do not be subtle. The two-second pause is intentional: it creates tension before the resume resolves it.

### Beat 5: The Stripe Moment (2:35 – 3:00)

```bash
nunchi run --share
```

A URL appears in the terminal. Shareable link to the full execution timeline.

> *"That URL has the full execution timeline, cost breakdown, and ZK proof. Open it on your phone."*

**Pause here.** Let at least one person in the room pull out their phone and open the URL. This is the "Stripe moment" — the first time Stripe demoed, they showed that 7 lines of code processed a real payment. The audience opened their phones and saw the charge appear. The emotional beat is the same.

### Close (3:00)

> *"Identity, prediction, shared memory, durability. Four primitives. Every multi-agent company will need them within 18 months. Everything you just saw is open source, 177,000 lines of Rust, running on your laptop right now."*

Pause. Let the room absorb. Do not rush to the next slide.

---

## 4. The Side-by-Side @roko vs @cursor Demo (R8)

The single most effective competitive artifact for enterprise procurement conversations. Documented by Cursor forum thread /158505: *"It used to preserve the session and only create new one with @cursor agent. Right now, I see a session per @cursor in the same issue. This breaks the context usage... Cursor is unusable."*

### Demo flow

1. Create two identical Linear issues.
2. Trigger `@roko` on one, `@cursor` on the other.
3. Roko emits a `thought` activity within 10s, drives the LLM round-trip, updates the issue cleanly.
4. Cursor spawns duplicate sessions, loses context across follow-ups.
5. Record both side-by-side.

**Timing.** Ship after Linear adapter v1 is complete (weeks 3–5 of 90-day plan). The recording doubles as Langfuse partnership blog content.

**Frame Cursor as portfolio company, not competitor.** Cursor remains an a16z portfolio company. Frame this gap-filling as complementary, never competitive. The honest framing: *"Cursor's Linear integration breaks predictably under load, no Rust-native alternative — Roko fills that gap."*

---

## 5. The Four Compounding Chains (R6)

Each chain should be presented as a compounding sequence, not independent features.

### Chain B: The Lead Demo (Linear → PR → CI → Linear)

The chain: Linear webhook → Roko plan → GitHub PR → CI passes → Linear status updated.

**Named precedents:** Devin and Cosine Genie 2 ship this loop closed-source. Cursor ships it broken. Sweep never shipped it. **Cognition publicly reports 659 Devin PRs merged in their best week** — magnitude of the prize.

**Roko's delta:** open-source, on-prem-able, sub-10s response window via Rust runtime (Linear enforces 10-second AgentSession response), zero seat cost in Linear.

**Demo script.** Trigger a Linear issue creation via the UI or API. Show Roko's webhook adapter receive the event, emit a `thought` activity within 100ms (visible in terminal), generate a plan, create a GitHub PR, wait for CI, update the Linear issue to "complete." The sub-100ms thought emission is the engineering moat moment — call it out explicitly.

### Chain D: The Killer Demo (Slack Thread → Agent → Trace URL)

**No closed-source competitor ships this.** Implementation: ~20 minutes of `recipe.toml` work using `slack-morphism` (Rust, MIT, 1.84M+ downloads on crates.io) socket-mode bot + the existing `genai-rs/opentelemetry-langfuse` Rust crate (caveat per R8: bus-factor 1, hobby scale; consider `opentelemetry-otlp` directly with basic-auth header from env vars).

**What the audience sees.** A Slack message triggers an agent. The agent executes, produces a trace, replies in the same Slack thread with a Langfuse public-share trace URL showing the entire agent decision tree: which tools were called, which model, how many tokens, latency per span.

**The narrative:** *"Cursor and Devin show you what happened. Roko shows you why, with receipts."*

**Implementation detail.** Langfuse partnership makes this free to demo. Langfuse is MIT-licensed, 50K observations/month free tier, no card required.

**Timing.** Ship during weeks 5–7 of the 90-day plan. Co-publish the Langfuse partnership blog post in the same week as a Roko Week launch for amplification.

### Chain A: Sentry Error → Plan → PR → OTel Trace → Linear Closed

Closest analog is Sentry Seer Autofix (closed, paid add-on). Seer ships through GitHub but **does not close Linear** and does not emit `gen_ai.*` spans for its own decisions. Roko's delta: the agent's plan/tool-use itself becomes a span attached to the originating trace ID, closing the loop with "this plan fixed this exact span." Time-to-value with pre-wired `recipe.toml`: ~20 minutes.

### Chain C: GitHub Label → Plan → PR → Slack Approval → Merge

Sweep's `sweep` label trigger is the strongest precedent (7.4k stars). Cursor's Slack integration handles approval but not label triggers. Roko combines both into one recipe; nobody else does.

### Chain E: Recipe-as-Template Composition

Validated by `terraform-aws-modules/eks/aws` — 139.9M total downloads, 10.3M YTD 2026, 1.2M this month. The lesson: one canonical recipe shipping submodule chains beats N independent recipes. Roko's parallel: ship one canonical "code-fix-loop" recipe with sub-recipes for Sentry-trigger / Linear-trigger / GitHub-label-trigger variants under one namespace.

### Demo narrative framing — present as compounding sequence

1. **Open with Chain B** (Linear → PR): *"This is what Devin charges $500/month for. Roko does it open-source with a 10-second response window Devin can't match."*
2. **Layer Chain D** (Slack → trace): *"Now watch: I trigger the same flow from Slack, and the trace URL appears inline. Nobody else ships this."*
3. **Reference Chain A** (Sentry → plan): *"When an error happens, the same system produces a fix and links the trace to the error span. Full circle."*
4. **Close with Chain E** (composition): *"Each of these is a recipe. They compose like Terraform modules. One namespace, multiple triggers."*

The compounding narrative is more powerful than showing features individually because it demonstrates the "coordination plane" thesis — the same infrastructure handles multiple trigger surfaces.

---

## 6. The 5-Task HAL Subset for Live Demo

Pure-Python tasks only — no Docker cold-starts on stage. Total wall-clock: ≤3 min with parallel=5.

| # | Bench | Task | Why this one | Time / cost (Roko / LG-style) |
|---|---|---|---|---|
| 1 | tau-bench Airline | task_0 (book a flight, simple constraint) | Customer-service archetype; deterministic user sim | ~20s / $0.02 vs $0.30 |
| 2 | tau-bench Airline | task_4 (multi-leg + loyalty rules) | Forces multi-step tool calls — where wasteful agents loop | ~30s / $0.05 vs $1.20 |
| 3 | AppWorld test_normal | "Play playlist for workout" (Fig. 1, arXiv:2407.18901) | Iconic, headline-friendly, multi-API | ~40s / $0.10 vs $4.00 |
| 4 | GAIA Level-1 | arXiv physics paper "society" question | No-tools retry-baseline; simple QA | ~20s / $0.01 vs $0.40 |
| 5 | GAIA Level-1 | "List vegetables from grocery list" (botany-mom task) | Diversity: textual reasoning | ~15s / $0.01 vs $0.20 |

Realistic GPT-4o pricing: Roko warming/escalation ~$0.20 total vs LangGraph/AutoGen-style scaffold ~$6–8 → ~30x ratio preserved on this small mix.

**Benchmarks NOT viable for live demo:** SWE-bench Verified (Docker spin-up takes minutes, 120GB disk, arm64 unsupported), WebArena (heavy self-hosted Docker stack), OSWorld (full VM required).

### Live benchmark widget — Bloomberg Two-Tape (recommended)

Two mirrored panels — Roko (cyan `#06B6D4`) | LangGraph (orange `#F97316`). Each panel shows:
- `$1.42` / `$44.86` (JetBrains Mono 36pt)
- `4/5 PASS` indicator
- Progress bar `▰▰▰▱▱ 50%`
- 60×24 cost sparkline

**Footer:** `COST RATIO 31.6x · p<0.01 · ROKO WINS`.

**Animated elements:** cost ticks up via Weave stream every 200ms; sparkline rolling 30s window; pass icons populate left-to-right; ratio recomputes per-task.

**Implementation pipeline:**
```
agent → litellm callback → Weave logger → tail JSON → Node WebSocket bridge (FastAPI ~40 lines) → React widget @ ws://localhost:8765/stream
```

Statistical significance via sequential proportion z-test: `proportion_z_test(roko_pass, n, lg_pass, n)`; declare WINNER at p<0.01.

---

## 7. Recovery Stack — Five Layers (R12 / R14 Updated)

Things will break during live demos. This is not a contingency plan. It is a certainty plan. Five layers of recovery, each one catching failures the previous layer missed. Practice transitioning until seamless.

### Layer 1: Live demo on primary laptop + LTE hotspot

The `nunchi` CLI against local Docker controller with cached LLM responses. Primary network is the personal LTE hotspot, NOT conference room WiFi. Second laptop mirroring the same demo environment as hot standby. Cache miss falls through to real APIs, so even uncached prompts work — they just take longer.

### Layer 2: Pre-recorded Loom loaded locally as QuickTime

NOT streamed from Loom's servers — downloaded and stored as a local QuickTime file on both laptops. Introduced as: *"Let me show you a recent run."* Record the week before the meeting, not the night before.

### Layer 3: Annotated screenshots in deck

Static screenshots of each beat's terminal output, embedded in appendix slides of the PDF deck. Each screenshot has annotations pointing to key numbers (cost, routing decision, gate outcome). Works even if both laptops fail and the projector is displaying a PDF from a USB drive.

### Layer 4: Whiteboard architecture by hand

Draw the layer cake (identity / execution / coordination), the CascadeRouter flow, and the gate pipeline on a whiteboard. The nuclear option. Demonstrates architectural fluency and works with zero technology. Casado's PhD is systems architecture — he will respect a clean whiteboard drawing more than a broken laptop.

### Layer 5: Pre-Generated Shareable URL

During cache warming, run `nunchi run --share` and copy the generated URL. Open it on your phone before the meeting. Verify it loads. Use when everything else has failed but you still have internet on your phone. Hand your phone to the nearest partner, or text/AirDrop the URL.

### Failure contingencies

| Failure mode | Mitigation |
|---|---|
| WiFi drops mid-demo | Pre-warmed cache means no network needed for demo prompts. Cache miss → Tier 2. |
| Laptop fails | Loom video on phone. Lightning-to-HDMI adapter in the bag. |
| LLM API timeout / rate limit | Pre-warmed cache absorbs all demo calls. Should never happen for demo prompts. |
| Agent takes longer than expected | Cache miss → "let me show you the cached version" → switch to Tier 2. |
| Chain RPC down | mirage-rs is local. No external dependency. |
| Investor asks to modify the task | Cache covers a range of /finance/ domain prompts. Pick closest match. If nothing matches, acknowledge and run live — real API fallback works. |
| Casado asks to type his own prompt | This IS the plan. Hand him the laptop. If prompt misses cache, real API response will take 5–15s — say "this one is hitting the live API, not cache" and let it run. Honesty is the play. |
| Container crashes mid-demo | This IS Beat 4. Let it happen, then show resume. If unintentional, treat as unplanned Beat 4 and keep going. |

### Demo god-mode safeguards

- Auto-skip + mark fail if a task hangs >45s (widget never freezes).
- Pre-pull Docker images, pre-download AppWorld data, pre-load GAIA dataset before pitch — but start clocks at zero on stage.
- Pre-record 90-sec MP4 fallback in case Wi-Fi dies.
- Pareto-frontier static image as backup-of-backup.

---

## 8. Objection Handling

These objections will surface. Practice the responses until they sound natural, not scripted.

### "Isn't this just Keycard?"

**Most likely from:** anyone who has read Aubakirova's Keycard investment essay and sees identity as Nunchi's primary feature.

**Response:**
> *"Keycard and Nunchi operate at different stages of the kill chain. Keycard handles identity issuance: who is this agent, what can it do, on whose behalf. Nunchi handles execution validation and settlement: did the agent do what it claimed, is the output correct, can you verify it cryptographically. They are complementary layers. We consume Keycard tokens as input."*

**Fallback (if they push):**
> *"Martin, you built Nicira to decouple the control plane from the data plane. Keycard is the identity data plane: it issues tokens. Nunchi is the control plane: it enforces policy across the fleet. Same architecture, different layers."*

### "This will not scale"

**Most likely from:** technical evaluators or partners with infrastructure backgrounds.

**Response:**
> *"177,000 lines of Rust, not Python. The gate pipeline validates at compile time, not runtime interpretation. The executor runs parallel task graphs with dependency resolution. The checkpoint system is append-only JSONL with sub-millisecond writes. We are building for the same performance envelope as Temporal, which runs billions of workflows per month."*

### "This is a feature, not a company"

**Most likely from:** Matt Bornstein or any partner skeptical of infrastructure-layer companies.

**Response:**
> *"Temporal proved that durable execution is a category, not a feature of every workflow engine. Before Temporal, every team built their own retry logic, state persistence, and failure recovery. After Temporal, they stopped. Agent coordination is the same pattern: every team is building their own routing, gates, and knowledge persistence. They will stop when infrastructure exists."*

### "Why should I believe a solo founder?"

**Most likely from:** anyone evaluating team risk.

**Response:**
> *"The system develops itself. Watch."*

Then demonstrate the self-hosting workflow: `roko prd idea`, `roko prd draft`, `roko prd plan`, `roko plan run`. The system reads its own PRDs, generates its own implementation plans, executes them with agents, validates with gates, persists results.

**Backup (if they want human team context):**
> *"177,000 lines of working Rust, 18 crates, 85 HTTP routes, a ratatui TUI, and the system develops itself. The bottleneck is not engineering headcount. It is the quality of the coordination plane, and the coordination plane is the product."*

### "Show me what happens when a tool call fails — right now"

Most likely R14 question. **Pre-script the failure into the demo.** Before they ask, trigger a tool-call failure in Beat 2 or Beat 3 (malformed API response, file-not-found error). Show the gate pipeline catching it, the CascadeRouter retrying on a different model tier, and the episode log recording the failure with a cost delta. **The failure IS the demo, not a bug in the demo.**

### "What does your observability/trace look like?"

Have a trace UI (Langfuse-style) open as a second browser tab during the demo. After the CLI output completes, click to the trace tab and show: per-step cost breakdown, model routing decisions, gate pass/fail at each rung, total wall time, token counts by model tier.

### "How does this work with [Casado portfolio company]?"

He will name Cursor, Convex, Braintrust, or another portfolio company. **Pre-build one integration** with at least one Casado-board company before the meeting. Strongest candidates: Cursor (largest developer surface) or Braintrust (eval traces feed directly into Nunchi's gate pipeline). 30-second walkthrough ready: *"Here is Roko coordinating a Cursor agent session with CascadeRouter active — the routing decisions feed into Braintrust for evaluation. Three portfolio companies in one stack."*

### "What if the model is slow or returns garbage?"

**Tiered retry with escalation:**
1. Retry on the original model with the same prompt.
2. Retry with error feedback appended to the prompt.
3. Retry with a simplified prompt on a smaller/faster model.
4. Route to human queue with full context attached.

Show this in the trace UI: *"Here is a task where the first model returned garbage. The gate pipeline caught it at rung 2, CascadeRouter retried on a different tier, and the task completed at 1.3x the original cost instead of failing."*

### "If I kill your worker right now, what happens?"

**DO IT YOURSELF before they ask.** This is Beat 4 of the demo script — the Temporal "Snakes" pattern. Temporal's signature sales demo was killing a workflow mid-execution to prove durable execution. Nunchi must do the same. Kill the worker visibly (Ctrl+C), wait two seconds, resume with `--resume`. Zero work lost, zero tokens wasted. If you do it before they ask, you own the narrative. If they ask first, you are on defense.

### Productive Tension Bridge (Joel vs Matt)

If both Joel de la Garza and Matt Bornstein are in the room, you will experience productive tension. Joel believes agent identity is the bottleneck (demand-side conviction). Matt believes agents do not really work yet (supply-side skepticism). Do not pick a side. Bridge:

> *"Joel is right about demand. Enterprises are deploying agents and the identity problem is urgent. Matt is right about frameworks. Most agent orchestration tools do not produce reliable output. Nunchi closes the gap: it makes agents work reliably by adding the coordination layer — identity, gates, routing, knowledge, durability — that frameworks are missing. The demand Joel sees is real. The skepticism Matt has is valid. The solution is infrastructure, not better frameworks."*

---

## 9. Match Aubakirova's Empirical Voice

This is not an objection. It is a behavioral note. Aubakirova's communication style is data-dense, measured, and empirical. She presents evidence, not visions.

When speaking with her:

- Use **"agentic inference"** (her term) rather than "agent calls" or "inference."
- Use **"validated paths"** rather than "proofs" or "verification."
- Present numbers before narratives: *"$0.019 actual cost on a $0.043 prediction"* before *"the system optimizes cost."*
- Cite benchmark methodology: *"measured on HAL, the Princeton framework with Weave cost integration."*
- Do not use superlatives. Do not say "revolutionary," "game-changing," or "unprecedented." Say *"30x cost reduction measured on HAL benchmarks."*
- Mirror her **"63,000x faster"** pattern — single, concrete multiplier, plain English, no qualification.

---

## 10. The Critical $44.86 → $1.42 Caveat

The exact `$44.86 → $1.42` figures are NOT directly verifiable in any single paper. They are consistent in spirit with AAM (LATS >50x warming) and EPiC ($9.30 vs $1.55). Two honest options:

1. **Reproduce them locally on the 5-task subset and print actual numbers** (preferred — this is what the live benchmark is for).
2. **Cite as "derived from HAL methodology, arXiv:2407.01502"** rather than as a verbatim paper quote.

**Do not invent precision. Do not take the precision risk into the room.**

**HAL methodology correction (R9).** HAL benchmark costs do not include caching benefits. HAL is therefore an upper bound on production cost, not a production cost estimate.

- HAL lists $44.86 for the benchmark workload (no caching).
- In production with standard caching (80–90% hit rate, Anthropic's 90% cache discount), the same system costs approximately $9–11. That is caching alone: ~4–5x reduction.
- Nunchi's full stack (caching + CascadeRouter routing + gate-based early stopping) brings it to approximately $1.42.
- This is still a 30x improvement over HAL's no-cache baseline, but the intermediate step (caching alone = ~4–5x) must be disclosed.

**Cache hit rate sources (R9 correction):** "92% Claude Code cache hit" is from LMCache (third-party, December 2025), NOT official Anthropic data. Anthropic's closest official figure is 99.8% on a specific internal pipeline (April 23, 2026 postmortem). ProjectDiscovery: 7% → 74% → 84% (their own engineering blog, 9.8B cached tokens). Use these attributions correctly.

---

## 11. Meeting Logistics (R14)

### Location and timing

- **Location:** 2865 Sand Hill Road OR 180 Townsend Street (San Francisco office). Confirm with EA 48 hours out.
- **Duration:** 45 minutes blocked. Plan for 20–25 minutes presented, 20+ minutes Q&A. Do not fill 45 minutes with slides — leave room for the conversation that closes deals.
- **Attendees:** could be 2–10 people in the room. Get a written attendee list from the EA 24 hours prior. Research every name. Know who is the sponsoring partner, who is an associate running diligence, who is a platform team member evaluating technical depth.

### AV and equipment

- MacBook with demo pre-loaded and pre-warmed.
- **HDMI + USB-C dongles** (both — conference rooms vary).
- **USB-C drive** with deck as PDF, demo video as QuickTime, asciicast recording. If laptop dies, plug into any available machine.
- **Cloud link** to deck as PDF (Google Drive or Dropbox, NOT DocSend — friction reads as paranoia).
- **Personal LTE hotspot** charged and tested. Conference room WiFi is unreliable at every VC firm.

### Pre-send (24–48 hours ahead)

- Deck as PDF attachment (never DocSend).
- One-page executive summary: thesis, traction triad, ask, team.
- Demo links (if applicable).
- Do NOT send the full data room pre-meeting. That comes post-meeting if the conversation warrants it.

### Calendar event awareness (May 6, 2026 example)

- **DeepSeek V4-Pro 75% promo expires May 5, 15:59 UTC** (day before meeting). One-liner ready.
- **OpenAI Workspace Agents goes PAID May 6** (same day as pitch). One-liner ready.
- No major conferences before May 6. Google I/O is May 19, Build is June 2. News cycle relatively quiet — DeepSeek and OpenAI headlines get more attention.

---

## 12. Demo Script Summary

| Element | Content |
|---|---|
| Default demo | R12 three-minute script (5 beats: Identity & Gates, Predict-Publish-Correct, Shared Knowledge, Kill & Resume, Stripe Moment) |
| Casado meeting | R15 five-minute `nunchi audit deployment payments-svc` flow |
| Killer competitive demo | Side-by-side @roko vs @cursor on identical Linear issues |
| Lead chain | Chain B (Linear → PR → CI → Linear) — what Devin charges $500/mo for, Roko does open-source with sub-10s response |
| Killer chain | Chain D (Slack thread → Agent → Trace URL inline) — no closed-source competitor ships this |
| Live benchmark | Bloomberg Two-Tape widget, 5-task HAL subset, ~30x ratio preserved live |
| Recovery stack | 5 layers: live + LTE → Loom → screenshots → whiteboard → shareable URL on phone |
| Hardware | MacBook Pro, Berkeley Mono 24–28pt, Tokyo Night, Ghostty, Clack symbols, no emoji |
| Honesty caveats | $44.86 → $1.42 derived from HAL methodology, not verbatim paper quote; cite cache rates correctly (LMCache vs Anthropic) |
| Boundary statements (volunteer in first 10 min) | Temporal, Keycard, Inngest distinctions |
| Opening line (Casado) | *"Martin, you wrote that we can't yet close the control loop on agents. That's exactly why we built Nunchi as the control plane. Five minutes."* |
| Closing line (general) | *"Identity, prediction, shared memory, durability. Four primitives. Every multi-agent company will need them within 18 months."* |
