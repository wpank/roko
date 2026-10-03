#!/usr/bin/env python3
"""Verify a showcase replay bundle, `showcase-bundle/1` (S10 §5.5, task 9314), before it is staged or uploaded.

    verify_bundle.py BUNDLE_DIR

A bundle passes only when every rule holds:

- `files`: `bundle.json` and `SHA256SUMS` are there, and `bundle.json` is a `showcase-bundle/1` manifest;
- `checksums`: `SHA256SUMS` lists every other file, each with its SHA-256, and nothing else;
- `kind`: the manifest's `kind` is `replay`;
- `simulated`: every `simulated` field in every file is `false`, and the manifest has one;
- `records`: every run record passes `schema/validate.py` and names a run of the bundle;
- `metrics`: every MetricRecord passes `schema/validate.py`;
- `run_ids`: every MetricRecord lists run ids, each a run of the bundle;
- `views`: every view the manifest names is there, re-derived from `data/metrics.jsonl` byte for byte, and no other
  view is;
- `transcripts`: no transcript file and no transcript field with a value;
- `hidden`: every prompt and hidden-test value, the names of failed truth-suite checks included, is `sha256:<hex>`;
- `timeline`: when `timeline/events.jsonl` is there, its rows are `showcase-event/1` replay events of the bundle's
  runs, by increasing `seq`.

Problems print one per line as `<rule>: <detail>`. Exit status: 0 when the bundle passes, 1 when a rule fails, 2 on a
usage error. `build_bundle.py` writes bundles that pass; the server loader (9324) applies the same rules.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any, Iterator

import build_bundle
from build_bundle import (
    BUNDLE_SCHEMA,
    EVENT_SCHEMA,
    HASHED,
    is_hidden_key,
    is_prompt_key,
    project_views,
    sha256_hex,
    validate,
)

SUM_LINE = re.compile(r"^([0-9a-f]{64})  (.+)$")


def walk(value: Any, key: str = "") -> Iterator[tuple[str, Any]]:
    """Every (key, value) pair in `value`, depth first, with the key each value sits under."""
    yield key, value
    if isinstance(value, dict):
        for child, item in value.items():
            yield from walk(item, child)
    elif isinstance(value, list):
        for item in value:
            yield from walk(item, key)


def json_documents(bundle: Path) -> Iterator[tuple[str, Any]]:
    """Every JSON document in `bundle`, by its path: `.json` files whole, `.jsonl` files a row at a time."""
    for path in sorted(bundle.rglob("*")):
        relative = path.relative_to(bundle).as_posix()
        if not path.is_file():
            continue
        if path.suffix == ".json":
            try:
                yield relative, json.loads(path.read_text(encoding="utf-8"))
            except (UnicodeDecodeError, json.JSONDecodeError):
                yield relative, None
        elif path.suffix == ".jsonl":
            for number, line in enumerate(path.read_text(encoding="utf-8", errors="replace").splitlines(), 1):
                if line.strip():
                    try:
                        yield f"{relative}:{number}", json.loads(line)
                    except json.JSONDecodeError:
                        yield f"{relative}:{number}", None


def read_rows(bundle: Path, path: str, problems: list[tuple[str, str]], rule: str) -> list[dict]:
    file = bundle / path
    if not file.is_file():
        problems.append((rule, f"{path} is missing"))
        return []
    rows = []
    for number, line in enumerate(file.read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError as error:
            problems.append((rule, f"{path}:{number} is not JSON: {error}"))
    return rows


def check_sums(bundle: Path, problems: list[tuple[str, str]]) -> None:
    listed: dict[str, str] = {}
    for number, line in enumerate((bundle / "SHA256SUMS").read_text(encoding="utf-8").splitlines(), start=1):
        match = SUM_LINE.match(line)
        if not match:
            problems.append(("checksums", f"SHA256SUMS line {number} is not `<sha256>  <path>`"))
            continue
        listed[match.group(2)] = match.group(1)
    present = {
        path.relative_to(bundle).as_posix()
        for path in bundle.rglob("*")
        if path.is_file() and path.name != "SHA256SUMS"
    }
    for path in sorted(present - set(listed)):
        problems.append(("checksums", f"{path} is not in SHA256SUMS"))
    for path, expected in sorted(listed.items()):
        file = bundle / path
        if path not in present:
            problems.append(("checksums", f"{path} is in SHA256SUMS but missing"))
        elif sha256_hex(file.read_bytes()) != expected:
            problems.append(("checksums", f"{path} does not match its SHA256SUMS line"))


def check(bundle: Path) -> list[tuple[str, str]]:
    """The rules `bundle` breaks, as (rule, detail) pairs; none for a bundle that passes."""
    problems: list[tuple[str, str]] = []
    manifest_path = bundle / "bundle.json"
    if not manifest_path.is_file() or not (bundle / "SHA256SUMS").is_file():
        return [("files", "bundle.json and SHA256SUMS must both be there")]
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as error:
        return [("files", f"bundle.json is not JSON: {error}")]
    if not isinstance(manifest, dict) or manifest.get("schema") != BUNDLE_SCHEMA:
        return [("files", f"bundle.json is not a {BUNDLE_SCHEMA} manifest")]
    check_sums(bundle, problems)

    if manifest.get("kind") != "replay":
        problems.append(("kind", f"the bundle's kind is {manifest.get('kind')!r}, not 'replay'"))
    if manifest.get("simulated") is not False:
        problems.append(("simulated", "bundle.json must say simulated: false"))
    for where, document in json_documents(bundle):
        if document is None:
            problems.append(("files", f"{where} is not JSON"))
            continue
        if where == "bundle.json":
            # The manifest's `redaction` names what was redacted, and how.
            document = {key: value for key, value in document.items() if key != "redaction"}
        for key, value in walk(document):
            if key == "simulated" and value is not False:
                problems.append(("simulated", f"{where} has simulated: {json.dumps(value)}"))
            elif "transcript" in key.lower() and value is not None:
                problems.append(("transcripts", f"{where} carries a {key} value"))
            elif (is_prompt_key(key) or is_hidden_key(key)) and isinstance(value, str) and not HASHED.match(value):
                problems.append(("hidden", f"{where} has a plaintext {key} value"))
    for path in sorted(bundle.rglob("*")):
        relative = path.relative_to(bundle).as_posix()
        if any("transcript" in part.lower() for part in Path(relative).parts):
            problems.append(("transcripts", f"{relative} is a transcript"))

    run_ids = set(manifest.get("run_ids") or [])
    for number, record in enumerate(read_rows(bundle, "data/records.jsonl", problems, "records"), start=1):
        for error in validate.validate("run-record", record)[:1]:
            problems.append(("records", f"run record {number}: {error}"))
        if record.get("run_id") not in run_ids:
            problems.append(("records", f"run record {number} names a run the bundle does not list"))
        for name in (record.get("vs") or {}).get("failed") or []:
            if not (isinstance(name, str) and HASHED.match(name)):
                problems.append(("hidden", f"run record {number} names a failed hidden check in plaintext"))

    metrics = read_rows(bundle, "data/metrics.jsonl", problems, "metrics")
    for number, metric in enumerate(metrics, start=1):
        for error in validate.validate("metric-record", metric)[:1]:
            problems.append(("metrics", f"MetricRecord {number}: {error}"))
        ids = metric.get("run_ids") if isinstance(metric, dict) else None
        if not ids:
            problems.append(("run_ids", f"MetricRecord {number} lists no run ids"))
        elif not set(ids) <= run_ids:
            problems.append(("run_ids", f"MetricRecord {number} names runs the bundle does not list"))

    metrics_file = bundle / "data/metrics.jsonl"
    if metrics_file.is_file():
        expected = project_views(manifest, metrics, metrics_file.read_bytes())
        named = list(manifest.get("views") or [])
        for name in named:
            view = bundle / "views" / f"{name}.json"
            if name not in expected:
                problems.append(("views", f"the manifest names a view no projection makes: {name}"))
            elif not view.is_file():
                problems.append(("views", f"views/{name}.json is missing"))
            elif view.read_bytes() != expected[name]:
                problems.append(("views", f"views/{name}.json is not what data/metrics.jsonl projects"))
        if (bundle / "views").is_dir():
            for view in sorted((bundle / "views").iterdir()):
                if view.stem not in named:
                    problems.append(("views", f"views/{view.name} is not a view the manifest names"))

    if (bundle / "timeline").exists():
        last = None
        for number, event in enumerate(read_rows(bundle, "timeline/events.jsonl", problems, "timeline"), 1):
            if event.get("schema") != EVENT_SCHEMA or event.get("source") != "replay":
                problems.append(("timeline", f"event {number} is not a {EVENT_SCHEMA} replay event"))
            if event.get("run_id") not in run_ids:
                problems.append(("timeline", f"event {number} names a run the bundle does not list"))
            seq = event.get("seq")
            if not isinstance(seq, int) or (last is not None and seq <= last):
                problems.append(("timeline", f"event {number} does not follow the one before it in seq"))
            last = seq if isinstance(seq, int) else last

    for entry in manifest.get("files") or []:
        file = bundle / str(entry.get("path"))
        if not file.is_file() or sha256_hex(file.read_bytes()) != entry.get("sha256"):
            problems.append(("checksums", f"bundle.json's entry for {entry.get('path')} does not match the file"))
    return problems


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    if len(args) != 1:
        print(__doc__.split("\n\n")[1].strip(), file=sys.stderr)
        return 2
    bundle = Path(args[0])
    if not bundle.is_dir():
        print(f"no bundle directory {bundle}", file=sys.stderr)
        return 2
    problems = check(bundle)
    for rule, detail in problems:
        print(f"{rule}: {detail}")
    if not problems:
        print(f"ok: {build_bundle.BUNDLE_SCHEMA} {bundle}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
