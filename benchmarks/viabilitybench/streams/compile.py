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

Usage:
    compile.py --seed N [--out-dir DIR] [--check]

--check recompiles and compares against the files already in DIR (default: this directory) without writing;
exit 1 if any differs. Without --check, compile.py writes p1_core.toml, p1_h3.toml and p1_pass5.toml into DIR.

API:
    compile_streams(seed: int) -> dict[str, dict]      # "p1_core"/"p1_h3"/"p1_pass5" -> a vb.stream/1 document
    render_toml(doc: dict, *, seed: int) -> str
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
FILES = {"p1_core": "p1_core.toml", "p1_h3": "p1_h3.toml", "p1_pass5": "p1_pass5.toml"}


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
    mismatched = []
    for name, doc in documents.items():
        path, text = args.out_dir / FILES[name], render_toml(doc, seed=args.seed)
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
