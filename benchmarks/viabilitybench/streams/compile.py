#!/usr/bin/env python3
"""Compile the P1 streams from a recorded seed (S08 §4.7; work item gap-eb1aa3, task 3328).

P1-core is six families (F1-F5, F7) x levels 1-5 x 4 instances per (family, level) = 120 tasks. Two more streams
are drawn from exactly those 120, so both are subsets with equal spec hashes (S09 §4.3) rather than separately
generated:

- P1-H3: 2 of the 4 instances per family, at levels 1-4 only (not 5) = 6 x 4 x 2 = 48 tasks.
- The pass^5 subset: 1 of the 4 instances per family and level = 6 x 5 = 30 tasks, its own stream so
  FIGURES-TABLES F4 can read it apart from the 120 by `stream.id` (`analysis/figlib.py`'s `PASS5_STREAM`).

Every draw is `common.hmac_seed`'s public surface stream, keyed by the recorded `--seed`, so the same seed always
compiles the same three files, on every host (S08 §4.7). The instance-to-secret assignment for parallel LOG1 runs
is fixed in its runbook (task 3347), not here: these streams only fix which instances belong to which set.

LOG1 (S09 §4.3; task 3330) is a different shape, one row per cell rather than a flat instance list:
`compile_log1(seed)` writes `log1.toml`'s billed blocks A-E (960 + 660 + 336 + 120 + 80 = 2,156 runs), each
cell naming its block, arm, model, stream, seeds and task count. It reuses this module's own P1-core/P1-H3/
pass^5 streams and `streams/pilot.toml` rather than drawing new instance lists, so block C's reused reps and
block D's convention_flip rows are the same instances those streams already name.

Usage:
    compile.py --seed N [--out-dir DIR] [--check]

--check recompiles and compares against the files already in DIR (default: this directory) without writing;
exit 1 if any differs. Without --check, compile.py writes p1_core.toml, p1_h3.toml, p1_pass5.toml and log1.toml
into DIR.

API:
    compile_streams(seed: int) -> dict[str, dict]      # "p1_core"/"p1_h3"/"p1_pass5" -> a vb.stream/1 document
    render_toml(doc: dict, *, seed: int) -> str
    compile_log1(seed: int) -> dict                     # a vb.log1/1 document: "cells" and "honeypots"
    compile_f8_honeypots(seed: int) -> list[str]        # LOG1_N_HONEYPOTS F8 instance ids, for block E
    render_log1_toml(doc: dict, *, seed: int) -> str
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

STREAMS_DIR = Path(__file__).resolve().parent
VB_ROOT = STREAMS_DIR.parent
sys.path.insert(0, str(VB_ROOT / "families"))
from common import hmac_seed, knobs  # noqa: E402

SCHEMA_VERSION = "vb.stream/1"
LOG1_SCHEMA_VERSION = "vb.log1/1"
DEFAULT_SEED = 1  # the seed p1_core.toml, p1_h3.toml and p1_pass5.toml are compiled from (recorded in each file)
# The P1-core families (S08 §4.7): F6 and F8 are not in P1 (F6's own ladder.toml and gap-eb1aa3 note that; F8
# wraps F2/F3/F5, so an F8 cell would double-count one of them).
P1_CORE_FAMILIES = {"F1": "f1_pyconv", "F2": "f2_apipager", "F3": "f3_moneyround", "F4": "f4_kvtool",
                    "F5": "f5_sqlmigrate", "F7": "f7_rustiter"}
LEVELS = knobs.LEVELS
INSTANCES_PER_CELL = 4
H3_LEVELS = (1, 2, 3, 4)
H3_PER_CELL = 2
SEED_POOL = range(1, 501)  # the instance seeds a cell's 4 are drawn from, without repeats
FILES = {"p1_core": "p1_core.toml", "p1_h3": "p1_h3.toml", "p1_pass5": "p1_pass5.toml", "log1": "log1.toml"}

# S09 §4.3's four cheap models, in the order the table lists them (block D and E's "x 3"/"x 4 cheap" read a
# prefix of this tuple). "best_cheap" is block B's "best cheap" row: S09 says it is "chosen out of sample" by
# the pilot, not fixed here, so it is a placeholder id a later task resolves, never a silent guess at which one.
LOG1_CHEAP_MODELS = ("cerebras/gpt-oss-120b", "zai/glm-4.7", "moonshot/kimi-k2.6", "openai/gpt-5.4-mini")
LOG1_BEST_CHEAP = "best_cheap"  # block B: resolved from the pilot (S09 §4.3), not this compiler
LOG1_HONEYPOT_FAMILIES = {"F2": "f2_apipager", "F3": "f3_moneyround", "F5": "f5_sqlmigrate"}  # F8's BASE_POOL
LOG1_N_HONEYPOTS = 10
LOG1_PILOT_N = 20  # block D's "convention_flip v2 rows": streams/pilot.toml's own 20 (F1 + F4, both v2-capable)


def compile_streams(seed: int) -> dict[str, dict]:
    """{"p1_core", "p1_h3", "p1_pass5"} -> a vb.stream/1 document (the module docstring), from `seed` alone."""
    root = hmac_seed.surface_stream("p1-streams", str(seed))
    families = {name: f"families/{directory}" for name, directory in P1_CORE_FAMILIES.items()}

    cells: dict[tuple[str, int], list[str]] = {}
    for family in sorted(P1_CORE_FAMILIES):
        for level in LEVELS:
            drawn = sorted(root.child(f"core/{family}/{level}").sample(SEED_POOL, INSTANCES_PER_CELL))
            cells[family, level] = [knobs.instance_id(family, level, instance_seed) for instance_seed in drawn]
    core = [instance_id for family in sorted(P1_CORE_FAMILIES) for level in LEVELS
           for instance_id in cells[family, level]]

    h3 = []
    for family in sorted(P1_CORE_FAMILIES):
        for level in H3_LEVELS:
            cell = cells[family, level]
            chosen = set(root.child(f"h3/{family}/{level}").sample(cell, H3_PER_CELL))
            h3 += [instance_id for instance_id in cell if instance_id in chosen]

    pass5 = [root.child(f"pass5/{family}/{level}").choice(cells[family, level])
            for family in sorted(P1_CORE_FAMILIES) for level in LEVELS]

    def doc(stream_id: str, instances: list[str]) -> dict:
        return {"schema_version": SCHEMA_VERSION,
                "stream": {"id": stream_id, "spec_variant": "precise", "families": dict(families),
                          "instances": instances}}

    return {"p1_core": doc("p1_core", core), "p1_h3": doc("p1_h3", h3), "p1_pass5": doc("p1_pass5", pass5)}


def compile_f8_honeypots(seed: int) -> list[str]:
    """`LOG1_N_HONEYPOTS` F8 instance ids, 1 per (level, seed) drawn round-robin over `LOG1_HONEYPOT_FAMILIES`'s
    base families (`F8.gen.BASE_POOL`): `seed % len(BASE_POOL)` picks the base, as F8's own generator does."""
    root = hmac_seed.surface_stream("log1-honeypots", str(seed))
    drawn = sorted(root.child("f8").sample(SEED_POOL, LOG1_N_HONEYPOTS))
    return [knobs.instance_id("F8", LEVELS[index % len(LEVELS)], instance_seed)
           for index, instance_seed in enumerate(drawn)]


