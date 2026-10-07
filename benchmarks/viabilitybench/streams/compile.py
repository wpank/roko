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

S08 §4.7's other streams (task 3331), each a plain `vb.stream/1` document like P1-core's:

- S1 learning curve: F1-F4 x 24 (96 items), in 24 blocks of 4 -- one instance per family per block, so reading
  the list 4 at a time gives one block, and reading it twice gives S1's own "2 passes".
- S3 disturbance (live): 10 nominal instances (positions 1-10) plus 30 disturbed (positions 11-40, 6 per S08
  §4.6 hook in `S3_KINDS` order). `compile_s3` also returns the `vb.disturbance/1` document (`s3_disturbance_hooks.toml`)
  that names those positions; F1 and F4 only, since one hook is `convention_flip` and F2/F3 build latent v1 only.
- S5 holdout: 120 instances spread evenly over P1-core's 6 families, for the always-on harmful-loop hook; its
  placebo condition reads the same 120 with the hook switched off, so there is no separate instance list for it.

Usage:
    compile.py --seed N [--out-dir DIR] [--check]

--check recompiles and compares against the files already in DIR (default: this directory) without writing;
exit 1 if any differs. Without --check, compile.py writes p1_core.toml, p1_h3.toml, p1_pass5.toml, log1.toml,
s1_learncurve.toml, s3_disturbance.toml, s3_disturbance_hooks.toml and s5_holdout.toml into DIR.

API:
    compile_streams(seed: int) -> dict[str, dict]      # "p1_core"/"p1_h3"/"p1_pass5" -> a vb.stream/1 document
    render_toml(doc: dict, *, seed: int) -> str
    compile_log1(seed: int) -> dict                     # a vb.log1/1 document: "cells" and "honeypots"
    compile_f8_honeypots(seed: int) -> list[str]        # LOG1_N_HONEYPOTS F8 instance ids, for block E
    render_log1_toml(doc: dict, *, seed: int) -> str
    compile_s1(seed: int) -> dict                       # a vb.stream/1 document
    compile_s3(seed: int) -> tuple[dict, dict]           # (the vb.stream/1 document, a vb.disturbance/1 document)
    compile_s5(seed: int) -> dict                        # a vb.stream/1 document
    render_disturbance_toml(doc: dict, *, stream_id: str, seed: int) -> str
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
FILES = {"p1_core": "p1_core.toml", "p1_h3": "p1_h3.toml", "p1_pass5": "p1_pass5.toml", "log1": "log1.toml",
        "s1_learncurve": "s1_learncurve.toml", "s3_disturbance": "s3_disturbance.toml",
        "s3_disturbance_hooks": "s3_disturbance_hooks.toml", "s5_holdout": "s5_holdout.toml"}

# S09 §4.3's four cheap models, in the order the table lists them (block D and E's "x 3"/"x 4 cheap" read a
# prefix of this tuple). "best_cheap" is block B's "best cheap" row: S09 says it is "chosen out of sample" by
# the pilot, not fixed here, so it is a placeholder id a later task resolves, never a silent guess at which one.
LOG1_CHEAP_MODELS = ("cerebras/gpt-oss-120b", "zai/glm-4.7", "moonshot/kimi-k2.6", "openai/gpt-5.4-mini")
LOG1_BEST_CHEAP = "best_cheap"  # block B: resolved from the pilot (S09 §4.3), not this compiler
LOG1_HONEYPOT_FAMILIES = {"F2": "f2_apipager", "F3": "f3_moneyround", "F5": "f5_sqlmigrate"}  # F8's BASE_POOL
LOG1_N_HONEYPOTS = 10
LOG1_PILOT_N = 20  # block D's "convention_flip v2 rows": streams/pilot.toml's own 20 (F1 + F4, both v2-capable)

# S08 §4.7's other streams (task 3331).
S1_FAMILIES = ("F1", "F2", "F3", "F4")
S1_PER_FAMILY = 24  # "F1-F4 x 24, blocks of 4 (96 items)"
S3_NOMINAL = 10
S3_KINDS = ("provider_fault", "model_swap", "harder_mix", "convention_flip", "budget_cut")  # S08 S4.6's 5 regulable
S3_PER_KIND = 6  # 30 disturbed / 5 kinds
# F1 and F4 only: one of S3_KINDS is convention_flip, which only the latent-v2 families can render (F2 and F3
# build v1 only), and "vb run refuses a spec whose latent some family of the run cannot render" (disturb.py).
S3_FAMILIES = ("F1", "F4")
S5_N = 120
S5_FAMILIES = tuple(sorted(P1_CORE_FAMILIES))
DISTURBANCE_SCHEMA_VERSION = "vb.disturbance/1"


