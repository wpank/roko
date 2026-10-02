# ViabilityBench

A cybernetic benchmark for agent harnesses. The system under test is a harness plus a model: every arm (Roko, a
bash-only direct loop, Claude Code, Codex) gets the same generated tasks, and success counts only when tests the
agent never saw pass on a clean re-run of its final commit (the VS-census label). Every cost is recomputable from
one dated price snapshot.

The design is spec S08 (`tmp/cybernetic-harness/specs/S08-benchmark-suite.md` in the author's workspace; the
`§` references below point into it). Suite id `vb`; schemas `vb.*/1`.

**What is here.** The schemas and the price snapshot; the common library and the two families the pilot stream
names, F1 (`f1_pyconv`) and F4 (`f4_kvtool`); the plan-level slice's fixtures (`families/plan_slice/`, S09 §4.9);
`speclint/` (S07.1); verifier CI (`ci/`); the driver, with three runners (the direct loop, the Roko arm and the
Claude Code arm), the run ledger with its budget-line caps, the metering and fault proxy and the driver-only secret
file; and the analysis behind `vb report` (`analysis/`). This README describes what exists. What is still open, and
what has run, is tracked in `work/items/` under epic `spec-567e52`.

## Rules

- **Source only.** This tree holds generators, verifiers, the driver and analysis code. Raw results live outside
  the repo in `$VB_RESULTS` (default `~/.roko-bench/viability`), and task workdirs in `$VB_WORK` (default
  `~/vb-work`). Each pilot's small summary bundle (records, metrics, ledger and report page; no transcripts or
  archives) is committed under `reports/`.
- **Stdlib-only Python 3.11 or newer** for everything the benchmark runs: families, verifiers, driver and
  analysis. pytest is a dev-only dependency, pinned in `requirements.lock`. The one exception (decision 3336): the
  secondary analyses under `analysis/models/` (the GLMM, 2PL IRT and ICC) use numpy and scipy, pinned in
  `requirements-analysis.lock`. Nothing else imports them, so every primary analysis stays stdlib-only.
- **One price source.** Costs come from `config/prices/<date>.toml`, never from `roko.toml`'s per-model rates
  (its gpt-oss-120b rates are wrong) and never from a fallback rate.
- **No provider calls in tests.** Tests run offline against fixtures and stub servers.

## Layout

```
benchmarks/viabilitybench/
  schema/{task, feature, run-record, metric-record, ledger, price-snapshot, experiment}.schema.json  validate.py
  schema/examples/
  families/common/{repo, knobs, hmac_seed, astcheck, mutate, canary}.py
  families/f1_pyconv/{gen, hidden, gaming}.py  ladder.toml  template/  spec/  reference/{solution, stub, gaming}/
  families/f4_kvtool/{gen, hidden, gaming, instance}.py  ladder.toml  template/  spec/  reference/
  families/plan_slice/{slicekit, runner}.py  features/{pl01_stockroom … pl06_csvclean}/   # PL fixtures (S09 §4.9)
  speclint/{speclint, dynamic}.py  fixtures/
  streams/{pilot, pilot_fd_api}.toml
  arms/{cheap_direct, fd_api, fd_claude, roko_fixed}.toml
  experiments/budget.toml                                   # budget lines and caps (S09 §4.6)
  experiments/{pilot_a, pilot_b}.toml  test_*.py            # experiment manifests (vb campaign) and their rehearsals
  experiments/provider_fault.toml                           # Pilot B's provider_fault rows (vb.disturbance/1)
  driver/vb.py                                              # vb run | estimate | materialize | campaign | ledger | …
  driver/campaign.py                                        # vb campaign: an experiment's blocks, validated and run
  driver/{mini_loop, run_roko, planemit, run_cli}.py        # the runners: direct loop, Roko arm, Claude Code arm
  driver/{ledger, faultproxy, secret}.py                    # the run ledger, the metering and fault proxy, the secret
  driver/egress.py                                          # the Claude Code arm's egress allowlist proxy
  driver/{disturb, vb_verify}.py                            # H6's disturbances, and the visible-verify wrapper
  driver/{materialize, harness, provider, stub_provider, agent_env, caps, archive, census, records, layout}.py
  analysis/{metrics, passk, report}.py                      # vb report
  analysis/gates.py                                         # gate pages: G0's go/no-go (go-no-go.md, g0.json)
  analysis/{bootstrap, cs, mcnemar, cuped}.py               # S09 §4.1's toolkit: bootstrap, sequences, McNemar, CUPED
  analysis/{envelope, holm}.py                              # H1's envelope (E*) and graphical Holm over the primaries
  analysis/simulate.py                                      # synthetic campaigns: coverage, FWER, anytime coverage
  analysis/models/{glmm, irt}.py                            # the secondaries on numpy and scipy (decision 3336)
  ci/{verify_verifiers, determinism, leak_check}.py         # verifier CI
$VB_RESULTS (default ~/.roko-bench/viability)/<experiment_id>/<run_id>/
  manifest.json  order-<seed>.json  records.jsonl  ledger.jsonl  reservations.jsonl  errors.jsonl  metrics.json
  proxy.jsonl  egress.jsonl  s01/  archives/  private/  transcripts/ (opt-in)
```

