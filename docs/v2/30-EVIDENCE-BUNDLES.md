# 30 — Run Evidence Bundles

> **Status**: IMPLEMENTED AND STRICT LOOPBACK SMOKE VERIFIED AS A DEVELOPMENT HARNESS
>
> **Scope**: Private, bounded evidence for FAST/self-hosting runs and arbitrary local commands.
> It does not make endpoint, browser, or screenshot probes mandatory unless the operator requests
> that policy explicitly.

The format described here is schema version 2. Validators read each artifact's `schema_version`
and fail on malformed content; version 1 minimum bundles remain historical evidence rather than
being silently promoted to this complete contract.

## Commands

Capture any command while keeping output live:

```bash
./dev.sh run-evidence --deadline 30 -- git status --short
```

Add behavior evidence without a shell wrapper:

```bash
./dev.sh run-evidence \
  --endpoint-base http://127.0.0.1:6677 \
  --cli-smoke 'status=target/debug/roko status --json' \
  --text-snapshot 'dashboard=target/debug/roko dashboard --text' \
  --png-hook 'web=tools/capture-page --url http://127.0.0.1:5173 --output {output}' \
  -- ./target/debug/roko plan run plans/example --no-tui --log-file {bundle}/events.jsonl
```

When the verification contract names an exact endpoint set, suppress the collector's standard
safe GET seed list and collect only the repeated explicit paths:

```bash
./dev.sh run-evidence \
  --endpoint-base http://127.0.0.1:6677 \
  --no-default-endpoints \
  --endpoint /health \
  --endpoint /ready \
  --require-endpoints-pass \
  -- ./target/debug/roko status --json
```

Without `--no-default-endpoints`, the standard safe GET paths remain enabled alongside explicit
`--endpoint` values. The option changes endpoint selection only; GET-only, loopback, redirect,
response-size, timeout, and validation bounds still apply.

Hooks use `NAME=COMMAND` syntax and are split as argv, not evaluated by a shell. Available
placeholders are `{bundle}`, `{run_id}`, and, for PNG hooks, `{output}`. The hook executable is an
explicit operator trust decision. Each hook has a deadline and bounded stdout/stderr.

Inspect or validate existing runs:

```bash
./dev.sh feedback --run-id <run-id>
./dev.sh feedback --run-id <run-id> --json
./dev.sh evidence-validate .roko/runs/<run-id>
./dev.sh score --bundle-root .roko/runs
```

`feedback` renders the deterministic `DEBRIEF.md`. `score` retains failures and timeouts and emits
p50/p95 latency rather than dropping them as outliers. `evidence-validate` exits nonzero for an
invalid bundle.

## Collection contract

The bundle is created before command dispatch. Its evidence run ID is exported as
`ROKO_EVIDENCE_RUN_ID`; Roko's internal runner ID is discovered separately from structured events
and fresh status samples.

During execution the collector:

- writes separate bounded stdout and stderr while preserving live output;
- samples only status-file revisions newer than the pre-run snapshot and locks onto the first
  observed runner ID;
- inventories only processes in the wrapped command's process group;
- enforces the deadline on the complete process group, including descendants left after leader
  exit;
- takes byte offsets for known append-only logs before launch.

Before a FAST launch it also records disk, swap/memory, and Cargo target allocation evidence. FAST
fails closed below either 5 GiB or 3% free disk. `--min-free-gib` and `--min-free-percent` tune the
floors, while `--allow-low-disk` (or `ROKO_EVIDENCE_ALLOW_LOW_DISK=1`) is an explicit override
recorded in `resource-admission.json`. Target sizing has a two-second cap and cannot stall launch.

After execution it:

- slices only bytes appended during the command and retains only JSON objects containing the
  evidence or observed runner ID;
- captures bounded Git state/diff and untracked path names;
- optionally discovers OpenAPI GET operations and queries safe paths;
- optionally executes CLI, text-snapshot, and browser/PNG hooks;
- optionally imports exactly one new Roko screenshot directory, failing closed when concurrent
  directories make ownership ambiguous;
- calculates event, status, provider, gate, Git, process, endpoint, screenshot, and latency metrics;
- creates `score.json`, `DEBRIEF.md`, and `validation.json` deterministically.

## Bundle layout

Every completed capture contains at least:

```text
manifest.json                 identity, command, Git/host/cache and selected policies
command.txt                   redacted shell-display form of argv
stdout.log / stderr.log       live command output, captured separately
events.jsonl                  optional runner-owned structured event stream
events-validation.json        lifecycle and run-ID analysis
status.jsonl                  evidence-wrapper start and exactly one terminal
status-samples.jsonl          fresh run-scoped runner status samples
commands.jsonl                primary command timing, PID/PGID, exit and capture bounds
processes.json[l]             aggregate and sampled process-group inventory
resource-admission.json       pre-launch disk/swap/memory/target facts and decision
filtered-logs/                newly appended records matching this run only
endpoints.json                GET-only request results and response artifacts
cli-smoke.json                explicit CLI hook results
screenshots/manifest.json     text/PNG evidence and dimensions/hashes
metrics.json / score.json     measured facts and target results
gates.json / usage.jsonl      compact verification/provider summaries
diff.patch / diff-stat.json   bounded tracked change evidence
summary.json                  terminal truth and artifact index
validation.json               strict validator result
DEBRIEF.md                    deterministic factual handoff
```

Optional services are recorded as skipped, not successful. An unresolved OpenAPI path parameter is
never guessed. The collector uses GET only, disables redirects, sends no authorization header, and
restricts endpoint bases to loopback unless `--allow-remote-endpoints` is explicit.

## Strict policies

The base validator checks:

- required files, JSON/JSONL parseability, private modes, no symlink artifacts, and byte bounds;
- one wrapper terminal and consistent timeout/signal/exit semantics;
- when events exist, exactly one run start, one terminal, one runner ID, and ordered timestamps;
- run-ID ownership for status and screenshot records;
- GET-only endpoint evidence and valid PNG headers/dimensions;
- common private-key, provider-token, bearer-token, and named-secret value patterns;
- total portable bundle size.

FAST passes `--require-events`. Other acceptance evidence can be promoted from optional to required
with `--require-status-sample`, `--require-cli-smoke-pass`, `--require-endpoints-pass`, and
`--require-screenshots`. If a wrapped command succeeds but its selected evidence policy fails, the
wrapper exits `125` and reports `evidence_invalid`; a command failure or timeout retains its own
exit semantics.

## Security and bounds

Bundle directories use mode `0700` and files use `0600`. The full environment and credential
values are never added to metadata. Endpoint JSON fields with secret-like names and hook argv are
redacted. Artifact validation rejects common leaked-secret shapes.

Stdout, stderr, Git diffs, and source logs can still contain sensitive data before validation. Do
not publish a bundle merely because it is local, and do not bypass a validator failure without
inspecting the named artifact.

The principal limits are 16 MiB per stdout/stderr or tracked diff, 8 MiB per filtered source log,
4 MiB for a direct runner JSONL artifact, 2 MiB per hook stream, 1 MiB per endpoint response, 8 MiB
per screenshot, 32 endpoint requests, and 128 MiB for the bundle as a whole. A direct runner event
log that crosses its limit terminates the process group and is truncated at its last complete JSONL
record so final validation stays deterministic.

## Known boundaries

- Metrics normalize the current runner event schema. Prompt tokens are estimates unless the runner
  emits provider usage, and the harness does not invent missing internal command spans.
- Browser automation is intentionally adapter-based; install and select the browser tool that fits
  the changed UI rather than adding one heavyweight mandatory dependency.
- The score command aggregates arbitrary existing bundles. The separate fixed-SHA orchestrator in
  [`benchmarks/dev-audit`](../../benchmarks/dev-audit/README.md) creates representative cold/warm
  matrices on linked worktrees and reuses this collector; neither command claims promotion evidence
  until the matrix, escaped regressions, and full-CI baseline have been reviewed.
- `./dev.sh benchmark history` builds deterministic JSON/Markdown series from a bounded newest
  suffix of those sessions. It compares the newest session with the previous session or an explicit
  reviewed baseline, returns nonzero for configured latency/correctness regressions, and exposes
  missing or undersampled data as inconclusive. Root entries, sessions, rows, groups, bytes, and
  scan time are capped; exceeding a global scan bound fails closed instead of emitting a biased
  partial dashboard.
- The collector does not authorize code scope or mutate an endpoint. Plan/task policy remains the
  source of allowed paths and verification intent.

## Integrated smoke checkpoint

The final integration fixture returned `200` from loopback health, readiness, status, run detail,
events, tasks, gates, and metrics. Event pagination advanced through opaque cursor positions `0`,
`40`, and `123`, and bounded run-filtered SSE replay completed.

The evidence wrapper then used `--no-default-endpoints` to make those eight selected paths the
complete endpoint contract. All eight requests and the explicit CLI smoke passed; strict
validation found no errors, warnings, or secret hits; and `feedback` plus `score` both reported
green. This proves the collector and seeded loopback fixture, not a representative provider run,
browser screenshot, paid benchmark matrix, or full-CI release lane.
