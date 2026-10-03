"""LOG1's runbook (`log1.toml`, 3347): its billed blocks are `streams/log1.toml`'s cells (3330), one block per cell
with S09 §4.3's counts; its subscription blocks are S09's must, should and could rows; and its dry run fits BL1 and
is refused only for want of the pre-registration lock (3345). Nothing runs.

Run from the repository root with the benchmark venv:
    benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/experiments/test_log1.py -q
"""

from __future__ import annotations

import json
import tomllib
from collections import Counter

import pytest

import campaign
import disturb
import layout
import ledger
import vb
from common import hmac_seed

LOG1 = layout.VB_ROOT / "experiments" / "log1.toml"
CELLS = layout.VB_ROOT / "streams" / "log1.toml"
RATES = {"roko": 0.075, "mini-loop": 0.06}  # S09 §3: planned $ per Roko run and per direct run
CHEAP_POOL = ("gpt-oss-120b", "glm-4.7", "kimi-k2.6", "gpt-5.4-mini")  # S09 §4.3's four cheap models


def _block_shape(block: campaign.Block) -> tuple[str, dict, vb.Stream, str | None]:
    """A block's letter, arm, stream and latent (from its convention_flip disturbance)."""
    arm = vb.load_arm(block.arm)
    stream = vb.load_stream(block.stream)
    latent = None
    if block.disturbance:
        [flip] = disturb.load(layout.VB_ROOT / block.disturbance)
        assert flip.kind == "convention_flip" and (flip.start_at, flip.end_at) == (1, len(stream.instances))
        latent = flip.params["latent"]
    return block.id[0].upper(), arm, stream, latent


def test_log1_manifest_matches_s09_cell_counts(tmp_path, capsys):
    manifest = campaign.load(LOG1)
    assert manifest.id == "LOG1" and manifest.requires_lock and manifest.order == "daily_interleave"
    assert campaign.requires_lock("LOG1")
    compiled = tomllib.loads(CELLS.read_text())

    # Every billed block is one of 3330's cells: block, arm, model, stream, spec variant, latent, seeds and count.
    billed, subscription, optional = Counter(), Counter(), Counter()
    blocks, planned = [], 0.0
    for block in manifest.blocks:
        letter, arm, stream, latent = _block_shape(block)
        assert block.line == "BL1" and block.model in arm["arm"]["models_allow"], block.id
        assert len(arm["arm"]["models_allow"]) == 1, f"{block.arm} must pin one model, or run_roko builds a ladder"
        runs = len(stream.instances) * len(block.seeds)
        if arm["arm"]["billed"]:
            billed[letter] += runs
            assert block.planned_usd == pytest.approx(runs * RATES[arm["arm"]["harness"]]), block.id
            planned += block.planned_usd
            blocks.append((letter, arm["arm"]["id"], block.model, stream.id, stream.spec_variant, latent,
                           tuple(block.seeds), runs))
        elif block.optional:
            optional[arm["arm"]["id"]] += runs
        else:
            subscription[arm["arm"]["id"]] += runs
        if not arm["arm"]["billed"]:
            assert block.off_hours and block.planned_usd is None, block.id
    cells = []
    for cell in compiled["cell"]:
        model = cell["model"].split("/")[-1]
        if model == "best_cheap":  # block B's best cheap model: 3349 picks it before block B runs (D34)
            [match] = [one for one in blocks if one[0] == "B" and one[2] != CHEAP_POOL[0] and one[3] == "p1_core"]
            assert match[2] in CHEAP_POOL
            model = match[2]
        cells.append((cell["block"], cell["arm"], model, cell["stream"], cell.get("variant", "precise"),
                      cell.get("latent"), tuple(cell["seeds"]), cell["n"]))
    assert sorted(blocks, key=repr) == sorted(cells, key=repr)
    assert billed == {"A": 960, "B": 660, "C": 336, "D": 120, "E": 80} and sum(billed.values()) == 2156
    assert subscription == {"fd_claude": 420, "fr_claude": 192} and sum(subscription.values()) == 612
    assert optional == {"fd_claude_lite": 90, "fd_codex": 120}  # S09's should and could rows
    assert planned == pytest.approx(151.8)  # $152.3 with block F's probe, which runs no `vb run`

    # The derived streams name exactly the instances 3330 compiled: p1_h3's 48 under two more variants, and the
    # F8 honeypots log1.toml draws.
    h3 = vb.load_stream("p1_h3")
    for variant in ("vague", "refined"):
        derived = vb.load_stream(f"log1_p1_h3_{variant}")
        assert (derived.id, derived.spec_variant, derived.instances) == ("p1_h3", variant, h3.instances)
        assert derived.families == h3.families
    honeypots = vb.load_stream("log1_f8_honeypots")
    assert honeypots.instances == compiled["honeypots"] and set(honeypots.families) == {"F8"}

    # Each per-model arm file is its base arm's but for the model, the line and the provider table.
    for base, suffixes in (("roko_fixed", ("glm", "kimi", "mini")), ("cheap_direct", ("glm", "kimi", "mini"))):
        reference = vb.load_arm(base)
        for suffix in suffixes:
            variant = vb.load_arm(f"{base}_{suffix}")
            assert variant["arm"]["id"] == base and variant["caps"] == reference["caps"]
            assert variant.get("roko") == reference.get("roko") and len(variant["providers"]) == 1

    # The dry run: 2,768 runs (2,156 billed, 612 subscription), BL1's worst case inside its $160, and the lock
    # the only refusal until 3345 commits it.
    secret_file = hmac_seed.write_secret_file(tmp_path / "config" / "secret")
    for include, runs in (([], 2768), (["--include", "s-fd-claude-lite", "--include", "s-fd-codex"], 2978)):
        assert vb.main(["campaign", "--manifest", str(LOG1), "--dry-run", "--results", str(tmp_path / "results"),
                        "--secret-file", str(secret_file), *include]) == 2
        shown = json.loads(capsys.readouterr().out)
        assert shown["runs"] == runs and shown["planned_usd"] == pytest.approx(151.8)
        [refusal] = shown["problems"]
        assert refusal.startswith("requires_lock: ") and "3345" in refusal
        line = shown["budget"]["line BL1"]
        cap = ledger.load_budget().lines["BL1"].cap_usd
        assert line["planned_usd"] == pytest.approx(151.8) and line["worst_case_usd"] <= line["cap_usd"] == cap
    assert max(unit["day"] for unit in shown["units"]) == 3  # S09 §4.1: each block's k-th seed on day k
