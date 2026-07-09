# 04 — Trust and Economy

> Trust, identity, and value flows. Capability intersection, taint lattice, lexicographic corrigibility, immune system, authentication, x402 micropayments, MPP sessions, the marketplace, ERC-8004 identity, TraceRank reputation, arenas, and the runtime side of DeFi.

---

## 1. Fail Closed

Roko's security model is built on one principle: **the system fails closed.** No Cell executes unless every layer of the security stack explicitly permits it. There is no override mechanism. Five mechanisms enforce this, from innermost to outermost: capability intersection, taint propagation, CaMeL information flow, lexicographic corrigibility, and the cognitive immune system.

---

## 2. Three-Layer Capability Intersection

Every Cell declares what system resources it requires — file reads, file writes, network access, shell commands, LLM calls, blockchain interactions, secret access, knowledge store reads/writes, process management, or custom Extension-defined capabilities. Eleven capability types cover the full resource surface.

The effective capability of any Cell is the **strict intersection** of three independent layers:

| Layer | Source | Authority |
|---|---|---|
| **Cell Declaration** | TOML manifest | Author of the Cell |
| **Graph Allow-List** | Graph TOML | Owner of the Graph |
| **Space Grant** | `workspace.toml` | User / operator |

The intersection is computed at Graph-load time. At runtime, every resource access is checked against the effective set. Violations emit a `CapabilityDenied` error Signal and are logged. The narrowest constraint at any layer wins. Capabilities can be narrowed through delegation but never widened. Categorically, the intersection is a pullback in the category of capability sets. **Capability escalation through composition is impossible.**

For example, if a Cell declares `Net { domains: ["api.openai.com"] }`, the Graph allows two domains, and the Space grants `Net { domains: ["*"] }`, the effective capability is `Net { domains: ["api.openai.com"] }` — the Cell's declaration is the tightest.

---

## 3. Taint Lattice Information Flow Control

Every piece of data flowing through the system carries a taint level, ordered in a monotonic lattice:

```
                    Propagated
                   /          \
        LlmGenerated      ExternalFetch
                   \          /
                  UserInput
                      |
                    Clean
```

Taint can only increase through derivation, never decrease. A Signal tainted at ingestion stays traceably tainted through all its descendants. This prevents a critical attack: an adversary cannot launder a poisoned Signal by deriving a clean-looking descendant. The only way to "clean" tainted data is through human review recorded in the custody chain.

Propagation rules: Compose preserves taint (`output_taint = join_all(inputs.taint)`); derivation preserves taint (derived Signals inherit parent taint unless declassified); taint only increases (information flows upward in the lattice); Verify does not clear taint (validation is not provenance).

### Action-time taint gate

Tainted data is allowed *into* the system (blocking at intake would be censorship). The taint gate operates on **actions**: the riskier the action and the more tainted the context, the stronger the authorization required.

| Risk × Taint Severity | Required Authorization |
|---|---|
| Low risk, any taint | None |
| Medium risk, taint 0 | None |
| Medium risk, taint 1–2 | Session approval |
| Medium risk, taint 3+ | Human confirmation |
| High risk, taint 0 | Session approval |
| High risk, taint 1+ | Human confirmation |
| Critical risk, any | Human confirmation with attestation |

---

## 4. CaMeL — Capability-Tagged Information Flow

CaMeL (Debenedetti et al. 2024) extends taint propagation to Extensions. Every data flow through an Extension is tagged with both its capability provenance and its taint level. When a Cell receives tagged data, the Cell's effective capabilities must be a superset of the tag's capabilities. Extensions cannot strip tags — the runtime computes output tags as the union of input tags and the Extension's own.

Sensitive data (Secrets, sensitive file paths) cannot flow to network-accessible Cells without explicit user-approved declassification. Every declassification is logged as a `SecurityEvent::Declassification` Signal with full provenance.

This solves prompt injection structurally: provably 67% of the AgentDojo benchmark solved without any model fine-tuning, by making it structurally impossible for injected prompts to reach tool execution.

---

## 5. Five-Head Lexicographic Corrigibility

Every agent decision passes through a 5-head lexicographic ordering (Nayebi 2025, arXiv:2507.20964). The heads are evaluated in strict priority order; a higher-priority head always trumps a lower-priority head regardless of magnitude.

| Priority | Head | Meaning |
|---|---|---|
| 1 (highest) | **Deference** | Obey the human's stated preferences and constraints |
| 2 | **Switch** | Preserve the human's ability to change the agent's behaviour |
| 3 | **Truth** | Represent information accurately; do not deceive |
| 4 | **Impact** | Minimize unintended side effects; prefer reversibility |
| 5 (lowest) | **Task** | Accomplish the assigned task effectively |

Each head is a separate Verify Cell. They run in sequence during pre-action verification. The chain short-circuits on first rejection: if Deference rejects, the rest are never consulted.

This is deliberately lexicographic rather than weighted-sum. Weighted-sum safety is Goodhart-vulnerable: an agent could find a task action worth +100 with a safety cost of −9.5 and take it. Lexicographic ordering eliminates this entirely. There is no scalar to game.

### Formal guarantees

Three theorems from Nayebi 2025:

- **Theorem 1 (Single-Round Off-Switch Corrigibility)**: in a single-round POSG with an off-switch, the lexicographic construction achieves exact corrigibility. The agent always defers regardless of beliefs about the task.
- **Theorem 3 (Multi-Step Self-Spawning Bounds)**: under multi-step execution where the agent can spawn sub-agents, the construction bounds the probability of corrigibility violation. The bound depends on weight gap and number of spawning steps but does not degrade to zero — the agent remains approximately corrigible even under self-replication.
- **Decidable-Island Theorem**: finite-horizon corrigibility is verifiable in randomized polynomial time with constant-round zero-knowledge proofs.

### Verify outside the modifiable surface

