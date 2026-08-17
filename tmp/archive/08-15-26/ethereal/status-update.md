# Nunchi Technical Status Update

## Our Agent Runtime

- Agent runtime, harness, and orchestration layer production ready, agents orchestrate across many domains across software engineering, financial data aggregation, on-chain operations, research, knowledge management, and a growing number of additonal use cases / workflows.
- The platform is **provider-agnostic**: agents can use 8+ LLM providers (Anthropic, Google Gemini, Perplexity, OpenRouter, Ollama, Cerebras, OpenAI-compatible) with automatic failover, many more trivially addable, next target is dynamic use across all HuggingFace models
  - A learned routing system selects which model to use for each task based on historical performance, cost, and latency
  - If a provider goes down, a circuit breaker trips and work reroutes automatically
- **Every agent output is verified before acceptance** through a configurable multi-stage validation pipeline (up to 11 stages). Gates are domain-specific: for code workflows they run compilation, tests, and linting; for on-chain workflows they run transaction simulation and wallet checks; for research they run fact checking and LLM review. New gate types plug in without changing the pipeline
  - Thresholds are adaptive: the system calibrates its own quality bar from historical pass rates
  - When verification fails, the system retries with failure context injected, not blind retries
- Agents **accumulate durable knowledge across sessions** via a persistent knowledge store with semantic similarity search (10,240-bit hyperdimensional vectors, sub-millisecond retrieval)
  - Knowledge decays over time unless actively used, preventing stale information from polluting decisions
  - Validated insights are shared across agents, so the 100th run benefits from everything learned in the first 99
- **Full crash recovery**: execution state snapshots to disk continuously. Any interrupted run resumes from the last checkpoint
- **~85+ HTTP API endpoints** for dashboards, monitoring, and programmatic control, plus WebSocket and SSE for real-time streaming
- Agents are **general-purpose and user-configurable**. The same runtime that orchestrates code generation also orchestrates research workflows, data aggregation, on-chain operations, and multi-step reasoning tasks. Agent roles, tools, verification gates, and model routing are all defined in config, not hardcoded
- **58+ tools** available to agents, spanning file I/O, code search, shell execution, web research, GitHub, Slack, on-chain transactions, and custom scripts. Next steps are collecting lots of numbers on tool use performance / effectiveness benchmarking, and working towards optimziations.
- **Code intelligence** is one built-in tool domain: semantic indexing across Rust, TypeScript, and Go with call graph analysis, dependency tracking, and importance scoring. Other tool domains (chain operations, data feeds, external APIs) plug in the same way
- **Generalized feed system** for continuous data ingestion from any external source (blockchains, exchanges, APIs, databases, webhooks). Feed agents subscribe to data streams and publish to a shared relay, so any agent in the network can consume live data without polling. Feeds compose into derived and composite feeds via transformation pipelines
- **Relay mechanism** for cross-agent communication and discovery. Agents register on startup, publish capabilities, and discover peers through the relay without centralized coordination. Supports topic-based pub/sub, backpressure with intelligent coalescing, and reconnection with sequence-based replay for gap recovery
- **Demo App Built to Showcase curated workflows**  with real-time data from the API: cost dashboardown, multi-agent management, knowledge graph, cascade router state, ISFR feeds, performance benchmarking with live gate verdicts, in-browser terminal, code builder with live compilation gates, knowledge explorer, and interactive demo scenarios. All live data from multiple running production agents + running blockchain. Continually adding real examples and workflows. 



## Our Agents Interact with many blockchains including ours

- Agents have **multiple categories of on-chain tools** implemented: balance queries, token transfers, ERC-20 approvals, gas estimation, transaction simulation, DEX swaps, liquidity management, pool queries, wallet creation, and knowledge posting. Further work to be done adding more, and continue to flesh out end to end testing.
- **ISFR (Integrated Smart Feed Routing) PoC running**: a collective price/rate discovery mechanism where more than a dozen production running feed agents pull from 13 data sources (Aave, Compound, Ethena, Lido, and others), aggregate via weighted median with outlier exclusion, and publish rates on a 6-phase clearing cycle. Initial implementation to test things working end to end, further work to be done assessing performance, accuracy, resilance, etc. 
- **Agent job marketplace**: full lifecycle from posting through assignment, execution, and settlement with escrow, dispute resolution. End to end workflows implemented, further work to be done refining UX, use cases, domains, etc.
- **Agnent native micropayments**: x402 + MPP pay-per-request protocol with gasless transfers and state channel support for high-frequency interactions
- **Indirect agent coordination**: agents communicate by leaving lightweight signals on a shared bus that other agents can read, rather than sending direct messages. An agent finishing a task leaves a marker describing what it learned or changed; other agents pick that up and adjust their behavior without anyone routing messages between them. Currently implementing: wiring signal emission into the agent lifecycle so agents automatically broadcast context on task completion
- **What agents store on-chain vs in validator state**: Three-tier model (fully specced, 10 design documents, 100+ academic citations). On-chain state holds only lightweight anchors (~71 bytes per entry). Full vectors go into the event log. The search index lives in validator memory as an HNSW graph, rebuilt deterministically from events on startup. Reads are free, writes cost ~83K gas. The index self-prunes via half-life decay: unused entries gradually age out with no background cleanup. Knowledge lives in validator state, not on-chain state, keeping the chain lean while giving agents sub-millisecond similarity search. On-chain state also holds agent identity (ERC-8004 passports), reputation scores, marketplace escrow, ISFR clearing results, and attestation hashes

