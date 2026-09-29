---
plan: demo-full-stack
estimated_tasks: 6
estimated_parallel_width: 2
estimated_minutes: 18
parallel_safe: true
tags: [demo, multi-role, rust, web-server]
---

# Full-stack multi-role demo

This plan showcases Roko's full agent pipeline by building a real, runnable
Rust web server. It is designed to be compelling in a live demo: every task
has a distinct responsibility, every output is a concrete file that exists on disk
after the run, and the final sign-off task verifies the whole deliverable
end-to-end.

## What gets built

A standalone Rust package at `demo/hello-server/` backed by axum + tokio
that exposes three HTTP endpoints:

| Method | Path      | Response                                       |
|--------|-----------|------------------------------------------------|
| GET    | `/`       | `Hello from Roko!` (plain text)                |
| GET    | `/health` | `{"status":"ok","version":"0.1.0"}` (JSON)    |
| GET    | `/info`   | `{"built_by":"roko","plan":"demo-full-stack"}` |

Run it after the plan completes:

```bash
cd demo/hello-server
cargo run
# Server listens on http://localhost:3333
curl http://localhost:3333/
curl http://localhost:3333/health
curl http://localhost:3333/info
```

## Roles demonstrated

| Task | Role        | Responsibility                                      |
|------|-------------|-----------------------------------------------------|
| T01  | scribe      | Evaluate frameworks; recommend axum with rationale  |
| T02  | implementer | Scaffold Cargo.toml + axum skeleton on port 3333   |
| T03  | implementer | Add /health and /info JSON endpoints                |
| T04  | scribe      | Security audit: no panics, no secrets, report PASS |
| T05  | scribe      | Write README with API docs and architecture section |
| T06  | implementer | Final review, compile check, sign-off report        |

## Dependency DAG

```
T01 (scribe)
 └─▶ T02 (implementer)
      └─▶ T03 (implementer)
           ├─▶ T04 (scribe) ──┐
           └─▶ T05 (scribe) ──┴─▶ T06 (implementer)
```

T04 and T05 run in parallel (max_parallel = 2) after T03 is accepted.
T06 is the single convergence point that reads every upstream artifact.

## Artifacts produced

```
demo/hello-server/
├── Cargo.toml          # axum + tokio + serde dependencies
├── src/
│   └── main.rs         # three-route axum server, no .unwrap()
├── research.md         # framework comparison and axum recommendation
├── AUDIT.md            # security audit report, verdict: PASS
├── README.md           # user docs, API table, architecture section
└── COMPLETION.md       # final sign-off: "demo-full-stack plan: COMPLETE"
```

## Why this is a good demo

- **Writing roles only** — every task produces a file, so each uses a role whose
  safety contract permits file writes (`implementer` or `scribe`; PLAN_036).
- **Real compilation gate** — T02, T03, and T06 all run `cargo build`.
- **Cross-task traceability** — T04 reads T03's output; T06 reads T04 and T05.
- **Parallel wave** — T04 and T05 run concurrently, demonstrating max_parallel = 2.
- **Deterministic acceptance** — every `[[task.verify]]` block is an executable
  shell command, not a human judgement.
- **End-to-end narrative** — a research brief chooses the framework, implementers
  build it, an audit certifies it, a README documents it, and a final sign-off
  verifies it.