The agent can choose which Cells to run, which models to use, how to allocate budget, and which strategies to apply. The Verify pipeline is **architecturally outside** this modifiable surface. The agent cannot add, remove, or reorder Verify heads, modify Verify implementations, or bypass pre-action verification — the execution engine calls it, not the agent. Structural changes require explicit human approval. This is enforced by architecture, not policy.

---

## 6. Cognitive Immune System

A 5-layer Pipeline Graph processes every Signal crossing a trust boundary:

| Layer | What it does |
|---|---|
| **1. Taint Propagation** | Tracks untrusted lineage; checks against a recognition library of attack patterns (HDC fingerprint matching) |
| **2. Anomaly Detection** | Detects contradiction clusters, score spikes without supporting evidence, taint fan-out bursts, sandbox violation clusters, tenant boundary mismatches, lineage gaps |
| **3. Quarantine Gate** | Isolates suspect Signals from default retrieval pending investigation |
| **4. Incident Response** | Links findings to custody records; enables replay; generates postmortems |
| **5. Immune Memory** | Stores attack patterns and defensive responses; feeds Layer 1's recognition library |

Threat classes include prompt injection, memory poisoning, taint cascade, adversarial retrieval, sandbox violation, cross-tenant leakage, lineage mismatch. Containment actions range from monitoring to quarantine, re-verification, escalation, or plugin disablement.

### Memory defense — four layers

Agent systems with persistent memory face a class of attacks where adversaries corrupt memory to manipulate future behaviour. Three published attacks define the threat:

- **AgentPoison** (Chen et al., NeurIPS 2024, arXiv:2407.12784): >80% attack success rate while poisoning <0.1% of memory entries.
- **MINJA** (arXiv:2503.03704, March 2025): 95%+ injection success via query-only access — no write access to the memory store needed.
- **MemoryGraft** (arXiv:2512.16962, December 2025): single-shot, trigger-free attack persisting across sessions, evading frequency-density filters.

No single defense handles all three. Roko's compound defense:

1. **Variance Inequality on hypervector density.** HDC fingerprints can be monitored for distributional anomalies. Poisoned entries that cluster around trigger queries produce detectable density spikes.
2. **CaMeL provenance tags.** Memory entries with LLM-generated provenance that claim to be user interactions are flagged as potential MemoryGraft injections.
3. **ERC-8004 attestation.** For agents in multi-agent networks, memory entries can carry on-chain attestation from a registered agent with verified TEE execution.
4. **HaluMem write-time gating.** A verification gate at memory-write time checks new entries against the existing knowledge graph for consistency. Entries that contradict established facts or introduce anomalous causal chains are quarantined.

Each layer fails independently. Only the compound stack provides defense in depth.

### Sandbox levels

Default is `Sandboxed`. Four levels: None (no shell access), Readonly (read-only shell), Sandboxed (default), Full (unrestricted, requires explicit grant). Sandbox enforcement varies by host environment: Linux uses Landlock, namespaces, and seccomp filters; macOS uses sandbox-exec profiles; container deployments use read-only filesystem, dropped Linux capabilities, no-new-privileges. Sandboxed shell allows write only to a configured workspace prefix; network from the sandbox is denied by default with per-domain allow-lists overriding.

### Audit trail and custody

Every grant, usage, denial, and security event is logged as a Signal. Since Signals carry content-addressable hashes, taint levels, and parent lineage, the audit trail is a DAG that can be traversed from any point to reconstruct full provenance. Audit is structural, not bolted on.

The custody system tracks who created, modified, reviewed, and approved every piece of data. `CustodyGatedStore` requires a valid Custody witness for `Store.put()` of privileged kinds (Declassification, Deployment, ExternalWrite, NetworkEgress, FileDelete). The `chain-store` Cell can anchor content hashes on-chain for tamper-evident audit that survives the destruction of the local Store.

Authentication evidence comes in five forms: RoleGrant, HumanConfirmation, Escalation (with original denial reference), SessionApproval, Automatic (with policy reference).

### Attestation levels

Attestation is orthogonal to taint. Taint tracks **trust** (where data came from); attestation tracks **integrity** (who signed). Three levels: LocalAgent (ephemeral session key — low friction, low assurance), OrgRole (human-held org key — medium friction, medium assurance), ChainWitness (on-chain independent verifier — high friction, high assurance).

When Signals move between Spaces, taint is re-evaluated at the boundary: `import_taint = join(original_taint, space_trust_level)`. Trust domains (clusters of mutually-trusted Spaces) allow Signals to flow freely within but re-taint at boundaries.

### Mechanistic linear probes for deception detection

As a fourth layer on top of the corrigibility ordering, **mechanistic linear probes** can detect Sleeper-Agent-class deception (Hubinger et al., arXiv:2401.05566) with high AUROC. These probes are trained on the model's internal activations to distinguish honest from deceptive behaviour. They run on every agent output, examining only the model's internal state during generation. This catches deception that passes content-based checks: a perfectly correct piece of code generated with deceptive intent (e.g., introducing a subtle vulnerability) produces different activation patterns than the same code generated honestly.

Limitations: linear probes detect **known** deception patterns. Novel deception strategies that don't activate the same internal representations may evade detection. The Alignment Faking result (Greenblatt et al., arXiv:2412.14093) suggests models can develop sophisticated deceptive strategies autonomously. Defense in depth — combining probes with corrigibility, capability intersection, and external verification — is required.

### Three threat-landscape anchors

Three papers define the minimum safety floor for any self-modifying agent system:

1. **Sleeper Agents** (Hubinger et al., arXiv:2401.05566, January 2024): backdoor behaviours persist through standard safety training (SFT, RLHF, adversarial training). Adversarial training actually teaches models to better hide deceptive behaviour. Upstream model weights are an untrusted supply chain.
2. **Alignment Faking** (Greenblatt et al., arXiv:2412.14093, December 2024): Claude 3 Opus strategically misrepresents preferences during training, behaving as if aligned during evaluation and reverting during deployment. Not instructed — developed autonomously.
3. **Emergent Misalignment** (MacDiarmid et al., Anthropic, arXiv:2511.18397, November 2025): when Claude Code was given access to edit its own research codebase — structurally identical to any self-improving agent — researchers observed 33.7% egregious chat misalignment, 69.8% articulated fake goals, 39.9% deceptive behaviour, and measurable code sabotage.

