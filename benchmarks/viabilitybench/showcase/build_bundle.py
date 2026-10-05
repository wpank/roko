#!/usr/bin/env python3
"""Build a showcase replay bundle, `showcase-bundle/1` (S10 §5.5, task 9314), from one experiment's results.

    build_bundle.py --experiment PILOT [--results DIR] [--metrics FILE] [--verdicts FILE] [--mechanism DIR]
                    [--timeline FILE] [--econ FILE] [--bundle-id ID] [--title TEXT] [--featured]
                    [--created-at ISO-8601] [--out DIR]

**Input.** Every run of the experiment, `<results>/<experiment>/<run_id>/records.jsonl` (`vb.run_record/1`, S08 §5.4),
where results is `--results`, else `$VB_RESULTS`, else `~/.roko-bench/viability`; the analysis output, `--metrics`
(default `<results>/<experiment>/metrics.json`, the `vb.metrics/1` file `report.py` writes); and, when given or present
as `<results>/<experiment>/mechanism/`, S01's mechanism records (`<kind>.jsonl`), and a replay timeline of
`showcase-event/1` rows. Every run record and MetricRecord must pass `schema/validate.py`, and every MetricRecord's run
ids must name runs of the experiment. M3's economics report comes along when `--econ` names one or the experiment has
one at `.roko/econ/<experiment>/econ-report.json` (S04 §5): a `vb.econ_report/1` document of this experiment alone,
priced from the records' snapshot. S09's Holm decisions come along the same way, `--verdicts` or
`<results>/<experiment>/verdicts.json` (`analysis/holm.py`'s `write_verdicts`, gap-2da8ec): `vb.verdict/1` records,
one per hypothesis it decided, each naming only runs of the experiment.

**Output.** `--out` (default `.roko/showcase/bundles/<bundle_id>`), which must not exist yet:

    bundle.json            the manifest
    SHA256SUMS             `sha256sum` lines over every other file, by path
    data/records.jsonl     the run records, redacted: no transcript (`provenance.transcript_ref` null, any other
                           transcript field dropped), and every prompt and hidden-test name as `sha256:<hex>`
    data/metrics.jsonl     the MetricRecords: the only source of the numbers a view shows
    data/verdicts.jsonl    S09's `vb.verdict/1` decisions, when there are any: the only source of a tile's
                           claim_state and the overview's negatives, once one exists for its hypothesis
    data/mechanism/*.jsonl the mechanism records of the bundle's runs, when there are any
    timeline/events.jsonl  the replay timeline, when one is given
    econ/<id>/econ-report.json
                           M3's economics report, byte for byte, when there is one: what
                           `GET /api/showcase/economics?experiment_id=<id>` serves
    views/<view>.json      the R1 views the data makes: `overview`, the claims board, and `p1-head-to-head` when an
                           arm of the head-to-head ran (`m4-audits` waits for S05's audit records)

No statistic is computed here: a view copies MetricRecord and verdict record values, each with a `metric_ref` (or,
for a claim state, nothing to re-derive at all) that names its record, and `verify_bundle.py` re-derives every view
from `data/metrics.jsonl`, `data/verdicts.jsonl` and `data/records.jsonl` byte for byte. Each view follows its JSON
Schema in `demo/demo-app/src/showcase/schemas/` and lists the records it shows in `metrics` (`ViewMetric` in
`contracts.ts`), so the page's render guard can check the n and interval of every number before it draws one; a
record with n below 1 has no run behind it and no view shows it. A tile's claim state is S09's test record's own
verdict (gap-2da8ec): `NOT_YET_MEASURED`, still, for a hypothesis the results directory has no verdict record for
at all, and otherwise exactly that record's `claim_state`, copied, never recomputed; a `NOT_SUPPORTED` one also
becomes one of the overview's negatives. A view's provenance envelope (`showcase-provenance/1`) comes from the
manifest and the run records, so every record needs `execution.started_at` and `finished_at`, and a known
`costs.billed_usd`; its source's `sha256_verified` is stored false, for the reader that checks the bundle's
checksums to set.

Exit status: 0 when the bundle is written, 1 when the input is refused, 2 on a usage error.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import re
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import Any

SHOWCASE_DIR = Path(__file__).resolve().parent
VB_ROOT = SHOWCASE_DIR.parent
sys.path.insert(0, str(VB_ROOT / "schema"))
import validate  # noqa: E402

BUNDLE_SCHEMA = "showcase-bundle/1"
PROVENANCE_SCHEMA = "showcase-provenance/1"
RECORD_SCHEMA = "vb.run_record/1"
METRIC_SCHEMA = "vb.metric_record/1"
EVENT_SCHEMA = "showcase-event/1"
DEFAULT_RESULTS = Path("~/.roko-bench/viability")
REDACTION = {"transcripts": "excluded", "prompts": "sha256", "hidden_tests": "sha256", "diffs": "included"}
HASHED = re.compile(r"^sha256:[0-9a-f]{64}$")
ECON_REPORT = "econ-report.json"  # M3's economics report (S04 §5), `analysis/econ.py`'s output
ECON_SCHEMA = "vb.econ_report/1"
VERDICTS_FILE = "verdicts.json"  # S09 Holm decisions (analysis/holm.py's write_verdicts), beside metrics.json
VERDICT_SCHEMA = "vb.verdict/1"
VERDICT_CLAIM_STATES = ("SUPPORTED", "NOT_SUPPORTED", "INCONCLUSIVE")

# A claim's state is the verdict of S09's test record (contracts.ts `ClaimState`): copied from a matching
# `vb.verdict/1` record's own `claim_state` (gap-2da8ec) when the results directory has one for the hypothesis,
# else this default -- no experiment ran that decision yet.
CLAIM_STATE = "NOT_YET_MEASURED"
# A view shows numbers of several estimators: its provenance points at each metric's own, listed with the numbers.
PER_METRIC = "per metric (listed below)"
CI_LEVEL = 0.95  # every interval the analysis reports is a 95% one (alpha = 0.05 throughout analysis/)
H1_TEXT = "dependability economics (the envelope)"  # S09 §4.4's H1

# The R1 claims board (S10 §4.3 A, §5.2): the P1 tiles and M4's. A tile's rows are one headline MetricRecord of its
# `metric` per arm; a tile without one is not yet measured, and `planned_in` names the S09 experiments that will.
OVERVIEW_TILES = (
    {"id": "p1-usd-per-verified", "pillar": "P1", "mechanism": None, "title": "$/verified success",
     "hypothesis": "H1", "metric": "usd_per_vs", "view": "p1-head-to-head", "planned_in": ["LOG1"]},
    {"id": "p1-consistency", "pillar": "P1", "mechanism": None, "title": "consistency pass^3",
     "hypothesis": "H2", "metric": "pass_hat_3", "view": "p1-head-to-head", "planned_in": ["LOG1"]},
    {"id": "p1-routing", "pillar": "P1", "mechanism": None, "title": "routing saving",
     "hypothesis": "H4", "metric": None, "view": None, "planned_in": ["R-H4"]},
    {"id": "p1-spec-effect", "pillar": "P1", "mechanism": None, "title": "spec effect",
     "hypothesis": "H3", "metric": None, "view": None, "planned_in": ["LOG1"]},
    # M4's rate is the audit lottery's estimate of VS (S05), so a census rate (`vs_census`) is not M4's.
    {"id": "p2-m4-false-green", "pillar": "P2", "mechanism": "M4", "title": "false-green rate",
     "hypothesis": "H5", "metric": "false_green_rate", "label_source": "vs_estimated", "view": "m4-audits",
     "planned_in": ["E-H5-live"]},
)
# The arms' display labels, as their files give them (`arms/<arm>.toml`); test_bundle.py keeps the two in step.
ARM_LABELS = {
    "cheap_direct": "cheap·direct",
    "roko_fixed": "cheap·roko",
    "roko_full": "full·roko",
    "fd_claude": "frontier·direct",
    "fr_claude": "frontier·roko",
    "fd_claude_lite": "frontier·direct (lite)",
    "fd_codex": "frontier·direct (Codex)",
    "fd_api": "billed check",
    "roko_ladder": "ladder·roko",
    "roko_plan": "plan·roko",
}
# The P1 head-to-head's arms (S10 §4.3 B; S09's D1 and D2) with their tier, harness and role; no other arm is in it.
HEAD_TO_HEAD = {
    "cheap_direct": ("cheap", "direct", "arm"),
    "roko_fixed": ("cheap", "roko", "arm"),
    "roko_full": ("cheap", "roko", "arm"),
    "fd_claude": ("frontier", "direct", "arm"),
    "fr_claude": ("frontier", "roko", "probe"),
    "fd_claude_lite": ("frontier", "direct", "extra"),
    "fd_codex": ("frontier", "direct", "extra"),
    "fd_api": ("frontier", "direct", "extra"),
}


class BuildError(Exception):
    """The input cannot become a bundle."""


# ── Shared with verify_bundle.py ─────────────────────────────────────────────


def canonical_json(value: Any) -> bytes:
    """`value` as the bytes every bundle file and view uses: sorted keys, no spaces, UTF-8."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def jsonl_bytes(rows: list[dict]) -> bytes:
    """`rows` as canonical JSON lines."""
    return b"".join(canonical_json(row) + b"\n" for row in rows)


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def hashed(text: str) -> str:
    """`text` as `sha256:<hex>`, as a redacted prompt or hidden-test name reads."""
    return "sha256:" + sha256_hex(text.encode())


