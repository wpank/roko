#!/usr/bin/env python3
"""Tests for the ViabilityBench schemas, their validator and the price snapshot.

Run: benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/schema/test_schemas.py
"""

from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent))
import validate  # noqa: E402

HERE = Path(__file__).resolve().parent
SNAPSHOT = HERE.parents[2] / "config" / "prices" / "2026-09-28.toml"
# S08 §5.6: every row has these columns, in this order, plus an optional note.
COLUMNS = ["slug", "provider", "input", "cache_read", "cache_write_5m", "cache_write_1h", "output",
           "reasoning_in_output", "source_url", "verified"]
S08_SLUGS = {"gpt-oss-120b", "glm-4.7", "kimi-k2.6", "gpt-5.4-mini", "gpt-5.4", "gpt-5.5", "claude-opus-5-5",
             "claude-sonnet-5", "claude-fable-5-1", "claude-haiku-4-5"}


class Delete:
    def __repr__(self) -> str:
        return "DELETE"


DELETE = Delete()

# (kind, dotted path, new value, a fragment of the error it must cause). The first three are S08's honesty
# invariants named by the work item: simulated is false, tokens never cost $0, and every metric lists run ids.
MUTANTS = [
    ("run-record", "simulated", True, "$.simulated: must be false"),
    ("run-record", "costs.api_equiv_usd", 0, "$.costs.api_equiv_usd: $0 for 63444 tokens"),
    ("metric-record", "run_ids", [], "$.run_ids: needs at least 1 item(s), has 0"),
    ("ledger", "api_equiv_usd", 0.0, "$.api_equiv_usd: $0 for 63444 tokens"),
    ("run-record", "costs.without_cache_usd", 0, "$.costs.without_cache_usd: $0 for"),
    ("run-record", "simulated", DELETE, "missing required field 'simulated'"),
    ("run-record", "simulated", 0, "$.simulated: must be false, not 0"),
    ("run-record", "costs.source", "unknown", "an unknown cost is null, not 0.0231"),
    ("run-record", "costs.api_equiv_usd", None, "null means the cost is unknown, but source is 'provider_usage'"),
    ("run-record", "execution.attempts.0.usage", None, "usage was not reported, so the cost is unknown"),
    ("run-record", "costs.source", "mock", "$.costs.source: \"mock\" is not one of"),
    ("run-record", "costs.provider_billed_usd", 0.0231, "$.costs.provider_billed_usd: unexpected field"),
    ("run-record", "vs.label", True, "$.vs.label: true is not one of"),
    ("run-record", "execution.attempts.0.usage.tokens_in", 61234.5, "expected integer, got number"),
    ("ledger", "billed_usd", 0, "$.billed_usd: $0 billed for 63444 tokens"),
    ("ledger", "reserved_usd", DELETE, "missing required field 'reserved_usd'"),
    ("task", "recoverability", [], "$.recoverability: needs at least 1 item(s)"),
    ("task", "spec.vague.operator", DELETE, "$.spec.vague: missing required field 'operator'"),
    ("metric-record", "label_source", "visible", "$.label_source: \"visible\" is not one of"),
    ("feature", "run_record_task.ladder", 5, "$.run_record_task.ladder: must be null, not 5"),
    ("feature", "family", "F1", '$.family: must be "PL", not "F1"'),
    ("feature", "canary", DELETE, "missing required field 'canary'"),
    ("run-record", "task.ladder", "plan", '$.task.ladder: "plan" is not one of [1, 2, 3, 4, 5, null]'),
    # S09 §4.9's per-class costs split api_equiv_usd (0.0231) without changing it; a queue wait is never below 0.
    ("run-record", "costs.by_class", {"plan": 0.0, "execute": 0.02, "retry": 0.0, "escalate": 0.0, "integrate": 0.0},
     "$.costs.by_class: the classes add up to 0.02, not api_equiv_usd 0.0231"),
    ("run-record", "costs.by_class", {"plan": 0.03, "execute": None, "retry": 0.0, "escalate": 0.0, "integrate": 0.0},
     "the known classes add up to 0.03, more than api_equiv_usd 0.0231"),
    ("run-record", "costs.by_class", {"plan": 0.0, "execute": 0.0231}, "missing required field 'retry'"),
    ("run-record", "execution.queue_wait_s", -1.0, "$.execution.queue_wait_s: a wait cannot be below 0"),
]


def example(kind: str) -> dict:
    return json.loads((HERE / "examples" / f"{kind}.json").read_text())


def load_snapshot() -> dict:
    with SNAPSHOT.open("rb") as handle:
        return tomllib.load(handle)


def mutate(doc: dict, path: str, value: object) -> None:
    *parents, last = path.split(".")
    for key in parents:
        doc = doc[int(key)] if isinstance(doc, list) else doc[key]
    if value is DELETE:
        del doc[last]
    else:
        doc[last] = value


@pytest.mark.parametrize(("kind", "path", "value", "expected"), MUTANTS,
                         ids=[f"{kind}:{path}={value!r}" for kind, path, value, _ in MUTANTS])