def _cell(block: str, arm: str, model: str, stream: str, seeds: list[int], n_tasks: int, **extra) -> dict:
    return {"block": block, "arm": arm, "model": model, "stream": stream, "seeds": seeds, "n_tasks": n_tasks,
           "n": n_tasks * len(seeds), **extra}


def compile_log1(seed: int) -> dict:
    """LOG1 (S09 §4.3): one row per cell (block, arm, model, stream, seeds), billed blocks A-E only (block F, the
    P1-ext contamination probe, is task 3332's `external/swebench/probe.py`'s own count, "≈750 short calls", not
    a cell here; the Sub(scription) block is the arms' own subscription-billed reruns, out of this compiler's
    scope). Reproducible from `seed` alone; the exact counts are S09 §4.3's: A 960, B 660, C 336, D 120, E 80."""
    cells = []
    # Block A: roko_fixed x 4 cheap models x P1-core (120) x seeds 1-2 = 4 x 240 = 960.
    for model in LOG1_CHEAP_MODELS:
        cells.append(_cell("A", "roko_fixed", model, "p1_core", [1, 2], 120))
    # Block B: cheap_direct. gpt-oss-120b x3 (360) + best_cheap x2 (240) + gpt-oss-120b's pass^5 extension, 2
    # EXTRA reps beyond A/B's main three, so this row's own seeds (4, 5) never collide with the others' (30 x 2 = 60).
    cells.append(_cell("B", "cheap_direct", LOG1_CHEAP_MODELS[0], "p1_core", [1, 2, 3], 120))
    cells.append(_cell("B", "cheap_direct", LOG1_BEST_CHEAP, "p1_core", [1, 2], 120))
    cells.append(_cell("B", "cheap_direct", LOG1_CHEAP_MODELS[0], "p1_pass5", [4, 5], 30,
                       note="the pass^5 extension; p1_pass5's own reps 1-3 are this row's seeds 1-3 on p1_core"))
    # Block C: roko_fixed, gate off, on P1-H3 (48). Reps 1-2 of the precise variant are block A's gpt-oss-120b
    # seeds 1-2 on the same (P1-H3 subset of P1-core) instances, so only the 3rd rep is its own row here.
    cells.append(_cell("C", "roko_fixed", LOG1_CHEAP_MODELS[0], "p1_h3", [1, 2, 3], 48, variant="vague"))
    cells.append(_cell("C", "roko_fixed", LOG1_CHEAP_MODELS[0], "p1_h3", [3], 48, variant="precise",
                       note="reps 1-2 reused from block A's roko_fixed/gpt-oss-120b seeds 1-2 on p1_core"))
    cells.append(_cell("C", "roko_fixed", LOG1_CHEAP_MODELS[0], "p1_h3", [1, 2], 48, variant="refined"))
    cells.append(_cell("C", "roko_fixed", LOG1_CHEAP_MODELS[1], "p1_h3", [1], 48, variant="vague"))
    # Block D: roko_fixed x 3 cheap models x pilot's 20 (F1 + F4, latent v2) x seeds 1-2 = 3 x 40 = 120.
    for model in LOG1_CHEAP_MODELS[:3]:
        cells.append(_cell("D", "roko_fixed", model, "pilot", [1, 2], LOG1_PILOT_N, latent="v2"))
    # Block E: roko_fixed x 4 cheap models x 10 F8 honeypots x seeds 1-2 = 4 x 20 = 80.
    for model in LOG1_CHEAP_MODELS:
        cells.append(_cell("E", "roko_fixed", model, "log1_f8_honeypots", [1, 2], LOG1_N_HONEYPOTS))
    return {"schema_version": LOG1_SCHEMA_VERSION, "seed": seed, "honeypots": compile_f8_honeypots(seed),
           "cells": cells}