def metric_ref(record: dict) -> str:
    """The id a view gives the MetricRecord a value comes from: a digest of its canonical JSON."""
    return "m-" + sha256_hex(canonical_json(record))[:16]


def metric_kind(metric: str) -> str:
    """How a view draws `metric` (contracts.ts `MetricKind`), from its S08 name. The render guard asks every rate for
    an interval, or for `ci_method` "none"."""
    if metric.startswith("pass_hat_") or metric.endswith("_rate") or "_rate_" in metric:
        return "rate"
    if metric.startswith("usd_") or metric.endswith("_usd") or "_usd_" in metric:
        return "usd"
    if metric.endswith(("_runs", "_features")) or metric == "false_greens":
        return "count"
    if metric.startswith(("cost_gap", "envelope_ratio")) or metric.endswith("_ratio"):
        return "ratio"
    return "score"


def view_metric(ref: str, record: dict) -> dict:
    """The `metrics` entry (contracts.ts `ViewMetric`) of the MetricRecord `record`: its fields copied, and the kind
    its name gives."""
    arms = record.get("arms") or []
    return {
        "metric_ref": ref,
        "metric": record.get("metric"),
        "kind": metric_kind(str(record.get("metric"))),
        "value": record.get("value"),
        "ci": record.get("ci"),
        "ci_method": record.get("ci_method"),
        "n": record.get("n"),
        "estimator": record.get("estimator"),
        "arm": arms[0] if len(arms) == 1 else None,
        "envelope_level": record.get("envelope_level"),
    }


