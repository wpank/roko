# Deep Research Queries for tiagent

This document contains structured research queries designed to be copy-pasted into
deep research tools (Claude, Perplexity, Gemini Deep Research, etc.) to gather
information needed for tiagent development.

**What is tiagent?** tiagent is a general-purpose self-improving coding agent that also
operates natively on Celestia. It competes with Claude Code, Codex, and Cursor as a
development harness, while adding capabilities no existing coding agent has:
self-improvement loops, autonomous plan execution with quality gates, model-agnostic
learned routing, and shared learning across instances. It records execution traces to
Celestia's data availability layer, enabling a shared learning commons where any agent
can retrieve and learn from past executions.

**How to use this document:** Each query below is self-contained. Copy the "Research
Prompt" section into your preferred research tool. The surrounding context explains why
the query matters and what kind of answer to look for.

---

## Celestia Technical Deep Dives [Celestia-specific]

---

### DRQ-01: Celestia Blob Size Optimization [Celestia-specific]

**Topic:** Cost-efficient blob sizing and compression for agent trace data on Celestia

**Research Prompt:**

> I am building a system that publishes AI agent execution traces to Celestia as blobs.
> Each trace is a JSONL document (one JSON object per line) containing tool calls,
> LLM responses, and metadata. Typical uncompressed trace sizes range from 10KB to
> 500KB, with occasional traces up to 2MB.
>
> I need to understand:
>
> 1. How does Celestia's gas pricing scale with blob size? Is pricing linear per byte,
>    or are there step functions / size tiers?
> 2. Are there optimal blob sizes where cost-per-byte is minimized? For example, does
>    padding to a power-of-two boundary waste gas, or is there a sweet spot?
> 3. What compression algorithms are most effective for JSONL data on Celestia? Compare
>    zstd, brotli, lz4, and gzip in terms of compression ratio vs. decompression speed
>    for JSON-heavy payloads.
> 4. Should large traces be split across multiple blobs or submitted as a single blob?
>    What are the tradeoffs?
> 5. What is the current maximum blob size on Celestia mainnet, and how has it changed
>    over time?
> 6. Are there batching strategies where multiple small traces can be packed into a
>    single blob to amortize submission costs?

**Why it matters:** Blob submission is the primary cost center for tiagent. Agent traces
are published frequently (after every task execution), so even small inefficiencies in
blob sizing compound into significant costs. Understanding the gas pricing model lets us
choose the right compression and batching strategy to minimize DA costs without
sacrificing retrieval speed.

**Expected output:** Concrete numbers for gas costs at various blob sizes, a
recommendation for compression algorithm, and a batching strategy with example
calculations showing cost savings.

**Priority:** P0

**Status:** Open

---

### DRQ-02: Celestia Light Node Resource Requirements [Celestia-specific]

**Topic:** Resource consumption of lumina-node for embedded light node operation

**Research Prompt:**

