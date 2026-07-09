# Batches

This is the recommended execution order.

## Completion rule

The batch set is complete when all required batches are done and the final
acceptance criteria in `AR07` pass.

## Required batches

| Batch | Title | Depends on | Main output |
|---|---|---|---|
| AR01 | Contracts and mirage fork bootstrap | — | target ERC-8004 surface on mirage, legacy registry no longer expanded |
| AR02 | Relay binary | — | `apps/agent-relay/` with directory + messaging surfaces |
| AR03 | `roko agent serve` | — | CLI entrypoint that starts `roko-agent-server` |
| AR04 | Agent relay client + chain registration | AR01, AR02, AR03 | agent can appear through relay and/or 8004 |
| AR05 | Mirage runtime, proxy, Docker, Railway shape | AR02 | default same-origin mirage + relay runtime |
| AR06 | In-repo mirage demo UI and quickstart | AR01, AR02, AR03, AR04, AR05 | static demo reads 8004 + relay and messages agents |
| AR07 | Remote demo verification and operator docs | AR04, AR05, AR06 | end-to-end remote demo works with remote and local agents |

## Optional batch

| Batch | Title | Depends on | Main output |
|---|---|---|---|
| AR08 | Kauri dashboard migration pack | AR01, AR02, AR04, AR05 | external dashboard reads 8004 + relay and drops endpoint cache |

## Final acceptance target

The minimum final proof is:

1. A remote Railway deployment runs `mirage-rs + agent-relay`.
2. One remote deployed agent is reachable.
3. One local laptop agent connects to the same remote relay.
4. The in-repo mirage demo UI, pointed at the remote mirage URL, lists both.
5. The in-repo mirage demo UI can send a test message to both and receive a
   response.
6. No production discovery path depends on mirage's Rust `AgentRegistry`
   endpoint metadata.

## Parallelization guidance

Safe early parallelism:

- `AR01`, `AR02`, `AR03`

Second wave:

- `AR04`, `AR05`

Third wave:

- `AR06`

Final wave:

- `AR07`
- `AR08` optional and can proceed in parallel with `AR07`
