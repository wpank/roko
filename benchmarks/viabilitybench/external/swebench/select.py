#!/usr/bin/env python3
"""The seeded P1-ext selection: 60 SWE-bench Verified tasks (S08 §4.8; task 3332).

`select(records, seed=SEED)` filters the full SWE-bench Verified corpus to the two easiest difficulty classes,
small patches (at most `MAX_FAIL_TO_PASS` FAIL_TO_PASS tests, a gold patch of at most `MAX_PATCH_LINES` changed
lines in at most `MAX_PATCH_FILES` files), then draws a stratified sample of `N` from the eligible pool: a
keyed shuffle per repository (`common.hmac_seed`, so the draw is the same on every host), at most
`MAX_PER_REPO` per repository, filled repository by repository in the shuffle's order until `N` is reached or
the pool is exhausted, refusing a selection with fewer than `MIN_REPOS` repositories. The selection log records
every eligible instance's repository and whether it was drawn, so the sample is auditable without the dataset.

`fetch_dataset` fetches the pinned revision once, offline tests never call it (the module docstring's `select`
is exercised against a fixture instead, as 3332's own Plan asks).

Usage:
    select.py --dataset PATH.jsonl [--seed 20260928] [--out selection.json]
  PATH.jsonl holds one SWE-bench Verified record per line (`instance_id`, `repo`, `difficulty`, `FAIL_TO_PASS`
  (a JSON list), `patch` (the gold diff)); writes the selection log as JSON (stdout with `--out -`).

API:
    SEED = 20260928; N = 60; DIFFICULTY_EASY; MAX_FAIL_TO_PASS = 5; MAX_PATCH_LINES = 40; MAX_PATCH_FILES = 2
    MAX_PER_REPO = 12; MIN_REPOS = 5
    SelectionError
    patch_files(patch: str) -> list[str]                 # the files a unified diff touches, in diff order
    patch_changed_lines(patch: str) -> int                # added + removed lines, diff/hunk headers excluded
    eligible(record: Mapping) -> bool
    select(records: Sequence[Mapping], seed: int = SEED, n: int = N, max_per_repo: int = MAX_PER_REPO,
          min_repos: int = MIN_REPOS) -> SelectionResult
    SelectionResult(instances: list[dict], log: list[dict]); .instance_ids -> list[str]; .as_json() -> dict
    fetch_dataset(revision: str, out_dir: Path) -> Path    # needs the network; no test calls it
    main(argv=None) -> int
"""

from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from pathlib import Path

_FAMILIES_DIR = Path(__file__).resolve().parents[2] / "families"
if str(_FAMILIES_DIR) not in sys.path:
    sys.path.insert(0, str(_FAMILIES_DIR))
from common import hmac_seed  # noqa: E402

SEED = 20260928
N = 60
# SWE-bench Verified's own human-annotated difficulty classes, easiest first (S08 §4.8: "the two easiest").
DIFFICULTY_EASY = ("<15 min fix", "15 min - 1 hour")
MAX_FAIL_TO_PASS = 5
MAX_PATCH_LINES = 40
MAX_PATCH_FILES = 2
MAX_PER_REPO = 12
MIN_REPOS = 5
DATASET_REVISION_ENV = "VB_SWEBENCH_REVISION"


class SelectionError(ValueError):
    """The eligible pool cannot give `n` instances across at least `min_repos` repositories."""


def patch_files(patch: str) -> list[str]:
    """The files a unified diff touches (`diff --git a/X b/Y`'s `b/Y`, in diff order, each once)."""
    files: list[str] = []
    for line in patch.splitlines():
        if line.startswith("diff --git "):
            parts = line.split(" ")
            if len(parts) >= 4 and parts[-1].startswith("b/"):
                path = parts[-1].removeprefix("b/")
                if path not in files:
                    files.append(path)
    return files


def patch_changed_lines(patch: str) -> int:
    """Added and removed lines of a unified diff; `+++`/`---` file headers and `@@` hunk headers do not count."""
    count = 0
    for line in patch.splitlines():
        if line.startswith(("+++", "---", "@@")):
            continue
        if line.startswith(("+", "-")):
            count += 1
    return count


