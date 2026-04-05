#!/usr/bin/env python3
"""Local patch validator for SWE-bench predictions.jsonl.

Runs `git apply --check` against the correct base_commit for every
prediction, and reports:

  - format_valid   : prediction contains a recognizable unified diff
  - apply_check    : `git apply --check` accepts the patch (would apply cleanly)
  - patch_bytes    : distribution of patch sizes
  - files_touched  : did the prediction touch the oracle file(s)?

This is NOT a substitute for the official SWE-bench harness — test pass
rates require Docker + specific test execution. But it's a fast,
Docker-free proxy: a prediction that can't even apply cleanly has
zero chance of resolving the instance, so apply_check ≈ upper-bound on
resolved rate.

Usage:
    python3 swebench_validate.py \\
      --predictions /tmp/preds.jsonl \\
      --dataset princeton-nlp/SWE-bench_Lite

    # Compare two prediction files side-by-side:
    python3 swebench_validate.py \\
      --predictions /tmp/preds-harness.jsonl /tmp/preds-baseline.jsonl
"""
import argparse
import json
import re
import statistics
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from swebench_run import oracle_files, clone_repo  # type: ignore


def parse_diff_files(patch: str) -> list[str]:
    """Which files does this unified diff touch?"""
    return re.findall(r"^diff --git a/(\S+) b/\S+", patch, re.MULTILINE)


def format_valid(patch: str) -> bool:
    """Does this look like a syntactically plausible unified diff?"""
    if not patch.strip():
        return False
    has_header = bool(re.search(r"^diff --git a/\S+ b/\S+", patch, re.MULTILINE))
    has_hunk = bool(re.search(r"^@@ -\d+(?:,\d+)? \+\d+(?:,\d+)? @@", patch,
                              re.MULTILINE))
    return has_header and has_hunk


def git_apply_check(repo_dir: Path, patch: str) -> tuple[bool, str]:
    """Run `git apply --check` on the patch. Returns (ok, stderr)."""
    res = subprocess.run(
        ["git", "apply", "--check", "-"],
        cwd=repo_dir, input=patch, text=True,
        capture_output=True,
    )
    return res.returncode == 0, res.stderr.strip()


def validate_one(
    instance: dict, prediction: dict, work_root: Path,
) -> dict:
    """Validate one prediction against its instance. Returns a stats dict."""
    instance_id = instance["instance_id"]
    patch = prediction.get("model_patch", "") or ""
    touched = parse_diff_files(patch)
    oracle = oracle_files(instance["patch"])
    touches_oracle = any(f in set(oracle) for f in touched)

    result = {
        "instance_id": instance_id,
        "format_valid": format_valid(patch),
        "patch_bytes": len(patch),
        "files_touched": len(touched),
        "touches_oracle": touches_oracle,
        "apply_check": False,
        "apply_error": "",
    }
    if not result["format_valid"]:
        result["apply_error"] = "format invalid (no diff header or hunk marker)"
        return result
    if not patch:
        return result

    # Need a repo at base_commit to run `git apply --check` against.
    workdir = work_root / instance_id
    if not workdir.exists() or not (workdir / ".git").exists():
        workdir.mkdir(parents=True, exist_ok=True)
        # Remove and re-clone if the parent dir exists but isn't a git repo.
        subprocess.run(["rm", "-rf", str(workdir)], check=False)
        workdir.mkdir(parents=True, exist_ok=True)
        if not clone_repo(instance["repo"], instance["base_commit"], workdir):
            result["apply_error"] = "clone failed"
            return result

    ok, err = git_apply_check(workdir, patch)
    result["apply_check"] = ok
    result["apply_error"] = err if not ok else ""
    return result


