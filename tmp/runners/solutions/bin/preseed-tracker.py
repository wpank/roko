#!/usr/bin/env python3
"""
preseed-tracker.py — flip ISSUE-TRACKER.md rows to [x] from a YAML/INI list.

Use this once after `generate-prompts.py` to mark items already shipped by
prior runners (mega-parity, post-parity) before this runner existed. After
that, use `sync-tracker.py` which reads commit-message trailers.

Input format: simple newline-delimited list of `BATCH_ID  notes`. Lines
starting with `#` are ignored.

Example:
  STAB_32     done in post-parity(PG_02): "Fix model '-' in TUI"
  PERF_03     covered by post-parity(PB_*) shared HTTP client wiring

Usage:
  python3 bin/preseed-tracker.py preseed.txt           # dry-run
  python3 bin/preseed-tracker.py preseed.txt --apply   # write
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

RUNNER_DIR = Path(__file__).resolve().parents[1]
TRACKER_MD = RUNNER_DIR / "ISSUE-TRACKER.md"

LINE_RE = re.compile(r"^([A-Z][A-Z0-9_]+_[0-9]+)\s*(.*)$")


def parse_preseed(path: Path) -> dict[str, str]:
    out: dict[str, str] = {}
    for raw in path.read_text().splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        m = LINE_RE.match(line)
        if not m:
            print(f"[warn] skipping unparseable line: {raw}", file=sys.stderr)
            continue
        out[m.group(1)] = m.group(2).strip() or "preseed"
    return out


def flip(text: str, ids: dict[str, str]) -> tuple[str, list[str], list[str]]:
    flipped: list[str] = []
    not_found = list(ids.keys())
    out: list[str] = []
    for line in text.splitlines():
        m = re.match(
            r'^(\| <a id="([a-z0-9-]+)"></a> )\[ \] (`([A-Z][A-Z0-9_]+_\d+)`)(\s*\|.*)$',
            line,
        )
        if m:
            bid = m.group(3)
            if bid in ids:
                # Keep HTML comment immediately after the batch-id cell so
                # `generate-prompts.py` can round-trip the note via
                # `load_tracker_state()`.
                line = (
                    f"{m.group(1)}[x] `{bid}`  <!-- preseed: {ids[bid]} -->{m.group(4)}"
                )
                flipped.append(bid)
                if bid in not_found:
                    not_found.remove(bid)
        out.append(line)
    return "\n".join(out) + ("\n" if not text.endswith("\n") else ""), flipped, not_found


def main(argv: list[str]) -> int:
    p = argparse.ArgumentParser()
    p.add_argument("preseed_file", help="newline-delimited BATCH_ID list")
    p.add_argument("--apply", action="store_true", help="write changes")
    args = p.parse_args(argv)

    pre_path = Path(args.preseed_file)
    if not pre_path.exists():
        print(f"[error] {pre_path} not found", file=sys.stderr)
        return 1

    if not TRACKER_MD.exists():
        print(f"[error] {TRACKER_MD} not found", file=sys.stderr)
        return 1

    ids = parse_preseed(pre_path)
    if not ids:
        print("[info] preseed file is empty")
        return 0

    text = TRACKER_MD.read_text()
    new_text, flipped, missing = flip(text, ids)

    print(f"[info] {len(flipped)} rows would flip to [x]")
    for bid in flipped[:20]:
        print(f"   - {bid}: {ids[bid]}")
    if missing:
        print(f"[warn] {len(missing)} preseed IDs had no tracker row:")
        for bid in missing:
            print(f"   - {bid}")

    if args.apply:
        TRACKER_MD.write_text(new_text)
        print(f"[done] wrote {TRACKER_MD}")
    else:
        print("[info] dry-run; pass --apply to write")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