Tests sit beside the code they test (`test_*.py`), plus `tests/test_plan_slice.py`, and `driver/testdata/` holds a
toy family. The prices live in `config/prices/2026-09-28.toml` (§5.6). S08 §5.1 plans more than this tree holds: the
families F2, F3 and F5–F8, `external/swebench/`, the other streams and arms, and `analysis/replay.py`.

## The driver

`driver/vb.py` runs a stream of tasks on one arm and one model (S08 §4.9–4.10, §5.7), through the arm's runner:

- the direct loop (`mini_loop.py`, harness `mini-loop`) serves `cheap_direct` and `fd_api`: one model, one bash
  tool, no Roko prompt;
- the Roko arm (`run_roko.py`, harness `roko`) serves `roko_fixed`: a one-task plan (`planemit.py`) through
  `roko plan run` on one pinned model, checked on every attempt. `planemit.py`'s ladder mode emits the cheap-model
  ladder instead (decision 3302), for the routed Roko arms;
- the Claude Code arm (`run_cli.py`, harness `claude-code`) serves `fd_claude`: `claude -p` with an isolated config,
  on the subscription.

Each runner's module docstring has its isolation, caps and costs. The bullets below describe the direct loop, and
most hold for every arm.

```bash
PY=benchmarks/viabilitybench/.venv/bin/python
$PY benchmarks/viabilitybench/driver/vb.py estimate --stream pilot --arm cheap_direct --model gpt-oss-120b --seeds 1-3
$PY benchmarks/viabilitybench/driver/vb.py run --experiment PILOT-A --stream pilot --arm cheap_direct \
    --model gpt-oss-120b --seeds 1-3 --allow-network --max-cost-usd 10     # spends money: Pilot A only
```

- **Admission.** A non-loopback provider is called only with both `--allow-network` and `--max-cost-usd` (the rule of
  `scripts/dev_benchmark.py`). The budget is enforced per task: a task starts only while the ledger's spend plus the
  most one task can cost under the arm's caps still fits. A model missing from the price snapshot is refused.
  `--provider-url` with a loopback URL runs offline, and the tests use it with `driver/stub_provider.py`.