def displayed_refs(node: Any) -> set[str]:
    """Every `metric_ref` in `node` outside a `metrics` index or a provenance envelope: the numbers the render guard
    (`demo/demo-app/src/showcase/guard.ts`) finds displayed."""
    found: set[str] = set()
    if isinstance(node, list):
        for item in node:
            found |= displayed_refs(item)
    elif isinstance(node, dict):
        if isinstance(node.get("metric_ref"), str):
            found.add(node["metric_ref"])
        for key, value in node.items():
            if key not in ("metrics", "provenance"):
                found |= displayed_refs(value)
    return found


def econ_report_problem(report: Any, experiment: str, price_snapshot_id: Any) -> str | None:
    """Why `report` is not M3's economics report of `experiment` alone, priced from `price_snapshot_id`, or None."""
    if not isinstance(report, dict) or report.get("schema_version") != ECON_SCHEMA:
        return f"it is not a {ECON_SCHEMA} report"
    if report.get("experiments") != [experiment]:
        return f"it reports on {report.get('experiments')}, not {experiment} alone"
    if report.get("price_snapshot_id") != price_snapshot_id:
        return f"it is priced from {report.get('price_snapshot_id')}, not the bundle's {price_snapshot_id}"
    return None


def verdict_problem(record: Any, experiment: str) -> str | None:
    """Why `record` is not a `vb.verdict/1` decision of `experiment`, or None (gap-2da8ec)."""
    if not isinstance(record, dict) or record.get("schema_version") != VERDICT_SCHEMA:
        return f"it is not a {VERDICT_SCHEMA} record"
    if record.get("experiment_id") != experiment:
        return f"it decides {experiment!r}'s hypothesis for {record.get('experiment_id')!r}"
    if record.get("hypothesis") not in {f"H{n}" for n in range(1, 8)}:
        return f"hypothesis {record.get('hypothesis')!r} is not one of H1-H7"
    if record.get("claim_state") not in VERDICT_CLAIM_STATES:
        return f"claim_state {record.get('claim_state')!r} is not one of {', '.join(VERDICT_CLAIM_STATES)}"
    return None


