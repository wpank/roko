# Agent Dispatch Benchmarks

## CLI Dispatch (claude --print)
- Cold start: ~1.2s
- Warm start: ~0.4s
- With MCP: +0.3s per server

## API Dispatch (Messages API)
- First token: ~0.8s
- Streaming: ~50ms TTFT after connection
- Tool use round-trip: ~1.5s

## Conclusions
- Use CLI for plan execution (tasks > 30s)
- Use API for interactive chat and quick queries
- Pool API connections for sidecar
