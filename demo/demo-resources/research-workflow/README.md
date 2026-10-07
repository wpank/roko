# Research Workflow Demo

Demonstrate research dispatch, research-backed plans, and the research panel.

## CLI commands

```bash
# Direct topic research (writes to .roko/research/)
roko research topic "MEV landscape on Ethereum L2s"

# Deep research (async, 1-10 min, more thorough)
roko research topic "MEV landscape on Ethereum L2s" --deep

# Direct search (fast, structured results)
roko research search "libp2p relay implementation patterns"

# Write a plan that draws on the research reports
roko plan generate "Wire knowledge into matchmaking" --context .roko/research

# Optimize the plan with research (takes the plan's directory name under plans/)
roko research enhance-plan wire-knowledge-into-matchmaking

# Optimize its tasks: split large ones, add file context, cut needless dependencies
roko research enhance-tasks wire-knowledge-into-matchmaking

# Analyze execution episodes for insights
roko research analyze
```

## Dashboard flow

1. Open the dashboard chat
2. Type `/research MEV landscape on Ethereum L2s`
3. Watch the **Research bounty** tab — stages progress: dispatching → gathering → analyzing → synthesizing → complete
4. Results show sources with relevance scores, findings with confidence levels, gaps, and follow-ups

## HTTP API

```bash
# Dispatch research
curl -X POST http://localhost:6677/api/research/topic \
  -H 'Content-Type: application/json' \
  -d '{"topic":"MEV landscape on Ethereum L2s","depth":"deep"}'

# List research artifacts
curl http://localhost:6677/api/research

# Optimize a plan
curl -X POST http://localhost:6677/api/research/enhance-plan/wire-knowledge-into-matchmaking

# Analyze episodes
curl -X POST http://localhost:6677/api/research/analyze
```

## What gets created

```
.roko/research/
├── mev-landscape-on-ethereum-l2s.md    # Research output
└── ...
```

## Dashboard tabs that light up

- **Research bounty** — Live research session with stages
- **Plans** — The enhanced plan's updated tasks
- **Network → Learning** — Research efficiency recorded