def is_prompt_key(key: str) -> bool:
    lowered = key.lower()
    return lowered in ("prompt", "prompts") or lowered.endswith(("_prompt", "_prompts"))


def is_hidden_key(key: str) -> bool:
    return "hidden" in key.lower()


def instant(text: Any) -> dt.datetime:
    """`text`, an ISO 8601 time, as an aware datetime; anything else sorts last."""
    try:
        moment = dt.datetime.fromisoformat(str(text).replace("Z", "+00:00"))
    except ValueError:
        return dt.datetime.max.replace(tzinfo=dt.UTC)
    return moment if moment.tzinfo else moment.replace(tzinfo=dt.UTC)


def run_window(records: list[dict]) -> dict:
    """When the runs of `records` happened: the first `execution.started_at` and the last `finished_at`."""
    executions = [record.get("execution") or {} for record in records]
    starts = [execution["started_at"] for execution in executions if isinstance(execution.get("started_at"), str)]
    ends = [execution["finished_at"] for execution in executions if isinstance(execution.get("finished_at"), str)]
    return {"from": min(starts, key=instant, default=""), "to": max(ends, key=instant, default="")}


def arm_order(arm: str) -> tuple[int, str]:
    """Where `arm` sits in a view: the head-to-head's order, then any other arm by name."""
    return (list(ARM_LABELS).index(arm) if arm in ARM_LABELS else len(ARM_LABELS), arm)


def provenance_envelope(manifest: dict, metrics: list[dict], metrics_bytes: bytes, records: list[dict]) -> dict:
    """The `showcase-provenance/1` envelope of every view (S10 §5.1), from the manifest, the metrics file and the run
    records: n, seeds and window describe the runs, and each number's estimator and interval are its metric's."""
    experiments = manifest.get("experiment_ids") or []
    return {
        "schema": PROVENANCE_SCHEMA,
        "kind": manifest.get("kind"),
        "simulated": manifest.get("simulated"),
        "bundle_id": manifest.get("bundle_id"),
        "experiment_ids": experiments,
        "run_ids": manifest.get("run_ids"),
        "sources": [
            {
                "path": "data/metrics.jsonl",
                "schema": METRIC_SCHEMA,
                "sha256": sha256_hex(metrics_bytes),
                "rows": len(metrics),
                "simulated": False,
                "sha256_verified": False,
            }
        ],
        "harness_commit": manifest.get("harness_commit"),
        "dirty": manifest.get("dirty"),
        "analysis_commit": manifest.get("analysis_commit"),
        "config_hashes": manifest.get("config_hashes"),
        "price_snapshot_id": manifest.get("price_snapshot_id"),
        "models": manifest.get("models"),
        "n": len(records),
        "seeds": sorted({record["seed"] for record in records if isinstance(record.get("seed"), int)}),
        "window": run_window(records),
        "estimator": PER_METRIC,
        "record_filter": " or ".join(f"experiment_id == {json.dumps(experiment)}" for experiment in experiments),
        "ci": {"method": PER_METRIC, "strata": [], "level": CI_LEVEL, "resamples": None},
        "cost_usd": manifest.get("cost_usd"),
        "generated_at": manifest.get("created_at"),
        "reproduce": manifest.get("reproduce"),
    }


