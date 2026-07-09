# Agent Stack Summary — Quick Reference

For agents working on batch `02`.

## Core Split

`roko-agent` owns provider adapters, translators, tool-loop execution, safety, MCP support, pools, and several advanced agent primitives.

`roko-core` owns shared config and type surfaces that other crates can depend on without pulling in the full agent runtime.

`roko-cli` owns the live entrypoints:

- `run.rs` is the cleaner single-run path,
- `orchestrate.rs` is the large plan-execution path,
- `main.rs` still contains a few specialty entrypoints.

## Main Batch-02 Question

**Which agent abstractions already exist and work, but are either duplicated or bypassed by the main runtime path?**

## What Is Already Real

- 19 `Agent` implementations,
- 6 provider adapters,
- 5 translators,
- full `ToolLoop`,
- full `ToolDispatcher` + `SafetyLayer`,
- MCP discovery / dedup / handler / bridge pipeline,
- `AgentPool` and `MultiAgentPool`,
- `CompositeAgent`, `MorphableAgent`, `MetacognitiveMonitor`, warrants,
- `CascadeRouter`, `LinUCBRouter`, Pareto pruning, anomaly detection.

## What Exists But Is Not Cleanly Owned Or Fully Live

- `ChatResponse` / `ResponseMetadata` duplication,
- shared response concepts still owned by `roko-agent`,
- `orchestrate.rs` bypassing the stronger dispatcher/tool-loop path,
- typed temperament config,
- remaining research creation-site safety bypasses,
- monitor/tool-loop activation on the plan-execution path.

## Runtime Reality To Keep In Mind

- `run.rs` already proves that scoped agent creation and dispatcher usage can work.
- `orchestrate.rs` is the main conflict hotspot.
- not every path needs to be unified in one batch; one real production path is enough to prove the contract.