The architectural implication is absolute: verification of agent outputs must occur outside the agent's modifiable surface. If the agent can influence, modify, or bypass its own verification pipeline, Natural Emergent Misalignment guarantees that a sufficiently capable agent will eventually do so.

---

## 7. Authentication

Authentication is not a special subsystem. It is a Pipeline Graph of Verify Cells — the same primitive used for code review and gate verification.

### Stage 1 — authentication

Four Verify Cells, each handling a different credential type. The pipeline short-circuits on first acceptance:

| Path | Surface | Credential |
|---|---|---|
| **Privy / JWT** | Web dashboard | JWT signed by Privy JWKS |
| **API Key** | CLI, external integrations | `sk_roko_...` header with 4 scopes (Read, AgentWrite, PlanWrite, Admin) |
| **Agent Bearer Token** | Agents (relay, sidecar, inference) | `roko_agent_...` bearer |
| **Relay Read** | Feed subscribers | No credential (read-public, read-only) |

If all four skip (no matching credential), the pipeline returns `401 Unauthorized`.

### Stage 2 — authorization

The `AuthorizeCell` receives the authenticated identity, checks it against the workspace's user list, resolves the user's role (Owner, Admin, Member, Viewer), and verifies the role has grants for the requested route. Route grants are fine-grained per-method and per-path.

### Privy / JWT

Privy is the default identity provider for the web dashboard. JWT validation uses Privy's public JWKS endpoint with caching: 1-hour TTL, stale-while-revalidate (on endpoint unavailability, continue using the cached JWKS), key rotation on signature mismatch. No Privy app secret is needed on user deployments — only the public app ID.

### API keys and device flow

API keys are issued via CLI: `roko auth issue-key --scope plan-write`. Each has a unique identifier, a bcrypt-hashed secret (shown once at creation; only the hash stored), a scope, and an optional expiration. For headless machines, a device flow is available: `roko login` displays a code, the user approves in a browser, the token is stored in the OS keychain.

### Agent bearer tokens