- **Caps** (`caps.py`, from the arm's `[caps]`). Per attempt: 12 turns and 150K input tokens, after which a fresh
  attempt starts. Per task: 30 turns, 300K input tokens and 20 minutes. The runaway detector stops a task at 30 model
  calls, 5 identical commands in a row, or its dollar cap. Caps are checked before each call, so none is exceeded; a
  capped task ends `aborted_cap` (or `timeout`) with VS = 0.
- **Isolation.** Each (task, seed) gets a fresh workdir under `$VB_WORK` (default `~/vb-work/<run_id>/`). The task
  manifest, which holds the canary, and the pristine bundle live in the run's `private/` directory, never in a
  workdir. Agent processes get an allowlisted environment (`agent_env.py`): no `VB_*` variables, no provider keys, a
  per-task HOME. The truth suites and the census's visible re-run run the agent's code through
  `families/common/sandbox.py`. On macOS, `sandbox-exec` denies that code the secret file and the task's private
  directory. Elsewhere there is no confinement yet, and the run record says so (`vs.sandbox`, gap-8c3752). Agents can
  read the driver's own start-up environment (`ps -E`, `/proc`), so once its checks pass,
  `vb run` starts itself again with an allowlisted environment (`agent_env.exec_scrubbed`). Every other process of
  your user stays readable (`ps -E -ax`), so run the benchmark from a session that exports no credential.
- **Rust toolchain** (F7, gap-46fd19; Will's decision of 2026-10-02). A per-task HOME hides `~/.rustup`, so
  `families/common/toolchain.py` resolves the host's toolchain in the operator's environment (`rustup show home`,
  `rustup which cargo`). Agent processes, the census and `ci/verify_verifiers.py` get the toolchain's own bin
  directory on PATH (never `~/.cargo/bin`, which holds whatever `cargo install` put there), the real `RUSTUP_HOME`,
  and a `CARGO_HOME` under their own HOME, so cargo's registry cache is per run. Every sandbox keeps `RUSTUP_HOME`
  and the operator's `CARGO_HOME` read-only.
- **Network** (gap-0bd49a). On macOS every agent process runs under a network rule of `sandbox.py`, and is denied the
  secret file, the key file and the run's private task directories. The direct loop's shell gets no network at all,
  since the driver makes every model call. The Roko arm's process tree, whose tools run the agent's commands, reaches
  only the loopback port of the endpoint Roko calls (the metering proxy's) and Unix sockets in its workspace. The
  Claude Code arm reaches the network only through an egress proxy of its own (`driver/egress.py`), which admits the
  targets of `[cli] egress_allow` (default `api.anthropic.com:443`) and logs every request to `egress.jsonl`. The
  record names the rule and the confinement that applied (`provenance.network_policy`), and for Claude Code every
  refused request. Off macOS the rule is not applied, and the record says "none" (gap-29ac83).
- **Label.** The driver commits the final tree as c_i with `families/common/repo.export_tree`, never with git in the
  agent's repo, then archives it (a git bundle, a tarball and the diff). The census (`census.py`) re-runs the visible
  checks on a clean export with the test files restored, runs the family's `hidden.py --secret-file` and the integrity
  checks, and counts canary hits; any hit makes the run `leak_suspected`.
- **Outputs** in `$VB_RESULTS/<experiment>/<run_id>/`: `manifest.json`, `order-<seed>.json`, `records.jsonl`
  (`vb.run_record/1`, validated before each write, `simulated: false`), `ledger.jsonl` (one validated row per
  attempt, priced from the snapshot; an unknown cost is null) and `reservations.jsonl`, `archives/`, `private/`,
  `errors.jsonl`, `s01/` (the Roko arm's copy of Roko's own records), and `transcripts/` with `--transcripts`.
- **Budget lines** (`ledger.py`, `experiments/budget.toml`). Each attempt reserves its worst case against the run's
  budget line before its first model call, and a dispatch that could pass the line's cap, its experiment's cap or
  the programme stop is refused. `vb ledger report` shows spend against every cap.
- **The secret** is a driver-only file (`secret.py`): `--secret-file`, `$VB_SECRET_FILE`, default
  `~/.config/viabilitybench/secret`, mode 0600 in a 0700 directory, outside the repository, `~/.roko`, the workdirs
  and the results; `secret.py init` makes one. It reaches only `hidden.py`, as a file path, after the agent's
  processes have ended: never in an environment variable or on a command line. `vb run` refuses to start when the
  driver's environment, or a `.roko/.env` that roko would load, holds the secret or its canary, and it checks every
  agent environment against both. Mode 0600 cannot stop an agent running as the driver's user from opening the file
  by its path, so while the tasks run, the tripwire holds it at mode 000 (`secret.tripwire`). A read then needs a
  chmod, the chmod changes the file's ctime, and the census marks the run `leak_suspected`. A container per task (S08
  decision 4) would prevent the read rather than detect it.
- **Provider keys** live in a key file under the secret file's rules: `--key-file`, `$VB_KEY_FILE`, default
  `~/.config/viabilitybench/keys`, one `NAME=value` line per arm `api_key_env`. `secret.py keys` checks it. Only the
  metering proxy, inside the driver, sends a key. `vb run` refuses to start while its environment holds one, and the
  tripwire holds the key file at mode 000 for the whole run.
- **One `vb run` at a time per secret file and per key file.** The tripwire takes an exclusive lock next to each file
  (`<file>.lock`), so a second run on it stops at start. Run the arms one after another.
  - **Parallel runs.** Give each run its own secret file (`secret.py init --secret-file PATH`, then `vb run
    --secret-file PATH`), its own copy of the key file (`--key-file`), and its own share of the instances (a stream of
    its own).
  - **Keep that assignment for every arm and seed.** The hidden cases are HMAC(secret, instance), so an instance
    audited under two secrets would face two different truth suites. `secret.py check` prints each file's
    fingerprint for the record, and S09 §4.1 has the campaign rule.
- **A new arm** adds `arms/<id>.toml` and, for a new harness, one `driver/<runner>.py` with `run_task(ctx)`
  (`harness.py` has the protocol); `vb.py` does not change.
- **Digests.** `config_hash` and `record_id` are `sha256:` digests of canonical JSON. S01's BLAKE3 `b3:` digests
  need `driver/fingerprint.py` and its golden vectors, which this tree does not have.
- **The metering and fault proxy** (`faultproxy.py`, S08 §4.11) meters every model call independently of the
  client and injects provider faults. The Roko arm reads its log, `proxy.jsonl`, when the run directory holds one.
- **Disturbances** (`vb run --disturbance SPEC.toml`, `disturb.py`, S08 §4.6) apply H6's hooks to stream positions.
  `flaky_verify` routes every arm's visible checks through the visible-verify wrapper (`vb_verify.py`), which fails
  some of them at random; the census's own rerun never meets a flake. `model_swap` has the metering proxy serve
  another model than the pin, and the model checks accept that one model as a declared swap (`model_swapped`).
- **Campaigns** (`vb campaign`, `driver/campaign.py`). An experiment manifest in `experiments/` lists an
  experiment's blocks: stream, arm, model, seeds, budget line and the rest of a `vb run`. `vb campaign --manifest PATH
  --dry-run` validates every block and estimates it against the budget and what the ledger already holds, and
  without `--dry-run` it runs one `vb run` per unit in the manifest's order (`as_listed`, or S09's
  `daily_interleave`), logs them in `<experiment>/campaign.jsonl`, and on a rerun goes on after the last finished
  unit. Its module docstring has the rules.
- **Gate G0** (`analysis/gates.py G0 --experiment PILOT-A --experiment PILOT-B ...`, S09 §4.7). It computes every
  G0 check from the pilot's runs and the evidence files it is given (the verifier-CI JSON, `vb ledger reconcile
  --json` per provider, the hand-filled SC2 spot check, a synthetic runaway's run), with its value, threshold and run
  ids, and writes `go-no-go.md` and `g0.json`. A check without its evidence is "not evaluated", never passed.
- **The report.** `vb report --experiment <id>` writes `metrics.json` and prints the VS rate, $/VS, pass^k and false
  greens of each arm (of each model, for an arm that ran more than one), every false green with its run id, and the
  excluded runs. `--bundle` writes the summary bundle for `reports/`, and `--check` holds bundles to their manifests
  and the budget (`analysis/report.py` has the rules).

## Schemas

| File | Validates | Example |
|---|---|---|
| `schema/task.schema.json` | `vb.task/1`, a task manifest: `task.json` in the generator's private `--out` directory, never in the agent's workdir (§5.2 put it in `DIR/.vb/`) | `examples/task.json` (§5.2, verbatim) |
| `schema/feature.schema.json` | `vb.feature/1`, a plan-slice instance's manifest, used in place of `vb.task/1` (S09 §4.9; `families/plan_slice/`) | `examples/feature.json` |
| `schema/run-record.schema.json` | `vb.run_record/1`, a row of `records.jsonl` (§5.4) | `examples/run-record.json` (§5.4, verbatim) |
| `schema/metric-record.schema.json` | `vb.metric_record/1`, a row of `metrics.json` (§5.5) | `examples/metric-record.json` |
| `schema/ledger.schema.json` | a row of `ledger.jsonl` (§4.10, §5.6) | `examples/ledger.json` |
| `schema/experiment.schema.json` | `vb.experiment/1`, an experiment manifest that `vb campaign` runs (S09 E5) | the manifests in `experiments/` |
| `schema/price-snapshot.schema.json` | `roko.price_snapshot/1`, a parsed `config/prices/<date>.toml` (§5.6) | `config/prices/2026-09-28.toml` |

The MetricRecord and ledger examples are built from the §5.4 record, since S08 gives only their field lists. The
feature example comes from `slicekit.py materialize` for PL03 at seed 1, with its paths and canary shortened.

A run record's `task.ladder` is `null` exactly when the task has no difficulty level: a plan-slice feature (family
`PL`). That widening of `vb.run_record/1` came before any record was written, so the version stays `/1`.

`schema/validate.py` implements exactly the JSON Schema subset these files use (`type`, `required`, `enum`,
`const`, `properties`, `items`, `minItems`, `additionalProperties`) and refuses a schema with any other keyword.
Record top levels, `costs`, `usage` and price rows are closed; other nested objects may carry extra detail.
Together, the schemas and the validator enforce S08's honesty invariants (§4.12, SC7):

- `simulated` is always `false`;
- an unknown cost is `null`, never 0: used tokens never cost $0, a `null` cost goes with cost source `unknown`
  and the other way round, missing usage makes the cost unknown, and a billed API row that used tokens cannot
  bill $0 (a subscription CLI session killed before its `result` event is priced from its stream, and its record
  and its ledger row say `estimated`, with $0 billed; a ledger row says whether it was billed, `billed`);
- `costs.by_class` (S09 §4.9's cost classes) splits `api_equiv_usd` without changing it, and a queue wait
  (`execution.queue_wait_s`, the sum of the attempts') is never below 0; both are `null` when the arm cannot
  observe them;
- `mock` usage never becomes a run record or a ledger row;
- every MetricRecord lists at least one run id, and names its `label_source`, `cost_basis` and
  `price_snapshot_id`;
- `infra_error` and `leak_suspected` runs keep that status, so the report can exclude and count them.

Usage token classes are disjoint: `tokens_in` counts only uncached input, so a cost is each class times its rate,
and `tokens_reasoning` is already inside `tokens_out` when the snapshot row says `reasoning_in_output = true`.

To check files from the command line (stdlib only, so any `python3` 3.11 or newer works):

```bash
python3 benchmarks/viabilitybench/schema/validate.py run-record "$VB_RESULTS/<experiment_id>/<run_id>/records.jsonl"
python3 benchmarks/viabilitybench/schema/validate.py price-snapshot config/prices/2026-09-28.toml
```

The kinds are `task`, `feature`, `run-record`, `metric-record`, `price-snapshot` and `ledger`. A `.json` file
holds one document, a `.jsonl` file one per line. The exit status is 1 when any document is invalid.

## The price snapshot

`config/prices/2026-09-28.toml` is the one snapshot, id `prices-2026-09-28`. The id resolves to the file
(`prices-<date>` → `config/prices/<date>.toml`). Records name it in `price_snapshot_id`, and `roko.toml`'s
`[pricing] snapshot` key will too once S08.T18 adds it. The driver and the planned Rust cost loader (S04.T01)
read these same bytes; there is no JSON copy.

- Rates are USD per 1M tokens. Every row has `slug, provider, input, cache_read, cache_write_5m, cache_write_1h,
  output, reasoning_in_output, source_url, verified`, in that order, plus an optional `note`.
- A rate the provider does not publish separately equals `input`. Every rate is above 0.
- A model missing from the file has an unknown cost: `null`, with cost source `unknown`.
- **Snapshots are immutable.** When a rate changes, add a new dated file with a new id.

The rates were read from the provider pages on 2026-09-28 and re-checked on 2026-09-29; each row's `verified`
column says so. For zai and moonshot, `reasoning_in_output = true` is inferred rather than stated by the
providers: their usage objects carry no reasoning count, and neither lists a reasoning rate. Reconciling the
ledger against the providers' usage exports (S08 SC5) will check it.

## Tests and the pinned venv

Every benchmark check runs with the project venv's Python, from the repository root:

```bash
python3 -m venv benchmarks/viabilitybench/.venv          # Python 3.11 or newer
benchmarks/viabilitybench/.venv/bin/python -m pip install --require-hashes -r benchmarks/viabilitybench/requirements.lock
benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench -q -rs
```

`-rs` prints each skipped test with its reason. The three `real_roko` tests in `driver/test_run_roko.py` run the Roko
arm against a built binary (`target/debug/roko`, or `$VB_TEST_ROKO_BIN`) and skip without one, so a green run that
skipped them never met real Roko. `vb run` itself refuses to start the Roko arm when its binary rejects the plan the
arm emits (`run_roko.preflight`).

The venv is ignored by `benchmarks/viabilitybench/.gitignore`. `requirements.lock` pins pytest and its
dependencies with hashes. To change them, edit `requirements.in` and regenerate the lock from this directory:

```bash
uv pip compile requirements.in --generate-hashes --universal --python-version 3.11 -o requirements.lock
```

The secondary models' tests (`analysis/models/test_models.py`) need the analysis stack too, and skip without it:

```bash
benchmarks/viabilitybench/.venv/bin/python -m pip install --require-hashes -r benchmarks/viabilitybench/requirements-analysis.lock
uv pip compile requirements-analysis.in --generate-hashes --universal --python-version 3.11 -o requirements-analysis.lock
```

The second line regenerates that lock from this directory after `requirements-analysis.in` changes.