def test_spec_examples_validate_and_mutants_fail(kind, path, value, expected):
    doc = example(kind)
    assert validate.validate(kind, doc) == [], f"the S08 example for {kind} must validate"
    mutate(doc, path, value)
    errors = validate.validate(kind, doc)
    assert any(expected in error for error in errors), errors


def test_a_plan_slice_row_validates_without_a_placeholder_ladder():
    # A plan-slice feature has no ladder level (S09 §4.9), so its run records carry ladder = null, never a level.
    sys.path.insert(0, str(HERE.parent / "families" / "plan_slice"))
    import slicekit
    features = slicekit.load_features()
    assert features and all("ladder" not in feature.source["run_record"] for feature in features)
    tasks = [example("feature")["run_record_task"], *(slicekit.run_record_task(feature, 1) for feature in features)]
    for task in tasks:
        assert task["family"] == "PL" and task["ladder"] is None, task
        for arm in ("roko_plan", "fd_claude"):  # the slice's two arms
            record = {**example("run-record"), "arm": arm, "task": task}
            assert validate.validate("run-record", record) == [], (arm, task["instance_id"])
    for ladder, valid in ((None, True), (3, True), (0, False), (6, False), ("plan", False)):
        record = {**example("run-record"), "task": {**tasks[0], "ladder": ladder}}
        assert (validate.validate("run-record", record) == []) is valid, ladder


def test_price_snapshot_rows_have_every_column():
    snapshot = load_snapshot()
    assert validate.validate("price-snapshot", snapshot) == []
    assert snapshot["id"] == f"prices-{SNAPSHOT.stem}" == f"prices-{snapshot['fetched_at']}"
    assert {row["slug"] for row in snapshot["model"]} == S08_SLUGS
    for row in snapshot["model"]:
        assert list(row) in (COLUMNS, [*COLUMNS, "note"]), row["slug"]


@pytest.mark.parametrize(("change", "expected"), [
    (lambda rows: rows[0].pop("reasoning_in_output"), "missing required field 'reasoning_in_output'"),
    (lambda rows: rows[0].update(cached_input=0.1), "$.model[0].cached_input: unexpected field"),
    (lambda rows: rows[1].update(slug=rows[0]["slug"]), "$.model[1].slug: duplicate slug"),
    (lambda rows: rows[2].update(cache_read=0.0), "$.model[2].cache_read: a rate must be above 0"),
    (lambda rows: rows.__setitem__(0, dict(reversed(rows[0].items()))), "$.model[0]: columns out of order"),
], ids=["missing-column", "extra-column", "duplicate-slug", "zero-rate", "column-order"])
def test_price_snapshot_mutants_fail(change, expected):
    snapshot = load_snapshot()
    change(snapshot["model"])
    errors = validate.validate("price-snapshot", snapshot)
    assert any(expected in error for error in errors), errors


def test_schemas_use_only_supported_keywords():
    for kind in validate.KINDS:
        validate.load_schema(kind)  # raises SchemaError on any keyword the validator does not implement
    with pytest.raises(validate.SchemaError, match="unsupported keyword.*pattern"):
        validate.check_schema({"type": "object", "properties": {"id": {"type": "string", "pattern": "^b3:"}}})


@pytest.mark.parametrize(("schema", "value", "valid"), [
    ({"type": "integer"}, True, False),
    ({"type": "integer"}, 3.0, True),
    ({"type": "integer"}, 3.5, False),
    ({"type": "number"}, float("nan"), False),
    ({"type": "number"}, float("inf"), False),
    ({"type": ["number", "null"]}, None, True),
    ({"const": False}, 0, False),
    ({"enum": [0, 1]}, False, False),
    ({"enum": [0, 1]}, 1.0, True),
    ({"type": "object", "properties": {"a": {}}, "additionalProperties": False}, {"a": 1, "b": 2}, False),
    ({"type": "object", "additionalProperties": {"type": "string"}}, {"a": "x", "b": 2}, False),
    ({"type": ["object", "null"], "required": ["a"]}, None, True),
    ({"type": "array", "items": {"type": "string"}, "minItems": 1}, [], False),
])
def test_subset_follows_json_schema_semantics(schema, value, valid):
    validate.check_schema(schema)
    assert (validate.schema_errors(value, schema) == []) is valid


def test_cli_checks_json_jsonl_and_toml(tmp_path, capsys):
    good = example("run-record")
    records = tmp_path / "records.jsonl"
    records.write_text(json.dumps(good) + "\n" + json.dumps({**good, "simulated": True}) + "\n")
    assert validate.main(["price-snapshot", str(SNAPSHOT)]) == 0
    assert validate.main(["task", str(HERE / "examples" / "task.json")]) == 0
    assert validate.main(["feature", str(HERE / "examples" / "feature.json")]) == 0
    assert validate.main(["run-record", str(records)]) == 1
    assert f"{records}:2: $.simulated: must be false, not true" in capsys.readouterr().out
