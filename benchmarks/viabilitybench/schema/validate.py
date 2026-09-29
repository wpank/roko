#!/usr/bin/env python3
"""Validate ViabilityBench records and price snapshots against the schemas in this directory.

Stdlib only. The validator implements exactly the JSON Schema subset the six schemas use (type, required, enum,
const, properties, items, minItems and additionalProperties) and refuses a schema with any other keyword, so no
rule is ever skipped in silence. Equality is JSON's: false is not 0, and true is not 1.

On top of the schemas it checks the honesty rules they cannot express (S08 §4.12, S01 §4.4):

- run records and ledger rows: an unknown cost is null, never 0. Used tokens never cost $0; a null cost goes with
  cost source "unknown" and the other way round; missing usage makes the cost unknown; and a billed API row
  (source provider_usage or estimated) that used tokens cannot bill $0. A subscription bills $0 and says so: a run
  record whose every attempt carries the CLI runner's `cli` block, or a ledger row marked `billed: false`. Either may
  be `estimated` with $0 billed, because a session killed before its `result` event is priced from its streamed
  messages, a partial total (bug-f62293, bug-a49003). A ledger row marked `billed: true` never bills $0 for tokens,
  whatever its source, and one marked `billed: false` bills exactly $0. A row from before the mark keeps the source
  rule;
- run records: `costs.by_class` splits `api_equiv_usd` without changing it. Its known classes add up to no more than
  the total, and to exactly the total when every class is known (which a null total then rules out); no class and no
  queue wait is below 0, and the record's `execution.queue_wait_s` is the sum of its attempts' when all are known;
- price snapshots: slugs are unique, every rate is above 0, and each row's columns follow the schema's order.

The schemas enforce the rest: `simulated` is always false, and every MetricRecord lists at least one run id.

Usage: validate.py KIND FILE...
  KIND is task, feature, run-record, metric-record, price-snapshot or ledger. A .json file holds one document, a
  .jsonl file one per line, and a .toml file a price snapshot. Exit status 0 means every document is valid, 1 that at
  least one is not, 2 a usage error.
"""

from __future__ import annotations

import argparse
import functools
import json
import math
import sys
import tomllib
from pathlib import Path
from typing import Any

SCHEMA_DIR = Path(__file__).resolve().parent
KINDS = ("task", "feature", "run-record", "metric-record", "price-snapshot", "ledger")

KEYWORDS = frozenset({"type", "required", "enum", "const", "properties", "items", "minItems", "additionalProperties"})
ANNOTATIONS = frozenset({"$schema", "$id", "$comment", "title", "description"})
TYPES = frozenset({"null", "boolean", "integer", "number", "string", "array", "object"})
RATE_COLUMNS = ("input", "cache_read", "cache_write_5m", "cache_write_1h", "output")
COST_CLASSES = ("plan", "execute", "retry", "escalate", "integrate")  # costs.by_class (S09 §4.9)
# The disjoint token classes a provider bills; tokens_reasoning is not one, since it sits inside tokens_out.
BILLED_TOKENS = ("tokens_in", "tokens_out", "tokens_cache_read", "tokens_cache_write_5m", "tokens_cache_write_1h")


class SchemaError(ValueError):
    """A schema uses something this validator does not implement."""


def check_schema(schema: Any, path: str = "#") -> None:
    """Raise SchemaError unless `schema` uses only the supported keywords, with well-formed values."""
    if not isinstance(schema, dict):
        raise SchemaError(f"{path}: a schema must be an object")
    unsupported = set(schema) - KEYWORDS - ANNOTATIONS
    if unsupported:
        raise SchemaError(f"{path}: unsupported keyword(s): {', '.join(sorted(unsupported))}")
    types = schema.get("type", [])
    for name in [types] if isinstance(types, str) else types:
        if name not in TYPES:
            raise SchemaError(f"{path}/type: unknown type {name!r}")
    required = schema.get("required", [])
    if not isinstance(required, list) or not all(isinstance(name, str) for name in required):
        raise SchemaError(f"{path}/required: must be a list of field names")
    if not isinstance(schema.get("enum", []), list):
        raise SchemaError(f"{path}/enum: must be a list")
    if not isinstance(schema.get("properties", {}), dict):
        raise SchemaError(f"{path}/properties: must be an object")
    min_items = schema.get("minItems", 0)
    if isinstance(min_items, bool) or not isinstance(min_items, int) or min_items < 0:
        raise SchemaError(f"{path}/minItems: must be a non-negative integer")
    for name, sub in schema.get("properties", {}).items():
        check_schema(sub, f"{path}/properties/{name}")
    if "items" in schema:
        check_schema(schema["items"], f"{path}/items")
    extra = schema.get("additionalProperties", True)
    if isinstance(extra, dict):
        check_schema(extra, f"{path}/additionalProperties")
    elif not isinstance(extra, bool):
        raise SchemaError(f"{path}/additionalProperties: must be a boolean or a schema")


