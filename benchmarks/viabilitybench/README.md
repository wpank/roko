# ViabilityBench

A cybernetic benchmark for agent harnesses. The system under test is a harness plus a model: every arm (Roko, a
bash-only direct loop, Claude Code, Codex) gets the same generated tasks, and success counts only when tests the
agent never saw pass on a clean re-run of its final commit (the VS-census label). Every cost is recomputable from
one dated price snapshot.

The design is spec S08 (`tmp/cybernetic-harness/specs/S08-benchmark-suite.md` in the author's workspace; the
`§` references below point into it). Suite id `vb`; schemas `vb.*/1`.

**Status (2026-09-29).** Only `schema/` and the price snapshot exist. The rest of the layout is built by the items
of epic `spec-567e52` in `work/items/`.

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
`requirements.in`, `requirements.lock` and a `.gitignore` for the venv.

## Schemas

| File | Validates | Example |
|---|---|---|
| `schema/task.schema.json` | `vb.task/1`, a task manifest `DIR/.vb/task.json` (§5.2) | `examples/task.json` (§5.2, verbatim) |
| `schema/run-record.schema.json` | `vb.run_record/1`, a row of `records.jsonl` (§5.4) | `examples/run-record.json` (§5.4, verbatim) |
| `schema/metric-record.schema.json` | `vb.metric_record/1`, a row of `metrics.json` (§5.5) | `examples/metric-record.json` |
| `schema/ledger.schema.json` | a row of `ledger.jsonl` (§4.10, §5.6) | `examples/ledger.json` |
| `schema/price-snapshot.schema.json` | `roko.price_snapshot/1`, a parsed `config/prices/<date>.toml` (§5.6) | `config/prices/2026-09-28.toml` |

The MetricRecord and ledger examples are built from the §5.4 record, since S08 gives only their field lists.

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

The kinds are `task`, `run-record`, `metric-record`, `price-snapshot` and `ledger`. A `.json` file holds one
document, a `.jsonl` file one per line. The exit status is 1 when any document is invalid.

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
