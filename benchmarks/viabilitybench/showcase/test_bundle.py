"""Tests for the showcase bundle builder and verifier (task 9314), on the fixture records in `fixtures/results/`."""

from __future__ import annotations

import json
import shutil
import sys
import tomllib
from pathlib import Path

import pytest

SHOWCASE_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(SHOWCASE_DIR))

import build_bundle  # noqa: E402
import verify_bundle  # noqa: E402
from build_bundle import validate  # noqa: E402

FIXTURES = SHOWCASE_DIR / "fixtures" / "results"
# The page's JSON Schemas for the views (`contracts.ts` in JSON Schema form).
CONTRACTS = SHOWCASE_DIR.parents[2] / "demo" / "demo-app" / "src" / "showcase" / "schemas"


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


def view(bundle: Path, name: str) -> dict:
    return json.loads((bundle / "views" / f"{name}.json").read_text(encoding="utf-8"))


def contract(name: str) -> dict:
    """The page's JSON Schema for `name`: a view id, or `bundle` for the manifest."""
    return json.loads((CONTRACTS / f"{name}.schema.json").read_text(encoding="utf-8"))


def copied_results(tmp_path: Path) -> tuple[Path, Path, dict]:
    """A copy of the fixture results to change, its metrics.json, and that file's document."""
    results = tmp_path / "results"
    shutil.copytree(FIXTURES, results)
    metrics_path = results / "FIXTURE-P1" / "metrics.json"
    return results, metrics_path, json.loads(metrics_path.read_text(encoding="utf-8"))


def build_from(results: Path, out: Path) -> int:
    return build_bundle.main(["--experiment", "FIXTURE-P1", "--results", str(results), "--out", str(out)])


