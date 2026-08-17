# Demo Flow: Nunchi Series A

**Purpose**: Beat-by-beat demo script with exact timing, commands, expected output, what to say, and failure contingencies. Two versions: 3-minute general VC and 5-minute a16z-specific. Written for someone with zero prior context.

**Date**: April 2026

---

## 1. Pre-Demo Setup

### Hardware

- MacBook Pro, fully charged, brightness at maximum
- External display or projector cable verified
- If projecting: use light-background deck slides, dark-background terminal
- If showing on laptop directly: all dark backgrounds are fine
- Terminal font: Geist Mono or Berkeley Mono at 24-28pt
- Terminal theme: Tokyo Night
- Terminal emulator: Ghostty (GPU-rendered, no frame drops)
- No notifications. Airplane mode except WiFi. All other apps closed.

### Software

- `roko serve` running on localhost:6677 (the backend)
- `npm run dev` running for the web dashboard on localhost:5173 (proxies to :6677)
- Browser tab pre-loaded to http://localhost:5173 (dashboard landing page)
- A second browser tab pre-loaded to the shareable URL page (pre-generated)
- Terminal window occupying full screen with one tab for demo commands
- All demo prompts pre-tested within the hour. Cache is warm.

### Pre-Warmed Cache

The demo depends on fast responses. Before the meeting:

```bash
# Run each demo prompt twice to warm caches
nunchi run "Summarize Q3 fintech earnings" --workdir /tmp/demo-1
nunchi run "Draft Q3 fintech earnings brief for CRO" --workdir /tmp/demo-2
nunchi run "Fix the failing test in src/auth.rs" --workdir /tmp/demo-3
nunchi run "Compare Q3 vs Q2 fintech margins" --workdir /tmp/demo-4
```

Verify: second runs should complete in under 10 seconds. If they don't, debug cache configuration before the meeting.

### Backup Plan (Full Recovery Stack)

If the live demo fails, do NOT apologize. Do NOT explain. Execute the recovery stack:

**Layer 1: Pre-recorded Loom** on a second laptop, ready to AirPlay. Indistinguishable from live at meeting distance.

**Layer 2: Wizard-of-Oz Keynote** stepped clickable mock — each click advances to the next terminal "screenshot."

**Layer 3: Scripted post-mortem pivot** — turn the crash into a demo: *"And what just broke is exactly the failure mode our gate pipeline catches. Let me show you the trace."* Then show the replay/audit trail.

**Layer 4: VHS-recorded terminal GIF** pre-embedded in slide 7. GIF for deck, MP4 for Keynote.

**Layer 5: Pre-generated shareable URL** that's already live and works on Casado's phone.

**Infrastructure**: Dedicated mobile hotspot on a different SIM/carrier from laptop tether. Test the complete flow 10 minutes before the meeting, not an hour before (Reprise SE community discipline rule).

**The "I don't know" recovery template** (Tobi Lütke pattern): *"I don't know. I wish I knew. The way I'd think about it is [structured falsification of the easy answer]. We'd find out by [specific experiment, ≤2 weeks]."*

**The Gates BSOD recovery** (Bill Gates, COMDEX 1998): Pre-script a one-line ready-to-fire joke for any failure. Own the moment before it owns you. The Steve Blank "LO" story (first internet transmission crashed after two letters — *"Lo, and behold"*) is the recovery anecdote to deploy if any hard crash happens. Casado will know it.

---

## 2. The 3-Minute General VC Demo

### Setup: Transition from Deck

Slide 7 of the deck is blank or says "Let me show you." Transition to the terminal.

"I'm going to show you four things in three minutes. Identity, cost prediction, shared knowledge, and durability."

---

### Beat 1: Identity and Gates (0:00 — 0:30)

**Type** (or trigger via demo-magic):
```
nunchi run agents/researcher.py --task "Summarize Q3 fintech earnings"
```

**Expected output**:
```
◆ Agent
│  researcher@v2  ·  nhi://acme/researcher.v2  (✔ verified)
│
◇ Predict
│  $0.043  ·  12.4s  ·  route: haiku → gpt-4o-mini
│
◇ Gates
│  ✔ pii_scan       ✔ cost_ceiling<$0.10    ✔ sox_compliance
```

**What to say** (while output appears):

