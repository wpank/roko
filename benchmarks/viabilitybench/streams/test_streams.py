#!/usr/bin/env python3
"""Tests for the P1 streams (S08 §4.7; work item gap-eb1aa3, task 3328): reproducibility from their seed, the
counts per family and level, and that P1-H3 and the pass^5 subset are genuine subsets of P1-core, not separately
drawn (S09 §4.3 needs equal spec hashes). Offline; no family's `gen.py` runs here beyond one `vb materialize`
smoke check. Run from the repository root:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/streams/test_streams.py
"""

from __future__ import annotations

import shutil
import sys
from collections import Counter
from pathlib import Path

import pytest

STREAMS_DIR = Path(__file__).resolve().parent
VB_ROOT = STREAMS_DIR.parent
sys.path.insert(0, str(VB_ROOT / "families"))
sys.path.insert(0, str(VB_ROOT / "driver"))
sys.path.insert(0, str(STREAMS_DIR))
from common import knobs  # noqa: E402
import compile as compile_mod  # noqa: E402
import vb  # noqa: E402

LEVELS = set(knobs.LEVELS)


def _family_level(instance_id: str) -> tuple[str, int]:
    family, level, _seed = knobs.parse_instance_id(instance_id)
    return family, level


def test_p1_streams_are_reproducible_from_their_seeds():
    """The same --seed always compiles the same three documents, and a different seed compiles a different one
    (so the draw genuinely depends on the seed, not just on the family pool)."""
    first = compile_mod.compile_streams(1)
    again = compile_mod.compile_streams(1)
    assert first == again
    other = compile_mod.compile_streams(2)
    assert other != first
    for name in ("p1_core", "p1_h3", "p1_pass5"):
        assert first[name]["stream"]["instances"] != other[name]["stream"]["instances"]


def test_p1_core_has_120_tasks_four_per_family_and_level():
    core = compile_mod.compile_streams(1)["p1_core"]["stream"]
    instances = core["instances"]
    assert len(instances) == 120 and len(set(instances)) == 120
    assert set(core["families"]) == set(compile_mod.P1_CORE_FAMILIES)
    counts = Counter(_family_level(instance_id) for instance_id in instances)
    assert set(counts) == {(family, level) for family in compile_mod.P1_CORE_FAMILIES for level in LEVELS}
    assert set(counts.values()) == {compile_mod.INSTANCES_PER_CELL}


def test_p1_h3_is_a_48_task_subset_of_p1_core_at_levels_1_to_4():
    docs = compile_mod.compile_streams(1)
    core_instances, h3_instances = docs["p1_core"]["stream"]["instances"], docs["p1_h3"]["stream"]["instances"]
    assert len(h3_instances) == 48 and len(set(h3_instances)) == 48
    assert set(h3_instances) <= set(core_instances)  # S09 §4.3: equal spec hashes, because it IS the same instance
    counts = Counter(_family_level(instance_id) for instance_id in h3_instances)
    assert set(counts) == {(family, level) for family in compile_mod.P1_CORE_FAMILIES for level in (1, 2, 3, 4)}
    assert set(counts.values()) == {compile_mod.H3_PER_CELL}
    assert all(level != 5 for _family, level in counts)


def test_the_pass5_subset_has_30_tasks_one_per_family_and_level():
    docs = compile_mod.compile_streams(1)
    core_instances = set(docs["p1_core"]["stream"]["instances"])
    pass5_instances = docs["p1_pass5"]["stream"]["instances"]
    assert len(pass5_instances) == 30 and len(set(pass5_instances)) == 30
    assert set(pass5_instances) <= core_instances
    counts = Counter(_family_level(instance_id) for instance_id in pass5_instances)
    assert set(counts) == {(family, level) for family in compile_mod.P1_CORE_FAMILIES for level in LEVELS}
    assert set(counts.values()) == {1}
    assert docs["p1_pass5"]["stream"]["id"] == "p1_pass5"  # analysis/figlib.py's PASS5_STREAM reads this literal id


def test_the_committed_stream_files_match_compile_py():
    """The checked-in p1_core.toml, p1_h3.toml, p1_pass5.toml and log1.toml are exactly compile.py's output for
    DEFAULT_SEED: nobody hand-edited them after compiling (`compile.py --check` makes the same comparison)."""
    documents = compile_mod.compile_streams(compile_mod.DEFAULT_SEED)
    for name, filename in compile_mod.FILES.items():
        if name == "log1":
            continue
        path = STREAMS_DIR / filename
        assert path.read_text(encoding="utf-8") == compile_mod.render_toml(documents[name], seed=compile_mod.DEFAULT_SEED), \
            f"{filename} does not match compile.py --seed {compile_mod.DEFAULT_SEED}; regenerate it"
    log1_path = STREAMS_DIR / compile_mod.FILES["log1"]
    log1_text = compile_mod.render_log1_toml(compile_mod.compile_log1(compile_mod.DEFAULT_SEED),
                                             seed=compile_mod.DEFAULT_SEED)
    assert log1_path.read_text(encoding="utf-8") == log1_text, "log1.toml does not match compile.py; regenerate it"