def summarize(rows: list[dict], label: str) -> None:
    """Pretty-print summary stats for one set of validated predictions."""
    if not rows:
        print(f"\n{label}: no rows\n")
        return
    n = len(rows)
    format_ok = sum(r["format_valid"] for r in rows)
    apply_ok = sum(r["apply_check"] for r in rows)
    oracle_hit = sum(r["touches_oracle"] for r in rows)
    sizes = [r["patch_bytes"] for r in rows if r["patch_bytes"] > 0]
    avg_size = int(statistics.mean(sizes)) if sizes else 0
    med_size = int(statistics.median(sizes)) if sizes else 0

    print(f"\n=== {label} ({n} predictions) ===")
    print(f"  format_valid    : {format_ok}/{n}   ({100*format_ok/n:.1f}%)")
    print(f"  apply_check_ok  : {apply_ok}/{n}   ({100*apply_ok/n:.1f}%)")
    print(f"  touches_oracle  : {oracle_hit}/{n}   ({100*oracle_hit/n:.1f}%)")
    if sizes:
        print(f"  patch bytes     : avg={avg_size} median={med_size} "
              f"min={min(sizes)} max={max(sizes)}")
    else:
        print(f"  patch bytes     : (all empty)")

    # Show first few failures for debugging.
    failures = [r for r in rows if not r["apply_check"]][:3]
    if failures:
        print(f"  first few failures:")
        for r in failures:
            err = (r["apply_error"] or "-").split("\n")[0][:80]
            print(f"    - {r['instance_id']}: {err}")


def load_predictions(path: Path) -> list[dict]:
    preds = []
    for line in path.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            preds.append(json.loads(line))
        except json.JSONDecodeError as e:
            print(f"WARNING: bad JSONL line in {path}: {e}", file=sys.stderr)
    return preds


def main() -> int:
    ap = argparse.ArgumentParser(description="Local SWE-bench prediction validator")
    ap.add_argument("--predictions", nargs="+", required=True,
                    help="One or more predictions.jsonl paths to validate/compare")
    ap.add_argument("--dataset", default="princeton-nlp/SWE-bench_Lite")
    ap.add_argument("--split", default="test")
    ap.add_argument("--workdir-root", default="/tmp/roko-swe-workdirs",
                    help="Reuses clones from swebench_run.py by default")
    ap.add_argument("--json-out", default="",
                    help="Optional path to dump per-prediction stats as JSONL")
    args = ap.parse_args()

    try:
        from datasets import load_dataset  # type: ignore
    except ImportError:
        print("ERROR: pip install datasets", file=sys.stderr)
        return 1

    print(f"loading {args.dataset}...", flush=True)
    ds = load_dataset(args.dataset, split=args.split)
    instances_by_id = {inst["instance_id"]: inst for inst in ds}

    work_root = Path(args.workdir_root)
    work_root.mkdir(parents=True, exist_ok=True)

    all_stats: list[tuple[str, list[dict]]] = []
    for pred_path in args.predictions:
        path = Path(pred_path)
        if not path.exists():
            print(f"ERROR: {path} not found", file=sys.stderr)
            return 1
        preds = load_predictions(path)
        print(f"\nvalidating {len(preds)} predictions in {path}...", flush=True)

        rows: list[dict] = []
        for pred in preds:
            instance_id = pred["instance_id"]
            inst = instances_by_id.get(instance_id)
            if inst is None:
                print(f"  WARNING: {instance_id} not in dataset", file=sys.stderr)
                continue
            r = validate_one(inst, pred, work_root)
            rows.append(r)
            mark = "OK " if r["apply_check"] else ("FMT" if not r["format_valid"] else "FAIL")
            print(f"  [{mark}] {instance_id} ({r['patch_bytes']}B)", flush=True)
        all_stats.append((str(path), rows))

    for label, rows in all_stats:
        summarize(rows, label)

    if args.json_out:
        with Path(args.json_out).open("w") as f:
            for label, rows in all_stats:
                for r in rows:
                    f.write(json.dumps({"source": label, **r}) + "\n")
        print(f"\nwrote per-prediction stats to {args.json_out}", flush=True)

    # If comparing 2+ files, print a delta.
    if len(all_stats) >= 2:
        print("\n=== A/B delta ===")
        labels = [lbl.split("/")[-1] for lbl, _ in all_stats]
        print(f"{'metric':<18}" + "".join(f"{lbl:>28}" for lbl in labels) + "  delta(2-1)")
        def rate(rows, key):
            return 100 * sum(r[key] for r in rows) / len(rows) if rows else 0
        for key in ("format_valid", "apply_check", "touches_oracle"):
            vals = [rate(rows, key) for _, rows in all_stats]
            delta = f"{vals[1] - vals[0]:+.1f}pp" if len(vals) >= 2 else ""
            print(f"{key:<18}"
                  + "".join(f"{v:>27.1f}%" for v in vals)
                  + f"  {delta}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