> "Every agent has a verified non-human identity — that's the `nhi://` line. Before the agent spends a single token, three policy gates fire: PII scan, cost ceiling, SOX compliance. Default-off. Nothing runs without passing policy."

**Timing**: Output should appear within 2-3 seconds. If delay, talk through the identity concept while waiting.

---

### Beat 2: Predict-Publish-Correct (0:30 — 1:15)

The agent continues running. A progress bar fills. Then:

**Expected output**:
```
◇ Running
│  ████████████████████████████████████  done in 9.8s
│
◇ Result
│  ✔ $0.031 actual  (-28% vs predicted)  ·  routed to haiku
│  → deposited 2 new facts → /finance/q3
│
└ Share: https://nunchi.network/runs/abc123
```

**What to say**:

> "$0.043 predicted. $0.031 actual. 28% below prediction. The system predicted the cost before execution. After execution, the delta is recorded. Every run improves the predictor. Two new facts deposited to the knowledge store."

Point to the `Share` URL: "That URL is a full execution timeline with cost breakdown. I'll show it in a moment."

**Presentation note (Aubakirova style)**: Her State-of-AI paper, Cinderella thesis, and Continual-Learning piece all reward depth and data over breadth and vibes. Present cost-reduction proof as she demands: empirical, charted, cohort-based, not slideware. Use the phrase "agentic inference" (her exact term). Say "validated paths" not "proofs" (from her Pentesting essay).

---

### Beat 3: Shared Knowledge (1:15 — 2:15)

**Type**:
```
nunchi run agents/analyst.py --task "Draft Q3 fintech earnings brief for CRO"
```

**Expected output**:
```
◆ Agent
│  analyst@v1  ·  nhi://acme/analyst.v1  (✔ verified)
│
◇ Knowledge
│  loaded 9 facts from /finance/q3  (4 agents, 0.93 avg conf)
│
◇ Predict
│  $0.038  ·  10.1s  ·  route: haiku
│
◇ Running
│  ████████████████████████████████████  done in 7.2s
│
◇ Result
│  ✔ $0.022 actual  (-42% vs predicted)  ·  routed to haiku
│  → deposited 3 new facts → /finance/q3
│
└ Share: https://nunchi.network/runs/def456
```

**What to say**:

> "Different agent. Same domain. It loaded 9 facts — the two the first agent deposited, plus 7 from prior agents. First agent: $0.031. This one: $0.022. That's not because we tuned the prompt. It's because the second agent started with knowledge from the first. The thousandth agent starts with knowledge from the previous 999."

**The key phrase**: "The thousandth agent joins smarter than the first." Pause after saying this. Let it land.

---

### Beat 4: Kill and Resume (2:15 — 2:45)

**Type**:
```
nunchi run agents/researcher.py --task "Compare Q3 vs Q2 fintech margins"
```

Wait for the progress bar to reach ~40%. Then visibly press **Ctrl+C**.

**Expected output after Ctrl+C**:
```
◇ Running
│  ████████████████░░░░░░░░░░░░░░░░░░  ^C
│
│  checkpoint saved: step 3/7  ·  $0.012 spent
```

**Pause for 2 seconds. Let the silence sit.** Then type:

```
nunchi run agents/researcher.py --task "Compare Q3 vs Q2 fintech margins" --resume
```

**Expected output**:
```
◇ Resuming
│  from checkpoint 3/7  ·  $0.012 spent  ·  4 steps remaining
│
◇ Running
│  ████████████████████████████████████  done in 5.1s
│
◇ Result
│  ✔ $0.029 actual  ·  routed to haiku
│  → deposited 1 new fact → /finance/q3-vs-q2
```

**What to say**:

> "Killed mid-run. Resumed from checkpoint. Zero work lost. Zero tokens wasted. The cost meter continued from $0.012, not from zero."

---

### Close (2:45 — 3:00)

**What to say**:

> "Identity, prediction, shared knowledge, durability. Four primitives that every production agent deployment needs. The coordination plane."

If time permits, open the shareable URL in the browser. Point to the cost breakdown and ZK proof. But the close is not the URL — the close is the four-word list.

---

## 3. The 5-Minute a16z Demo (Casado Version)

This version is architecturally deeper. It uses Casado's vocabulary (control plane, policy, audit) and maps to his portfolio (Kong, Truffle, Pindrop for security/infra; Cursor, Braintrust for developer tools).