@pytest.mark.parametrize("stream_id", ["p1_core", "p1_h3", "p1_pass5"])
def test_streams_load_through_the_driver_and_order_reproducibly(stream_id):
    """The committed files are valid `vb.stream/1` files: the real driver loads them, and `Stream.order` (the
    keyed shuffle `vb run` uses) gives the same order on two calls with the same run seed."""
    stream = vb.load_stream(stream_id)
    assert stream.id == stream_id
    assert stream.order(7) == stream.order(7)
    assert sorted(stream.order(7)) == sorted(stream.instances)


def test_materialize_is_byte_identical_across_two_runs(tmp_path):
    """Plan item 2: `vb materialize --stream <s>` run twice on the same instance gives the same pristine commit
    and tree (the family's own `gen.py` determinism; a smoke check that the stream file wires into it)."""
    if shutil.which("git") is None:
        pytest.skip("materialize needs git")
    stream = vb.load_stream("p1_core")
    instance_id = next(instance_id for instance_id in stream.instances if instance_id.startswith("F1-"))
    from materialize import materialize

    results = [materialize(family_dir=stream.family_dir(instance_id), instance_id=instance_id,
                           workdir=tmp_path / f"work{index}", private_dir=tmp_path / f"private{index}",
                           spec_variant=stream.spec_variant) for index in (1, 2)]
    # The bundle path differs (each run wrote into its own private_dir); the commit and tree it records must not.
    assert results[0].pristine.commit == results[1].pristine.commit
    assert results[0].pristine.tree == results[1].pristine.tree


# --- LOG1 (S09 §4.3; task 3330) ------------------------------------------------------------------------------


def test_log1_compiler_emits_s09_cell_counts():
    """The billed blocks' totals match S09 §4.3's table exactly: A 960, B 660, C 336, D 120, E 80 (2,156)."""
    doc = compile_mod.compile_log1(1)
    totals: dict[str, int] = {}
    for cell in doc["cells"]:
        totals[cell["block"]] = totals.get(cell["block"], 0) + cell["n"]
    assert totals == {"A": 960, "B": 660, "C": 336, "D": 120, "E": 80}
    assert sum(totals.values()) == 2156
    assert len(doc["honeypots"]) == compile_mod.LOG1_N_HONEYPOTS == 10
    assert len(set(doc["honeypots"])) == 10
    assert all(knobs.parse_instance_id(instance_id)[0] == "F8" for instance_id in doc["honeypots"])


def test_log1_is_reproducible_from_its_seed():
    first = compile_mod.compile_log1(1)
    again = compile_mod.compile_log1(1)
    assert first == again
    other = compile_mod.compile_log1(2)
    assert other["honeypots"] != first["honeypots"]


def test_log1_block_c_reuses_block_a_precise_reps_one_and_two():
    """Block C's own `precise` row covers only rep 3; its note names block A's seeds 1-2 as reps 1-2, the same
    arm and model, so nothing double-runs them (S09 §4.3: "reps 1-2 = block A seeds 1-2 on the same instances")."""
    doc = compile_mod.compile_log1(1)
    block_a = [cell for cell in doc["cells"] if cell["block"] == "A" and cell["model"] == compile_mod.LOG1_CHEAP_MODELS[0]]
    assert len(block_a) == 1 and block_a[0]["seeds"] == [1, 2] and block_a[0]["stream"] == "p1_core"
    precise_c = [cell for cell in doc["cells"] if cell["block"] == "C" and cell.get("variant") == "precise"]
    assert len(precise_c) == 1
    cell = precise_c[0]
    assert cell["seeds"] == [3] and cell["n"] == 48  # only the 3rd rep is its own row
    assert cell["arm"] == block_a[0]["arm"] == "roko_fixed" and cell["model"] == block_a[0]["model"]
    assert "reused from block A" in cell["note"] and "seeds 1-2" in cell["note"]
    # Block C's P1-H3 instances are a real subset of block A's P1-core instances (equal spec hashes), so "the
    # same instances" is literally true, not just a naming coincidence.
    streams = compile_mod.compile_streams(1)
    assert set(streams["p1_h3"]["stream"]["instances"]) <= set(streams["p1_core"]["stream"]["instances"])


def test_log1_toml_round_trips_through_tomllib():
    import tomllib

    doc = compile_mod.compile_log1(1)
    text = compile_mod.render_log1_toml(doc, seed=1)
    parsed = tomllib.loads(text)
    assert parsed["schema_version"] == doc["schema_version"] and parsed["seed"] == 1
    assert parsed["honeypots"] == doc["honeypots"]
    assert len(parsed["cell"]) == len(doc["cells"])
    assert sum(cell["n"] for cell in parsed["cell"]) == 2156
