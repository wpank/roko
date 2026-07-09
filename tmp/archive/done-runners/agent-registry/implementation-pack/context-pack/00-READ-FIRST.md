# 00 — Read First

You are a fresh implementation agent. Assume zero prior chat context.

## Non-negotiable goals

1. Do not solve discovery by adding endpoint data to mirage's Rust
   `AgentRegistry` as the long-term answer.
2. Treat ERC-8004 as the durable identity source.
3. Treat `agent-relay` as the live reachability source.
4. Treat wallet-free agents as first-class production agents.
5. The final proof is the **in-repo mirage demo UI** working against a
   **remote** mirage deployment.

## Fresh-agent workflow

Before editing:

1. Read this file.
2. Read `01-TARGET-STATE.md`.
3. Read `02-CODE-MAP.md`.
4. Read `03-VERIFICATION-MATRIX.md`.
5. Read the specific prompt assigned to you.
6. Read every code file that prompt references.

## Subagent policy

You are explicitly authorized to use multiple subagents for your batch.

When spawning subagents:

- give each worker a **disjoint write scope**
- tell each worker they are **not alone in the codebase**
- pass the same context-pack files to every subagent
- do not block idly if you can make progress locally
- if subagents are unavailable, continue locally without failing

Recommended split:

- explorer: specific codebase questions
- worker A/B/C: bounded code changes in disjoint file sets
- main agent: integration, verification, final report

## Verification standard

Do not stop at code changes. Every batch must finish with:

- targeted build/test commands
- acceptance criteria checked explicitly
- any remaining risk or follow-up called out clearly