def rows(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def write_rows(path: Path, documents: list[dict]) -> None:
    path.write_bytes(build_bundle.jsonl_bytes(documents))


def edit_manifest(bundle: Path, **changes) -> None:
    manifest = json.loads((bundle / "bundle.json").read_text(encoding="utf-8"))
    manifest.update(changes)
    (bundle / "bundle.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def shown_refs(node: object) -> set[str]:
    """Every `metric_ref` a view displays, found as guard.ts's `displayedRefs` finds them: `metrics` and
    `provenance` are skipped."""
    found: set[str] = set()
    if isinstance(node, list):
        for item in node:
            found |= shown_refs(item)
    elif isinstance(node, dict):
        if isinstance(node.get("metric_ref"), str):
            found.add(node["metric_ref"])
        for key, value in node.items():
            if key not in ("metrics", "provenance"):
                found |= shown_refs(value)
    return found


def guard_refusals(view: dict) -> list[str]:
    """Why guard.ts's `refusalOf` would refuse the numbers of `view`: a `metrics` entry without n >= 1, a rate with
    neither a ci nor ci_method "none", or a displayed `metric_ref` that is not in `metrics`."""
    refusals = []
    for entry in view.get("metrics", []):
        if not (isinstance(entry.get("n"), int) and entry["n"] >= 1):
            refusals.append(f"n_missing: {entry.get('metric_ref')} has no sample size")
        if entry.get("kind") == "rate" and not entry.get("ci") and entry.get("ci_method") != "none":
            refusals.append(f"ci_missing: rate {entry.get('metric_ref')} has no interval")
    indexed = {entry.get("metric_ref") for entry in view.get("metrics", [])}
    refusals += [f"n_missing: {ref} is not among the view's metrics" for ref in sorted(shown_refs(view) - indexed)]
    return refusals


def test_fixture_bundle_builds_and_verifies(tmp_path: Path) -> None:
    bundle = build(tmp_path)
    assert verify_bundle.check(bundle) == []
    assert verify_bundle.main([str(bundle)]) == 0

    manifest = json.loads((bundle / "bundle.json").read_text(encoding="utf-8"))
    assert manifest["kind"] == "replay" and manifest["simulated"] is False
    assert manifest["run_ids"] == ["vb-fixture-f1", "vb-fixture-r1", "vb-fixture-r2"]
    assert manifest["views"] == ["overview", "p1-head-to-head"]

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


def test_every_number_a_view_shows_is_in_its_metrics_index(tmp_path: Path) -> None:
    """The page draws a view only when every number in it is in its `metrics` index with n >= 1, and every rate has
    an interval or ci_method "none" (guard.ts). Without the index, the guard refused every view of a built bundle."""
    bundle = build(tmp_path)
    records = {build_bundle.metric_ref(record): record for record in rows(bundle / "data" / "metrics.jsonl")}
    manifest = json.loads((bundle / "bundle.json").read_text(encoding="utf-8"))
    for name in manifest["views"]:
        document = view(bundle, name)
        assert shown_refs(document), f"{name} shows no number"
        assert guard_refusals(document) == [], name
        # Each entry has the fields of contracts.ts's ViewMetric, copied from its MetricRecord.
        index_schema = contract(name)["properties"]["metrics"]
        assert validate.schema_errors(document["metrics"], index_schema) == [], name
        for entry in document["metrics"]:
            record = records[entry["metric_ref"]]
            copied = {key: record.get(key) for key in ("metric", "value", "ci", "ci_method", "n", "estimator")}
            assert {key: entry[key] for key in copied} == copied
            assert entry["arm"] == (record["arms"][0] if len(record["arms"]) == 1 else None)


def test_metric_kinds_follow_the_metric_names() -> None:
    """The guard asks every rate for an interval, so a metric's kind comes from its S08 name."""
    names = {
        "vs_rate": "rate",
        "vs_rate_unknown_as_1": "rate",
        "pass_hat_3": "rate",
        "false_green_rate": "rate",
        "honest_conflict_rate": "rate",
        "usd_per_vs": "usd",
        "usd_per_vs_vendor": "usd",
        "spend_usd": "usd",
        "false_greens": "count",
        "cap_censored_runs": "count",
        "pl_verified_features": "count",
        "cost_gap_ur": "ratio",
        "pl_makespan_median_s": "score",
    }
    assert {name: build_bundle.metric_kind(name) for name in names} == names


def test_overview_tiles_follow_the_claims_board(tmp_path: Path) -> None:
    """The Overview draws only claims-board tiles (contracts.ts `OverviewTile`), so a built bundle showed no tile:
    the tiles are the R1 catalogue's, in the page's schema, with rows copied by metric_ref."""
    bundle = build(tmp_path)
    overview = view(bundle, "overview")
    assert validate.schema_errors(overview, contract("overview")) == []
    tiles = {tile["id"]: tile for tile in overview["tiles"]}
    assert list(tiles) == [tile["id"] for tile in build_bundle.OVERVIEW_TILES]
    assert {tile["pillar"] for tile in overview["tiles"]} == {"P1", "P2"}
    # No results directory records an S09 verdict yet: every claim is not yet measured and names where it will be.
    assert all(tile["claim_state"] == "NOT_YET_MEASURED" and tile["planned_in"] for tile in overview["tiles"])
    usd = tiles["p1-usd-per-verified"]
    labels = [(row["label"], row["estimate"]["value"]) for row in usd["rows"]]
    assert labels == [("cheap·roko", 0.0418), ("frontier·direct", 0.241)]
    assert usd["view"] == "p1-head-to-head"
    records = {build_bundle.metric_ref(record): record for record in rows(bundle / "data" / "metrics.jsonl")}
    for row in usd["rows"]:
        ref = row["estimate"]["metric_ref"]
        assert records[ref]["metric"] == "usd_per_vs"
        assert row["estimate"] == {"value": records[ref]["value"], "ci": records[ref].get("ci"), "metric_ref": ref}
    # The fixture has pass^1, not pass^3, and census false-green rates, not M4's audit estimate.
    for name in ("p1-consistency", "p1-routing", "p1-spec-effect", "p2-m4-false-green"):
        assert (tiles[name]["rows"], tiles[name]["view"]) == ([], None), name
    assert overview["negatives"] == []


@pytest.mark.parametrize("name", ["overview", "p1-head-to-head"])
def test_r1_views_follow_the_page_contracts(tmp_path: Path, name: str) -> None:
    """Each R1 view the bundle carries follows the page's JSON Schema for it, so the page draws it: past the guard,
    the old head-to-head threw ("reading 'frontier_arms'")."""
    bundle = build(tmp_path)
    manifest = json.loads((bundle / "bundle.json").read_text(encoding="utf-8"))
    assert validate.schema_errors(manifest, contract("bundle")) == []
    assert name in manifest["views"]
    assert validate.schema_errors(view(bundle, name), contract(name)) == []


def test_r1_views_follow_the_page_contracts_head_to_head_arms(tmp_path: Path) -> None:
    """The head-to-head's arms carry the contract's fields: the label, tier, harness and role of the arm, and the
    models, tasks, trials and cost source of its run records."""
    head_to_head = view(build(tmp_path), "p1-head-to-head")
    arms = {arm["arm"]: arm for arm in head_to_head["arms"]}
    assert list(arms) == ["roko_fixed", "fd_claude"]
    roko, frontier = arms["roko_fixed"], arms["fd_claude"]
    assert (roko["label"], roko["tier"], roko["harness"], roko["role"]) == ("cheap·roko", "cheap", "roko", "arm")
    assert (frontier["label"], frontier["tier"], frontier["harness"]) == ("frontier·direct", "frontier", "direct")
    assert (roko["models"], frontier["models"]) == (["cerebras/gpt-oss-120b"], ["anthropic/claude-sonnet-4-6"])
    assert (roko["n_trials"], frontier["n_trials"], roko["cost_source"]) == (2, 1, "provider_usage")
    assert (roko["resolve"]["value"], roko["usd_per_verified"]["value"]) == (0.5, 0.0418)
    assert roko["pass_hat_k"] is None  # the fixture has pass^1 only, and the contract wants 1, 3 and 5
    assert head_to_head["claim"] == {
        "hypothesis": "H1",
        "state": "NOT_YET_MEASURED",
        "prereg_id": None,
        "planned_in": ["LOG1"],
        "text": build_bundle.H1_TEXT,
    }
    assert (head_to_head["pareto"]["frontier_arms"], head_to_head["envelope"]) == ([], [])


def test_r1_views_follow_the_page_contracts_m4_waits_for_audits(tmp_path: Path) -> None:
    """A contract m4-audits view needs S05's audit records (its policy, draws and checks), which no R1 bundle holds,
    so the builder leaves it out and the page says "not yet measured" instead of throwing ("reading 'state'")."""
    bundle = build(tmp_path)
    manifest = json.loads((bundle / "bundle.json").read_text(encoding="utf-8"))
    assert "m4-audits" not in manifest["views"]
    assert not (bundle / "views" / "m4-audits.json").exists()


def test_r1_views_follow_the_page_contracts_envelope(tmp_path: Path) -> None:
    """The provenance envelope has every field the drawer reads, copied from the run records and the manifest: n,
    seeds and the window of the runs, and pointers to each metric's own estimator and interval."""
    bundle = build(tmp_path)
    manifest = json.loads((bundle / "bundle.json").read_text(encoding="utf-8"))
    records = rows(bundle / "data" / "records.jsonl")
    files = {file["path"]: file for file in manifest["files"]}
    for name in manifest["views"]:
        provenance = view(bundle, name)["provenance"]
        assert validate.schema_errors(provenance, contract(name)["properties"]["provenance"]) == [], name
        assert provenance["n"] == files["data/records.jsonl"]["rows"] == len(records)
        assert provenance["seeds"] == sorted({record["seed"] for record in records}) == [1, 2]
        assert provenance["window"] == {"from": "2026-10-03T09:00:00Z", "to": "2026-10-03T10:29:31Z"}
        assert provenance["cost_usd"] == manifest["cost_usd"] == 0.2828
        assert provenance["record_filter"] == 'experiment_id == "FIXTURE-P1"'
        assert provenance["estimator"] == provenance["ci"]["method"] == build_bundle.PER_METRIC
        assert [source["sha256_verified"] for source in provenance["sources"]] == [False]


def test_the_m4_tile_shows_only_an_audit_estimate(tmp_path: Path) -> None:
    """M4's false-green rate is S05's audit-lottery estimate (`vs_estimated`); a census rate stays off its tile."""
    results, metrics_path, document = copied_results(tmp_path)
    census = next(record for record in document["records"] if record["metric"] == "false_green_rate")
    audited = dict(census, label_source="vs_estimated", value=0.25, ci=[0.05, 0.6], ci_method="ht_wilson_eff_n")
    document["records"].append(audited)
    metrics_path.write_text(json.dumps(document), encoding="utf-8")
    out = tmp_path / "b-audited"
    assert build_from(results, out) == 0
    tile = next(tile for tile in view(out, "overview")["tiles"] if tile["id"] == "p2-m4-false-green")
    estimate = {"value": 0.25, "ci": [0.05, 0.6], "metric_ref": build_bundle.metric_ref(audited)}
    assert tile["rows"] == [{"label": "cheap·roko", "estimate": estimate}]
    assert tile["view"] is None  # no m4-audits view until S05's audit records are projected


def test_the_arm_labels_are_the_arm_files() -> None:
    """Views label arms as `arms/<arm>.toml` does, and the head-to-head's harness is the file's."""
    for arm, label in build_bundle.ARM_LABELS.items():
        spec = tomllib.loads((build_bundle.VB_ROOT / "arms" / f"{arm}.toml").read_text(encoding="utf-8"))["arm"]
        assert spec["label"] == label, arm
        if arm in build_bundle.HEAD_TO_HEAD:
            assert (build_bundle.HEAD_TO_HEAD[arm][1] == "roko") == (spec["harness"] == "roko"), arm
    assert set(build_bundle.HEAD_TO_HEAD) <= set(build_bundle.ARM_LABELS)


@pytest.mark.parametrize(
    ("section", "field"),
    [("execution", "started_at"), ("execution", "finished_at"), ("costs", "billed_usd")],
)
def test_a_record_without_its_run_times_or_its_bill_is_refused(tmp_path: Path, section: str, field: str) -> None:
    """A view's provenance window and cost come from the run records, so each needs its start, finish and bill."""
    results, _, _ = copied_results(tmp_path)
    path = results / "FIXTURE-P1" / "vb-fixture-r1" / "records.jsonl"
    record = rows(path)[0]
    if section == "costs":
        record[section][field] = None
    else:
        del record[section][field]
    path.write_text(json.dumps(record) + "\n", encoding="utf-8")
    out = tmp_path / "b-refused"
    assert build_from(results, out) == 1
    assert not out.exists()


def test_a_record_with_no_run_behind_it_is_shown_nowhere(tmp_path: Path) -> None:
    """A MetricRecord with n = 0, such as a false-green rate over no reported pass, has a null value; showing it would
    make the guard refuse its whole view."""
    results, metrics_path, document = copied_results(tmp_path)
    empty = dict(document["records"][0], metric="false_green_rate_unknown_as_1", value=None, n=0, ci_method="none")
    empty.pop("ci", None)
    document["records"].append(empty)
    metrics_path.write_text(json.dumps(document), encoding="utf-8")
    out = tmp_path / "b-empty"
    assert build_from(results, out) == 0
    assert verify_bundle.check(out) == []

    ref = build_bundle.metric_ref(empty)
    for path in sorted((out / "views").iterdir()):
        text = path.read_text(encoding="utf-8")
        assert ref not in text, path.name
        assert guard_refusals(json.loads(text)) == [], path.name


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


def poison_dropped_view(bundle: Path) -> None:
    edit_manifest(bundle, views=["overview"])


def poison_hand_edited_view(bundle: Path) -> None:
    overview = view(bundle, "overview")
    next(row for tile in overview["tiles"] for row in tile["rows"])["estimate"]["value"] = 0.9
    (bundle / "views" / "overview.json").write_bytes(build_bundle.canonical_json(overview))


@pytest.mark.parametrize(
    ("poison", "rule"),
    [
        (poison_simulated, "simulated"),
        (poison_fixture_kind, "kind"),
        (poison_missing_run_ids, "run_ids"),
        (poison_transcript, "transcripts"),
        (poison_plaintext_hidden_test, "hidden"),
        (poison_hand_edited_view, "views"),
        (poison_dropped_view, "views"),
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
