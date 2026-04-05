#!/usr/bin/env python3
"""SWE-bench-Lite baseline runner — bypasses roko-cli entirely.

Runs the same oracle-retrieval task flow as `swebench_run.py`, but pipes
the prompt directly to `ollama run <model>` with zero roko-cli in the
loop. Output format is identical (predictions.jsonl), so you can A/B
the two and attribute any score delta to the harness.

This is the **control**. Pair with `swebench_run.py` (the harness) and
`swebench_validate.py` (the scorer) to answer "does my harness actually
help?" quantitatively.

What this baseline does NOT do:
  - no output cleaning (raw ollama stdout, including thinking trace)
  - no token budgeting (whole file contents dumped in)
  - no structured prompt sections — just issue + file contents concatenated
  - no per-file hard caps
  - no lineage / substrate / signals

What it DOES share with swebench_run.py:
  - same SWE-bench-Lite tasks (same dataset + oracle retrieval)
  - same repo clone at same base_commit
  - same patch-extraction regex
  - same output JSONL format
"""
import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

# Re-use pure helpers from the harness driver to keep A/B identical where
# possible — only the "generate output" step differs.
sys.path.insert(0, str(Path(__file__).parent))
from swebench_run import oracle_files, extract_patch, clone_repo  # type: ignore


def run_baseline_one(
    instance: dict,
    model_cmd: str,
    model_args: list[str],
    work_root: Path,
    *,
    timeout_s: int,
    role: str,
    max_file_bytes: int,
) -> dict | None:
    """Run one SWE-bench task through raw ollama. Returns a prediction dict."""
    instance_id = instance["instance_id"]
    print(f"\n=== {instance_id} ===", flush=True)

    files = oracle_files(instance["patch"])
    if not files:
        print("    skip: no files in gold patch", flush=True)
        return None
    print(f"    oracle files: {files}", flush=True)

    workdir = work_root / instance_id
    if workdir.exists():
        subprocess.run(["rm", "-rf", str(workdir)], check=False)
    workdir.mkdir(parents=True, exist_ok=True)
    if not clone_repo(instance["repo"], instance["base_commit"], workdir):
        return None

    existing = [f for f in files if (workdir / f).exists()]
    if not existing:
        print("    skip: no oracle files exist at base_commit", flush=True)
        return None

    # Build the raw prompt: role + issue + concatenated files.
    # No sections, no composer, no budget enforcement. Just dump it all.
    parts: list[str] = [role, ""]
    parts.append("# Issue\n\n" + instance["problem_statement"].strip())
    parts.append("")
    for f in existing:
        content = (workdir / f).read_text(errors="replace")
        if len(content.encode("utf-8")) > max_file_bytes:
            content = content[:max_file_bytes] + "\n\n[... truncated ...]"
        parts.append(f"# File `{f}`\n\n```\n{content}\n```\n")
    parts.append("")
    parts.append("Respond with the patch only, inside a single ```diff code block.")
    prompt = "\n".join(parts)

    cmd = [model_cmd, *model_args]
    print(f"    running: {' '.join(cmd)} (prompt: {len(prompt)} bytes)", flush=True)
    try:
        res = subprocess.run(
            cmd, input=prompt, capture_output=True, text=True, timeout=timeout_s,
        )
    except subprocess.TimeoutExpired:
        print(f"    timed out after {timeout_s}s", flush=True)
        return {
            "instance_id": instance_id,
            "model_name_or_path": f"baseline/{model_cmd}:{' '.join(model_args)}",
            "model_patch": "",
        }
    if res.returncode != 0:
        print(f"    subprocess rc={res.returncode}: {res.stderr[:200]}", flush=True)
        return {
            "instance_id": instance_id,
            "model_name_or_path": f"baseline/{model_cmd}:{' '.join(model_args)}",
            "model_patch": "",
        }

    output_text = res.stdout
    # Persist the raw output alongside the workdir, for inspection.
    (workdir / "raw_output.txt").write_text(output_text)
    patch = extract_patch(output_text)
    if not patch:
        print(f"    no patch extracted (output was {len(output_text)} bytes)",
              flush=True)
        patch = ""
    else:
        print(f"    patch extracted: {len(patch)} bytes", flush=True)
    return {
        "instance_id": instance_id,
        "model_name_or_path": f"baseline/{model_cmd}:{' '.join(model_args)}",
        "model_patch": patch,
    }


def main() -> int:
    ap = argparse.ArgumentParser(
        description="SWE-bench-Lite baseline runner (raw ollama, no roko-cli)",
    )
    ap.add_argument("--model", required=True, help="Ollama model tag")
    ap.add_argument("--backend", default="ollama")
    ap.add_argument("--dataset", default="princeton-nlp/SWE-bench_Lite")
    ap.add_argument("--split", default="test")
    ap.add_argument("--limit", type=int, default=5)
    ap.add_argument("--offset", type=int, default=0)
    ap.add_argument("--output", default="predictions-baseline.jsonl")
    ap.add_argument("--workdir-root", default="/tmp/roko-swe-baseline-workdirs")
    ap.add_argument("--timeout-s", type=int, default=600)
    ap.add_argument("--max-file-bytes", type=int, default=16000,
                    help="Per-file byte cap for prompt inclusion (default 16KB)")
    ap.add_argument("--role",
                    default="You are a senior software engineer fixing a bug. "
                            "Produce a git-apply-compatible unified diff that "
                            "resolves the issue. Respond with ONLY the patch "
                            "inside a ```diff code block.")
    ap.add_argument("--instance-ids", nargs="*")
    args = ap.parse_args()

    try:
        from datasets import load_dataset  # type: ignore
    except ImportError:
        print("ERROR: pip install datasets", file=sys.stderr)
        return 1

    print(f"loading {args.dataset} ({args.split})...", flush=True)
    ds = load_dataset(args.dataset, split=args.split)
    print(f"dataset size: {len(ds)}", flush=True)

    work_root = Path(args.workdir_root)
    work_root.mkdir(parents=True, exist_ok=True)
    out_path = Path(args.output)

    model_cmd = args.backend
    model_args = ["run", args.model] if args.backend == "ollama" else []

    if args.instance_ids:
        wanted = set(args.instance_ids)
        instances = [ds[i] for i in range(len(ds)) if ds[i]["instance_id"] in wanted]
    else:
        end = min(args.offset + args.limit, len(ds))
        instances = [ds[i] for i in range(args.offset, end)]

    print(f"running {len(instances)} BASELINE instances with "
          f"{model_cmd} {' '.join(model_args)}", flush=True)

    ok = 0
    skipped = 0
    for inst in instances:
        pred = run_baseline_one(
            inst, model_cmd, model_args, work_root,
            timeout_s=args.timeout_s, role=args.role,
            max_file_bytes=args.max_file_bytes,
        )
        if pred is None:
            skipped += 1
            continue
        ok += 1
        with out_path.open("a") as f:
            f.write(json.dumps(pred) + "\n")

    print(f"\n=== baseline done: {ok} predictions, {skipped} skipped → {out_path} ===",
          flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
