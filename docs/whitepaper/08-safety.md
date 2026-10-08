Status: draft · budget 500 words · owner spec-ce1484

# 8 Safety

This section describes the bounds Roko places on the actions agents take and on the changes it makes to itself.

## 8.1 Fail closed

Roko's safety rests on one default: an action that no rule permits does not run. Only listed tools run, a role with no
contract gets no tools, and an unknown safety contract denies, so an omission is a refusal with a reason, not a silent
grant: Saltzer and Schroeder's fail-safe default [@saltzer1975protection]. A prompt cannot enforce this, since text an
agent reads can override its instructions [@greshake2023not]; every bound below is enforced in code.

## 8.2 The tool-call guard and agent CLIs

For API models, Roko runs the tool loop itself, and every tool call passes the dispatcher's seven steps
(`crates/roko-agent/src/dispatcher/mod.rs`): validate, authorize, apply the safety policy, execute under a timeout,
bound and scrub the result, screen it, and write one sanitized audit record. The safety step runs ten ordered checks:
sandbox level, tool permission policy, rate limit, capability warrant, command policy, network, path, risk budget,
temporal properties and the role contract (`crates/roko-agent/src/safety/mod.rs`). The first failure stops the call.
Each agent runs under one of nine role contracts, from architect to strategist; a role's allowlist and its contract's
intersect, and a denial always wins.

Agent CLIs such as Claude Code run their own tool loops, so Roko bounds them from outside. Claude Code runs under an
isolation profile that ignores the user's own settings and admits only the MCP servers Roko passes, with the git guard
as a hook (§8.3). Turn caps, the attempt timeout and a stall watchdog bound every CLI attempt.

## 8.3 The git guard and clean environments

Roko integrates work itself (§4), so an agent never needs to discard work, move branches or publish. One git guard
denies `git stash` (except listing and showing), `git clean` (except dry runs), checkout, switch, restore and every
push, in every segment of a chained command. It runs in three places: Roko's tool loop, the ACP server's shell tool,
and a hook in Claude Code sessions that blocks the call if the guard itself cannot run.

Child processes would inherit Roko's provider keys, so gate commands and the shell commands agents run through Roko's
tools start from an empty environment plus an allowlist (`crates/roko-core/src/child_env.rs`). Provider CLIs lose
every other provider's keys, and commands that name Roko's key files are refused.

## 8.4 Isolation controls

Each agent's output crosses the runtime safety boundary, where a fixed screening graph scores it and withholds
suspicious output. A high or critical finding writes an isolation control on the agent's id, and later calls from that
id are refused before they reach a provider. Each attempt in a plan run has its own id, so a control covers one
attempt. It lasts seven days by default; an operator can release it early with `roko safety release`, which records
who and why.[^8-isolation]

## 8.5 Bounds on self-modification

Regulators tune spending and checking, never what counts as correct: no slower loop widens permissions, removes an
authored check or raises a budget ceiling. Every self-modification passes guarded commit, which rolls back a change
that does not help on held-out work (§5).

[^8-isolation]: Design default (`1f860d408`): `DEFAULT_ISOLATION_TTL` in `crates/roko-agent/src/immune_evidence.rs`;
    the release command in `crates/roko-cli/src/commands/safety.rs`.