def eligible(record: Mapping) -> bool:
    """Whether `record` passes every S08 §4.8 filter: difficulty, FAIL_TO_PASS count, patch size and file count."""
    if record.get("difficulty") not in DIFFICULTY_EASY:
        return False
    fail_to_pass = record.get("FAIL_TO_PASS") or []
    if not isinstance(fail_to_pass, (list, tuple)) or not 1 <= len(fail_to_pass) <= MAX_FAIL_TO_PASS:
        return False
    patch = record.get("patch")
    if not isinstance(patch, str) or not patch.strip():
        return False
    files = patch_files(patch)
    return 1 <= len(files) <= MAX_PATCH_FILES and patch_changed_lines(patch) <= MAX_PATCH_LINES


@dataclass(frozen=True)
class SelectionResult:
    instances: list[dict]
    log: list[dict]  # every eligible instance: {"instance_id", "repo", "selected"}

    @property
    def instance_ids(self) -> list[str]:
        return [instance["instance_id"] for instance in self.instances]

    def as_json(self) -> dict:
        return {"ev": "swebench.selection", "seed": self.log and self.log[0].get("seed"), "n": len(self.instances),
                "instance_ids": self.instance_ids, "log": self.log}


def select(records: Sequence[Mapping], seed: int = SEED, n: int = N, max_per_repo: int = MAX_PER_REPO,
          min_repos: int = MIN_REPOS) -> SelectionResult:
    """`n` instances from `records`' eligible pool, stratified by repository and reproducible from `seed`."""
    pool = [record for record in records if eligible(record)]
    by_repo: dict[str, list[Mapping]] = {}
    for record in pool:
        by_repo.setdefault(record["repo"], []).append(record)
    root = hmac_seed.surface_stream("swebench-select", str(seed))
    shuffled_repos = root.child("repos").shuffled(sorted(by_repo))
    for repo in shuffled_repos:
        by_repo[repo] = root.child(f"repo/{repo}").shuffled(by_repo[repo])

    selected: list[Mapping] = []
    taken: dict[str, int] = {repo: 0 for repo in by_repo}
    progressed = True
    while len(selected) < n and progressed:
        progressed = False
        for repo in shuffled_repos:
            if len(selected) >= n:
                break
            if taken[repo] < max_per_repo and taken[repo] < len(by_repo[repo]):
                selected.append(by_repo[repo][taken[repo]])
                taken[repo] += 1
                progressed = True
    if len(selected) < n or len({record["repo"] for record in selected}) < min_repos:
        raise SelectionError(f"the eligible pool ({len(pool)} across {len(by_repo)} repositories) cannot give "
                             f"{n} instances across at least {min_repos} repositories (got {len(selected)} "
                             f"across {len({r['repo'] for r in selected})})")
    chosen_ids = {record["instance_id"] for record in selected}
    log = [{"instance_id": record["instance_id"], "repo": record["repo"], "selected": record["instance_id"] in
           chosen_ids, "seed": seed} for record in pool]
    return SelectionResult(instances=[dict(record) for record in selected], log=log)


def fetch_dataset(revision: str, out_dir: Path) -> Path:
    """Fetch SWE-bench Verified at `revision` into `out_dir` once, recording its sha256 beside it. Needs the
    network; no test here calls it (3332's Plan item 4: offline tests run against a fixture instead)."""
    import hashlib
    import urllib.request

    out_dir = Path(out_dir)
    out_dir.mkdir(mode=0o700, parents=True, exist_ok=True)
    dest = out_dir / f"swebench-verified-{revision}.jsonl"
    if dest.is_file():
        return dest
    url = f"https://huggingface.co/datasets/princeton-nlp/SWE-bench_Verified/resolve/{revision}/data/test-00000-of-00001.parquet"
    with urllib.request.urlopen(url, timeout=120) as response:  # noqa: S310 (a pinned, operator-chosen URL)
        data = response.read()
    dest.write_bytes(data)
    (out_dir / f"{dest.name}.sha256").write_text(hashlib.sha256(data).hexdigest() + "\n", encoding="utf-8")
    return dest


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--dataset", type=Path, required=True, help="a JSONL file, one SWE-bench record per line")
    parser.add_argument("--seed", type=int, default=SEED)
    parser.add_argument("--out", default=None, help="the selection log's JSON path; '-' for stdout (default)")
    args = parser.parse_args(argv)
    try:
        records = [json.loads(line) for line in args.dataset.read_text(encoding="utf-8").splitlines() if line.strip()]
        result = select(records, args.seed)
    except (OSError, ValueError, KeyError) as err:
        print(f"select: {err}", file=sys.stderr)
        return 2
    text = json.dumps(result.as_json(), indent=2, ensure_ascii=False) + "\n"
    if args.out in (None, "-"):
        sys.stdout.write(text)
    else:
        Path(args.out).write_text(text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