### Setup

Same pre-demo requirements. Additionally:

- Pre-seed a deployment audit scenario in `agents/auditor.py` that includes a known leaked AWS secret
- Pre-seed a failure at step 5 of the audit (panic or timeout)
- The audit runs against a dummy service `payments-svc` with a pre-built git rev

**Opening line options** (choose one based on room read):

**Option A — The Aubakirova cold-open (RECOMMENDED if she's in the room):**
Display her Big Ideas 2026 pull-quote on screen verbatim:

> "The bottleneck becomes coordination: routing, locking, state management, and policy enforcement across massive parallel execution."

And the adjacent line:

> "a single agentic 'goal' to trigger a recursive fan-out of 5,000 sub-tasks"

These are verified verbatim quotes from Big Ideas 2026. While the quotes are visible, trigger Pulse fanning out sub-tasks. Say:

> "Malika, you wrote this five months ago. This is what it looks like when someone builds it. Martin, five minutes."

This converts the pitch into a recognition event. She sees her own ideas instantiated. This is the single highest-leverage opening in this dossier.

**Option B — The Casado control-loop address:**

> "Martin, you wrote that we can't yet close the control loop on agents. That's exactly why we built Nunchi as the control plane. Five minutes."

**Option C — The durable-system hook (if pre-meeting agent run exists):**

> "Five weeks ago, Roko started running. It hasn't stopped. Here's what it's done in your portfolio's open-source repos."

Then turn the screen to show 30 days of execution logs.

**Live benchmark corner widget**: Throughout the entire a16z meeting (not just the demo section), a 400x300px Bloomberg Two-Tape widget runs in the corner of the screen showing Roko vs LangGraph side-by-side on the 5-task HAL subset. The widget shows live cost ticking up, pass/fail indicators, and declares a winner at p<0.01. This runs as ambient proof while the conversation happens — it is not narrated unless someone asks about it. See DEMO-BUILD.md T2c.2 for implementation details.

**What NOT to open with:** "Hi, we're Nunchi, we build agent infrastructure." That's the cliché every other pitch uses.

---

### Min 1: Fleet Identity and Attestation (0:00 — 1:00)

**Type**:
```
nunchi agents list --env=prod
```

**Expected output**:
```
◆ Agent Fleet  ·  env: prod  ·  4 active

│ NAME           IDENTITY                      STATUS    ATTESTED  TASKS  UPTIME
│ researcher@v2  nhi://acme/researcher.v2      ✔ active  ✔ SPIFFE  47     4h 23m
│ analyst@v1     nhi://acme/analyst.v1         ✔ active  ✔ SPIFFE  31     3h 52m
│ auditor@v3     nhi://acme/auditor.v3         ✔ active  ✔ SPIFFE  12     1h 08m
│ builder@v1     nhi://acme/builder.v1         ◐ idle    ✔ SPIFFE   0     4h 23m
```

**What to say**:

> "Four agents in the fleet. Every one has a SPIFFE identity. Every one is attestation-verified before it can touch any resource. This is non-negotiable infrastructure — like TLS for agents."

This hits the Pindrop/Chronicle Detect thesis (security identities) that Casado and Aubakirova evaluate.

---

### Min 2: Policy-Gated Audit (1:00 — 2:15)

**Type**:
```
nunchi audit deployment payments-svc --rev=abc123 --policy=prod-sec
```

**Expected output** (appears progressively over ~30 seconds):
```
◆ Audit  ·  payments-svc@abc123  ·  policy: prod-sec

│ STEP  CHECK                STATUS    DETAIL
│ 1/8   dependency-audit     ✔ pass    0 CVEs, 0 deprecated
│ 2/8   secret-scan          ✔ pass    no secrets in diff
│ 3/8   credential-rotation  ✖ FAIL    AWS_SECRET_KEY found in config/env.yaml
│       ↳ blocked: credential exposure. rotated key + opened PR #4821
```

**Pause. Let the failure sit visually.**

**What to say**:

> "Step 3 found a leaked AWS secret. The agent didn't close the loop — the coordination plane did. It blocked the deployment, rotated the key, and opened a PR. The agent never got past the violation."

This is the line that addresses Casado's April 2025 objection directly. The coordination plane closes the control loop, not the agent.

---

### Min 3: Pre-Seeded Failure and Kill (2:15 — 3:00)

The audit continues running. At step 5:

**Expected output**:
```
│ 4/8   static-analysis      ✔ pass    clippy: 0 warnings
│ 5/8   integration-smoke    ✖ PANIC   connection refused: payments-svc:3000
```

**The terminal shows the panic.** Visibly press **Ctrl+C**.

**Let the room sit with a dead terminal for 2 seconds.** Do not rush.

**What to say** (during the silence):

> "Step 5 panicked. In any other system, that's a full restart. 12 minutes of audit work gone."

---

### Min 4: Resume and Remediation (3:00 — 4:15)

**Type**:
```
nunchi resume run_4823
```

**Expected output**:
```
◇ Resuming
│  from event 47 of 52  ·  recovering step 5/8

│ 5/8   integration-smoke    ✔ pass    (retry: payments-svc:3000 healthy)
│ 6/8   load-test-soak       ✔ pass    p99 < 200ms, 0 errors
│ 7/8   rollback-validation  ✔ pass    canary: 100% → 0% in 4.2s
│ 8/8   compliance-snapshot  ✔ pass    ISO 42001 attestation anchored

◆ Audit Complete
│  8/8 passed (1 remediated)  ·  $0.34 total  ·  14m 22s
│  PR #4821: credential rotation merged
│  Attestation: block 1,204,387
│
└ Replay: nunchi replay run_4823
```

**What to say**:

> "Recovered from event 47. Retried step 5 — the service was healthy on retry. Continued through load testing, rollback validation, and compliance attestation. Total cost: $0.34. Total time: 14 minutes. The credential was rotated before the audit finished."

---

### Min 5: Replay and Audit Trail (4:15 — 5:00)

**Type**:
```
nunchi replay run_4823 --as-of="step 05"
```

**Expected output**:
```
◆ Replay  ·  run_4823  ·  from step 05

│ EVENT  TIME         TYPE          DETAIL
│ 47     14:23:01.3   gate_check    integration-smoke: connection refused
│ 48     14:23:01.3   prediction    retry_success_prob: 0.72
│ 49     14:23:03.5   retry         payments-svc:3000 → 200 OK
│ 50     14:23:03.8   gate_check    integration-smoke: ✔ pass
│ 51     14:23:12.1   gate_check    load-test-soak: ✔ p99=142ms
│ 52     14:23:18.4   gate_check    compliance-snapshot: ✔ anchored

│ ZK Proof: 0x7a3f...b219  ·  block 1,204,387
│ Verify:   https://nunchi.network/proof/0x7a3f...b219
```

**What to say**:

> "Full JSON replay from step 5 forward. Every decision timestamped and replayable. The ZK proof is anchored on-chain. This is what you hand the compliance officer when they ask what happened on March 15th."

**Close**:

> "Same primitives — identity, policy, replay — work for deployments, triage, migrations, anything. The agent didn't close the loop. The coordination plane did."

---

## 4. The "Hand Them the Laptop" Moment

After the scripted demo, if the room energy is high and the investor is engaged, offer the laptop.

**What to say**: "Want to try it? Type any prompt."

The pre-warmed cache covers a range of prompts related to the demo domain. Even if the investor types something unexpected, the system should produce reasonable output within 10 seconds.

**Fallback**: If the typed prompt hits a cache miss and takes longer than 10 seconds, say: "That's a cold cache — the system is actually doing full inference. Notice the routing decision it made." Point to the routing line. This turns a slow response into a feature demonstration.

**What to avoid**: Do not look nervous during the wait. Do not apologize. Do not explain caching. The investor should experience the product, not learn about its optimization strategy.

---

## 5. Dashboard Demo (Technical Diligence Follow-Up)

This is not part of the 3-minute or 5-minute demo. It's shown during technical diligence calls or when the conversation shifts to "show me the architecture."

### Transition

"Let me show you what this looks like from the monitoring side."

Switch to the browser tab with the dashboard.

### Walk-Through (2-3 minutes)

**View 1: Cost Dashboard** (30 seconds)
- Point to the four stat cards: Total Cost, Cache Hit Rate, Routing Distribution, Gate Pass Rate
- Point to the cumulative cost chart: "The gap between the dashed line and the solid line is your savings. That gap widens with every run."

**View 2: Agent Fleet** (30 seconds)
- Point to the agent cards: identity, 7-domain reputation scores, current task, cost vs prediction
- "Every agent has a reputation score across 7 domains. This is earned through execution, not self-reported."

**View 3: Knowledge Graph** (45 seconds)
- This is the visual differentiator. Let it render fully before speaking.
- "Each node is a knowledge entry. Each edge is a citation. Brighter nodes have been cited more often. Fading nodes are in demurrage — they haven't been used recently and will be pruned if not reinforced."
- Point to a cluster: "This cluster is finance domain. Three different agents contributed to it. The knowledge compounds."

**View 4: Chain View** (30 seconds)
- "This is the live block feed. Knowledge publications, identity attestations, ZK proof verifications. Every event is on-chain and auditable."
- Point to block time: "50ms block times. Co-located Tokyo validators. Same architecture as Hyperliquid."

---

## 6. Objection Handling During the Demo

**General principle**: Casado is famously fast and adversarial in meetings. When he pushes back, never "yes but." Always "yes, and" (improv principle). When he attacks, find the disagreement faster (Mark Suster: "A great meeting is a debate, not a pitch"), then ask: *"What would have to be true for this to be a no-brainer?"* — converts attack-mode into spec-mode.

**Bring up Nicira when stuck**: *"When you were at Nicira pitching VMware…"* gets him out of a16z-partner mode into founder-empathy mode.

### "That looks scripted"

"The prompts are pre-chosen, but the execution is live. Want to type your own?" (Hand them the laptop — the Collison move)

### "What if the agent fails?"

"That's Beat 4. Watch." (Skip ahead to the kill-and-resume)

### "This won't scale"

"Yes, and that constraint is the moat — here's why." Then point to the gate pipeline that terminates bad paths early, the CascadeRouter that routes cheap, and the knowledge store that compounds. Scaling is the product.

### "This is a feature, not a company"

"Yes, and the feature has 177,000 lines of Rust around it because it's actually a substrate." Point to the 33 crates, the 115 API routes, the self-hosting loop. Quote Casado back to himself: *"Customers don't buy platforms; customers buy products. Focus on the product, build a viable business, then turn it into a platform."* Roko is the product. Nunchi is the platform.

### "Is the chain real?"

"The chain runs locally on mirage-rs, our EVM simulator. Same mechanics, same block production, same precompiles. Mainnet is the Phase 1 milestone post-funding. The ZK proofs are real cryptographic constructions." Frame it in his vocabulary: "It's a vertical cloud for agent identity and settlement."

### "What's the cache hit rate in production?"

"In this demo, it's pre-warmed — near 100%. In production, cold start is ~7%. After one day of operation, published benchmarks show 65-84% depending on workload. ProjectDiscovery reported going from 7% to 84% cache hit rate on 9.8 billion cached tokens."

### "How do you handle provider outages?"

"The CascadeRouter routes across multiple providers simultaneously — Anthropic, OpenAI-compatible endpoints, Ollama for local fallback. If one provider is down, the router automatically selects the next best option. Zero manual intervention."

### "Why won't Anthropic / OpenAI just build this?"

The CascadeRouter routes *away from expensive models* when cheaper ones suffice. Anthropic has no incentive to route away from its own models. Nunchi does — the chain economics reward accurate routing, not loyalty to any single provider.

### "Why should I believe a solo founder can build this?"

Don't show team photos. **Call the engineer on Zoom** — pre-warm a top engineer for the 20 minutes around the meeting. Or: the runtime is 177K lines of Rust, self-hosting, with 18 crates. The code is the team evidence.

### "Isn't this just Keycard?"

Use her own Kill-Chain framework: "Keycard = intra-org issuance, runtime enforcement. Nunchi = cross-org reputation, settlement. Different stages of the same lifecycle. Same axis — static identity to dynamic intent — perpendicular extension: centralized issuer to sovereign verification. We're the next stage, not a competitor."

If Casado presses: invoke the Nicira callback — "Keycard is to Nunchi as a switch ASIC is to the control plane. Both necessary. The switch processes packets; the control plane decides where they go."

### Productive tension bridge (Joel de la Garza + Matt Bornstein in the room)

If both Joel de la Garza AND Matt Bornstein are in the room, bridge the tension: "Joel is right about demand — 2026 is the year of agents. Matt is right that today's frameworks fail. Nunchi is the missing coordination + verifiability layer that closes that gap."

### If Sarah Wang or Anjney Midha drops in mid-meeting

Reset 60 seconds. Give a one-sentence description: "Nunchi is the agent-native control plane — sovereign identity, cost optimization, shared knowledge, durable execution." Point at the screen: "Martin's just become a temporary verifier on our reputation graph live." Pick up the demo flow without restarting the deck.

---

## 7. Meeting Structure (30-Minute Partner Meeting)

Per Steve Jobs MacWorld 2007 pattern — three modality switches max in a 30-minute meeting:

```
0:00 - 3:00   Opening story (Aubakirova quote or Casado control-loop)
3:00 - 12:00  Live runtime demo (the 5-minute a16z version above)
12:00 - 17:00 Adversarial self-attack (pre-seeded failure, gate catches it, resume)
17:00 - 22:00 Narrative arc + L1 reveal as "and one more thing"
22:00 - 25:00 The ask
25:00 - 30:00 Q&A / "hand them the laptop" moment
```

**Bret Victor principle**: State a principle first, then make every demo step prove the principle. The principle for Nunchi: *"Agents need a sovereign substrate to be trustworthy."* Every demo beat must be visible evidence of that single claim, not a feature parade.

**If the meeting goes long**: Jason Lemkin's inverse rule — if Casado is investing his own time past the scheduled end, that's the buy signal. Don't try to wrap. Let him drive past time. The "next step" should be his idea by minute 40.

**If meeting collapses to 20 minutes**: Keep the control-plane/SDN moment, the predict-publish-correct cycle, and the kill-and-resume. Drop the dream cycle and ISFR ticker.

**The dream cycle withhold**: Keep the dream consolidation cycle OUT of the room (Suster: "hold back some news that you have so you can bring it up later"). The week-2 follow-up has a *"wanted to show you something we couldn't fit"* hook. It's poetic, technically deep, and gives a reason to come back.

### The Reluctance Posture

Linear's Karri Saarinen took Sequoia's DM two days after launch, walked in without a deck and said they didn't really want to raise yet, and got term sheets from Sequoia and Index within weeks. Tuomas Artman: *"raising money for us was simple because we didn't want it."*

Opening posture: *"We took this meeting because you've been right about infrastructure twice — Nicira and [portfolio company] — and we want one shot at the agent thesis."* Reluctance is the highest-status posture in infra, only credible if runtime metrics back it up.

### The Scribe Play

The junior partner taking notes writes the internal memo. Get their email at the door. Send a personal "loved your question about [X]" within 4 hours (Mark Suster: *"If any junior people attended send them separate emails and help them get up to speed"*).

### The Bathroom Break

**Always go to the bathroom when offered** — it gives Casado 90 seconds to text his partners and form a positive impression independent of you, which is the actual unit-of-work in a16z's internal momentum.

---

## 8. Post-Demo Artifacts

After the meeting, the investor should have:

1. **The PDF deck** — emailed within 1 hour of the meeting
2. **The shareable URL** — the `--share` URL from the demo, still live and clickable
3. **The GitHub repo** — link to the public Roko repository with real commit history
4. **The demo replay** — `nunchi replay <run_id>` command they can run themselves if they install the CLI
5. **Technical specification** — the v3.0 architecture doc, trimmed to 10 pages

The shareable URL is the most important artifact. It's the one that gets forwarded to the Monday partner meeting. It must load fast, look premium, and tell the full story without narration.

**Post-meeting cadence**:
- Day 1: Thank-you email with PDF deck attached
- Day 4: Customer/design-partner update email ("just shipped X with partner Y")
- Day 14: "Exciting news" follow-up — this is where the withheld dream cycle demo gets revealed: *"Wanted to show you something we couldn't fit — the knowledge consolidation cycle running offline."*

**The personalized demo URL**: After the meeting, `demo.nunchi.dev/martin` stays live — a session pre-loaded with his context that keeps incrementing. He can curl it from his phone. It becomes a persistent connection point.

---

## 9. Timing Rehearsal Checkpoints

Practice the demo until each beat hits its mark:

| Beat | Target Time | Max Allowed | Rehearsal Check |
|------|------------|-------------|-----------------|
| Identity + Gates | 0:30 | 0:40 | Can you explain identity in one sentence? |
| Predict-Publish-Correct | 1:15 | 1:30 | Can you point to the delta without explaining statistics? |
| Shared Knowledge | 2:15 | 2:30 | Do you say "the thousandth agent" naturally? |
| Kill and Resume | 2:45 | 3:00 | Can you stay silent for 2 seconds after the kill? |
| Close | 3:00 | 3:10 | Four words: identity, prediction, knowledge, durability |

The 2-second silence after the kill is the hardest part. It feels uncomfortable. It must happen. The silence is the demo.

---

---

## 9. Technical Implementation Notes for the Demo Flow

This section maps each demo beat to the real code that needs to produce the output. For the complete codebase reference, see CODEBASE-CONTEXT.md.

### How `roko run` Actually Works Today

The current `roko run "<prompt>"` command (`crates/roko-cli/src/run.rs`) follows the universal loop: compose → dispatch → gate → persist. It outputs the agent's text response and a summary of gate results, but **NOT** in the Clack-style format shown in the demo beats above.

The demo beats require these data points to be surfaced in the CLI output:

| Data Point | Where It Exists in Code | Currently Surfaced? |
|-----------|------------------------|-------------------|
| Agent identity (`researcher@v2`, `nhi://...`) | Not yet implemented — needs ERC-8004 identity registration or a local identity config | **No** |
| Cost prediction (`$0.043`) | CascadeRouter computes this during model selection, but the value is internal to the routing logic | **No** |
| Routing decision (`haiku → gpt-4o-mini`) | CascadeRouter returns the selected model, but it's not formatted for display | **No** |
| Gate results (`✔ pii_scan`) | Gate pipeline runs and returns verdicts, but they're printed as plain text | **Partial** (text, not Clack-style) |
| Knowledge loaded (`9 facts from /finance/q3`) | NeuroStore is queried during dispatch enrichment in `orchestrate.rs`, but the result is injected silently into the prompt | **No** |
| Cost actual + delta (`$0.031, -28%`) | Efficiency events record cost, but the delta vs prediction is not computed | **No** |
| Facts deposited (`2 new facts → /finance/q3`) | Not yet implemented — agents don't currently deposit knowledge explicitly | **No** |
| Share URL | Does not exist | **No** |
| Checkpoint saved / resume from | Plan executor has checkpoint/resume; single `roko run` has partial support via signal substrate | **Partial** |

### What the `roko serve` Backend Provides

The HTTP server runs on port 6677 and provides all the API endpoints the web dashboard needs. The React SPA is served from embedded assets (compiled into the binary via rust-embed). During development, Vite's dev server on port 5173 proxies API calls to port 6677.

For the dashboard demo (section 5 above), the relevant endpoints are:
- `GET /api/dashboard` — full dashboard snapshot (for Cost Dashboard)
- `GET /api/agents` — aggregated agent list (for Agent Fleet)
- `GET /api/knowledge/entries` + `/edges` — knowledge graph data
- `GET /api/chain/status` — chain connection status
- `GET /api/learn/cascade-router` — model routing weights
- `GET /api/learn/efficiency` — task efficiency history

### The Terminal WebSocket Pipeline

The terminal panes shown in the demo (both CLI demo and web dashboard) work via this pipeline:

1. **Browser** sends `POST /api/terminal/sessions` with `{session_id, cols, rows}` → server creates a PTY (pseudo-terminal) via `portable-pty`
2. **Browser** opens WebSocket at `/ws/terminal/{id}` → bidirectional JSON messages
3. **User types** → `{type: "input", data: "ls\n"}` sent over WebSocket → written to PTY stdin
4. **PTY produces output** → `{type: "output", data: "file1.rs file2.rs\n"}` sent to browser → rendered by xterm.js
5. **Browser resizes** → `{type: "resize", cols, rows}` sent → PTY window size updated

This pipeline is **fully real** — no simulation. When the demo shows a terminal, it's running actual shell commands on the server.

---

*Cross-references: CODEBASE-CONTEXT.md (complete technical reference), DEMO-STRATEGY.md (what and why), DEMO-VISUAL-SPEC.md (detailed design), DEMO-COMPETITIVE.md (competitive landscape), DEMO-BUILD.md (what to implement).*