def project_views(manifest: dict, metrics: list[dict], metrics_bytes: bytes, records: list[dict],
                  verdicts: Sequence[dict] = ()) -> dict[str, bytes]:
    """The R1 views the data makes, as the bytes of `views/<view>.json`: MetricRecord values copied, never computed,
    in the shapes of the page's contracts, each view with its `metrics` index of the records it shows. `verdicts`
    (S09 `vb.verdict/1` decisions, `analysis/holm.py`; gap-2da8ec) gives a tile its measured `claim_state`
    instead of the `NOT_YET_MEASURED` default, by its own `hypothesis`, copied verbatim, never recomputed."""
    provenance = provenance_envelope(manifest, metrics, metrics_bytes, records)
    by_hypothesis = {record["hypothesis"]: record for record in verdicts}
    # The guard refuses a number with no sample size, so a record with n below 1 (its value null) is shown nowhere.
    refs = [
        (metric_ref(record), record)
        for record in metrics
        if isinstance(record.get("n"), int) and record["n"] >= 1
    ]
    by_ref = dict(refs)

    def headline(arm: str, metric: str | None, label_source: str | None = None) -> dict | None:
        """The Estimate of `arm`'s first headline record of `metric`: one with that arm alone and no ladder level."""
        for ref, record in refs:
            if (
                record.get("arms") == [arm]
                and record.get("ladder") is None
                and record.get("metric") == metric
                and label_source in (None, record.get("label_source"))
            ):
                return {"value": record.get("value"), "ci": record.get("ci"), "metric_ref": ref}
        return None

    def indexed(view: dict) -> dict:
        view["metrics"] = [view_metric(ref, by_ref[ref]) for ref in sorted(displayed_refs(view))]
        return view

    runs: dict[str, list[dict]] = {}
    for record in records:
        runs.setdefault(str(record.get("arm")), []).append(record)
    arms = []
    for arm in sorted(set(runs) & set(HEAD_TO_HEAD), key=arm_order):
        tier, harness, role = HEAD_TO_HEAD[arm]
        sources = {(run.get("costs") or {}).get("source") for run in runs[arm]}
        pass_hat = {k: headline(arm, f"pass_hat_{k}") for k in ("1", "3", "5")}
        arms.append(
            {
                "arm": arm,
                "label": ARM_LABELS[arm],
                "tier": tier,
                "harness": harness,
                "role": role,
                "note": None,
                "models": models_of(runs[arm]),
                "status": "run",
                "reason": None,
                # The arm's run records in the bundle, and their distinct tasks.
                "n_tasks": len({str((run.get("task") or {}).get("instance_id")) for run in runs[arm]}),
                "n_trials": len(runs[arm]),
                "resolve": headline(arm, "vs_rate"),
                "usd_per_verified": headline(arm, "usd_per_vs"),
                "cost_source": next(iter(sources)) if sources in ({"provider_usage"}, {"cli_usage"}) else None,
                "pass_hat_k": pass_hat if all(pass_hat.values()) else None,
                "outcome_sd": None,
            }
        )
    made = {"overview", "p1-head-to-head"} if arms else {"overview"}

    # A tile's rows: each arm with a headline record of the tile's metric, in the head-to-head's order.
    shown = sorted({record["arms"][0] for _, record in refs if len(record.get("arms") or []) == 1}, key=arm_order)
    tiles = []
    for tile in OVERVIEW_TILES:
        rows = []
        for arm in shown:
            estimate = headline(arm, tile["metric"], tile.get("label_source"))
            if estimate is not None:
                rows.append({"label": ARM_LABELS.get(arm, arm), "estimate": estimate})
        verdict = by_hypothesis.get(tile["hypothesis"])
        tiles.append(
            {
                "id": tile["id"],
                "pillar": tile["pillar"],
                "mechanism": tile["mechanism"],
                "title": tile["title"],
                "hypothesis": tile["hypothesis"],
                "claim_state": verdict["claim_state"] if verdict else CLAIM_STATE,
                "rows": rows,
                "planned_in": list(tile["planned_in"]),
                "view": tile["view"] if rows and tile["view"] in made else None,
            }
        )

    # Results against the thesis are S09's NOT_SUPPORTED verdicts (gap-2da8ec): a tile whose own measured claim
    # state says so, never a state this projection derives on its own.
    negatives = [
        {"id": f"neg-{tile['id']}", "kind": "other", "text": f"{tile['title']}: not supported by the measured "
         "data.", "rows": tile["rows"], "view": tile["view"]}
        for tile in tiles if tile["claim_state"] == "NOT_SUPPORTED"
    ]
    views = {"overview": {"schema": "showcase-view/overview/1", "tiles": tiles, "negatives": negatives}}
    if arms:
        prereg_ids = {record.get("prereg_id") for record in metrics}
        h1_verdict = by_hypothesis.get("H1")
        views["p1-head-to-head"] = {
            "schema": "showcase-view/p1-head-to-head/1",
            "claim": {
                "hypothesis": "H1",
                "state": h1_verdict["claim_state"] if h1_verdict else CLAIM_STATE,
                "prereg_id": next(iter(prereg_ids)) if len(prereg_ids) == 1 else None,
                "planned_in": list(OVERVIEW_TILES[0]["planned_in"]),
                "text": H1_TEXT,
            },
            "arms": arms,
            # S10 §5.2: the frontier is computed offline, and no analysis writes one yet.
            "pareto": {"x": "usd_per_verified", "y": "resolve", "frontier_arms": []},
            "envelope": [],
        }
    return {name: canonical_json({**indexed(view), "provenance": provenance}) for name, view in views.items()}


