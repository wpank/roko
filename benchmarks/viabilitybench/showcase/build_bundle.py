#!/usr/bin/env python3
"""Build a showcase replay bundle, `showcase-bundle/1` (S10 §5.5, task 9314), from one experiment's results.

    build_bundle.py --experiment PILOT [--results DIR] [--metrics FILE] [--mechanism DIR] [--timeline FILE]
                    [--bundle-id ID] [--title TEXT] [--featured] [--created-at ISO-8601] [--out DIR]

**Input.** Every run of the experiment, `<results>/<experiment>/<run_id>/records.jsonl` (`vb.run_record/1`, S08 §5.4),
where results is `--results`, else `$VB_RESULTS`, else `~/.roko-bench/viability`; the analysis output, `--metrics`
(default `<results>/<experiment>/metrics.json`, the `vb.metrics/1` file `report.py` writes); and, when given or present
as `<results>/<experiment>/mechanism/`, S01's mechanism records (`<kind>.jsonl`), and a replay timeline of
`showcase-event/1` rows. Every run record and MetricRecord must pass `schema/validate.py`, and every MetricRecord's run
ids must name runs of the experiment.

**Output.** `--out` (default `.roko/showcase/bundles/<bundle_id>`), which must not exist yet:

    bundle.json            the manifest
    SHA256SUMS             `sha256sum` lines over every other file, by path
    data/records.jsonl     the run records, redacted: no transcript (`provenance.transcript_ref` null, any other
                           transcript field dropped), and every prompt and hidden-test name as `sha256:<hex>`
    data/metrics.jsonl     the MetricRecords: the only source of the numbers a view shows
    data/mechanism/*.jsonl the mechanism records of the bundle's runs, when there are any
    timeline/events.jsonl  the replay timeline, when one is given
    views/<view>.json      the R1 views, `overview`, `p1-head-to-head` and `m4-audits`

No statistic is computed here: a view copies MetricRecord values, each with a `metric_ref` that names its record, and
`verify_bundle.py` re-derives every view from `data/metrics.jsonl` byte for byte. A view's provenance envelope
(`showcase-provenance/1`) comes from the manifest; the reader that checks the bundle's checksums says so
(`sha256_verified`), so the stored view does not.

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
VIEWS = ("overview", "p1-head-to-head", "m4-audits")
DEFAULT_RESULTS = Path("~/.roko-bench/viability")
REDACTION = {"transcripts": "excluded", "prompts": "sha256", "hidden_tests": "sha256", "diffs": "included"}
HASHED = re.compile(r"^sha256:[0-9a-f]{64}$")
PASS_HAT = re.compile(r"^pass_hat_(\d+)$")


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


def is_prompt_key(key: str) -> bool:
    lowered = key.lower()
    return lowered in ("prompt", "prompts") or lowered.endswith(("_prompt", "_prompts"))


def is_hidden_key(key: str) -> bool:
    return "hidden" in key.lower()


def provenance_envelope(manifest: dict, metrics: list[dict], metrics_bytes: bytes) -> dict:
    """The `showcase-provenance/1` envelope of every view (S10 §5.1), from the manifest and the metrics file."""
    return {
        "schema": PROVENANCE_SCHEMA,
        "kind": manifest.get("kind"),
        "simulated": manifest.get("simulated"),
        "bundle_id": manifest.get("bundle_id"),
        "experiment_ids": manifest.get("experiment_ids"),
        "run_ids": manifest.get("run_ids"),
        "sources": [
            {
                "path": "data/metrics.jsonl",
                "schema": METRIC_SCHEMA,
                "sha256": sha256_hex(metrics_bytes),
                "rows": len(metrics),
                "simulated": False,
            }
        ],
        "harness_commit": manifest.get("harness_commit"),
        "dirty": manifest.get("dirty"),
        "analysis_commit": manifest.get("analysis_commit"),
        "config_hashes": manifest.get("config_hashes"),
        "price_snapshot_id": manifest.get("price_snapshot_id"),
        "models": manifest.get("models"),
        "cost_usd": manifest.get("cost_usd"),
        "generated_at": manifest.get("created_at"),
        "reproduce": manifest.get("reproduce"),
    }


def project_views(manifest: dict, metrics: list[dict], metrics_bytes: bytes) -> dict[str, bytes]:
    """The R1 views, as the bytes of `views/<view>.json`: MetricRecord values copied, never computed."""
    provenance = provenance_envelope(manifest, metrics, metrics_bytes)
    refs = [(metric_ref(record), record) for record in metrics]

    def shown(ref: str, record: dict) -> dict:
        return {
            "value": record.get("value"),
            "ci": record.get("ci"),
            "ci_method": record.get("ci_method"),
            "n": record.get("n"),
            "metric_ref": ref,
        }

    tiles = [
        {"metric": record.get("metric"), "arms": record.get("arms"), "ladder": record.get("ladder"), **shown(ref, record)}
        for ref, record in refs
    ]
    tiles.sort(key=lambda tile: (str(tile["metric"]), json.dumps(tile["arms"]), str(tile["ladder"]), tile["metric_ref"]))

    # A cell of one arm with no ladder level is the arm's headline row.
    cells: dict[str, dict[str, tuple[str, dict]]] = {}
    for ref, record in refs:
        arms = record.get("arms") or []
        if len(arms) == 1 and record.get("ladder") is None:
            cells.setdefault(arms[0], {}).setdefault(str(record.get("metric")), (ref, record))

    def cell_value(arm: str, metric: str) -> dict | None:
        found = cells.get(arm, {}).get(metric)
        return shown(*found) if found else None

    head_to_head = []
    audits = []
    for arm in sorted(cells):
        pass_hat = {
            match.group(1): shown(*cells[arm][metric])
            for metric in sorted(cells[arm])
            if (match := PASS_HAT.match(metric))
        }
        usd = cell_value(arm, "usd_per_vs")
        if usd is not None:
            usd["cost_basis"] = cells[arm]["usd_per_vs"][1].get("cost_basis")
        head_to_head.append(
            {"arm": arm, "resolve": cell_value(arm, "vs_rate"), "usd_per_verified": usd, "pass_hat_k": pass_hat}
        )
        audits.append(
            {
                "arm": arm,
                "false_green_rate": cell_value(arm, "false_green_rate"),
                "false_greens": cell_value(arm, "false_greens"),
            }
        )
    views = {
        "overview": {"schema": "showcase-view/overview/1", "tiles": tiles, "provenance": provenance},
        "p1-head-to-head": {"schema": "showcase-view/p1-head-to-head/1", "arms": head_to_head, "provenance": provenance},
        "m4-audits": {"schema": "showcase-view/m4-audits/1", "arms": audits, "provenance": provenance},
    }
    return {name: canonical_json(view) for name, view in views.items()}


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
    price_snapshot_id = only({record["price_snapshot_id"] for record in records}, "price snapshot")
    if metrics_doc.get("price_snapshot_id", price_snapshot_id) != price_snapshot_id:
        raise BuildError("the metrics file and the records use different price snapshots")

    bundle_id = args.bundle_id or f"b-{args.experiment.lower()}"
    out = Path(args.out) if args.out else Path(".roko/showcase/bundles") / bundle_id
    if out.exists():
        raise BuildError(f"{out} exists: a bundle is written once")
    files: dict[str, bytes] = {
        "data/records.jsonl": jsonl_bytes([redact_record(record) for record in records]),
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

    created_at = args.created_at or dt.datetime.now(dt.UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    billed = [record["costs"].get("billed_usd") for record in records]
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
        "views": list(VIEWS),
        "cost_usd": None if any(amount is None for amount in billed) else round(sum(billed), 6),
        "redaction": dict(REDACTION),
        "files": [
            {"path": path, "schema": schemas[path], "sha256": sha256_hex(data), "rows": rows[path]}
            for path, data in sorted(files.items())
        ],
        "reproduce": [f"vb run --experiment {args.experiment}", f"vb report --experiment {args.experiment}"],
    }
    views = project_views(manifest, metrics, files["data/metrics.jsonl"])

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
    parser.add_argument("--mechanism", help="a directory of S01 mechanism records, <kind>.jsonl")
    parser.add_argument("--timeline", help="a JSONL file of showcase-event/1 rows")
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
