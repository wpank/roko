# Concern Index

Use this index when assigning Codex agents by concern rather than by source document. Source-specific files remain authoritative because they embed full source context and explicit extracted details.

| Concern | Plan | Tasks | Primary Gate |
|---------|------|-------|--------------|
| `api` | [concern-api.md](concern-api.md) | 1996 | `roko parity gates routes --strict` |
| `realtime` | [concern-realtime.md](concern-realtime.md) | 1996 | `roko parity gates realtime --strict` |
| `storage` | [concern-storage.md](concern-storage.md) | 1996 | `roko parity check --strict --include-links` |
| `auth` | [concern-auth.md](concern-auth.md) | 550 | `cargo test -p roko-serve auth` |
| `chain` | [concern-chain.md](concern-chain.md) | 1996 | `roko parity gates chain --strict` |
| `agent-runtime` | [concern-agent-runtime.md](concern-agent-runtime.md) | 1039 | `roko parity gates lifecycle --strict` |
| `dashboard-support` | [concern-dashboard-support.md](concern-dashboard-support.md) | 1996 | `roko parity gates surfaces --strict` |
| `verification` | [concern-verification.md](concern-verification.md) | 684 | `roko parity gates static --strict` |
| `config-deployment` | [concern-config-deployment.md](concern-config-deployment.md) | 1807 | `roko parity gates surfaces --strict` |
| `knowledge-learning` | [concern-knowledge-learning.md](concern-knowledge-learning.md) | 244 | `roko parity gates cognitive --strict` |