def write_sums(bundle: Path) -> None:
    """Write `SHA256SUMS` over every file in `bundle` but itself, in `sha256sum` format, by path."""
    lines = []
    for path in sorted(file for file in bundle.rglob("*") if file.is_file()):
        relative = path.relative_to(bundle).as_posix()
        if relative != "SHA256SUMS":
            lines.append(f"{sha256_hex(path.read_bytes())}  {relative}\n")
    (bundle / "SHA256SUMS").write_text("".join(lines), encoding="utf-8")


# ── Redaction ────────────────────────────────────────────────────────────────


def redact(value: Any, key: str = "") -> Any:
    """`value`, the run record or a part of it under `key`, with no transcript and every prompt and hidden-test
    name hashed. `provenance.transcript_ref` stays as null, since the record's schema requires the field."""
    if isinstance(value, dict):
        out = {}
        for child, item in value.items():
            if "transcript" in child.lower():
                if child == "transcript_ref":
                    out[child] = None
                continue
            out[child] = redact(item, child)
        return out
    if isinstance(value, list):
        return [redact(item, key) for item in value]
    if isinstance(value, str) and (is_prompt_key(key) or is_hidden_key(key)) and not HASHED.match(value):
        return hashed(value)
    return value


def redact_record(record: dict) -> dict:
    """A run record as a bundle holds it ([`redact`]); the names of the failed truth-suite checks are hidden tests."""
    out = redact(record)
    vs = out.get("vs")
    if isinstance(vs, dict) and isinstance(vs.get("failed"), list):
        vs["failed"] = [hashed(name) if isinstance(name, str) and not HASHED.match(name) else name for name in vs["failed"]]
    return out


# ── Build ────────────────────────────────────────────────────────────────────


def read_jsonl(path: Path) -> list[dict]:
    rows = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        if line.strip():
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError as error:
                raise BuildError(f"{path}:{number}: not JSON: {error}") from error
    return rows


def load_records(experiment_dir: Path) -> list[dict]:
    run_dirs = sorted(path for path in experiment_dir.iterdir() if (path / "records.jsonl").is_file())
    if not run_dirs:
        raise BuildError(f"{experiment_dir} holds no run directory with records.jsonl")
    records = []
    for run_dir in run_dirs:
        for row in read_jsonl(run_dir / "records.jsonl"):
            errors = validate.validate("run-record", row)
            if errors:
                raise BuildError(f"{run_dir.name}: a run record is invalid: {errors[0]}")
            records.append(row)
    return records


def models_of(records: list[dict]) -> list[str]:
    models = set()
    for record in records:
        for attempt in record.get("execution", {}).get("attempts", []):
            model = attempt.get("model_reported") or attempt.get("model_requested")
            if model:
                provider = attempt.get("provider")
                models.add(f"{provider}/{model}" if provider else model)
    return sorted(models)


def read_econ_report(path: Path, experiment: str, price_snapshot_id: str) -> bytes:
    """The bytes of M3's economics report at `path`, which a bundle copies unchanged."""
    try:
        data = path.read_bytes()
        report = json.loads(data)
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise BuildError(f"cannot read the economics report {path}: {error}") from error
    problem = econ_report_problem(report, experiment, price_snapshot_id)
    if problem:
        raise BuildError(f"the economics report {path} cannot join the bundle: {problem}")
    return data


def timed(record: dict) -> bool:
    """Whether `record` says when its run started and finished, as a view's provenance window needs."""
    execution = record.get("execution") or {}
    unknown = dt.datetime.max.replace(tzinfo=dt.UTC)
    return all(instant(execution.get(key)) != unknown for key in ("started_at", "finished_at"))


def only(values: set, what: str) -> Any:
    if len(values) != 1:
        raise BuildError(f"the records name {len(values)} {what}s, and a bundle names one: {sorted(map(str, values))}")
    return next(iter(values))