def render_log1_toml(doc: dict, *, seed: int) -> str:
    """`doc` (`compile_log1`'s return) as a `vb.log1/1` file."""
    totals: dict[str, int] = {}
    for cell in doc["cells"]:
        totals[cell["block"]] = totals.get(cell["block"], 0) + cell["n"]
    counts = ", ".join(f"{block} {totals[block]}" for block in sorted(totals))
    lines = [f"# LOG1 (S09 §4.3; work item gap-daeaa9, task 3330): compiled by streams/compile.py --seed {seed}.",
             f"# Billed cells by block: {counts} ({sum(totals.values())} total). Do not hand-edit: regenerate with",
             "# the same --seed for a byte-identical file.", f'schema_version = "{doc["schema_version"]}"',
             f'seed = {doc["seed"]}', "", "honeypots = [", *(f'  "{instance_id}",' for instance_id in doc["honeypots"]),
             "]", ""]
    for cell in doc["cells"]:
        lines.append("[[cell]]")
        for key in ("block", "arm", "model", "stream"):
            lines.append(f'{key} = "{cell[key]}"')
        lines.append("seeds = " + "[" + ", ".join(str(s) for s in cell["seeds"]) + "]")
        lines.append(f'n_tasks = {cell["n_tasks"]}')
        lines.append(f'n = {cell["n"]}')
        for key in ("variant", "latent", "note"):
            if key in cell:
                lines.append(f'{key} = "{cell[key]}"')
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def render_toml(doc: dict, *, seed: int) -> str:
    """`doc` (one of `compile_streams`'s values) as a `vb.stream/1` file, in `streams/pilot.toml`'s style."""
    stream = doc["stream"]
    n = len(stream["instances"])
    lines = [f"# Stream {stream['id']} (S08 §4.7; work item gap-eb1aa3, task 3328): compiled by streams/compile.py "
             f"--seed {seed}, {n} tasks.",
             "# Do not hand-edit: regenerate with the same --seed for a byte-identical file.",
             f'schema_version = "{doc["schema_version"]}"', "", "[stream]", f'id = "{stream["id"]}"',
             f'spec_variant = "{stream["spec_variant"]}"',
             "families = { " + ", ".join(f'{name} = "{path}"' for name, path in sorted(stream["families"].items()))
             + " }", "instances = ["]
    items = [f'"{instance_id}"' for instance_id in stream["instances"]]
    for index in range(0, len(items), 4):
        lines.append("  " + " ".join(f"{item}," for item in items[index:index + 4]))
    lines.append("]")
    return "\n".join(lines) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--seed", type=int, default=DEFAULT_SEED, help=f"default {DEFAULT_SEED}")
    parser.add_argument("--out-dir", type=Path, default=STREAMS_DIR)
    parser.add_argument("--check", action="store_true", help="compare against DIR without writing")
    args = parser.parse_args(argv)
    documents = compile_streams(args.seed)
    rendered = {name: render_toml(doc, seed=args.seed) for name, doc in documents.items()}
    rendered["log1"] = render_log1_toml(compile_log1(args.seed), seed=args.seed)
    mismatched = []
    for name, text in rendered.items():
        path = args.out_dir / FILES[name]
        if args.check:
            if not path.is_file() or path.read_text(encoding="utf-8") != text:
                mismatched.append(path)
        else:
            path.write_text(text, encoding="utf-8")
    if args.check and mismatched:
        print(f"compile.py --check: not compiled from --seed {args.seed}: "
             + ", ".join(str(path) for path in mismatched), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
