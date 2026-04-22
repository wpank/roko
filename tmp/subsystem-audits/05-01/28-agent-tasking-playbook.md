# 28 - Agent Tasking Playbook

Purpose: make the 05-01 redesign work executable by fresh agents with no prior
context. This document defines the packet format, prompt template, status vocabulary,
global non-goals, anti-patterns, and proof rules. Use the packet docs `29`-`33` as
the actual work backlog.

## How To Use These Packets

Give one packet to one agent. Do not give a low-tier agent a whole wave, a broad
redesign goal, or ownership of multiple subsystems. The packet should be executable
with only:

- the packet text;
- the listed context files;
- the listed write scope;
- the listed verification commands.

If the agent discovers that the correct fix belongs outside the write scope, the
correct result is `PartialBlocked`, not a local workaround.

## No Prior Context Contract

Every packet must stand on its own. A fresh agent should not need old chat history,
runner logs, or unstated architecture knowledge to make a correct attempt.

A packet is ready to assign only when it answers:

- Which crate/module owns the fix?
- Which files may be edited?
- What is the before/after invariant?
- Which shortcuts are forbidden?
- Which command or static check proves the invariant?

The agent may inspect nearby code and tests to implement the packet, but it must not
use that inspection to expand the write scope. If the implementation requires a
design choice not stated in the packet, the agent should stop at `PartialBlocked`
and explain the missing decision.

## Mechanical Change Standard

Low-tier agents should receive syntax-level or adapter-level changes, not open-ended
architecture tasks. Good mechanical work includes:

- add a type with constructors/tests;
- rename one local type or function;
- move one parser to a shared representation;
- replace one duplicate map with a shared registry;
- add one golden test or one static guard script;
- block one fallback path with a typed error.

Do not assign low-tier agents work that asks them to invent a migration strategy,
choose among competing owners, rewrite a god function, or make broad behavior
changes across several crates.

## Standard Agent Prompt

```text
You are working in /Users/will/dev/nunchi/roko/roko.

You are not alone in the codebase. Do not revert edits made by others. Do not edit
outside the write scope below.

Task packet: <PACKET ID AND TITLE>

Read these context files first:
<CONTEXT FILES>

Write scope:
<FILES OR MODULES>

Mechanical steps:
<NUMBERED STEPS>

Do not:
<NON-GOALS>

Anti-patterns to avoid:
<ANTI-PATTERNS>

Verification:
<COMMANDS AND EXPECTED RESULTS>

Final response must include:
- status: Changed | NoopAlreadySatisfied | PartialBlocked | Failed
- files changed
- verification run and result
- any old path deleted, blocked, or still reachable
```

## Status Vocabulary

Use these exact statuses in agent final responses and runner reports.

| Status | Meaning |
|---|---|
| `Changed` | The requested mechanical change was made and verified. |
| `NoopAlreadySatisfied` | The repo already satisfied the packet and verification proves it. |
| `PartialBlocked` | The packet cannot be completed safely inside the write scope. The agent must explain the missing owner or dependency. |
| `Failed` | The agent attempted the packet but could not make it work. |

Do not use `Done`, `Resolved`, or `Wired` unless the packet explicitly asks for a
docs status update and product-path proof exists.

## Global Non-Goals

These are not tasks for low-tier agents unless a packet explicitly says otherwise:

- Do not redesign provider dispatch while editing ACP or chat surface code.
- Do not add a new provider HTTP client, SSE parser, or auth env lookup in a surface crate.
- Do not change broad config semantics without a migration/validation packet.
- Do not make production services optional to make tests pass.
- Do not encode new states in strings, booleans, empty strings, or sentinel values.
- Do not remove large legacy modules unless the packet includes a deletion proof.
- Do not update old status docs to `Resolved` or `Wired` without product-path proof.
- Do not expand scope because the local patch is easier than the shared-owner fix.

## Global Anti-Patterns

Each agent should check its diff against these before final response.

| Anti-pattern | What to avoid |
|---|---|
| Surface-local shared logic | Adding provider/gate/prompt/config/runtime logic to ACP, chat, serve, or demo instead of the owner crate. |
| Fake success | Returning success for no-op, unsupported, skipped, or failed states. |
| Unknown-to-zero | Turning missing usage, cost, duration, context, or ids into `0`, `0.0`, or `""`. |
| Optional production services | Letting dispatch run without feedback, safety, budget, config, or event persistence when production mode requires them. |
| String contracts | Branching on terminal text, debug strings, rendered JSON, or status display text. |
| Shadow path | Adding a new path while leaving an old bypass silently reachable. |
| Grep-only proof | Claiming correctness because a grep passes, without a unit/integration test when behavior changed. |

## Verification Rules

Every packet needs at least one of these proof types:

- compile proof: `cargo check -p <crate>`;
- unit proof: specific `cargo test -p <crate> <test_name>`;
- static proof: `rg` command expected to return no production violations;
- golden proof: fixture/snapshot proves external wire format;
- product-path proof: a command or mocked live entry point emits the typed event/result.

If a command is not run, the final response must say why.

## Review Checklist

Before accepting an agent result:

- The agent stayed inside write scope.
- The diff changes the owner or a thin adapter, not a new duplicate owner.
- The packet status is one of the allowed statuses.
- Verification commands were run or a concrete blocker is given.
- New types are used instead of strings/booleans/sentinels.
- Old paths are deleted, blocked, or explicitly reported as still reachable.
- No new placeholder comments were added as substitutes for the requested change.
- Docs status claims use coverage vocabulary, not vague completion language.
- No broad unrelated refactors or formatting churn were introduced.

## Packet Granularity Rules

Good packets:

- one crate or one narrow cross-crate type move;
- one behavior change;
- 1-5 files usually, 8 maximum unless mechanical renaming;
- clear before/after invariant;
- test or static check included.

Bad packets:

- "unify provider dispatch";
- "fix learning";
- "make config safe";
- "clean up workflow";
- "remove legacy runtime";
- "wire everything end to end".

Those are waves, not low-tier agent tasks.