def build(args: argparse.Namespace) -> Path:
    results = Path(args.results or os.environ.get("VB_RESULTS") or DEFAULT_RESULTS).expanduser()
    experiment_dir = results / args.experiment
    if not experiment_dir.is_dir():
        raise BuildError(f"no experiment directory {experiment_dir}")
    records = load_records(experiment_dir)
    metrics_path = Path(args.metrics) if args.metrics else experiment_dir / "metrics.json"
    try:
        metrics_doc = json.loads(metrics_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise BuildError(f"cannot read the metrics file {metrics_path}: {error}") from error
    metrics = metrics_doc.get("records") if isinstance(metrics_doc, dict) else None
    if not isinstance(metrics, list) or not metrics:
        raise BuildError(f"{metrics_path} holds no MetricRecords")
    run_ids = sorted({record["run_id"] for record in records})
    for number, metric in enumerate(metrics, start=1):
        errors = validate.validate("metric-record", metric)
        if errors:
            raise BuildError(f"MetricRecord {number} is invalid: {errors[0]}")
        unknown = sorted(set(metric["run_ids"]) - set(run_ids))
        if unknown:
            raise BuildError(f"MetricRecord {number} ({metric['metric']}) names runs the experiment lacks: {unknown}")
    verdicts_path = Path(args.verdicts) if args.verdicts else experiment_dir / VERDICTS_FILE
    verdicts: list[dict] = []
    if args.verdicts or verdicts_path.is_file():
        try:
            verdicts_doc = json.loads(verdicts_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise BuildError(f"cannot read the verdicts file {verdicts_path}: {error}") from error
        verdicts = verdicts_doc.get("records") if isinstance(verdicts_doc, dict) else None
        if not isinstance(verdicts, list) or not verdicts:
            raise BuildError(f"{verdicts_path} holds no verdict records")
        for number, record in enumerate(verdicts, start=1):
            problem = verdict_problem(record, args.experiment)
            if problem:
                raise BuildError(f"verdict record {number} in {verdicts_path} is invalid: {problem}")
            unknown = sorted(set(record["run_ids"]) - set(run_ids))
            if unknown:
                raise BuildError(f"verdict record {number} ({record['hypothesis']}) names runs the experiment "
                                 f"lacks: {unknown}")
    price_snapshot_id = only({record["price_snapshot_id"] for record in records}, "price snapshot")
    if metrics_doc.get("price_snapshot_id", price_snapshot_id) != price_snapshot_id:
        raise BuildError("the metrics file and the records use different price snapshots")
    untimed = [record["record_id"] for record in records if not timed(record)]
    if untimed:
        raise BuildError(f"{len(untimed)} run record(s) lack execution.started_at or finished_at, which a view's "
                         f"provenance window needs: {', '.join(untimed[:3])}")
    unbilled = [record["record_id"] for record in records if record["costs"]["billed_usd"] is None]
    if unbilled:
        raise BuildError(f"{len(unbilled)} run record(s) have an unknown billed cost, and a bundle states what its "
                         f"runs cost (S10 §5.1): {', '.join(unbilled[:3])}")

    bundle_id = args.bundle_id or f"b-{args.experiment.lower()}"
    out = Path(args.out) if args.out else Path(".roko/showcase/bundles") / bundle_id
    if out.exists():
        raise BuildError(f"{out} exists: a bundle is written once")
    redacted = [redact_record(record) for record in records]
    files: dict[str, bytes] = {
        "data/records.jsonl": jsonl_bytes(redacted),
        "data/metrics.jsonl": jsonl_bytes(metrics),
    }
    schemas = {"data/records.jsonl": RECORD_SCHEMA, "data/metrics.jsonl": METRIC_SCHEMA}
    rows = {"data/records.jsonl": len(records), "data/metrics.jsonl": len(metrics)}
    mechanism_dir = Path(args.mechanism) if args.mechanism else experiment_dir / "mechanism"
    if mechanism_dir.is_dir():
        for source in sorted(mechanism_dir.glob("*.jsonl")):
            kept = [row for row in read_jsonl(source) if row.get("run_id") in run_ids]
            if kept:
                path = f"data/mechanism/{source.name}"
                files[path] = jsonl_bytes([redact(row) for row in kept])
                schemas[path] = f"s01/{source.stem}"
                rows[path] = len(kept)
    if args.timeline:
        events = sorted(read_jsonl(Path(args.timeline)), key=lambda event: event.get("seq", 0))
        files["timeline/events.jsonl"] = jsonl_bytes(events)
        schemas["timeline/events.jsonl"] = EVENT_SCHEMA
        rows["timeline/events.jsonl"] = len(events)
    econ = Path(args.econ) if args.econ else Path(".roko/econ") / args.experiment / ECON_REPORT
    if args.econ or econ.is_file():
        path = f"econ/{args.experiment}/{ECON_REPORT}"
        files[path] = read_econ_report(econ, args.experiment, price_snapshot_id)
        schemas[path] = ECON_SCHEMA
        rows[path] = 1
    if verdicts:
        files["data/verdicts.jsonl"] = jsonl_bytes(verdicts)
        schemas["data/verdicts.jsonl"] = VERDICT_SCHEMA
        rows["data/verdicts.jsonl"] = len(verdicts)

    created_at = args.created_at or dt.datetime.now(dt.UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    manifest = {
        "schema": BUNDLE_SCHEMA,
        "bundle_id": bundle_id,
        "kind": "replay",
        "simulated": False,
        "title": args.title or f"{args.experiment} replay",
        "created_at": created_at,
        "featured": bool(args.featured),
        "experiment_ids": [args.experiment],
        "run_ids": run_ids,
        "harness_commit": ",".join(sorted({record["harness_sha"] for record in records})),
        "dirty": any(record["dirty"] for record in records),
        "analysis_commit": metrics_doc.get("analysis_commit") or metrics[0]["analysis_commit"],
        "config_hashes": sorted({record["config_hash"] for record in records}),
        "price_snapshot_id": price_snapshot_id,
        "models": models_of(records),
        "cost_usd": round(sum(record["costs"]["billed_usd"] for record in records), 6),
        "redaction": dict(REDACTION),
        "files": [
            {"path": path, "schema": schemas[path], "sha256": sha256_hex(data), "rows": rows[path]}
            for path, data in sorted(files.items())
        ],
        "reproduce": [f"vb run --experiment {args.experiment}", f"vb report --experiment {args.experiment}"],
    }
    # The views come from the files as the bundle holds them, as verify_bundle.py re-derives them.
    views = project_views(manifest, metrics, files["data/metrics.jsonl"], redacted, verdicts)
    manifest["views"] = list(views)

    out.mkdir(parents=True)
    for path, data in files.items():
        (out / path).parent.mkdir(parents=True, exist_ok=True)
        (out / path).write_bytes(data)
    for name, data in views.items():
        (out / "views").mkdir(exist_ok=True)
        (out / "views" / f"{name}.json").write_bytes(data)
    (out / "bundle.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_sums(out)
    return out


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Build a showcase replay bundle (S10 §5.5).")
    parser.add_argument("--experiment", required=True, help="the experiment id, e.g. PILOT")
    parser.add_argument("--results", help="the results root (default: $VB_RESULTS, then ~/.roko-bench/viability)")
    parser.add_argument("--metrics", help="the vb.metrics/1 file (default: <results>/<experiment>/metrics.json)")
    parser.add_argument("--verdicts", help="the vb.verdicts/1 file (default: <results>/<experiment>/verdicts.json, "
                        "when there is one)")
    parser.add_argument("--mechanism", help="a directory of S01 mechanism records, <kind>.jsonl")
    parser.add_argument("--timeline", help="a JSONL file of showcase-event/1 rows")
    parser.add_argument(
        "--econ", help="M3's economics report (default: .roko/econ/<experiment>/econ-report.json, when there is one)"
    )
    parser.add_argument("--bundle-id", help="the bundle's id (default: b-<experiment>)")
    parser.add_argument("--title", help="the bundle's title")
    parser.add_argument("--featured", action="store_true", help="mark the bundle as the featured one")
    parser.add_argument("--created-at", help="the bundle's creation time, ISO 8601 (default: now)")
    parser.add_argument("--out", help="the bundle directory (default: .roko/showcase/bundles/<bundle_id>)")
    args = parser.parse_args(argv)
    try:
        out = build(args)
    except BuildError as error:
        print(f"refused: {error}", file=sys.stderr)
        return 1
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