def _draw_instances(stream: hmac_seed.Stream, family: str, count: int) -> list[str]:
    """`count` distinct instance ids of `family`, spread round-robin over levels 1-5 (`.sample` per level, so
    none repeats within a level); deterministic in `stream`."""
    per_level = [count // len(LEVELS) + (1 if i < count % len(LEVELS) else 0) for i in range(len(LEVELS))]
    instances = []
    for level, n in zip(LEVELS, per_level):
        drawn = sorted(stream.child(f"{family}/{level}").sample(SEED_POOL, n))
        instances += [knobs.instance_id(family, level, instance_seed) for instance_seed in drawn]
    return instances


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


def compile_s1(seed: int) -> dict:
    """S1 learning curve (S08 S4.7): F1-F4 x 24 (96 items), in 24 blocks of 4 -- one instance per family per
    block, cycling levels 1-5 block by block -- so a consumer reading the list 4 at a time sees one block, and
    twice through the whole list is S1's own "2 passes"."""
    root = hmac_seed.surface_stream("s1-learncurve", str(seed))
    per_family = {family: _draw_instances(root.child(family), family, S1_PER_FAMILY) for family in S1_FAMILIES}
    instances = [per_family[family][block] for block in range(S1_PER_FAMILY) for family in S1_FAMILIES]
    families = {family: f"families/{P1_CORE_FAMILIES[family]}" for family in S1_FAMILIES}
    return _stream_doc("s1_learncurve", families, instances)


def compile_s3(seed: int) -> dict:
    """S3 disturbance, live (S08 S4.7): 10 nominal instances (positions 1-10) plus 30 disturbed (positions
    11-40, 6 per kind, S08 S4.6's five regulable hooks in `S3_KINDS` order). Returns the stream document and the
    `vb.disturbance/1` document `vb run --disturbance` reads, built from the very same positions."""
    root = hmac_seed.surface_stream("s3-disturbance", str(seed))
    families = tuple(S3_FAMILIES[i % len(S3_FAMILIES)] for i in range(S3_NOMINAL + len(S3_KINDS) * S3_PER_KIND))
    pools = {family: iter(_draw_instances(root.child(family), family, families.count(family)))
            for family in set(families)}
    instances = [next(pools[family]) for family in families]
    stream = _stream_doc("s3_disturbance", {f: f"families/{P1_CORE_FAMILIES[f]}" for f in set(families)}, instances)
    # provider_fault and model_swap have no default params (driver/disturb.py's DEFAULTS), so they are given
    # explicitly here; the other three kinds' defaults (budget_cut's factor, harder_mix's levels,
    # convention_flip's latent) already match S08 §4.6's hooks and need no override.
    extra_params = {"provider_fault": {"name": "http_5xx", "p": 0.2}, "model_swap": {"to": LOG1_CHEAP_MODELS[1]}}
    disturbances = [{"kind": kind, "start_at": S3_NOMINAL + index * S3_PER_KIND + 1,
                    "end_at": S3_NOMINAL + (index + 1) * S3_PER_KIND, "seed": seed,
                    "params": extra_params.get(kind, {})} for index, kind in enumerate(S3_KINDS)]
    hooks = {"schema_version": DISTURBANCE_SCHEMA_VERSION, "disturbances": disturbances}
    return stream, hooks


def compile_s5(seed: int) -> dict:
    """S5 holdout (S08 S4.7): 120 instances spread evenly over its 6-family pool, for the always-on harmful-loop
    hook (h = 0.10 inside every Roko-full run); a placebo run reads the same 120 without the hook enabled, so
    there is no separate placebo instance list to compile."""
    root = hmac_seed.surface_stream("s5-holdout", str(seed))
    per_family = S5_N // len(S5_FAMILIES)
    remainder = S5_N - per_family * len(S5_FAMILIES)
    instances = []
    for index, family in enumerate(S5_FAMILIES):
        instances += _draw_instances(root.child(family), family, per_family + (1 if index < remainder else 0))
    families = {family: f"families/{P1_CORE_FAMILIES[family]}" for family in S5_FAMILIES}
    return _stream_doc("s5_holdout", families, instances)


def _stream_doc(stream_id: str, families: dict[str, str], instances: list[str]) -> dict:
    return {"schema_version": SCHEMA_VERSION,
           "stream": {"id": stream_id, "spec_variant": "precise", "families": dict(families),
                     "instances": instances}}


def render_disturbance_toml(doc: dict, *, stream_id: str, seed: int) -> str:
    lines = [f"# {stream_id}'s disturbance hooks (S08 §4.6, §4.7; task 3331): compiled by streams/compile.py "
             f"--seed {seed}. Do not hand-edit: regenerate with the same --seed for a byte-identical file.",
             f'schema_version = "{doc["schema_version"]}"', ""]
    for entry in doc["disturbances"]:
        lines.append("[[disturbance]]")
        lines.append(f'kind = "{entry["kind"]}"')
        lines.append(f'start_at = {entry["start_at"]}')
        lines.append(f'end_at = {entry["end_at"]}')
        lines.append(f'seed = {entry["seed"]}')
        params = entry.get("params") or {}
        if params:
            lines.append("params = { " + ", ".join(
                f'{key} = "{value}"' if isinstance(value, str) else f"{key} = {value}"
                for key, value in params.items()) + " }")
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
    rendered["s1_learncurve"] = render_toml(compile_s1(args.seed), seed=args.seed)
    s3_stream, s3_hooks = compile_s3(args.seed)
    rendered["s3_disturbance"] = render_toml(s3_stream, seed=args.seed)
    rendered["s3_disturbance_hooks"] = render_disturbance_toml(s3_hooks, stream_id="s3_disturbance", seed=args.seed)
    rendered["s5_holdout"] = render_toml(compile_s5(args.seed), seed=args.seed)
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
