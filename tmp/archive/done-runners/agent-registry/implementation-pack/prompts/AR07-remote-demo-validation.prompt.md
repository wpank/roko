# Batch AR07: Remote demo verification and operator docs

You are a fresh coding agent. Zero prior chat context. Read these first:

- `tmp/agent-registry/implementation-pack/context-pack/00-READ-FIRST.md`
- `tmp/agent-registry/implementation-pack/context-pack/01-TARGET-STATE.md`
- `tmp/agent-registry/implementation-pack/context-pack/03-VERIFICATION-MATRIX.md`
- `tmp/agent-registry/04-deployment-and-dev.md`

Also inspect:

- `docker/mirage.Dockerfile`
- `docker/roko.Dockerfile`
- `docker/docker-compose.yml`
- `railway.toml`
- `apps/mirage-rs/static/quickstart.sh`
- `apps/mirage-rs/static/js/state.js`
- `tmp/agent-registry/implementation-pack/context-pack/02-CODE-MAP.md`

## Task

Produce the final repo-local implementation and operator docs required to prove
the remote mixed-topology demo.

This batch is successful only if the repo contains a concrete, reproducible
path for:

1. deploying `mirage-rs + agent-relay` to Railway
2. deploying one remote agent container
3. starting one local laptop agent against the remote relay
4. running the in-repo mirage demo UI against the remote mirage URL
5. using the demo UI to discover and message both agents

Expected implementation shape:

- the runbook is an operator document, not architecture prose
- every remote step names the concrete file, env var, command, and expected URL
- the helper script validates or prints the same path the human operator will
  follow

Minimum runbook sections expected:

- prerequisites
- remote mirage + relay deploy
- remote agent deploy
- local laptop agent startup against remote relay
- demo UI startup against remote mirage
- validation checklist
- failure recovery / relay restart recovery

## Deliverables

1. Repo-local operator docs at
   `tmp/agent-registry/remote-demo-runbook.md` for the remote mixed-topology
   demo.
2. Any small scripts/config helpers needed to make the remote demo reproducible.
3. Final acceptance checklist inside the runbook.
4. If useful, example env files or command templates under `tmp/agent-registry/`
   that keep secrets out of git but make the required variable set obvious.

## Suggested subagent split

- explorer: inspect current Railway/Docker/runtime assumptions and list the
  exact operator gaps
- worker A: remote mirage + relay deployment instructions/assets
- worker B: remote/local agent operator instructions/assets
- worker C: final demo UI remote-target instructions and acceptance checklist

## Write scope

- repo-local docs under `tmp/agent-registry/` or another clearly-scoped local
  operator-doc path
- small helper scripts under `scripts/` or `tmp/agent-registry/` if needed

## Constraints

1. This batch is about reproducible proof, not more architecture prose.
2. The final proof surface is the in-repo mirage demo UI.
3. Be explicit about environment variables, commands, and URLs.
4. Prefer runnable helpers over “the operator should remember to do X”.

## Acceptance criteria

- repo contains `tmp/agent-registry/remote-demo-runbook.md`
- that runbook contains a step-by-step remote mixed-topology demo procedure
- procedure includes one remote deployed agent and one local laptop agent
- procedure ends with the in-repo mirage demo UI targeting the remote mirage
  URL and messaging both agents
- procedure includes recovery / reconnect validation after relay restart
- repo includes `tmp/agent-registry/scripts/remote-demo-check.sh`
- the helper script can at least validate local prerequisites and print the
  exact remote-demo command sequence in `--dry-run` mode

## Verification

At minimum:

```bash
test -f tmp/agent-registry/remote-demo-runbook.md
test -f tmp/agent-registry/scripts/remote-demo-check.sh
bash -n tmp/agent-registry/scripts/remote-demo-check.sh
bash tmp/agent-registry/scripts/remote-demo-check.sh --dry-run
```