@functools.cache
def load_schema(kind: str) -> dict:
    """The checked schema for `kind`. The dict is shared between callers: do not modify it."""
    if kind not in KINDS:
        raise ValueError(f"unknown kind {kind!r}; expected one of {', '.join(KINDS)}")
    schema = json.loads((SCHEMA_DIR / f"{kind}.schema.json").read_text())
    check_schema(schema)
    return schema


def schema_errors(value: Any, schema: dict, path: str = "$") -> list[str]:
    """Every way `value` breaks `schema`, as "<path>: <problem>" strings; empty when it conforms."""
    if "type" in schema:
        types = [schema["type"]] if isinstance(schema["type"], str) else schema["type"]
        if not any(_is_type(value, name) for name in types):
            return [f"{path}: expected {' or '.join(types)}, got {_type_name(value)}"]
    errors = []
    if "const" in schema and not _same(value, schema["const"]):
        errors.append(f"{path}: must be {json.dumps(schema['const'])}, not {_show(value)}")
    if "enum" in schema and not any(_same(value, option) for option in schema["enum"]):
        errors.append(f"{path}: {_show(value)} is not one of {json.dumps(schema['enum'], ensure_ascii=False)}")
    if isinstance(value, dict):
        for name in schema.get("required", []):
            if name not in value:
                errors.append(f"{path}: missing required field {name!r}")
        properties = schema.get("properties", {})
        extra = schema.get("additionalProperties", True)
        for name, item in value.items():
            if name in properties:
                errors += schema_errors(item, properties[name], _child(path, name))
            elif extra is False:
                errors.append(f"{_child(path, name)}: unexpected field")
            elif isinstance(extra, dict):
                errors += schema_errors(item, extra, _child(path, name))
    if isinstance(value, list):
        if len(value) < schema.get("minItems", 0):
            errors.append(f"{path}: needs at least {schema['minItems']} item(s), has {len(value)}")
        if "items" in schema:
            for index, item in enumerate(value):
                errors += schema_errors(item, schema["items"], f"{path}[{index}]")
    return errors


def invariant_errors(kind: str, doc: dict) -> list[str]:
    """The honesty rules the schema subset cannot express. `doc` must already conform to its schema."""
    if kind == "run-record":
        attempts = doc["execution"]["attempts"]
        cli = bool(attempts) and all(isinstance(attempt.get("cli"), dict) for attempt in attempts)
        return (_cost_errors(doc["costs"], [attempt["usage"] for attempt in attempts], "$.costs",
                             subscription=True if cli else None)
                + _class_errors(doc["costs"], "$.costs") + _wait_errors(doc["execution"], "$.execution"))
    if kind == "ledger":
        marked = doc.get("billed")
        errors = _cost_errors(doc, [doc["usage"]], "$", subscription=None if marked is None else not marked)
        if marked is False and doc["billed_usd"] != 0:
            errors.append(f"$.billed_usd: a subscription row (billed false) bills $0, not {doc['billed_usd']}")
        return errors
    if kind == "price-snapshot":
        return _snapshot_errors(doc)
    return []


def validate(kind: str, doc: Any) -> list[str]:
    """All errors for one document of `kind`; an empty list means it is valid."""
    return schema_errors(doc, load_schema(kind)) or invariant_errors(kind, doc)


def load_documents(path: Path) -> list[tuple[str, Any]]:
    """(location, document) pairs from a .json, .jsonl or .toml file."""
    if path.suffix == ".toml":
        with path.open("rb") as handle:
            return [(str(path), tomllib.load(handle))]
    if path.suffix == ".jsonl":
        lines = path.read_text().splitlines()
        return [(f"{path}:{number}", json.loads(line)) for number, line in enumerate(lines, 1) if line.strip()]
    return [(str(path), json.loads(path.read_text()))]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Validate ViabilityBench records and price snapshots.")
    parser.add_argument("kind", choices=KINDS)
    parser.add_argument("files", nargs="+", type=Path)
    args = parser.parse_args(argv)
    checked = invalid = 0
    for path in args.files:
        try:
            documents = load_documents(path)
        except (OSError, ValueError) as err:  # JSON and TOML decode errors are ValueErrors
            print(f"{path}: {err}")
            invalid += 1
            continue
        for location, doc in documents:
            errors = validate(args.kind, doc)
            for error in errors:
                print(f"{location}: {error}")
            checked += 1
            invalid += bool(errors)
    print(f"{args.kind}: {checked} document(s) checked, {invalid} invalid", file=sys.stderr)
    return 1 if invalid else 0


