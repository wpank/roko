"""Tests for the showcase bundle builder and verifier (task 9314), on the fixture records in `fixtures/results/`."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest

SHOWCASE_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SHOWCASE_DIR))

import build_bundle  # noqa: E402
import verify_bundle  # noqa: E402

FIXTURES = SHOWCASE_DIR / "fixtures" / "results"


def build(tmp_path: Path) -> Path:
    out = tmp_path / "b-fixture-p1"
    argv = [
        "--experiment", "FIXTURE-P1",
        "--results", str(FIXTURES),
        "--out", str(out),
        "--bundle-id", "b-fixture-p1",
        "--title", "Fixture bundle",
        "--created-at", "2026-10-04T00:00:00Z",
    ]
    assert build_bundle.main(argv) == 0
    return out


def rules(bundle: Path) -> set[str]:
    return {rule for rule, _ in verify_bundle.check(bundle)}


def rows(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def write_rows(path: Path, documents: list[dict]) -> None:
    path.write_bytes(build_bundle.jsonl_bytes(documents))


def edit_manifest(bundle: Path, **changes) -> None:
    manifest = json.loads((bundle / "bundle.json").read_text(encoding="utf-8"))
    manifest.update(changes)
    (bundle / "bundle.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def test_fixture_bundle_builds_and_verifies(tmp_path: Path) -> None:
    bundle = build(tmp_path)
    assert verify_bundle.check(bundle) == []
    assert verify_bundle.main([str(bundle)]) == 0

    manifest = json.loads((bundle / "bundle.json").read_text(encoding="utf-8"))
    assert manifest["kind"] == "replay" and manifest["simulated"] is False
    assert manifest["run_ids"] == ["vb-fixture-f1", "vb-fixture-r1", "vb-fixture-r2"]
    assert manifest["views"] == ["overview", "p1-head-to-head", "m4-audits"]

    # Redacted: no transcript, the prompt and the failed hidden checks only as hashes.
    records = rows(bundle / "data" / "records.jsonl")
    assert all(record["provenance"]["transcript_ref"] is None for record in records)
    prompt = "Implement the partial-failure rollback in store_state."
    assert all(record["task"]["prompt"] == build_bundle.hashed(prompt) for record in records)
    failed = [name for record in records for name in record["vs"]["failed"]]
    assert failed == [build_bundle.hashed("store_state.partial_failure")]

    # The views copy MetricRecord values; they compute none.
    head_to_head = json.loads((bundle / "views" / "p1-head-to-head.json").read_text(encoding="utf-8"))
    arms = {arm["arm"]: arm for arm in head_to_head["arms"]}
    assert arms["roko_fixed"]["resolve"]["value"] == 0.5
    assert arms["roko_fixed"]["resolve"]["ci"] == [0.0945, 0.9055]
    assert arms["fd_claude"]["usd_per_verified"]["value"] == 0.241
    assert head_to_head["provenance"]["kind"] == "replay"


def test_a_written_bundle_is_never_overwritten(tmp_path: Path) -> None:
    bundle = build(tmp_path)
    argv = ["--experiment", "FIXTURE-P1", "--results", str(FIXTURES), "--out", str(bundle)]
    assert build_bundle.main(argv) == 1


def poison_simulated(bundle: Path) -> None:
    edit_manifest(bundle, simulated=True)


def poison_fixture_kind(bundle: Path) -> None:
    edit_manifest(bundle, kind="fixture")


def poison_missing_run_ids(bundle: Path) -> None:
    metrics_path = bundle / "data" / "metrics.jsonl"
    metrics = rows(metrics_path)
    metrics[0]["run_ids"] = []
    write_rows(metrics_path, metrics)


def poison_transcript(bundle: Path) -> None:
    (bundle / "transcripts").mkdir()
    (bundle / "transcripts" / "vb-fixture-r1.jsonl").write_text('{"role":"user"}\n', encoding="utf-8")


def poison_plaintext_hidden_test(bundle: Path) -> None:
    records_path = bundle / "data" / "records.jsonl"
    records = rows(records_path)
    for record in records:
        if record["vs"]["failed"]:
            record["vs"]["failed"] = ["store_state.partial_failure"]
    write_rows(records_path, records)


def poison_hand_edited_view(bundle: Path) -> None:
    view = bundle / "views" / "overview.json"
    view.write_bytes(view.read_bytes().replace(b'"value":0.5', b'"value":0.9', 1))


@pytest.mark.parametrize(
    ("poison", "rule"),
    [
        (poison_simulated, "simulated"),
        (poison_fixture_kind, "kind"),
        (poison_missing_run_ids, "run_ids"),
        (poison_transcript, "transcripts"),
        (poison_plaintext_hidden_test, "hidden"),
        (poison_hand_edited_view, "views"),
    ],
)
def test_each_poisoned_bundle_fails_its_rule(tmp_path: Path, poison, rule: str) -> None:
    """SHA256SUMS is rewritten after the poison, so the bundle fails the rule under test, not only its checksums."""
    bundle = build(tmp_path)
    poison(bundle)
    build_bundle.write_sums(bundle)
    assert rule in rules(bundle)
    assert verify_bundle.main([str(bundle)]) == 1


def test_a_tampered_file_fails_the_checksums(tmp_path: Path) -> None:
    bundle = build(tmp_path)
    with (bundle / "data" / "records.jsonl").open("a", encoding="utf-8") as records:
        records.write("\n")
    assert rules(bundle) == {"checksums"}
    assert verify_bundle.main([str(bundle)]) == 1
