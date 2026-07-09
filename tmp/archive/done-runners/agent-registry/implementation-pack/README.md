# Agent Registry Implementation Pack

This directory is an execution pack for implementing the agent discovery and
relay design without relying on prior chat context.

## Goal

The end state of this batch is:

- `mirage-rs` is not the durable registry of agent identity
- agent identity comes from ERC-8004
- live reachability comes from `agent-relay`
- `roko agent serve` exists and can participate in relay and/or chain paths
- the in-repo mirage demo UI can run against a **remote** mirage deployment and
  successfully discover and message both remote and local agents

## Important architectural stance

The target system should not use mirage's in-process Rust `AgentRegistry` as
the production source of truth for identity. `mirage-rs` should behave as a
chain host:

- fork Ethereum mainnet by default for the demo path
- surface ERC-8004 for identity
- co-deploy relay by default for easy local and Railway verification

If ERC-8004 is not present on the upstream fork, the target implementation is
to deploy the target contracts into the forked/local chain state at boot rather
than storing agent endpoints in mirage's Rust structs.

## Files

- [BATCHES.md](./BATCHES.md) — task order, dependencies, and done definition
- `context-pack/` — shared context every fresh agent should read first
- `prompts/` — self-contained implementation prompts
- `runner/` — batch runner that executes the prompts in a separate worktree

## How to use

1. Pick the next unlocked batch from [BATCHES.md](./BATCHES.md).
2. Give the assigned prompt to a fresh agent with no prior thread context.
3. Require the agent to read the referenced context-pack files and code files.
4. Require the agent to satisfy the prompt's acceptance criteria before
   claiming completion.

## Batch runner

Use:

```bash
bash tmp/agent-registry/implementation-pack/runner/run-agent-registry.sh
```

Default runner behavior:

- creates a separate git worktree
- uses `codex exec` with `gpt-5.4` in Codex fast mode
- commits after each successful batch
- retries failed batches
- cleans temporary Rust build artifacts after each successful commit while
  keeping the worktree itself

## Notes on subagents

The prompts explicitly authorize the assignee to use multiple subagents.
Expected pattern:

- explorers for bounded codebase discovery
- workers for disjoint write scopes
- the main agent integrates and runs verification

If the environment does not support subagents, the prompts should still be
usable by a single agent acting locally.