def _cost_errors(cost: dict, usages: list, path: str, *, subscription: bool | None = None) -> list[str]:
    """`subscription` is True or False when the document says whether it was billed, and None when only its source
    can tell (module docstring)."""
    source = cost["source"]
    tokens = sum(usage.get(key, 0) for usage in usages if usage for key in BILLED_TOKENS)
    errors = []
    if any(usage is None for usage in usages) and source != "unknown":
        errors.append(f"{path}.source: usage was not reported, so the cost is unknown; source must be \"unknown\", "
                      f"not {source!r}")
    for field in ("api_equiv_usd", "without_cache_usd"):
        if field not in cost:
            continue
        amount = cost[field]
        if amount is None and source != "unknown":
            errors.append(f"{path}.{field}: null means the cost is unknown, but source is {source!r}")
        elif amount is not None and source == "unknown":
            errors.append(f"{path}.{field}: an unknown cost is null, not {amount}")
        elif amount == 0 and tokens:
            errors.append(f"{path}.{field}: $0 for {tokens} tokens; an unknown cost is null, never 0")
    if subscription is None:
        billed = source in ("provider_usage", "estimated")
    else:  # a subscription may estimate its cost, but usage a provider reported was billed by that provider
        billed = not subscription or source == "provider_usage"
    if cost["billed_usd"] == 0 and tokens and billed:
        errors.append(f"{path}.billed_usd: $0 billed for {tokens} tokens; only a subscription run bills $0")
    return errors


def _class_errors(cost: dict, path: str) -> list[str]:
    by_class = cost.get("by_class")
    if by_class is None:
        return []
    amounts = [by_class[name] for name in COST_CLASSES]
    errors = [f"{path}.by_class.{name}: a cost cannot be below 0, not {amount}"
              for name, amount in zip(COST_CLASSES, amounts) if amount is not None and amount < 0]
    total, known = cost["api_equiv_usd"], [amount for amount in amounts if amount is not None]
    tolerance = 1e-9 * max(1.0, abs(total or 0.0))
    if len(known) == len(amounts) and total is None:
        errors.append(f"{path}.by_class: every class is known, so api_equiv_usd cannot be unknown")
    elif len(known) == len(amounts) and abs(math.fsum(known) - total) > tolerance:
        errors.append(f"{path}.by_class: the classes add up to {math.fsum(known)}, not api_equiv_usd {total}")
    elif total is not None and math.fsum(known) > total + tolerance:
        errors.append(f"{path}.by_class: the known classes add up to {math.fsum(known)}, more than api_equiv_usd "
                      f"{total}")
    return errors


def _wait_errors(execution: dict, path: str) -> list[str]:
    waits = [(f"{path}.attempts[{index}].queue_wait_s", attempt["queue_wait_s"])
             for index, attempt in enumerate(execution["attempts"]) if "queue_wait_s" in attempt]
    total = execution.get("queue_wait_s")
    errors = [f"{where}: a wait cannot be below 0, not {wait}"
              for where, wait in [*waits, (f"{path}.queue_wait_s", total)] if wait is not None and wait < 0]
    if total is not None and waits and len(waits) == len(execution["attempts"]) and all(
            wait is not None for _, wait in waits):
        summed = math.fsum(wait for _, wait in waits)
        if abs(summed - total) > 1e-6:
            errors.append(f"{path}.queue_wait_s: {total}, but the attempts' waits add up to {summed}")
    return errors


def _snapshot_errors(snapshot: dict) -> list[str]:
    columns = list(load_schema("price-snapshot")["properties"]["model"]["items"]["properties"])
    errors = []
    seen = set()
    for index, row in enumerate(snapshot["model"]):
        path = f"$.model[{index}]"
        if row["slug"] in seen:
            errors.append(f"{path}.slug: duplicate slug {row['slug']!r}")
        seen.add(row["slug"])
        expected = [column for column in columns if column in row]
        if list(row) != expected:
            errors.append(f"{path}: columns out of order; expected {', '.join(expected)}")
        for column in RATE_COLUMNS:
            if row[column] <= 0:
                errors.append(f"{path}.{column}: a rate must be above 0, or real tokens would cost $0")
    return errors


def _is_type(value: Any, name: str) -> bool:
    if name == "null":
        return value is None
    if name == "boolean":
        return isinstance(value, bool)
    if isinstance(value, bool):  # a bool is never a number in JSON
        return False
    if name == "integer":
        return isinstance(value, int) or (isinstance(value, float) and value.is_integer())
    if name == "number":
        return isinstance(value, int) or (isinstance(value, float) and math.isfinite(value))
    if name == "string":
        return isinstance(value, str)
    if name == "array":
        return isinstance(value, list)
    return isinstance(value, dict)


def _same(a: Any, b: Any) -> bool:
    """JSON equality: false is not 0, true is not 1, and 1 equals 1.0."""
    if isinstance(a, bool) or isinstance(b, bool):
        return type(a) is type(b) and a == b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return a == b
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(map(_same, a, b))
    if isinstance(a, dict) and isinstance(b, dict):
        return a.keys() == b.keys() and all(_same(a[key], b[key]) for key in a)
    return type(a) is type(b) and a == b


def _type_name(value: Any) -> str:
    for name in ("null", "boolean", "integer", "number", "string", "array", "object"):
        if _is_type(value, name):
            return name
    return type(value).__name__


def _show(value: Any) -> str:
    try:
        return json.dumps(value, ensure_ascii=False)
    except (TypeError, ValueError):
        return repr(value)


def _child(path: str, key: str) -> str:
    return f"{path}.{key}" if key.isidentifier() else f"{path}[{json.dumps(key, ensure_ascii=False)}]"


if __name__ == "__main__":
    sys.exit(main())