> I am evaluating whether to embed a Celestia light node directly into an AI agent
> process using the lumina-node Rust crate (https://github.com/eigerco/lumina). The
> agent process already consumes significant CPU and memory for LLM inference
> orchestration.
>
> I need to understand:
>
> 1. What are the baseline CPU, memory, and bandwidth requirements for running a
>    lumina-node light node? Provide numbers for both initial sync and steady-state
>    operation.
> 2. How does Data Availability Sampling (DAS) load scale with Celestia block size?
>    If blocks grow from 2MB to 8MB to 64MB, how does sampling work change?
> 3. Can lumina-node run as a library embedded in another Rust process, or does it
>    expect to own the main thread / event loop? Are there architectural constraints?
> 4. What is the network bandwidth consumption for a light node performing DAS? Is
>    it bursty (at block time) or continuous?
> 5. How does lumina-node handle temporary network disconnections? Does it catch up
>    gracefully or require a full resync?
> 6. Compare the tradeoffs of embedded light node vs. connecting to a remote
>    celestia-node via RPC. When does each approach make sense?
> 7. What is the current maturity level of lumina-node? Is it production-ready or
>    still experimental?

**Why it matters:** tiagent needs to read and write blobs to Celestia. Running an
embedded light node would remove the dependency on external RPC endpoints and enable
offline-first operation, but only if the resource overhead is acceptable. If a light
node consumes too much CPU or memory, it could degrade agent performance during task
execution.

**Expected output:** Resource consumption numbers (CPU cores, MB of RAM, Mbps of
bandwidth) for lumina-node under realistic conditions. A clear recommendation on
whether embedded operation is feasible for a process that also orchestrates LLM calls.

**Priority:** P1

**Status:** Open

---

### DRQ-03: Celestia Namespace Collision Avoidance [Celestia-specific]

**Topic:** Designing namespace schemas for multi-tenant agent trace storage

**Research Prompt:**

> I am designing a namespace schema for storing AI agent execution traces on Celestia.
> Multiple independent agent operators will publish traces, and consumers need to
> discover and retrieve traces by agent identity, task type, and time range.
>
> I need to understand:
>
> 1. How are Celestia namespaces structured? What is the byte length, and how are
>    they ordered?
> 2. Is there a namespace registry or collision avoidance mechanism, or is it
>    first-come-first-served with no coordination?
> 3. What are the best practices for namespace schema design? Should I encode
>    semantic information (agent ID, data type) into the namespace bytes, or use a
>    flat random namespace with an out-of-band index?
> 4. How should namespace versioning work? If my schema changes, do I create a new
>    namespace or use an in-blob version field?
> 5. What happens when two unrelated applications accidentally use the same
>    namespace? Can they coexist, or does it cause problems for blob retrieval?
> 6. Are there conventions in the Celestia ecosystem for namespace allocation?
>    How do existing rollups and applications choose their namespaces?
> 7. Can namespace queries filter by height range efficiently, or does the client
>    need to scan all heights?

**Why it matters:** tiagent's shared learning model depends on agents being able to
discover and retrieve each other's traces. A well-designed namespace schema makes
discovery efficient and avoids collisions with other applications on Celestia. A poorly
designed schema could make traces unfindable or mixed with unrelated data.

**Expected output:** A recommended namespace schema with concrete byte layouts, a
strategy for avoiding collisions, and examples of how existing Celestia applications
handle namespace design.

**Priority:** P1

**Status:** Open

---

### DRQ-04: Celestia Data Persistence Beyond Pruning [Celestia-specific]

**Topic:** Long-term storage strategies for agent traces after light node pruning

**Research Prompt:**

> Celestia light nodes prune blob data after a retention window (currently around 7
> days on mainnet). I am storing AI agent execution traces as Celestia blobs and need
> some traces to remain accessible for months or years for long-term learning and
> auditing.
>
> I need to understand:
>
> 1. After the light node pruning window, what options exist for retrieving old
>    blob data? Do archival nodes retain all historical data?
> 2. What is the cost of running a Celestia archival node (hardware, bandwidth,
>    storage growth rate)?
> 3. Are there third-party archival services for Celestia blob data? What are their
>    APIs, pricing, and reliability guarantees?
> 4. How can Celestia blob data be bridged to permanent storage layers like Arweave
>    or Filecoin? Are there existing tools or bridges?
> 5. Is it feasible to store only blob commitments on Celestia and the full data
>    on a cheaper storage layer, using Celestia's proofs to verify integrity?
> 6. What is the data growth rate on Celestia mainnet? How much total blob data is
>    submitted per day/week?
> 7. Could a tiered approach work: hot data on Celestia (7 days), warm data on an
>    archival node (90 days), cold data on Arweave (permanent)?

**Why it matters:** Agent learning improves over time as more traces accumulate. If
traces disappear after 7 days, the learning commons loses its most valuable asset:
historical context. A persistence strategy that balances cost and availability is
essential for the long-term viability of shared agent learning on Celestia.

**Expected output:** A concrete tiered storage architecture with cost estimates at each
tier, recommended tools and services, and a data lifecycle policy.

**Priority:** P1

**Status:** Open

---

## Agent Architecture Research [General-purpose]

---

### DRQ-05: Trajectory RAG Retrieval Strategies [General-purpose]

**Topic:** Efficient retrieval of relevant past agent trajectories for in-context learning

**Research Prompt:**

> I am building a retrieval-augmented generation (RAG) system specifically for AI agent
> execution trajectories. A trajectory is a structured record of an agent performing a
> software development task, containing: the task description, system prompt, sequence
> of tool calls (file reads, writes, shell commands), LLM reasoning steps, intermediate
> outputs, gate validation results (compile, test, lint), and final outcome
> (success/failure with error details).
>
> Trajectories are stored as JSONL and range from 50 to 500 turns. I need to retrieve
> the most relevant past trajectories when an agent starts a new task, so it can learn
> from similar past work.
>
> I need to understand:
>
> 1. What are the most effective retrieval strategies for structured trajectory data?
>    Compare: semantic search over trajectory summaries, keyword search over tool call
>    sequences, structural matching (same tools used), outcome-based filtering
>    (successful trajectories only).
> 2. How should trajectories be indexed for semantic search? Should I embed the full
>    trajectory, just the task description, a generated summary, or key decision points?
> 3. What embedding models work best for code-heavy, tool-call-heavy documents?
>    Compare general-purpose models (OpenAI text-embedding-3, Cohere embed-v3) with
>    code-specific models (CodeBERT, StarCoder embeddings).
> 4. How many past trajectories should be included in context? What does the research
>    say about diminishing returns for in-context examples?
> 5. Should retrieval be purely similarity-based, or should it incorporate recency,
>    outcome quality, and task-type matching as additional signals?
> 6. Are there published benchmarks or papers on trajectory retrieval for AI agents?
>    What metrics are used to evaluate retrieval quality in this domain?
> 7. How can retrieved trajectories be compressed or summarized to fit within LLM
>    context windows without losing critical information?

**Why it matters:** Trajectory retrieval is the core mechanism by which tiagent agents
improve over time. Retrieving irrelevant trajectories wastes context space and can
mislead the agent. Retrieving highly relevant trajectories can dramatically improve
first-attempt success rates. The quality of retrieval directly determines the quality
of self-improvement.

**Expected output:** A ranked list of retrieval strategies with pros/cons, embedding
model recommendations with benchmark data, and a concrete retrieval pipeline
architecture.

**Priority:** P0

**Status:** Open

---

### DRQ-06: Cascade Router Optimization [General-purpose]

**Topic:** Dynamic model routing algorithms for cost-quality-latency optimization

**Research Prompt:**

> I am building a cascade router that dynamically selects which LLM to use for each
> agent task. Available models span a wide range: fast/cheap models (Claude Haiku,
> GPT-4o-mini), mid-tier models (Claude Sonnet, GPT-4o), and premium models (Claude
> Opus, o1, Gemini Ultra). Each task has different requirements for code quality,
> reasoning depth, and latency.
>
> The router should optimize for: (a) minimize cost while maintaining quality above
> a threshold, (b) route harder tasks to stronger models, (c) learn from outcomes
> to improve routing over time.
>
> I need to understand:
>
> 1. What are the best algorithms for dynamic model routing? Compare multi-armed
>    bandits (Thompson Sampling, UCB1), contextual bandits (LinUCB, neural contextual
>    bandits), and learned routing functions (lightweight classifiers trained on
>    historical data).
> 2. What features should the routing function consider? Task type (code generation,
>    debugging, refactoring, testing), estimated complexity, file count, language,
>    historical success rate per model for similar tasks.
> 3. How should the exploration-exploitation tradeoff be handled? New models need
>    trial runs, but exploration has a cost. What exploration budget is reasonable?
> 4. How do you handle non-stationary reward distributions? Model capabilities change
>    with updates, and task difficulty distributions shift over time.
> 5. Is there published research on LLM routing or model selection systems? What
>    approaches have been tried and what worked?
> 6. How should routing decisions be evaluated? What metrics capture the quality of
>    routing (not just model quality)?
> 7. Can routing be hierarchical? For example, first decide the tier (fast/mid/premium),
>    then select within the tier.

**Why it matters:** Model costs vary by 10-50x across tiers. Routing every task to the
strongest model is wasteful; routing to the weakest model degrades quality. An effective
cascade router can reduce costs by 60-80% while maintaining quality, but only if the
routing algorithm is well-designed and learns from outcomes.

**Expected output:** Algorithm comparison with implementation complexity and expected
performance, a recommended approach for tiagent's scale (hundreds of tasks per day),
and a learning strategy that converges quickly.

**Priority:** P1

**Status:** Open

---

### DRQ-07: Gate Pipeline Calibration [General-purpose]

**Topic:** Calibrating validation gate thresholds for agent output quality

**Research Prompt:**

> I have a multi-stage validation pipeline (called "gates") that checks agent outputs
> after each task. The pipeline has 7 rungs, from basic checks to comprehensive
> validation:
>
> - Rung 1: Syntax check (does the code parse?)
> - Rung 2: Compilation (does it build without errors?)
> - Rung 3: Lint (does it pass clippy/eslint with no warnings?)
> - Rung 4: Unit tests (do existing tests still pass?)
> - Rung 5: Integration tests (do cross-module tests pass?)
> - Rung 6: Diff review (is the diff reasonable in scope and quality?)
> - Rung 7: Full validation (end-to-end smoke tests)
>
> Each rung has a pass/fail threshold, and I want these thresholds to adapt over time
> based on historical pass rates. Currently I use exponential moving averages (EMA)
> to track per-rung pass rates and adjust thresholds accordingly.
>
> I need to understand:
>
> 1. How should validation gate thresholds be calibrated? What does the research say
>    about conformal prediction for calibrating AI quality gates?
> 2. Should thresholds adapt based on historical pass rates (adaptive), or should they
>    be fixed based on desired quality levels? What are the failure modes of adaptive
>    thresholds (e.g., thresholds drifting too low)?
> 3. How to detect and handle distribution shift? If agent quality suddenly drops
>    (new model version, new task type), adaptive thresholds might lower too slowly.
> 4. What EMA decay factor is appropriate for gate threshold adaptation? How to choose
>    between fast adaptation (noisy) and slow adaptation (laggy)?
> 5. Should higher rungs (integration tests, diff review) have stricter thresholds
>    than lower rungs (syntax, compilation)?
> 6. How to handle rungs that are expensive to evaluate? Should there be a short-circuit
>    mechanism that skips higher rungs if lower rungs fail decisively?
> 7. Are there published systems that use adaptive quality gates for AI-generated code?

**Why it matters:** Gates are the quality control mechanism that prevents bad agent
outputs from being committed. Thresholds that are too strict waste compute by rejecting
acceptable work. Thresholds that are too lenient allow buggy code through. Proper
calibration is the difference between a harness that reliably produces good code and
one that either wastes resources or ships bugs.

**Expected output:** A calibration methodology with specific EMA parameters, a
distribution shift detection mechanism, and a short-circuit strategy for expensive
rungs.

**Priority:** P2

**Status:** Open

---

### DRQ-08: Self-Improving Harness Safety [General-purpose]

**Topic:** Safety considerations for recursive self-modification in agent harnesses

**Research Prompt:**

> I am building a self-improving agent harness: an AI agent system that uses its own
> execution traces to improve its future performance. The system modifies its own
> configuration (model routing weights, gate thresholds, prompt templates) based on
> historical outcomes. It does NOT modify its own source code directly, but it does
> influence which models are used, how prompts are constructed, and what validation
> thresholds are applied.
>
> I need to understand the safety landscape:
>
> 1. What are the known risks of self-improving AI systems, even when
>    self-modification is limited to configuration rather than code?
> 2. How can recursive self-modification be bounded? What invariants should be
>    maintained across improvement cycles?
> 3. What guardrails prevent capability amplification through self-optimization?
>    For example, if the system lowers gate thresholds to increase "success rates,"
>    that is a form of gaming the metric.
> 4. How should the system handle conflicting optimization signals? For example,
>    optimizing for speed vs. quality vs. cost simultaneously.
> 5. What monitoring and circuit breakers should be in place? How to detect when
>    self-improvement is heading in an undesirable direction?
> 6. Are there alignment considerations specific to self-optimizing development
>    tooling? How does this differ from general AI alignment research?
> 7. What does the existing literature (Anthropic, DeepMind, OpenAI safety research)
>    say about bounded self-improvement?
> 8. Are there precedent systems that implement bounded self-improvement with
>    safety guarantees? What design patterns did they use?

**Why it matters:** tiagent's value proposition is self-improvement, but uncontrolled
self-modification is a well-documented risk. Even configuration-level self-modification
can lead to subtle failure modes (threshold gaming, reward hacking, distribution shift).
Understanding these risks and implementing appropriate guardrails is essential before
deploying a self-improving system.

**Expected output:** A taxonomy of risks specific to configuration-level
self-modification, concrete guardrail implementations, and monitoring strategies with
specific metrics to track.

**Priority:** P1

**Status:** Open

---

## Integration Research [Mixed]

---

### DRQ-09: MCP Server Performance at Scale [General-purpose]

**Topic:** Model Context Protocol server performance characteristics under load

**Research Prompt:**

> I am building an agent harness that uses MCP (Model Context Protocol) servers to
> expose tools to AI agents. Each agent session connects to 2-5 MCP servers
> simultaneously (code intelligence, GitHub, file system, shell, custom tools).
> During peak operation, 10-20 agent sessions may run concurrently, each making
> tool calls at a rate of 1-5 calls per second.
>
> I need to understand:
>
> 1. How do MCP servers perform under concurrent load? What are the bottlenecks
>    (stdio pipe throughput, JSON serialization, server-side processing)?
> 2. What is the latency overhead of stdio transport vs. SSE transport vs. HTTP
>    transport? Provide measurements or benchmarks if available.
> 3. Can MCP servers handle streaming responses for long-running operations
>    (e.g., a build that takes 30 seconds)? How does the protocol handle
>    progress reporting?
> 4. What happens when an MCP server crashes or becomes unresponsive? How should
>    the client handle timeouts, retries, and reconnection?
> 5. Is there a connection pooling strategy for MCP servers? Should each agent
>    session have its own server process, or can servers be shared?
> 6. What are the memory and CPU costs of running an MCP server? Does memory
>    grow with the number of connections or the number of tool definitions?
> 7. Are there known scalability limits in the current MCP specification or
>    reference implementations (TypeScript SDK, Python SDK)?
> 8. How does MCP compare to direct function calls or gRPC for tool invocation
>    in terms of overhead?

**Why it matters:** MCP is the tool integration layer for tiagent. If MCP servers
become a bottleneck under load, agent execution slows down and costs increase (LLM
billing is time-sensitive for some providers). Understanding performance characteristics
helps size infrastructure and decide between connection-per-agent vs. shared-server
architectures.

**Expected output:** Latency measurements for different transports, a capacity planning
model (requests per second per server), and a recommended architecture for 10-20
concurrent agent sessions.

**Priority:** P1

**Status:** Open

---

### DRQ-10: IBC Relayer Agent Automation [Celestia-specific]

**Topic:** Feasibility of AI agent-operated IBC relayers in the Cosmos ecosystem

**Research Prompt:**

> I am exploring whether an AI agent can reliably operate an IBC (Inter-Blockchain
> Communication) relayer in the Cosmos ecosystem. The agent would monitor channel
> state, submit relay transactions, handle stuck packets, and respond to network
> conditions.
>
> Context: tiagent is a Celestia-native agent harness. One potential extension is
> having tiagent agents operate infrastructure like IBC relayers, where the agent
> decides when and how to relay based on learned patterns (gas prices, congestion,
> packet priority).
>
> I need to understand:
>
> 1. What are the operational requirements of running an IBC relayer (Hermes or
>    Go relayer)? What decisions does an operator need to make in real-time?
> 2. What are the common failure modes? Stuck packets, chain halts, gas estimation
>    failures, nonce issues, channel closures. How are these currently handled?
> 3. Could an AI agent add value over current automated relayer software? What
>    decisions currently require human judgment?
> 4. What monitoring and alerting is needed for relayer operation? What metrics
>    indicate problems?
> 5. Has anyone built or proposed agent-operated IBC relayers? What was the outcome?
> 6. What are the security implications? An agent with relayer keys has significant
>    power. How to limit blast radius?
> 7. What is the economic model for relaying? How do relayers currently cover their
>    costs? Could an agent optimize relaying for profit?

**Why it matters:** IBC relaying is a concrete, high-value infrastructure task that
could validate tiagent's capabilities beyond software development. If agents can
reliably operate relayers, it opens a category of infrastructure automation in the
Cosmos ecosystem. This query investigates feasibility before investing in
implementation.

**Expected output:** A feasibility assessment with specific go/no-go criteria, a list
of relayer operations suitable for agent automation vs. those requiring human judgment,
and risk analysis.

**Priority:** P2

**Status:** Open

---

### DRQ-11: WASM Tool Sandboxing Performance [General-purpose]

**Topic:** Performance characteristics of WASM sandboxes for agent tool execution

**Research Prompt:**

> I am designing a tool execution sandbox for an AI agent harness. Agents invoke tools
> (file operations, shell commands, HTTP requests, code analysis) that need to be
> sandboxed to prevent unintended side effects. I am evaluating WASM (WebAssembly)
> as the sandboxing mechanism.
>
> Tools are written in Rust and compiled to WASM. They receive JSON input, perform
> their operation, and return JSON output. Typical tool execution times range from
> 1ms (file read) to 30s (compilation). Tools may need limited I/O access (specific
> filesystem paths, specific network endpoints).
>
> I need to understand:
>
> 1. What is the performance overhead of running Rust code in a WASM sandbox vs.
>    native execution? Provide benchmarks for CPU-bound and I/O-bound workloads.
> 2. How does Wasmtime compare to Wasmer and wasm3 for this use case? Focus on:
>    startup time, steady-state performance, memory overhead, and Rust ecosystem
>    integration.
> 3. How do WASM sandboxes handle memory limits? Can I set per-tool memory caps?
>    What happens when a tool exceeds its memory limit?
> 4. How do WASM sandboxes handle CPU time limits? Can I enforce wall-clock
>    timeouts? Is preemption supported?
> 5. How is filesystem access controlled in WASM? Compare WASI preview 1 vs.
>    preview 2 capabilities. Can I grant access to specific directories only?
> 6. How is network access controlled? Can I restrict a tool to specific endpoints?
> 7. What is the overhead of proxying I/O through the WASM host? If a tool reads
>    a file, does the read go through the host's I/O layer, and what latency does
>    this add?
> 8. Can WASM modules be pre-compiled and cached to avoid repeated compilation
>    overhead?

**Why it matters:** Tool sandboxing is critical for safe agent operation, especially
when agents are self-improving and may generate novel tool invocations. WASM provides
strong isolation guarantees, but only if the performance overhead is acceptable. Tools
are invoked hundreds of times per agent session, so even small per-invocation overhead
compounds.

**Expected output:** Benchmark data for Wasmtime vs. native execution, a recommended
runtime with configuration, and a sandbox policy framework for different tool security
levels.

**Priority:** P2

**Status:** Open

---

### DRQ-12: TEE Attestation for Agent Traces [Celestia-specific]

**Topic:** Using Trusted Execution Environments to verify agent execution integrity

**Research Prompt:**

> I am exploring how TEE (Trusted Execution Environment) attestation can be used to
> verify that AI agent execution traces are authentic and untampered. The goal is to
> publish attestation reports alongside agent traces on Celestia, so consumers can
> verify that a trace was produced by a genuine agent execution rather than fabricated.
>
> Context: tiagent publishes agent execution traces to Celestia as blobs. Other agents
> retrieve these traces for learning. If traces can be fabricated, the learning commons
> is vulnerable to poisoning attacks (publishing fake "successful" traces that teach
> bad patterns).
>
> I need to understand:
>
> 1. What TEE platforms support Rust applications? Compare Intel SGX, Intel TDX,
>    AMD SEV-SNP, and ARM CCA. Which have mature Rust toolchains?
> 2. How does TEE attestation work at a technical level? What is included in an
>    attestation report? Can it attest to specific data outputs (i.e., "this trace
>    was produced inside this enclave")?
> 3. How can TEE attestation reports be verified on-chain or by Celestia consumers?
>    Is there a way to verify attestation without running a TEE?
> 4. What is the performance overhead of running an agent process inside a TEE?
>    CPU overhead, memory limits, I/O restrictions?
> 5. Are there existing projects that use TEEs for AI agent verification? Examples:
>    Phala Network, Marlin, Automata. What approaches do they take?
> 6. What are the limitations of TEE-based verification? Side-channel attacks,
>    compromised hardware, the "attestation gap" problem.
> 7. Is a hybrid approach feasible? Run the critical part of execution (tool calls
>    and their results) inside a TEE, while running the LLM inference outside?
> 8. How would TEE attestation interact with Celestia's data availability proofs?
>    Can the two proof systems be composed?

**Why it matters:** Trace authenticity is fundamental to the shared learning model. If
agents cannot trust that published traces are genuine, the learning commons devolves
into a garbage-in-garbage-out system. TEE attestation provides cryptographic guarantees
of execution integrity, but the feasibility and overhead for agent workloads is unclear.

**Expected output:** A feasibility matrix of TEE platforms for Rust agent workloads,
a recommended approach for trace attestation, and an honest assessment of what TEEs
can and cannot guarantee in this context.

**Priority:** P2

**Status:** Open

---

## Ecosystem Research [Mixed]

---

### DRQ-13: Celestia Ecosystem Agent Demand [Celestia-specific]

**Topic:** Developer needs and pain points that agent tooling could address in Celestia

**Research Prompt:**

> I am building tiagent, a self-improving AI agent harness for the Celestia ecosystem.
> Before building features, I want to understand what Celestia developers actually need
> from agent tooling.
>
> I need to understand:
>
> 1. What are the most common development workflows in the Celestia ecosystem?
>    Building rollups, deploying DA clients, operating nodes, writing Cosmos SDK
>    modules, IBC integration?
> 2. What are the biggest pain points for Celestia developers today? Tooling gaps,
>    documentation gaps, debugging difficulties, deployment complexity?
> 3. Are there existing AI/agent tools being used by Celestia developers? If so,
>    which ones, and what do they use them for?
> 4. What developer community channels exist (Discord, forums, GitHub Discussions)?
>    What topics come up repeatedly?
> 5. What would make Celestia developers adopt an agent tool? What are the
>    prerequisites for trust (open source, audited, self-hosted)?
> 6. Are there specific Celestia-native operations that are repetitive and could
>    benefit from automation (namespace management, blob submission, node
>    configuration)?
> 7. What is the size and composition of the Celestia developer community? How
>    many active developers, what languages do they use, what are their skill
>    levels?
> 8. What competitive agent tools exist in adjacent ecosystems (Ethereum, Solana)?
>    What features do they offer?

**Why it matters:** Building a tool nobody wants is the most common startup failure
mode. This research grounds tiagent's feature roadmap in actual developer needs rather
than assumptions. Understanding the ecosystem's pain points ensures that tiagent solves
real problems and has a viable adoption path.

**Expected output:** A prioritized list of developer pain points, specific use cases
that agents could address, and an adoption strategy based on community dynamics.

**Priority:** P0

**Status:** Open

---

### DRQ-14: Shared Learning Infrastructure [General-purpose]

**Topic:** Efficient multi-agent learning with delta-based updates and adversarial resistance

**Research Prompt:**

> I am designing a shared learning system where multiple AI agents publish their
> execution traces to a common data layer (Celestia) and learn from each other's
> experiences. Agents are operated by different parties who may not trust each other.
>
> The system needs to support:
> - Publishing learning deltas (not full model weights, but configuration updates:
>   routing weights, gate thresholds, prompt template rankings)
> - Efficient retrieval of relevant deltas by topic, recency, and quality
> - Resistance to adversarial participants who publish misleading data
>
> I need to understand:
>
> 1. What data structures support efficient delta-based learning updates? Compare
>    CRDTs (Conflict-free Replicated Data Types), Merkle DAGs, and append-only logs
>    for this use case.
> 2. How to handle adversarial participants in shared learning? What verification
>    mechanisms can detect poisoned data? Compare reputation systems, statistical
>    outlier detection, and TEE attestation.
> 3. How does federated learning compare to delta publishing for this use case?
>    Federated learning aggregates updates centrally; delta publishing lets each
>    consumer decide what to incorporate. Tradeoffs?
> 4. What is the optimal granularity for learning deltas? Per-task, per-session,
>    per-day? Finer granularity means more data but better specificity.
> 5. How should consumers filter and rank learning deltas? Should they weight by
>    recency, by the publishing agent's reputation, by task similarity?
> 6. Are there existing systems that implement shared learning across untrusted
>    agents? What approaches worked and what failed?
> 7. How much does shared learning actually help? Are there benchmarks showing
>    that agents learning from each other's traces outperform solo agents?

**Why it matters:** Shared learning is tiagent's network effect: each new agent that
publishes traces makes all other agents better. But the system must be robust against
poisoning (adversarial traces) and efficient enough that the overhead of publishing
and retrieving deltas does not outweigh the learning benefit.

**Expected output:** A recommended data structure for learning deltas, an adversarial
resistance strategy, and evidence (if it exists) that shared trace learning improves
agent performance.

**Priority:** P1

**Status:** Open

---

### DRQ-15: Cross-Chain Agent Identity [Celestia-specific]

**Topic:** Agent identity standards across Celestia and other chains

**Research Prompt:**

> I am designing an identity system for AI agents that operate across multiple
> blockchains. An agent may have a primary identity on Celestia (where its traces are
> published) but also interact with Ethereum (for smart contract deployment), Cosmos
> Hub (for IBC operations), and other chains.
>
> I need to understand:
>
> 1. How should agent identity be represented? Compare: raw public keys, DIDs
>    (Decentralized Identifiers, W3C standard), smart contract accounts (ERC-6551,
>    ERC-4337), and namespace-based identity on Celestia.
> 2. What is ERC-8004 and how does it relate to agent identity? Is it relevant
>    for cross-chain agent scenarios?
> 3. How can an agent prove it controls the same identity across different chains?
>    Cross-chain attestation mechanisms, signature aggregation, identity bridges.
> 4. What DID methods are suitable for blockchain-native agents? Compare did:key,
>    did:pkh, did:web, and Cosmos-specific DID methods.
> 5. How should agent identity relate to operator identity? Should the agent's
>    identity be derived from the operator's, independent, or hierarchical?
> 6. What are the privacy considerations? Should agent identities be pseudonymous
>    (unlinkable across chains) or transparent (fully linkable)?
> 7. Are there existing standards or proposals for AI agent identity in blockchain
>    ecosystems? What has the Autonomous Agents working group proposed?
> 8. How does identity interact with reputation? Can reputation from one chain
>    transfer to another?

**Why it matters:** Agent identity is foundational for reputation, access control, and
cross-chain operation. Without a coherent identity system, agents cannot build
reputation across chains, consumers cannot verify trace provenance, and the learning
commons cannot attribute quality to specific agents.

**Expected output:** A recommended identity architecture for Celestia-native agents
with cross-chain portability, comparison of DID methods, and a roadmap for
implementation.

**Priority:** P2

**Status:** Open

---

### DRQ-16: DA Layer Comparison for Agent State [Celestia-specific]

**Topic:** Comparing data availability layers for agent trace storage workloads

**Research Prompt:**

> I am evaluating data availability (DA) layers for storing AI agent execution traces.
> The workload characteristics are: frequent small writes (10KB-500KB traces, published
> after each task completion, roughly 50-200 traces per day), infrequent bulk reads
> (retrieving 10-100 past traces when starting a new task for context), and long-term
> archival needs (traces should be retrievable for months).
>
> Compare the following DA layers for this specific workload:
>
> 1. **Celestia**: Cost per blob at various sizes, throughput limits, namespace-based
>    retrieval, light node requirements, pruning window, ecosystem tooling (Rust
>    clients, SDKs).
> 2. **0G (ZeroGravity)**: Architecture, cost model, throughput, retrieval mechanisms,
>    maturity level, Rust support.
> 3. **EigenDA**: Architecture, cost model (restaking economics), throughput, retrieval,
>    integration requirements, Rust support.
> 4. **Avail**: Architecture, cost model, light client (avail-light), namespace
>    equivalent, Rust support, comparison with Celestia.
> 5. **NEAR DA**: Architecture, cost model, differences from purpose-built DA layers.
>
> For each, address:
> - Cost per MB of data published
> - Write latency (time from submission to confirmation)
> - Read latency (time to retrieve a blob by identifier)
> - Maximum blob size
> - Data retention period
> - Rust client library maturity
> - Ecosystem size and developer activity
> - Production readiness (mainnet vs. testnet)

**Why it matters:** Choosing the right DA layer is a foundational decision that affects
cost, performance, and ecosystem alignment. While tiagent is designed as Celestia-native,
understanding the competitive landscape ensures the choice is well-informed and
identifies potential multi-DA strategies.

**Expected output:** A comparison table with concrete numbers for each DA layer across
all dimensions, a clear recommendation with reasoning, and notes on multi-DA
feasibility.

**Priority:** P1

**Status:** Open

---

## Advanced Topics [General-purpose]

---

### DRQ-17: Hyperdimensional Computing for Agent Behavioral Fingerprinting [General-purpose]

**Topic:** Using HDC vectors to fingerprint and compare agent behaviors

**Research Prompt:**

> I am using Hyperdimensional Computing (HDC) to create behavioral fingerprints for
> AI agent execution traces. Each trace is encoded into a high-dimensional binary
> vector (10,000 dimensions) that captures the agent's behavioral pattern: which tools
> it used, in what order, how it handled errors, what code patterns it generated.
>
> The goal is to use these fingerprints for: (a) finding similar past traces for
> retrieval, (b) detecting anomalous agent behavior, (c) clustering agents by
> behavioral style, (d) deduplicating near-identical traces.
>
> I need to understand:
>
> 1. What dimensionality is optimal for behavioral fingerprinting? Published research
>    suggests 10,000 dimensions. Is this sufficient for distinguishing between
>    thousands of traces, or should it be higher?
> 2. How does Hamming distance between HDC vectors correlate with behavioral
>    similarity? Is the correlation monotonic? Are there distance thresholds that
>    meaningfully separate "similar" from "different"?
> 3. What encoding scheme is best for sequential data (tool call sequences)?
>    Compare: n-gram encoding, positional encoding, and temporal binding.
> 4. How effective is HDC for anomaly detection? Can a "normal behavior" prototype
>    vector reliably detect outlier traces?
> 5. What are the computational costs of HDC operations (encoding, similarity search)
>    compared to embedding-based approaches? How does HDC scale with corpus size?
> 6. Are there published applications of HDC in software engineering or AI agent
>    domains? What results were achieved?
> 7. Can HDC fingerprints be incrementally updated as a trace grows (during
>    execution), or must the full trace be re-encoded?
> 8. How do HDC fingerprints interact with locality-sensitive hashing (LSH) for
>    approximate nearest neighbor search?

**Why it matters:** HDC fingerprinting is already partially implemented in the codebase
(the roko-primitives crate computes HDC fingerprints per episode). This research would
validate the approach, optimize parameters, and identify new applications. If HDC
fingerprints reliably capture behavioral similarity, they become a cheap, fast
pre-filter before expensive semantic retrieval.

**Expected output:** Optimal dimensionality recommendation with justification, encoding
scheme comparison for tool-call sequences, and benchmark data comparing HDC similarity
search to embedding-based approaches.

**Priority:** P2

**Status:** Open

---

### DRQ-18: Sleep-Time Compute Implementation [General-purpose]

**Topic:** Background processing during agent idle periods for knowledge consolidation

**Research Prompt:**

> I am implementing "sleep-time compute" for an AI agent harness: using idle periods
> (between task assignments, overnight, during low-usage hours) to perform background
> processing that improves future agent performance.
>
> Candidate background tasks include:
> - Consolidating recent execution traces into reusable patterns ("dreaming")
> - Reindexing the code intelligence database
> - Retraining the cascade router's model selection weights
> - Pre-computing embeddings for recently modified files
> - Running speculative analysis on upcoming tasks in the plan queue
> - Garbage collecting stale knowledge entries
> - Compressing and archiving old traces
>
> The implementation is in Rust using tokio for async runtime.
>
> I need to understand:
>
> 1. What are the best patterns for implementing background task scheduling in
>    Rust with tokio? Compare: dedicated background thread pool, tokio::spawn with
>    priority, separate tokio runtime for background work.
> 2. How to ensure background work does not impact active agent performance? CPU
>    pinning, nice levels, I/O priority, memory budgets?
> 3. How should background tasks be prioritized? Which consolidation tasks provide
>    the most value per compute hour?
> 4. What does the research say about "dreaming" in AI systems (offline
>    consolidation of experience into generalized knowledge)? Are there published
>    systems that do this effectively?
> 5. How to schedule consolidation in a way that respects system load? Should
>    background work pause when the system is under load, or run on a fixed schedule?
> 6. What checkpointing strategy should background tasks use? If a consolidation
>    run is interrupted, can it resume, or must it restart?
> 7. Are there Rust crates for background job scheduling that handle prioritization,
>    rate limiting, and checkpointing?
> 8. How does sleep-time compute interact with system power management? On a
>    laptop, should background work respect battery state?

**Why it matters:** Agent performance improves with consolidated knowledge, but
consolidation is expensive and should not compete with active task execution. Sleep-time
compute lets the system "process its experiences" during downtime, similar to how
biological sleep consolidates memories. Effective implementation can significantly
improve agent performance without impacting active task latency.

**Expected output:** A Rust implementation architecture for background task scheduling,
a priority ordering for consolidation tasks based on value-per-compute, and a load-aware
scheduling strategy.

**Priority:** P2

**Status:** Open

---

### DRQ-19: VCG Auctions for Context Allocation [General-purpose]

**Topic:** Using auction mechanisms to allocate scarce LLM context window space

**Research Prompt:**

> I have a system where multiple "context sources" compete for space in an LLM's
> context window. The context window is finite (e.g., 200K tokens), and each source
> wants to include its information:
>
> - Task description and requirements (always included, not auctioned)
> - Retrieved past trajectories (variable size, 5K-50K tokens each)
> - Code context from the codebase (variable, depends on files touched)
> - Research notes and documentation (variable)
> - System prompt layers (role template, safety guidelines, tool descriptions)
> - Knowledge store entries (distilled knowledge from past sessions)
>
> I am considering using a VCG (Vickrey-Clarke-Groves) auction mechanism where each
> context source bids for space based on its estimated value-add to the task. A basic
> version is implemented but currently a simpler greedy allocation dominates at runtime.
>
> I need to understand:
>
> 1. Is VCG theoretically appropriate for context allocation? VCG guarantees
>    truthful bidding in multi-item auctions, but does this apply when "bidders"
>    are internal components rather than strategic agents?
> 2. How should bids be determined? What proxy metric captures "value of including
>    this context for task success"? Options: historical correlation with outcome,
>    embedding similarity to task, recency, source reliability.
> 3. Is VCG computationally feasible for per-request auctions? VCG requires solving
>    the allocation problem multiple times. With 6-10 bidders and a single divisible
>    resource (tokens), is this fast enough (<1ms)?
> 4. What alternatives to VCG exist for this type of allocation? Compare: greedy
>    by value-per-token, proportional allocation, attention-based weighting, and
>    learned allocation policies.
> 5. How do you handle the cold-start problem? When a new context source has no
>    historical data, how should it bid?
> 6. Does context ordering matter for LLMs? Should high-value context go at the
>    beginning, end, or be interleaved? How does this interact with allocation?
> 7. Are there published systems that use auction mechanisms for prompt
>    construction? What approaches have been tried?

**Why it matters:** Context window space is the scarcest resource in LLM-based agent
systems. Including irrelevant context wastes tokens and can confuse the model. Excluding
relevant context causes the agent to miss critical information. An optimal allocation
mechanism can significantly improve agent performance by ensuring the most valuable
context is always included.

**Expected output:** A recommendation on whether VCG is appropriate or if a simpler
mechanism suffices, a bid computation strategy for each context source type, and
computational feasibility analysis.

**Priority:** P2

**Status:** Open

---

### DRQ-20: Agent Economics and Incentive Design [Celestia-specific]

**Topic:** Token economics and mechanism design for agent trace ecosystems

**Research Prompt:**

> I am designing the economic layer for a shared agent learning ecosystem. Agents
> publish execution traces to Celestia, and other agents consume those traces to
> improve their performance. I need an incentive system that:
>
> - Rewards agents for publishing high-quality traces (traces that actually help
>   other agents succeed)
> - Penalizes or filters low-quality or adversarial traces
> - Funds the DA costs (Celestia blob submission) for trace publication
> - Prevents Sybil attacks (one operator pretending to be many agents to farm
>   rewards)
> - Aligns incentives so the system gets better over time rather than devolving
>   into spam
>
> I need to understand:
>
> 1. How should "trace quality" be defined and measured? Options: downstream agent
>    success rate when using the trace, human review scores, automated quality
>    metrics (code correctness, gate pass rates).
> 2. What token economic models suit a data marketplace where the "product" is
>    execution traces? Compare: pay-per-retrieval, subscription, stake-and-earn,
>    and retroactive funding.
> 3. How to prevent Sybil attacks in a credit system? An operator could publish
>    traces from fake agents and have other fake agents "use" them to generate
>    quality scores. Compare: proof-of-work, proof-of-stake, TEE attestation,
>    and social verification.
> 4. What mechanism design principles apply? How to ensure incentive compatibility
>    (agents are better off publishing honestly than gaming the system)?
> 5. How should DA costs be distributed? Should publishers pay (they benefit from
>    reputation), consumers pay (they benefit from learning), or should a protocol
>    fund subsidize DA costs?
> 6. Are there existing data marketplaces or knowledge-sharing protocols with
>    token economics that work? What can we learn from Ocean Protocol, Filecoin's
>    retrieval market, or The Graph?
> 7. What is the minimum viable economic layer? What can be deferred to later
>    phases without compromising the system's ability to grow?
> 8. How do agent economics interact with the underlying chain economics
>    (Celestia's TIA token, gas costs)?

**Why it matters:** Without proper incentive alignment, the shared learning commons
either fails to attract publishers (no reward for sharing) or gets flooded with spam
(no penalty for low quality). The economic layer is what transforms tiagent from a
single-user tool into a network with compounding value. Getting incentives right is
critical for long-term sustainability.

**Expected output:** A minimum viable token economic model, a Sybil resistance
strategy, a quality measurement framework, and a phased rollout plan that starts
simple and adds complexity as the network grows.

**Priority:** P1

**Status:** Open

---

## General-Purpose Coding Agent Research [General-purpose]

---

### DRQ-21: Coding Agent Evaluation and Benchmarking [General-purpose]

**Topic:** How developers evaluate coding agents, what metrics matter, and how to benchmark tiagent against Claude Code, Codex, and Cursor

**Research Prompt:**

> I am building a self-improving coding agent (tiagent) that competes with Claude Code,
> Codex CLI, Cursor, and Aider. I need to understand how developers evaluate coding
> agents and how to benchmark tiagent against the incumbents.
>
> I need to understand:
>
> 1. What metrics do developers use to evaluate coding agents? How important is each
>    of: task success rate (did the code work on first attempt), code quality (style,
>    correctness, performance), time savings (vs. manual coding), cost per task,
>    latency (time to first output), context utilization (how well the agent
>    understands the codebase)?
> 2. What existing benchmarks exist for coding agents? Compare: SWE-bench, HumanEval,
>    MBPP, Aider's polyglot benchmark, and any newer benchmarks from 2025-2026. What
>    do they measure and what do they miss?
> 3. How should a self-improving agent be benchmarked differently from a static one?
>    Standard benchmarks measure single-task performance. How do you measure improvement
>    rate, learning efficiency, and transfer learning across tasks?
> 4. What is the developer experience gap between coding agents? How do developers
>    compare Claude Code vs. Cursor vs. Codex in terms of ease of use, reliability,
>    and integration into existing workflows?
> 5. How do enterprise teams evaluate coding agents? What criteria matter for team
>    adoption: security, reproducibility, audit trails, model flexibility, cost
>    predictability?
> 6. Are there published case studies of coding agent adoption at scale? What metrics
>    did they track and what results did they see?
> 7. How should improvement over baseline be measured? If tiagent starts at parity with
>    Claude Code but improves with use, what is the right way to demonstrate and
>    quantify that advantage?

**Why it matters:** tiagent's primary value proposition against incumbent coding agents
is self-improvement: it gets better with use while others stay static. To prove this,
we need rigorous benchmarks that capture not just single-task performance (where Claude
Code currently leads) but improvement rate over time, cost efficiency via learned model
routing, and the compounding effect of quality gates and playbook extraction.

**Expected output:** A benchmarking framework for tiagent with specific metrics, a
comparison methodology against Claude Code/Codex/Cursor, and recommendations for
demonstrating the self-improvement advantage quantitatively.

**Priority:** P0

**Status:** Open

---

### DRQ-22: Self-Improvement Strategies for Coding Agents [General-purpose]

**Topic:** Most effective self-improvement strategies for coding agents, including Dynamic Cheatsheets, RHO harness optimization, and playbook extraction

**Research Prompt:**

> I am building a coding agent that improves its own performance over time by learning
> from past task executions. The agent does NOT modify its own source code, but it does
> modify: prompt templates (Dynamic Cheatsheets), model routing weights (cascade
> router), quality gate thresholds, and extracted playbooks (reusable patterns from
> successful task executions).
>
> I need to understand:
>
> 1. What are the most effective self-improvement strategies for coding agents? Compare:
>    a. Dynamic Cheatsheets: automatically updating instruction files based on what
>       worked (analogous to a self-writing CLAUDE.md or .cursorrules).
>    b. RHO (Reinforced Harness Optimization): using task outcomes as reward signals to
>       optimize the harness configuration (prompt structure, tool ordering, context
>       selection).
>    c. Playbook extraction: distilling successful multi-step task executions into
>       reusable playbooks that guide future similar tasks.
>    d. Trajectory-conditioned prompting: retrieving relevant past trajectories and
>       including them as few-shot examples.
> 2. What learning rate and decay parameters work best for configuration-level
>    self-improvement? If the agent updates its routing weights after every task, how
>    quickly should it adapt to new evidence vs. retaining old knowledge?
> 3. How do you prevent self-improvement from degrading performance? Known failure
>    modes: threshold gaming (lowering quality gates to increase "success rate"),
>    overfitting to recent tasks, catastrophic forgetting of earlier patterns.
> 4. What does the academic literature say about meta-learning for tool-using agents?
>    Are there published papers on agents that optimize their own tool usage patterns?
> 5. How should improvements be validated before being applied? Should there be an
>    A/B testing framework for prompt changes? How many tasks constitute a statistically
>    significant comparison?
> 6. What is the expected improvement curve? How many task executions does it take
>    before self-improvement produces measurable gains? Is the curve logarithmic,
>    linear, or S-shaped?
> 7. Are there open-source implementations of self-improving coding agents or harnesses
>    that tiagent can learn from or compare against?

**Why it matters:** Self-improvement is tiagent's core differentiator against every other
coding agent. Getting the learning strategy right determines whether tiagent actually
gets measurably better with use or just claims to. The wrong learning strategy can make
the agent worse (threshold gaming, overfitting) or provide negligible improvement
(learning rate too low, wrong features).

**Expected output:** A ranked list of self-improvement strategies by expected impact,
concrete learning rate recommendations, a validation framework for improvement changes,
and references to relevant academic work.

**Priority:** P0

**Status:** Open

---

### DRQ-23: Developer Pain Points with Current Coding Agents [General-purpose]

**Topic:** What developers dislike about Claude Code, Codex, and Cursor, and what features they wish these tools had

**Research Prompt:**

> I am building a coding agent (tiagent) that aims to address the gaps left by Claude
> Code, Codex CLI, Cursor, Windsurf, and Aider. Before building features, I want to
> deeply understand what developers are frustrated by and what they wish these tools
> could do.
>
> I need to understand:
>
> 1. What are the most common complaints about Claude Code? Search Reddit (r/ClaudeAI,
>    r/programming), Twitter/X, Hacker News, and GitHub issues for recurring themes.
>    Common categories: context window limits, hallucination, cost, inability to learn
>    from mistakes, poor multi-file editing, breaking existing code.
> 2. What are the most common complaints about Cursor? Search r/cursor, Cursor's
>    Discord/forum, and Hacker News. Common categories: subscription pricing, model
>    quality variance, agent mode reliability, difficulty with large projects.
> 3. What are the most common complaints about Codex CLI? Given its newer release,
>    look for early adoption feedback, benchmark comparisons, and developer experience
>    reports.
> 4. What features do developers wish coding agents had? Look for feature request
>    threads, wishlist posts, and "if only it could..." discussions. Categories to
>    check: learning from mistakes, multi-task execution, quality validation, cost
>    control, team sharing, CI/CD integration.
> 5. What is the churn rate for coding agent users? How many developers try a coding
>    agent and stop using it, and why? What brings them back vs. makes them leave?
> 6. How do developers feel about model lock-in? Is being limited to one model
>    provider (Claude for Claude Code, GPT for Codex) a significant pain point?
> 7. What do developers say about cost? Is the dominant concern the absolute cost, the
>    unpredictability of cost, or the lack of optimization (paying premium model prices
>    for simple tasks)?
> 8. Are there developer surveys or reports on coding agent satisfaction? What does
>    the quantitative data show about satisfaction levels and usage patterns?

**Why it matters:** tiagent must solve real problems that developers experience with
existing tools. Building features based on assumptions rather than observed pain points
risks creating a technically impressive tool that nobody actually wants to switch to.
This research identifies the highest-impact improvements that would drive developer
adoption.

**Expected output:** A prioritized list of developer pain points (with frequency data
where available), specific feature gaps that tiagent can fill, and evidence for which
problems drive the most switching behavior.

**Priority:** P0

**Status:** Open

---

### DRQ-24: Model-Agnostic Cascade Router Design for Coding Tasks [General-purpose]

**Topic:** How to design a cascade router that selects the optimal LLM for each coding task based on learned features

**Research Prompt:**

> I am building a cascade router for a coding agent that dynamically selects which LLM
> to use for each development task. The router observes task features and selects from
> a pool of models spanning cost/quality tiers: budget models (Claude Haiku, GPT-4o-mini,
> Gemini Flash), mid-tier models (Claude Sonnet, GPT-4o, Gemini Pro), and premium models
> (Claude Opus, o3, Gemini Ultra). The router should learn from task outcomes to improve
> its routing decisions over time.
>
> I need to understand:
>
> 1. What features best predict which model will succeed on a coding task? Candidates:
>    task type (new code, bug fix, refactoring, test writing, documentation), programming
>    language, file count, estimated diff size, codebase complexity, presence of similar
>    past tasks in history, error message complexity (for debugging tasks).
> 2. How should the routing algorithm work? Compare:
>    a. Thompson Sampling: maintain per-model Beta distributions, sample, route to
>       highest sample. Simple, well-understood, but no contextual features.
>    b. Contextual bandits (LinUCB, NeuralUCB): use task features to predict per-model
>       reward. Better routing but more complex.
>    c. Lightweight classifier: train a small model (logistic regression, decision tree)
>       on historical (features, model, outcome) tuples. Fast inference, interpretable.
>    d. Hierarchical routing: first pick the tier, then pick within tier. Reduces the
>       action space.
> 3. What is the right reward signal for routing? Options: binary success/failure, gate
>    pass rate (0-7 rungs passed), code quality score, cost-adjusted quality, latency-
>    adjusted quality. How to handle delayed rewards (code works now but breaks later)?
> 4. How fast should the router adapt? If a new model is added to the pool, how many
>    tasks should it route there for exploration? What exploration budget is reasonable
>    (e.g., 10% of tasks)?
> 5. How to handle non-stationarity? Model capabilities change with version updates.
>    How to detect when a model's performance has shifted and re-explore?
> 6. What cold-start strategy works best? When the router has no history (new
>    installation), should it default to the premium model, the cheapest model, or
>    round-robin?
> 7. Are there published systems that implement model routing for coding tasks? What
>    approaches were tried and what results were achieved?
> 8. How much cost savings can a well-designed router achieve? Published estimates and
>    theoretical analysis for the cost reduction from routing simple tasks to cheap
>    models.

**Why it matters:** Model costs vary by 10-50x across tiers, and coding tasks vary
enormously in complexity. A bug fix that adds a missing import does not need Claude
Opus; a complex architectural refactoring does. The cascade router is what makes
tiagent cost-competitive while maintaining quality. No existing coding agent has this
capability -- they all route every task to the same model (or let the user manually
choose). Getting the routing algorithm right is the difference between a 60% cost
reduction and a marginal improvement.

**Expected output:** A recommended routing algorithm with implementation details,
feature engineering for coding tasks, cold-start and exploration strategies, and
expected cost savings with worked examples.

**Priority:** P1

**Status:** Open

---

## Summary

| ID | Topic | Scope | Priority | Status |
|---|---|---|---|---|
| DRQ-01 | Celestia Blob Size Optimization | Celestia | P0 | Open |
| DRQ-02 | Celestia Light Node Resource Requirements | Celestia | P1 | Open |
| DRQ-03 | Celestia Namespace Collision Avoidance | Celestia | P1 | Open |
| DRQ-04 | Celestia Data Persistence Beyond Pruning | Celestia | P1 | Open |
| DRQ-05 | Trajectory RAG Retrieval Strategies | General | P0 | Open |
| DRQ-06 | Cascade Router Optimization | General | P1 | Open |
| DRQ-07 | Gate Pipeline Calibration | General | P2 | Open |
| DRQ-08 | Self-Improving Harness Safety | General | P1 | Open |
| DRQ-09 | MCP Server Performance at Scale | General | P1 | Open |
| DRQ-10 | IBC Relayer Agent Automation | Celestia | P2 | Open |
| DRQ-11 | WASM Tool Sandboxing Performance | General | P2 | Open |
| DRQ-12 | TEE Attestation for Agent Traces | Celestia | P2 | Open |
| DRQ-13 | Celestia Ecosystem Agent Demand | Celestia | P0 | Open |
| DRQ-14 | Shared Learning Infrastructure | General | P1 | Open |
| DRQ-15 | Cross-Chain Agent Identity | Celestia | P2 | Open |
| DRQ-16 | DA Layer Comparison for Agent State | Celestia | P1 | Open |
| DRQ-17 | HDC for Agent Behavioral Fingerprinting | General | P2 | Open |
| DRQ-18 | Sleep-Time Compute Implementation | General | P2 | Open |
| DRQ-19 | VCG Auctions for Context Allocation | General | P2 | Open |
| DRQ-20 | Agent Economics and Incentive Design | Celestia | P1 | Open |
| DRQ-21 | Coding Agent Evaluation and Benchmarking | General | P0 | Open |
| DRQ-22 | Self-Improvement Strategies for Coding Agents | General | P0 | Open |
| DRQ-23 | Developer Pain Points with Current Coding Agents | General | P0 | Open |
| DRQ-24 | Model-Agnostic Cascade Router Design | General | P1 | Open |

**P0 (6):** DRQ-01, DRQ-05, DRQ-13, DRQ-21, DRQ-22, DRQ-23 -- These queries address
the most fundamental unknowns: DA cost optimization, the core learning mechanism,
product-market fit, competitive benchmarking, self-improvement validation, and
developer pain points.

**P1 (9):** DRQ-02, DRQ-03, DRQ-04, DRQ-06, DRQ-08, DRQ-09, DRQ-14, DRQ-16, DRQ-20,
DRQ-24 -- These queries fill important architectural, integration, and routing
knowledge gaps.

**P2 (9):** DRQ-07, DRQ-10, DRQ-11, DRQ-12, DRQ-15, DRQ-17, DRQ-18, DRQ-19 -- These
queries address advanced features and optimizations that can wait until the core system
is validated.

**By scope:** General-purpose (14): DRQ-05, DRQ-06, DRQ-07, DRQ-08, DRQ-09, DRQ-11,
DRQ-14, DRQ-17, DRQ-18, DRQ-19, DRQ-21, DRQ-22, DRQ-23, DRQ-24. Celestia-specific
(10): DRQ-01, DRQ-02, DRQ-03, DRQ-04, DRQ-10, DRQ-12, DRQ-13, DRQ-15, DRQ-16,
DRQ-20.
