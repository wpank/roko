# Learnings Rewrite Prompt

The current learnings docs (01-05) were built additively over a long session — each research round and source set added content without refactoring. They need a clean rewrite that's focused, coherent, and written from scratch for someone with zero prior context.

Copy everything below the `---` into a new Claude Code session.

---

## Instructions

You are rewriting the learnings documents at `/Users/will/dev/nunchi/roko/roko/tmp/learnings/` from scratch. These must be **self-contained briefings** — a new team member or a new Claude Code session should understand the full system, strategy, and context from these docs alone.

### Step 1: Read the authoritative sources (in this order)

**The unified spec** (the protocol — this is the authority):
- Read all `.md` files in `/Users/will/dev/nunchi/roko/roko/tmp/unified/` (22 docs). These define Signal/Pulse, Cell, Graph, 9 protocols, 10 specializations. They were rewritten from scratch using v2 vocabulary.

**The research** (7 rounds of deep research with citations):
- Read all files in `/Users/will/dev/nunchi/roko/roko/tmp/research/` (research.md through research7.md)

**The conversation summary** (what happened in the prior session):
- Read `/Users/will/dev/nunchi/roko/roko/tmp/learnings/06-CONVERSATION-SUMMARY.md`
- Read `/Users/will/dev/nunchi/roko/roko/tmp/learnings/07-SOURCE-MAP.md`

**The Nunchi blockchain** (purpose-built EVM for agents):
- Read key docs in `/Users/will/dev/nunchi/roko/roko/docs/08-chain/` — at minimum: 00, 01, 03, 04, 06, 10, 14

**The dashboard** (the product surface):
- Read `/Users/will/dev/nunchi/nunchi-dashboard/tmp/ux-refresh-context/doc-3-optimal-redesign.md`
- Read `/Users/will/dev/nunchi/nunchi-dashboard/tmp/ux-refresh-context/uxresearch.md`

### Step 2: Write 5 focused documents

Rewrite these 5 docs from scratch. Each should be readable standalone. No assumptions of prior context. Use "Nunchi" for the project/blockchain, "Roko" for the agent runtime, "Cell" for the computation primitive (not "Block").

**01-ARCHITECTURE.md** (~600-800 lines)
The full technical system for a senior engineer who has never heard of Nunchi:
- What it is (one paragraph — the trust layer for the agent economy, open-source runtime + purpose-built blockchain)
- 3 fundamentals: Signal (durable, demurrage, HDC), Pulse (ephemeral, Bus), Cell (computation, 9 protocols, predict-publish-correct), Graph (composition, Hot Graphs, Workflow/Activity split)
- 9 protocols with signatures and what's novel about each
- 10 specializations (especially Agent with vitality, type-state, CorticalState, EFE gating, somatic markers)
- The Nunchi blockchain (sovereign EVM L1, Simplex consensus, HDC precompile, ERC-8004 agent identities, 7-domain reputation, ERC-8183 job market, ZK-HDC)
- Design principles (11 + anti-principles)
- 5 compounding mechanisms
- HDC section
- End-to-end execution trace

**02-RESEARCH-SYNTHESIS.md** (~800-1000 lines)
All 7 research rounds organized by topic (not by round). Each finding: citation, numbers, architectural implication, readiness rating:
- Coordination, active inference, self-evolution, knowledge/memory, HDC, formal methods, safety, verification, ZK/on-chain, collective intelligence, performance, scaling laws, competitive landscape, production economics, category creation

**03-STRATEGY-AND-PITCH.md** (~600-800 lines)
Series A pitch narrative — everything an investor or pitch-prep session needs:
- Repositioned pitch (trust layer, NOT "Stripe for agents")
- The problem (coordination failure data)
- The solution (cost wedge + trust moat)
- Market sizing
- Competitive landscape with named competitors
- The moat (5 components)
- Go-to-market (MCP playbook, ACP distribution, forward-deployed engineering)
- a16z partner map (Casado, Aubakirova, Dixon — from research6)
- The landing page as the pitch deck (7 sections, recommended pitch flow)
- The 3-minute demo (HAL numbers: $44.86 → $1.42)
- Named design partners (Cleric, Decagon, Harvey, Hebbia, Resolve.ai)
- Series A comps ($15-35M at $150-250M)
- Bear cases and responses (3 dangerous, 3 dismissible)
- Regulatory timeline (EU AI Act Aug 2)
- What NOT to do (anti-patterns with precedents)
- 5 things to do this week

**04-IMPLEMENTATION-PRIORITIES.md** (~500-600 lines)
What exists, what to build, in what order:
- Current state (18 crates, 177K LOC, what's wired vs unwired)
- Phase 0: Launch artifacts (MCP playbook) + dashboard refocus
- Phase 1: Core protocol (Pulse, Bus, CalibrationPolicy, Verify redesign, Hot Graph)
- Phase 2: Differentiation (EFE, demurrage, heuristics, CognitiveWorkspace, vitality)
- Phase 3: Distribution (ACP, packages, marketplace, arenas, brain export)
- Phase 4: Self-evolution (L4 with CMP, c-factor, safety)
- Phase 4+: Nunchi blockchain (testnet, HDC precompile, 6 contracts, ChainWitness)
- Cost-reduction proof methodology (HAL Pareto format)

**05-RISKS-AND-ANTIPATTERNS.md** (~400-500 lines)
Everything that can go wrong — honest and evidence-based:
- Technical (diversity collapse, error amplification, DGM reward hacking, single-agent > multi-agent on 64%)
- Security (Nasr 90%+ ASR, AutoInject, supply chain, MASpi)
- Regulatory (EU AI Act, MSB classification, Colorado, Product Liability)
- Market (window closing, Chinese commoditization, Temporal adding agent primitives, "Stripe for agents" taken)
- Product (GPT Store failure, Replit database deletion, inference margins, Cognition gap)
- Research7 reality checks (50ms global blocks, demurrage, HDC commercial traction, Nava analog, agent identity competitors)
- Anti-patterns (8 documented failures with citations)
- Mitigations

### Step 3: Update the index

Rewrite `00-INDEX.md` with:
- "What is Nunchi" paragraph (repositioned pitch)
- Reading order table
- Key numbers reference card (all metrics from research1-7)
- Source file map (reference 07-SOURCE-MAP.md)

### Rules

- **Self-contained**: each doc readable without the others
- **Focused**: these are briefings, not encyclopedias. Cut what doesn't serve the reader's purpose
- **Current**: use the repositioned pitch (trust layer, not Stripe for agents). Use "Cell" not "Block" for the primitive. Demurrage dropped from token. 50ms via Hyperliquid-style clustering.
- **Honest**: include the research7 reality checks. Don't hide weaknesses.
- **No duplication**: if a topic is covered in one doc, others reference it, don't repeat it
- **Citations inline**: research findings include arXiv IDs or source
- **Unified vocabulary**: Signal, Pulse, Bus, Store, Cell, Graph, 9 protocols, 10 specializations
