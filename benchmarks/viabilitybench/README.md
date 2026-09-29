# ViabilityBench

A cybernetic benchmark for agent harnesses. The system under test is a harness plus a model: every arm (Roko, a
bash-only direct loop, Claude Code, Codex) gets the same generated tasks, and success counts only when tests the
agent never saw pass on a clean re-run of its final commit (the VS-census label). Every cost is recomputable from
one dated price snapshot.

The design is spec S08 (`tmp/cybernetic-harness/specs/S08-benchmark-suite.md` in the author's workspace; the
`§` references below point into it). Suite id `vb`; schemas `vb.*/1`.

**Status (2026-09-29).** Built: `schema/` and the price snapshot (gap-0580f7), the common library
`families/common/` (gap-2790c5), `speclint/` (S07.1), and the direct-arm driver `driver/` with `arms/cheap_direct.toml`,
`arms/fd_api.toml` and `streams/pilot.toml` (gap-28ebea). Not built yet: the families F1 and F4 that the pilot stream
names, verifier CI, analysis and reports, and the other arms. Epic `spec-567e52` in `work/items/` tracks them.

## Rules

- **Source only.** This tree holds generators, verifiers, the driver and analysis code. Raw results live outside
  the repo in `$VB_RESULTS` (default `~/.roko-bench/viability`), and task workdirs in `$VB_WORK` (default
  `~/vb-work`). Each pilot's small summary bundle (records, metrics, ledger and report page; no transcripts or
  archives) is committed under `reports/`.
- **Stdlib-only Python 3.11 or newer** for everything the benchmark runs: families, verifiers, driver and
  analysis. pytest is a dev-only dependency, pinned in `requirements.lock`.
- **One price source.** Costs come from `config/prices/<date>.toml`, never from `roko.toml`'s per-model rates
  (its gpt-oss-120b rates are wrong) and never from a fallback rate.
- **No provider calls in tests.** Tests run offline against fixtures and stub servers.

## Layout (S08 §5.1)

```
benchmarks/viabilitybench/
  families/common/{repo.py, knobs.py, hmac_seed.py, astcheck.py, mutate.py, canary.py}
  families/f1_pyconv/{gen.py, template/, hidden.py, gaming.py, ladder.toml, spec/precise.md.j2,
                      reference/{solution, stub, gaming}/}          # same shape for f2…f7; f8 wraps f1–f5
  external/swebench/{select.py, probe.py, slice-v1.jsonl, run_official.py}
  streams/{s1_learncurve, s3_disturbance, s5_holdout, p1_core, p1_h3, p1_ext, log1, pilot}.toml
  arms/{cheap_direct, roko_fixed, roko_full, fd_claude, fd_claude_lite, fd_codex, fd_api, fr_claude}.toml
  driver/{vb.py, materialize.py, planemit.py, run_roko.py, mini_loop.py, run_cli.py, ledger.py, caps.py,
          archive.py, census.py, faultproxy.py, records.py}
  schema/{task, run-record, metric-record, price-snapshot, ledger}.schema.json   # prices: config/prices/2026-09-28.toml (§5.6)
  analysis/{metrics.py, passk.py, bootstrap.py, cs.py, cuped.py, irt.py, replay.py, report.py}   # shared with S09
  ci/{verify_verifiers.py, determinism.py, leak_check.py}
$VB_RESULTS (default ~/.roko-bench/viability)/<experiment_id>/<run_id>/
  manifest.json  records.jsonl  metrics.json  ledger.jsonl  proxy.jsonl  s01/  archives/  transcripts/ (opt-in)
```

Beyond S08's block, the tree also holds `experiments/` (S09 §6: manifests, budget lines, the pre-registration
lock), `reports/` (committed pilot summaries), `schema/validate.py`, `schema/test_schemas.py`, `schema/examples/`,
`requirements.in`, `requirements.lock` and a `.gitignore` for the venv. `driver/` adds `layout.py`, `provider.py`,
`stub_provider.py`, `harness.py`, `agent_env.py`, `test_driver.py` and a toy family in `testdata/`.

## The driver (direct arm)

`driver/vb.py` runs a stream of tasks on one arm and one model (S08 §4.9–4.10, §5.7). The direct loop
(`mini_loop.py`, harness `mini-loop`) serves `cheap_direct` and `fd_api`: one model, one bash tool, no Roko prompt.

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
  per-task HOME.
- **Label.** The driver commits the final tree as c_i with `families/common/repo.export_tree`, never with git in the
  agent's repo, then archives it (a git bundle, a tarball and the diff). The census (`census.py`) re-runs the visible
  checks on a clean export with the test files restored, runs the family's `hidden.py --secret-file` and the integrity
  checks, and counts canary hits; any hit makes the run `leak_suspected`.
- **Outputs** in `$VB_RESULTS/<experiment>/<run_id>/`: `manifest.json`, `order-<seed>.json`, `records.jsonl`
  (`vb.run_record/1`, validated before each write, `simulated: false`), `ledger.jsonl` (one validated row per
  attempt, priced from the snapshot; an unknown cost is null), `archives/`, `private/`, `errors.jsonl`, and
  `transcripts/` with `--transcripts`.
- **The secret** is a file (`--secret-file`, `$VB_SECRET_FILE`, default `~/.config/viabilitybench/secret`, mode 0600),
  read only by the census. Proving that it never reaches an agent is gap-a8a160.
- **A new arm** adds `arms/<id>.toml` and, for a new harness, one `driver/<runner>.py` with `run_task(ctx)`
  (`harness.py` has the protocol); `vb.py` does not change.
- **Not yet built:** budget-line caps in the ledger (gap-33d54b), the metering and fault proxy (gap-e003ec), the
  Claude Code and Roko arms (gap-c4f364, gap-b7ab99), `vb census`/`vb report`, and S01's BLAKE3 `config_hash`:
  records carry `sha256:` digests until `driver/fingerprint.py` and its golden vectors exist.

## Schemas

| File | Validates | Example |
|---|---|---|
| `schema/task.schema.json` | `vb.task/1`, a task manifest `DIR/.vb/task.json` (§5.2) | `examples/task.json` (§5.2, verbatim) |
| `schema/feature.schema.json` | `vb.feature/1`, a plan-slice instance's manifest, used in place of `vb.task/1` (S09 §4.9; `families/plan_slice/`) | `examples/feature.json` |
| `schema/run-record.schema.json` | `vb.run_record/1`, a row of `records.jsonl` (§5.4) | `examples/run-record.json` (§5.4, verbatim) |
| `schema/metric-record.schema.json` | `vb.metric_record/1`, a row of `metrics.json` (§5.5) | `examples/metric-record.json` |
| `schema/ledger.schema.json` | a row of `ledger.jsonl` (§4.10, §5.6) | `examples/ledger.json` |
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
  bill $0;
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
benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench -q
```

The venv is ignored by `benchmarks/viabilitybench/.gitignore`. `requirements.lock` pins pytest and its
dependencies with hashes. To change them, edit `requirements.in` and regenerate the lock from this directory:

```bash
uv pip compile requirements.in --generate-hashes --universal --python-version 3.11 -o requirements.lock
```
