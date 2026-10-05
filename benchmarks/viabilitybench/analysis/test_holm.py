"""Tests for S09 verdict records (holm.py's claim_state, verdict_records and write_verdicts; gap-2da8ec).

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/analysis/test_holm.py -q
"""

from __future__ import annotations

import json

import pytest

import holm

# One primary rejected (H1, via its first level), one not (H2); every other primary absent from `p` entirely
# would be an error (decide() needs all eleven nodes), so they get an unambiguous non-rejection too.
P = {"H1:l1": 0.001, "H1:l2": 0.5, "H1:l3": 0.5, "H1:l4": 0.5, "H1:l5": 0.5,
     "H2": 0.5, "H3": 0.5, "H4": 0.5, "H5": 0.5, "H6": 0.5, "H7": 0.5}


def test_claim_state_follows_rejection_and_the_adverse_flag():
    result = holm.decide(P, alpha=0.05)
    assert result.supported == {"H1": True, "H2": False, "H3": False, "H4": False, "H5": False, "H6": False,
                                "H7": False}
    assert holm.claim_state("H1", result) == "SUPPORTED"
    # Not rejected: INCONCLUSIVE by default (the conservative reading, module docstring), NOT_SUPPORTED only when
    # the caller also says the point estimate ran the other way.
    assert holm.claim_state("H2", result) == "INCONCLUSIVE"
    assert holm.claim_state("H2", result, adverse=True) == "NOT_SUPPORTED"
    assert holm.claim_state("H2", result, adverse=False) == "INCONCLUSIVE"
    with pytest.raises(ValueError, match="H9"):
        holm.claim_state("H9", result)


def test_verdict_records_cover_every_primary_with_its_links():
    result = holm.decide(P, alpha=0.05)
    records = holm.verdict_records(result, "LOG1", metrics={"H1": ["usd_per_vs", "vs_rate"]},
                                    run_ids={"H1": ["r-1"]}, adverse={"H3": True})
    assert [record["hypothesis"] for record in records] == list(holm.PRIMARIES)
    by_hypothesis = {record["hypothesis"]: record for record in records}
    h1 = by_hypothesis["H1"]
    assert h1["schema_version"] == holm.VERDICT_SCHEMA
    assert (h1["experiment_id"], h1["claim_state"], h1["rejected"], h1["test"]) == ("LOG1", "SUPPORTED", True,
                                                                                    "graphical_holm")
    assert h1["alpha"] == result.alpha and h1["p_adjusted"] == result.adjusted["H1:l1"]
    assert (h1["metrics"], h1["run_ids"]) == (["usd_per_vs", "vs_rate"], ["r-1"])
    # H3 was not rejected, but is marked adverse: NOT_SUPPORTED, not the INCONCLUSIVE default.
    assert by_hypothesis["H3"]["claim_state"] == "NOT_SUPPORTED"
    # A primary named by neither metrics nor run_ids still gets a record, with empty links rather than a KeyError.
    assert (by_hypothesis["H7"]["metrics"], by_hypothesis["H7"]["run_ids"]) == ([], [])
    assert all(state in holm.CLAIM_STATES for state in (record["claim_state"] for record in records))


def test_write_verdicts_writes_a_versioned_document_beside_metrics_json(tmp_path):
    result = holm.decide(P, alpha=0.05)
    path = holm.write_verdicts(tmp_path, "LOG1", result, created_at="2026-10-05T00:00:00Z")
    assert path == tmp_path / holm.VERDICTS_FILE
    document = json.loads(path.read_text(encoding="utf-8"))
    assert (document["schema_version"], document["experiment_id"]) == (holm.VERDICTS_SCHEMA, "LOG1")
    assert document["created_at"] == "2026-10-05T00:00:00Z"
    assert [record["hypothesis"] for record in document["records"]] == list(holm.PRIMARIES)
    assert path.read_text(encoding="utf-8").endswith("\n")

    # A second write overwrites, like report.py's own metrics.json writer.
    again = holm.write_verdicts(tmp_path, "LOG1", result, created_at="2026-10-06T00:00:00Z")
    assert json.loads(again.read_text(encoding="utf-8"))["created_at"] == "2026-10-06T00:00:00Z"