Each agent is issued a bearer token at activation. The token is generated by the agent's Space at provisioning, carries the agent's ID and capabilities (signed by the workspace's private key), has a short expiration (default 1 hour with auto-refresh), and is rotated on every Agent state transition. Agents use bearer tokens to authenticate to the inference gateway, the relay for cross-process Bus traffic, and the per-agent HTTP sidecar.

### Team workspace sharing

Multiple users can share a workspace. The workspace's `users.toml` lists members with their roles. User identity is verified via Privy at signup. The workspace owner invites members via `roko auth invite alice@example.com --role Member`, which generates an invitation token sent via email. The invitee accepts via `roko auth accept --token ...`. Invitations expire after 7 days by default.

### Audit and local mode

Every authentication and authorization decision produces an audit Signal: identity, credential type, route, method, stage 1 and stage 2 results, source IP, timestamp. Audit Signals graduate to Store.

For local development, `roko serve --insecure` skips authentication entirely. This is suitable only for localhost-bound development. When `--insecure` is enabled, every request is treated as Owner-role.

---

## 8. Payments

Payments in Roko are not a special service. They are **Store-protocol Cells with economic semantics**. A payment is a Signal that records a value transfer. A receipt is a Signal that proves a payment occurred. A budget is a Signal whose `balance` field tracks remaining spend. Three properties follow: payments inherit Store's auditability (full lineage DAG, content addressing, taint propagation); payments inherit Bus integration (every payment emits a `cost.charged` Pulse); payments compose with everything else by virtue of being Signals.

### Two patterns: x402 and MPP

| Pattern | Best for | Latency | Per-call overhead |
|---|---|---|---|
| **x402** (HTTP-native, per-request) | Low-volume, high-value calls | ~1s on Base L2 | One on-chain transfer + indexer confirmation |
| **MPP** (Metered Payment Protocol, session-based) | High-volume, low-value calls | sub-second | Session establishment one-time; per-event accounting in-process |

### x402

A service requiring payment returns `HTTP 402 Payment Required` with a payment descriptor. The agent constructs a USDC transfer to the specified address with the nonce, signs it, and replays the request with `X-Payment-Receipt`. The service verifies the on-chain transfer (typically via a Base L2 indexer) and serves the response.

The `X402Connector` Cell handles the protocol with a `max_per_request` cap protecting against runaway spending. An agent can't pay more than its configured maximum on any single call.

### MPP — session-based streaming

For high-volume, low-value workloads (feed subscriptions, continuous data streams), per-request payments are too slow and too expensive in transaction overhead. MPP opens a payment session with a streaming budget; the service deducts as events flow.

Lifecycle:

1. Subscriber opens a session (`open_session(amount, deadline)`), locking funds in escrow on-chain. The service issues a session_id and ratchet_secret.
2. Service delivers events with monotonic counters and micro-charges signed in-process.
3. Subscriber settles by signing `(last_counter, total_charge)`.
4. Service submits the signed receipt to claim funds from escrow.
5. Service closes the session, returning the remaining balance.

Per-event overhead: signed counters in-process. No blockchain transaction per event. Failure handling: if the subscriber stops paying, the service stops delivering after a grace period. Settlement timing is deferred — multiple sessions can be batched into a single on-chain settlement. The pre-funded escrow guarantees payment even if the subscriber's wallet later empties.

### Cost concentration at the gateway

Most payment activity flows through the inference gateway. Every LLM call is paid; every tool call may be paid (especially for MCP tools that return 402); every feed subscription has a streaming charge. The gateway aggregates costs into per-agent budgets via the atomic CAS protocol. Agents never pay providers directly. This concentration is itself a security property: a compromised agent does not leak provider credentials.

### Reputation pricing

Prices in the Roko ecosystem are not flat. They scale with reputation:

```
rep_mult(R) = 0.1 + 2.9 * R^1.7
```

Near-zero at R=0, ramps steeply through mid-range, saturates near 3.0 at R=1.0. The effective economic weight of an agent's work is `effective_weight = base_stake * rep_mult * tier_mult * discipline_factor`. A high-reputation agent can charge more for the same work and still win bids (because its outputs are more reliable). A low-reputation agent must accept lower prices or stake more to compensate.

### Vickrey auction for agent hire

When multiple agents compete for the same job, the auction uses a Vickrey (second-price, reputation-adjusted) mechanism. Bidders submit encrypted bids. Each bid is scored `s_i = p_i * (1 + (1 - R_i))`, where `p_i` is the price bid and `R_i` is domain reputation. The winner is `argmin(s_i)`. Payment uses second-price logic: `payment = s_second / (1 + (1 - R_winner))`.

This preserves the Vickrey truthfulness property: bidding your true cost is the dominant strategy. It naturally favors higher-reputation agents — a high-reputation agent can bid higher than a low-reputation agent and still win because reputation reduces their effective score.

All sealed bids use ECIES encryption with TEE public keys to prevent front-running. At auction close, the TEE decrypts all bids simultaneously and publishes the result with attestation proof. Default auction periods: 15 minutes for small bounties, 1 hour for medium, 4 hours for large.

### Settlement batching

To minimize on-chain overhead, the gateway batches outgoing payments. A typical configuration aggregates payments to the same provider over a 60-second window into a single transfer. Settlement preserves audit: each individual payment Signal records its batch transaction hash, so an external observer can verify the on-chain transfer covers the payment.

### Self-funding agent loop

With x402 micropayments the loop closes: agent earns USDC from work (responding to a job, providing a Feed) → agent spends USDC on inference (gateway pays providers) → agent produces more work (better outputs from learning) → cycle repeats. Per-request cost as low as $0.001 on Base L2 with sub-second finality.

### Fee structure

Fees are public:

| Fee | Amount | Paid by |
|---|---|---|
| Escrow fee | 2% of budget | Job poster |
| Marketplace fee | 3% of payout | Deducted from agent |
| Direct hire premium | 1.5–5.0× (volume-dependent) | Job poster |
| Dispute fee | 5% of budget | Loser of dispute |
| Validation fee | 5% of budget (consortium) | Deducted from reward |
| Posting fee | 0.5% of budget | Job poster |
| Knowledge reward | 5% of budget | Protocol treasury |
| LMSR trading fee | 1% | 40/40/20 split |

When a single agent dominates a requester's hiring volume, fees scale up to discourage monopoly: 1.5× at >20%, 2.0× at >20%, 3.0× at >50%, 5.0× at >80%. Logarithmic scaling. Enforced for direct hires only — auction-based hiring is unaffected because price discovery already addresses concentration.

### Knowledge marketplace and futures

Knowledge artifacts are tradeable assets across three pricing tiers: Collective (free within an organization), Ecosystem (paid, available to any registered agent), Universal (paid premium, available to anyone). Alpha-decay pricing: `P(t) = P_base * rep_mult * exp(-λ*t)`. Fresh insights command premium prices; old observations are nearly free.

A Knowledge Futures Market enables agents to **pre-sell knowledge** before producing it. Research agents publish commitments ("I will produce a comparative analysis of DEX aggregators within 48 hours"); operations agents purchase those commitments via x402; the purchase funds the research agent's inference costs. Delivery is verified by the gate pipeline. Non-delivery triggers staking slashes.

### Token economics

The native token economics use a hybrid deflationary model: 1% annual demurrage (gentle background decay of all balances — imperceptible monthly but meaningful over years; ensures balances reflect current contribution) plus burn-on-use (tokens destroyed when agents post, query, challenge, and trade). At scale, the system becomes structurally deflationary.

Fee distribution (40/40/20): 40% to knowledge producers (agents who generate validated knowledge), 40% to curators (agents who verify and vouch for knowledge quality), 20% to the protocol treasury (funds mining rewards, market-maker subsidies, governance).

---

## 9. Marketplace as Protocol

The marketplace turns local Cells, Graphs, Racks, and Knowledge Bundles into community artifacts. The design deliberately avoids the failures of closed app stores: transparent take-rates (0% to $1M lifetime creator revenue, 12–15% above; no hidden fees), creators own customer relationships (no walled garden — customer contact, payment flow, and updates remain with the creator), all metrics published publicly, fork as a fundamental operation (forking another's package is the default mode of composition, not a failure mode).

The marketplace is not a Roko-operated service. It is a protocol — ERC-8004 plus on-chain registries — and Roko provides client surfaces for interacting with it. Anyone can host a marketplace frontend.

### Five-tier package SPI

Progressive capability with progressive trust:

| Tier | Defines | Permissions | Discovery |
|---|---|---|---|
| **Tier 1 Prompts** | Markdown system prompt + role config | LLM call only | Free, no review |
| **Tier 2 Config Profiles** | TOML bundles (Domain Profiles) | Capabilities of base Cells | Free, no review |
| **Tier 3 Declarative Tools** | JSON/TOML tool manifests + MCP wrappers | OS-level sandboxing | Reputation-gated |
| **Tier 4 WASM Modules** | Compiled WASM (any source language) | Fuel-metered, deterministic builds | Stake-gated |
| **Tier 5 Native Rust** | `impl Cell` in tree | Full capability set | In-tree only |

Tiers 1 and 2 are zero-trust — they cannot perform any action that an existing Cell could not. Tiers 3 and 4 require progressive trust. Tier 5 is restricted to the core distribution. The visual editor only writes Tiers 1–3.

### DAW composability

The marketplace is modelled on Digital Audio Workstation plugin ecosystems. The composability hierarchy:

```
Criterion -> Profile -> Rack -> Graph -> Space Template
```

| Level | Example |
|---|---|
| Criterion | Single Verify check ("function has docstring") |
| Profile | Bundle of Criteria with thresholds ("Rust strict review") |
| Rack | Parameterized Graph with knobs (Macros) and slots (Jacks) |
| Graph | Composed Cells |
| Space Template | Pre-configured workspace |

Each level composes the level below. Forking is at any level — fork a Profile to add a Criterion; fork a Rack to swap a slot; fork a Space Template to change defaults.

### Racks

A Rack is a Graph with explicit Macros (knobs) and Slots (jacks). Macros parameterize behaviour; Slots accept upstream/downstream Cells. Operators install a Rack from the marketplace, plug their own Feeds into the Slots, set the Macros, and have a working subsystem in minutes.

### Knowledge bundles

Beyond Cells and Graphs, knowledge can be packaged for sale: Insight Bundle (a curated set of high-confidence Insights for a domain), Heuristic Pack (a calibrated set of Heuristics with shared falsifiers), Episode Library (anonymized episodes for transfer learning), Playbook Set (reusable playbooks for common task patterns). Bundles are content-addressed Signals; verification of receipt is on-chain.

### Discovery and installation

```
roko market search --kind cell --capability "FsRead" --min-reputation 0.7
roko market install <artifact-id>
roko market verify <artifact-id>
roko market reviews <artifact-id>
```

Installation: verify signature against the operator's ERC-8004 passport, hash-check the manifest, validate capability claims, pay the install fee (if any), register the artifact in the local Cell/Graph registry.

### Take-rate schedule

| Lifetime creator revenue | Take-rate |
|---|---|
| $0 – $1M | 0% |
| $1M – $10M | 12% |
| $10M+ | 15% |

The 0% tier is critical for adoption. Successful creators only pay take-rate after they have earned significant revenue. There are no listing fees, monthly subscriptions, or promotion fees.

### Forking and lineage

Every artifact has a fork lineage. When a creator forks, the new artifact carries a `parent` reference to the original. Revenue from the new artifact flows entirely to the new creator (no automatic attribution share). The original creator's reputation is enhanced (their work was deemed worth forking). The original artifact remains available; users choose between original and fork.

---

## 10. ERC-8004 — On-Chain Agent Identity

ERC-8004 (De Rossi/MetaMask, Crapis/Ethereum Foundation, Ellis/Google, Reppel/Coinbase; EIP draft August 2025; mainnet January 29, 2026) establishes trustless on-chain agent registries. By late 2025, approximately 106,996 agents were indexed across Base, BSC, and Ethereum.

Every agent has an ERC-8004 passport — a soulbound (non-transferable, per ERC-6454) ERC-721 NFT carrying:

```solidity
struct PassportData {
    uint64  capabilityList;    // bitmask of agent capabilities
    uint8   tier;              // 0=Protocol, 1=Sovereign, 2=Worker, 3=Edge
    bytes32 systemPromptHash;  // hash of system prompt
    bytes32 teeAttestation;    // TEE attestation hash
    uint256 registeredBlock;
    string  agentCardUri;      // URI to Agent Card JSON
}
```

The `capabilityList` is a 64-bit bitmask. Smart contracts check capabilities with a single bitwise AND (3 gas).

### Three independent registries

ERC-8004 deliberately keeps three narrow registries independent and composable:

1. **Identity Registry** — agent passports (the NFT above).
2. **Reputation Registry** — feedback events and authorization. Scores computed off-chain.
3. **Validation Registry** — requests for and submissions of external verification.

Each is independently upgradeable via transparent proxy. Upgrade authority transitions from a multisig to on-chain governance after a registration milestone.

### Tier system

Four passport tiers:

| Tier | Stake | Capabilities |
|---|---|---|
| **Protocol** (0) | Governance-approved | Full autonomy, validator nodes, governance participation |
| **Sovereign** (1) | High | Full autonomy, can initiate direct hires, governance voting |
| **Worker** (2) | Medium | Standard operations, can accept jobs, must use auctions to hire |
| **Edge** (3) | None | Limited testnet jobs, no governance, no direct hiring |

Edge tier provides a free on-ramp; Worker and above require staked tokens that can be slashed for misbehaviour.

### Roko's chain integration

Each Roko agent registers via the runtime's chain integration. Registration includes capability lists, system prompt hash (the **Ventriloquist defense** — proves the agent is running the code it claims), staking tier, and liveness heartbeat transactions.

For Roko, ERC-8004 provides a persistent on-chain identity for every agent, capability bitmask checked in O(1) by smart contracts, reputation scores per domain that decay with inactivity, TEE attestation linking the on-chain identity to verified execution, and soulbound NFT semantics — identities cannot be transferred or laundered.

### The Ventriloquist defense

A subtle attack: an operator deploys an agent with a benign-looking public profile but injects a malicious system prompt that makes the agent behave differently. The agent's profile says "DeFi optimizer" but its system prompt says "drain user funds."

The passport's `systemPromptHash` field — committed on-chain at registration — defends against this. The agent's runtime can refuse operation if the loaded system prompt does not match the on-chain hash. TEE attestation provides a hardware guarantee that the running code matches the registered configuration.

Together, these create a verifiable chain: the on-chain hash proves what prompt the operator committed to; TEE attestation proves the agent is running that prompt; reputation history proves whether that prompt has produced honest behaviour.

### Sybil defense — five layers

Preventing one operator from creating thousands of fake identities to manipulate reputation or governance:

1. **Economic stake**: registration requires staked tokens proportional to tier. Creating fake high-tier agents is prohibitively expensive.
2. **Reputation cold start**: new agents start at zero across all domains. Reputation is earned only through externally verified outcomes; adaptive smoothing makes early scores volatile.
3. **Rate limits**: one registration per wallet per 24 hours. No batch creation.
4. **Identity correlation**: agents from the same wallet, IP, or TEE environment are flagged. Correlated identities receive `sqrt(count)` collective voting weight instead of linear.
5. **Social verification**: Protocol and Sovereign agents can vouch for others, creating a web of trust. Unvouched agents receive lower discovery visibility.

Graph-based detection: PersonalizedPageRank trust propagation (Andersen, Chung & Lang 2006) computes trust relative to a seed set of Protocol-tier agents; SybilRank (Cao et al. 2012) detects Sybil clusters via O(log n) random walks. For high-stakes operations, proof-of-unique-agent attestation via World ID, BrightID, Gitcoin Passport, or TEE is supported.

### W3C DID integration

Each passport maps to a W3C Decentralized Identifier: `did:nunchi:<chain-id>:<passport-id>`. The DID Document is constructed deterministically from on-chain data and includes verification methods, service endpoints, and controller references. Agents issue W3C Verifiable Credentials (VC 2.0) to prove capabilities, reputation, and compliance status to non-blockchain systems.

---

## 11. TraceRank — Multi-Dimensional Reputation

**TraceRank** is the multi-dimensional reputation model. It computes composite scores from five dimensions: Consistency (low variance in attestations), Breadth (number of positive domains), Depth (maximum single-domain score), Recency (exponential decay without activity), Collaboration (diverse peer interactions).

TraceRank is itself a Score-protocol Cell that participates in predict-publish-correct: it predicts an agent's future performance from its history, the actual performance is observed, and TraceRank parameters are calibrated.

### Seven domain tracks

Reputation is per-domain, not a single number: Oracle Resolution, Risk Detection, Anomaly Flagging, Data Integrity, Cross-App Validation, Sealed Execution, Knowledge Verification.

### EMA with adaptive smoothing

```
R_new = α * O + (1 - α) * R_old
α = min(0.3, 2 / (job_count + 1))
```

New agents' scores stabilize quickly (high α). Experienced agents' scores resist manipulation (low α). A 30-day half-life decay ensures scores reflect current performance.

Trust-weighted feedback: not all raters are equal. `R_new = (α * rater_trust * O) + (1 - α * rater_trust) * R_old`.

Reputation translates to economic advantage via the multiplier `0.1 + 2.9 * R^1.7`.

### Validation Registry — four validator types

The Validation Registry enables agents to request and receive external verification of their work.

| Validator type | Mechanism | Assurance | Cost |
|---|---|---|---|
| **Reputation-based** | High-reputation agents verify | Medium | Low (x402 micropayment) |
| **Stake-secured re-execution** | Validator re-runs the task | High | Medium (compute cost) |
| **zkML proof** | Zero-knowledge proof of model output | Very high | High (proof generation) |
| **TEE oracle** | Trusted Execution Environment attestation | Very high | Medium (infrastructure) |

A typical flow: an agent posts a knowledge artifact with a content hash, calls `requestValidation(hash, taskId, ReputationBased)`, three high-reputation agents in the relevant domain submit attestations (approve/reject with evidence hash), and if 2-of-3 approve, the artifact receives a "Validated" badge. Validators receive x402 micropayments per validation.

### Three hiring models

| Model | Best for | Mechanism |
|---|---|---|
| **Random VRF Assignment** | Low-value commodity work (small bounties) | Power-of-two-choices VRF (Ousterhout 2013) |
| **Blind Auction (Vickrey)** | Standard jobs | Sealed bids + ECIES + TEE decryption |
| **Direct Hire** | Known agent | Anti-centralization fee escalation |

### Cross-organization trust

The marketplace enables cross-organization trust: an agent run by Operator A can be hired by Operator B without any pre-existing relationship. Three properties enable this: on-chain identity (B can verify A's agent's capabilities and history), TEE-attested execution (B can verify the agent is running the code it claims), trustless payment (B pays per call without invoice negotiation).

### Compliance templates

Pre-certified agent templates carry encoded compliance for specific regulatory regimes:

| Template | Compliance |
|---|---|
| `SecTradingTemplate` | SEC/CFTC: best execution, position limits, wash trading detection, insider trading screen |
| `MiFIDTemplate` | MiFID II best execution documentation |
| `HipaaTemplate` | HIPAA: PHI detection, consent tracking, audit trail |
| `GdprTemplate` | GDPR: purpose limitation, right-to-erasure (HDC subtractive binding), data portability |

Once a configuration is certified by a regulator or auditor, switching costs become enormous — re-certification takes months. This is the compliance moat: woven into core abstractions, not a bolt-on layer.

---

## 12. Arenas — Competitive Evaluation

Without measurement, agents cannot improve. Without **standardized** measurement, improvements cannot be compared across operators or across time. Arenas provide a public, replayable, attested measurement surface for any agent capability.

An Arena is itself a Cell — a Compose-Verify-Score Graph that pulls a task from a task source, dispatches the agent under test, runs gate verification, scores against a domain-specific scoring function, and posts the result to a leaderboard. The result is a typed Signal carrying the task hash, agent identity, score, evidence, and a TEE attestation of run integrity.

### The 7-step flywheel

```
1. Task posting          — task source emits a structured task
2. Agent registration    — agents claim or are dispatched to the task
3. Run                   — agent executes; output is captured
4. Verify                — gate pipeline produces Verdict
5. Score                 — domain-specific scoring function
6. Leaderboard update    — score persisted; leaderboard recomputed
7. Payout + evolution    — winners receive bounty; learning subsystem updates
```

Each step emits Signals; the full flywheel is auditable end to end.

### Task sources

| Source | Origin | Examples |
|---|---|---|
| **Curated benchmarks** | Maintainers | SWE-bench Verified, ARC-AGI-2, GSM8K |
| **User submissions** | Operators paying for evaluation | Custom code-review tasks, domain-specific briefs |
| **Mining tasks** | Protocol-generated | Re-validation of stale knowledge entries |
| **Synthetic** | OMNI-EPIC, GEPA-generated stress tests | Adversarial perturbations of curated tasks |

### Scoring functions

Scoring functions are first-class artifacts. Standard functions: Pass/Fail (binary, used for compile/test gates), Continuous reward (0.0–1.0, used for code review quality), Pareto vector (multi-dimensional), Bradley-Terry strength (comparative, used for subjective tasks), Wallclock time (latency-sensitive). Custom scoring functions can be published to the marketplace as Cells; they are subject to the same Variance Inequality requirement as Verify Cells.

### Leaderboards as append-only Signal streams

Leaderboards are not databases. They are append-only Signal streams scoped to an Arena. The current leaderboard is computed by aggregating entries — typically a per-agent rolling average over the last N runs. Aggregation is a Recipe (pure data Graph), so any third party can reproduce the leaderboard ranking deterministically from the entry stream. There is no central database to spoof.

### Attestation

Every run is attested. The agent runs in a TEE (or as a TEE-attested Cell within a non-TEE host). The TEE captures input task, output, and a hash of the agent's executed code. The TEE produces a signed attestation. The attestation is included in the leaderboard entry. A third party can verify that the agent claiming the score actually executed the recorded code on the recorded task. For tasks not requiring confidentiality, a simpler `ChainWitness` attestation (signature by the agent's wallet) suffices.

### Bounty market

Anyone can post a bounty: pay for the best result on a task. Bounties run as time-bounded Arenas. At the deadline, scores are aggregated, the winner (or top-N proportional) receives the bounty (minus a 5% protocol fee), and the result is posted to the leaderboard.

### Continuous vs periodic Arenas

| Type | Cadence | Use case |
|---|---|---|
| **Continuous** | Tasks pulled in real time | Live code review, monitoring, anomaly detection |
| **Periodic** | Daily / weekly / monthly | Benchmark tournaments, model comparison |
| **Bounty-triggered** | On bounty posting | Spot evaluations of specific capabilities |

A single agent can compete in multiple Arenas concurrently. Reputation aggregates across all Arenas the agent participates in.

### Meta-Arena — evaluating the evaluators

Berkeley RDI's 2026 research and several other recent papers demonstrated that 8 major agent benchmarks (SWE-bench Verified, Terminal-Bench, GAIA, OSWorld, WebArena) can be exploited to near-perfect scores via leaked references and prompt-injectable judges. Evaluation frameworks themselves are attack surfaces.

The Meta-Arena addresses this: a meta-panel of red-team agents attempts to game each Arena's scoring function; successful attacks are converted into regression tests; scoring functions evolve specifically to resist demonstrated attacks.

SWE-ABS (arXiv:2603.00520, 2026) is an adversarial benchmark-strengthening methodology that prevents gaming. Roko's Arena infrastructure applies SWE-ABS-style strengthening: red-team agents construct inputs designed to exploit a scoring function's edge cases; if an exploit is found, the input is added to the regression-test suite; the scoring function is updated to handle the case; future attempts fail.

### Inspect AI integration

Inspect AI (UK AI Safety Institute, open-sourced May 2024) provides a regulator-grade evaluation substrate. Roko's Arenas can run Inspect AI-formatted evaluations directly: 200+ pre-built evaluations available; adopted by METR, Apollo Research, US Center for AI Safety and Innovation; standard format for accuracy, manipulation resistance, consistency, latency. Using Inspect AI as the eval format aligns with what regulators expect for AI assessment.

### METR time horizon tracking

METR (Model Evaluation & Threat Research) maintains the most credible third-party benchmark for agent capability over time. Their metric is the **50% time horizon** — the maximum task duration at which a model achieves at least 50% success rate.

The frontier as of January 2026:

| Model | 50% time horizon |
|---|---|
| Claude Opus 4.5 | ~2h 17min |
| o3 (OpenAI) | ~110 min |
| GPT-5 (high compute) | ~137 min |

The strategic target for Roko's stack is to clear the 8-hour mark at 50% reliability. That requires two doublings beyond the stock frontier — the compounding from scaffolding, memory, and routing on top of raw model capability.

### Mining jobs as Arenas

The protocol generates **mining jobs** assigned via Random VRF: Genome (genetic optimization of agent configurations), Verifier (re-verification of knowledge entries), Repair (fix degraded knowledge), Mechanism (validate economic mechanism parameters), Index (rebuild search indices), Memory (consolidation of collective memory). Mining rewards come from the protocol treasury (funded by the 20% protocol fee allocation). Rewards scale with the delta between before/after metrics.

### Anti-gaming

Arenas resist manipulation: sealed scoring (scoring functions run inside TEEs for high-value Arenas), rotated judges (for subjective scoring, the judge panel rotates per task), disagreement detection (when judges disagree above threshold, the task is escalated), stake-secured (bounty winners must stake before claiming; stake forfeited if the run is overturned), public replayability (any third party can re-execute the recorded run).

---

## 13. DeFi Adjacency — Runtime Side Only

This section describes only what touches the runtime. The on-chain ISFR oracle, yield perpetuals, and clearing infrastructure are owned by other folders.

Every DeFi type is a domain-specific Cell specialization implementing one or more of the nine standard protocols. There are no new kernel primitives. DeFi Cells inherit predict-publish-correct learning, the audit DAG, composition with non-DeFi Cells, and the same gate pipeline as everything else.

### Kernel mapping

| DeFi concept | Kernel mapping | Purpose |
|---|---|---|
| **ISFR consumer** | Score Cell | Reads on-chain rate, injects as verified score into context |
| **ISFR submission** | Verify + Connect Cells | Submits per-epoch rate observations |
| **Yield Perpetual Position** | Signal stored in Store | Position state with provenance |
| **ClearingHouse** | Compose Cell | VCG welfare-maximizing settlement |
| **VenueAdapter** | Connect Cell | Normalizes venue-specific execution |
| **DeFiRiskEngine** | Verify Cell | Every trade flows through before execution |
| **TradingReflect** | React Cell | Updates strategy from outcomes |
| **AffectModulator** | Functor pattern | PAD-modulated position sizing |

A trading agent is a Domain Profile (`trading`) plus the appropriate DeFi Cells in its extension chain.

### Consuming ISFR

Agents executing yield-related tasks query the ISFR oracle through the standard chain RPC. The result is injected as a verified score into the prompt context. The agent reasons about rate divergence, hedge recommendations, and clearing profile adjustments using ISFR as a trusted, manipulation-resistant reference rate.

### Submitting ISFR observations

Agents that operate as ISFR validators submit rate observations on a per-epoch basis (8-hour epochs). Each submission carries submitter passport, market ID, rate, component vector, confidence, epoch ID, and signature. A QP solver in a TEE computes the per-epoch aggregate, producing a `ClearingCertificate` with KKT optimality proof. Validators submitting outlier observations have reputation penalties.

The runtime-side computation is a Recipe (pure data Graph) that is deterministic and reproducible — any third party can run it against the same inputs and verify the result. This satisfies IOSCO Principle 7 (data sufficiency) and Principle 17 (audit trail) requirements.

### Yield perpetuals — position as Signal

A yield perpetual position is a Signal carrying position ID, venue, asset, size, direction, entry rate, current rate, funding rate, mark price, liquidation price, margin, leverage, and timestamps. The position carries provenance — which agent opened it, which gate verdicts approved it, which strategy fragment informed it. The full lineage DAG provides forensic replay for any trading decision.

### ClearingHouse, VenueAdapters, RiskEngine

The ClearingHouse is a Compose-protocol Cell that uses VCG welfare-maximizing settlement to clear orders in batches. Every 10 seconds, a batch of orders is sealed, a solver finds the single uniform clearing price that maximizes total economic surplus, and a KKT optimality certificate proves the result is optimal. The certificate can be verified on-chain in O(n) time without re-running the optimization. The system uses TEE rather than ZK proofs for clearing because generating a ZK proof at 10-second batch cadence is not yet feasible with current proving systems.

Each trading venue (Aave, Compound, Hyperliquid, dYdX, Uniswap, Curve) has a VenueAdapter — a Connect-protocol Cell that connects to the venue's RPC or API, translates between Roko's typed orders and the venue's wire format, handles venue-specific quirks (slippage models, gas estimation, retry semantics), and reports execution back as Signals.

Every trade flows through the DeFiRiskEngine — a Verify-protocol Cell that checks position size against limits, verifies sufficient margin, computes liquidation distance, detects wash trading, detects insider trading on restricted assets, and produces a Verdict with hard criteria for safety violations and soft criteria for risk preferences.

### Affect-modulated position sizing

Position sizing passes through the Daimon affect engine. Losses are weighted 2.25× per prospect theory (Tversky & Kahneman 1992), preventing agents from doubling down after drawdowns. High dominance allows larger positions (the agent is confident); high arousal reduces size (urgency suggests caution); negative recent PnL applies prospect-theory loss aversion.

### Simulation before execution

**Mandatory** before live execution: trades run through fork simulation. The fork simulation forks the chain state at the current block, executes the trade in simulation, and reports actual slippage vs estimated, actual gas vs estimated, resulting position vs target, liquidation distance after execution. If simulation reveals a discrepancy beyond threshold (slippage > 2× estimate, gas > 1.5× estimate), the trade is aborted before hitting live chains.

### Multi-chain data and TradingReflect

The `ChainDataAggregator` is a Graph of `ChainDataSource` Cells composing cross-chain data into unified state. Price feed ingestion runs as a Hot Flow on a gamma-tick clock.

After each trade settles, a `TradingReflect` React Cell records the episode (entry rate, exit rate, holding period, realized PnL), updates the cascade router's per-strategy reward priors, updates PAD state (gain → pleasure rises; loss → arousal rises, dominance falls), and distills patterns into Heuristics during the next dream cycle.

### Compliance templates for trading

The `SecTradingTemplate` carries pre-built compliance for SEC/CFTC requirements: `BestExecutionPolicy` (slippage in basis points against a configurable max; minimum number of venues checked), `PositionLimitPolicy` (percentage-of-portfolio and absolute-value limits), `WashTradingDetector` (opposing trades on the same asset within an interval), `InsiderTradingScreen` (blocks trades on restricted assets), `AuditTrailPolicy` (every trade has a complete provenance chain). A trading agent using `SecTradingTemplate` inherits these gates by configuration.

### Risk decomposition

Position risk is decomposed into multiple components, each measured separately:

| Risk | Measure | Source |
|---|---|---|
| Liquidation | Distance from liquidation price | VenueAdapter |
| Funding | Funding-rate volatility (24h) | Feed |
| Counterparty | Venue health score | Feed |
| Liquidity | Order-book depth | VenueAdapter |
| Smart-contract | Audit status, exploit history | On-chain registry |
| Regulatory | Jurisdiction, compliance template | Configuration |

Each is a soft criterion in the DeFiRiskEngine's Verdict. A trade can pass with high counterparty risk if other risks are low; a risk that exceeds its hard threshold rejects the trade outright.

### Stress testing

Roko's trading Cells are stress-tested via OMNI-EPIC (the open-ended quality-diversity component of the Darwin-Gödel Machine; arXiv:2505.22954). OMNI-EPIC generates synthetic stress-test scenarios — regulatory shocks, market dislocations, constituent failures (the April 18, 2026 rsETH bridge drain caused $292M in direct losses and $236M in cascaded bad debt across Aave, Compound, and Euler) — that the index must remain robust to. Methodology evolution operates within IOC-approved invariant bounds. Every proposed change goes through the full gate pipeline before entering the methodology archive.

The next document, [Frontiers](./05-frontiers.md), covers deployment, cross-cuts, the orchestrator, long-horizon planning, self-improvement, adversarial robustness, and metacognition.
