#!/usr/bin/env python3
"""SWE-bench-Lite runner for roko-cli.

Loads SWE-bench-Lite tasks from HuggingFace, runs each through roko-cli
with oracle file retrieval, and writes predictions.jsonl compatible with
the official SWE-bench evaluation harness.

Usage:
    pip install datasets swebench
    python swebench_run.py --model llama3.2:latest --limit 5

This is a minimal, single-shot driver — no tool use, no multi-turn, no
repo exploration. Each task gets one shot at producing a unified diff.
Expect very low scores with small local models (<7B params). See the
crate README "Expected scores" section for context.
"""
import argparse
import json
import os
import re
import subprocess
import sys
from pathlib import Path


def oracle_files(patch: str) -> list[str]:
    """Extract file paths touched by a unified diff (the gold patch)."""
    return re.findall(r"^diff --git a/(\S+) b/\S+", patch, re.MULTILINE)


def clone_repo(repo: str, commit: str, dest: Path, *, depth: int = 100) -> bool:
    """Clone `repo` at `commit` into `dest`. Returns True on success."""
    url = f"https://github.com/{repo}.git"
    # Try shallow clone first; if commit isn't in recent history, unshallow.
    res = subprocess.run(
        ["git", "clone", "--quiet", f"--depth={depth}", url, str(dest)],
        capture_output=True, text=True,
    )
    if res.returncode != 0:
        print(f"    clone failed: {res.stderr.strip()}", flush=True)
        return False
    checkout = subprocess.run(
        ["git", "checkout", "--quiet", commit],
        cwd=dest, capture_output=True, text=True,
    )
    if checkout.returncode != 0:
        # Unshallow and retry
        subprocess.run(["git", "fetch", "--quiet", "--unshallow"], cwd=dest)
        checkout = subprocess.run(
            ["git", "checkout", "--quiet", commit],
            cwd=dest, capture_output=True, text=True,
        )
        if checkout.returncode != 0:
            print(f"    checkout {commit[:8]} failed: {checkout.stderr.strip()}",
                  flush=True)
            return False
    return True


def extract_patch(text: str) -> str:
    """Pull a unified diff out of the model's free-form response."""
    # Prefer fenced diff/patch blocks.
    for fence in ("diff", "patch", ""):
        pat = rf"```{fence}\s*\n(.*?)\n```"
        for m in re.finditer(pat, text, re.DOTALL):
            candidate = m.group(1).strip()
            if candidate.startswith("diff --git") or candidate.startswith("--- "):
                return candidate
    # Fallback: everything from the first `diff --git` line to EOF.
    m = re.search(r"^diff --git .*$", text, re.MULTILINE)
    if m:
        return text[m.start():].strip()
    return ""


def build_config(role: str, model_cmd: str, model_args: list[str],
                 files: list[str], token_budget: int, timeout_ms: int,
                 file_hard_cap: int, *, clean_output: bool = True,
                 inject_files: bool = True, use_hard_cap: bool = True) -> str:
    """Generate a roko.toml for one SWE-bench task.

    Ablation knobs:
      clean_output   : on/off for ANSI + thinking-trace stripping
      inject_files   : on/off for `[[prompt.files]]` entries
      use_hard_cap   : on/off for per-file token caps
    """
    if inject_files:
        if use_hard_cap:
            file_entries = ",\n  ".join(
                f'{{ path = {json.dumps(f)}, name = {json.dumps(f)}, '
                f'priority = "high", hard_cap = {file_hard_cap} }}'
                for f in files
            )
        else:
            file_entries = ",\n  ".join(
                f'{{ path = {json.dumps(f)}, name = {json.dumps(f)}, '
                f'priority = "high" }}'
                for f in files
            )
        files_block = f"files = [\n  {file_entries}\n]"
    else:
        files_block = "files = []"
    role_escaped = role.replace("\\", "\\\\").replace('"', '\\"')
    return f"""[agent]
command = {json.dumps(model_cmd)}
args = {json.dumps(model_args)}
timeout_ms = {timeout_ms}
clean_output = {"true" if clean_output else "false"}

[prompt]
token_budget = {token_budget}
role = "{role_escaped}"
{files_block}
"""


def find_latest_agent_output(signals_jsonl: Path) -> str:
    """Find the most recent cleaned AgentOutput signal's body text."""
    best_text = ""
    best_ts = -1
    best_is_cleaned = False
    for line in signals_jsonl.read_text().splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            sig = json.loads(line)
        except json.JSONDecodeError:
            continue
        if sig.get("kind") != "agent_output":
            continue
        is_cleaned = sig.get("tags", {}).get("cleaned") == "true"
        ts = sig.get("created_at_ms", 0)
        # Prefer cleaned; within cleaned/raw cohorts, prefer latest.
        if is_cleaned and not best_is_cleaned:
            best_text = sig["body"]["data"]
            best_ts = ts
            best_is_cleaned = True
        elif is_cleaned == best_is_cleaned and ts > best_ts:
            best_text = sig["body"]["data"]
            best_ts = ts
    return best_text


RICH_ROLE = (
    "You are a senior software engineer fixing a bug. You will see "
    "an issue description and the current contents of the files that "
    "need editing. Produce a git-apply-compatible unified diff that "
    "resolves the issue. Respond with ONLY the patch, inside a single "
    "```diff code block. Do not explain your reasoning."
)

MINIMAL_ROLE = "Fix the bug. Output only a unified diff."


def run_one(instance: dict, roko_bin: str, model_cmd: str,
            model_args: list[str], work_root: Path, *,
            token_budget: int, timeout_ms: int, file_hard_cap: int,
            clean_output: bool = True, inject_files: bool = True,
            use_hard_cap: bool = True, minimal_role: bool = False,
            suffix: str = "") -> dict | None:
    """Process one SWE-bench instance. Returns a prediction dict or None."""
    instance_id = instance["instance_id"]
    print(f"\n=== {instance_id} ===", flush=True)

    files = oracle_files(instance["patch"])
    if not files:
        print("    skip: no files in gold patch", flush=True)
        return None
    print(f"    oracle files: {files}", flush=True)

    workdir = work_root / instance_id
    if workdir.exists():
        # Fresh every run — avoid stale state.
        subprocess.run(["rm", "-rf", str(workdir)], check=False)
    workdir.mkdir(parents=True, exist_ok=True)

    if not clone_repo(instance["repo"], instance["base_commit"], workdir):
        return None

    # Keep only files that actually exist at base_commit.
    existing = [f for f in files if (workdir / f).exists()]
    if not existing:
        print("    skip: no oracle files exist at base_commit", flush=True)
        return None

    role = MINIMAL_ROLE if minimal_role else RICH_ROLE
    config_toml = build_config(
        role, model_cmd, model_args, existing,
        token_budget=token_budget, timeout_ms=timeout_ms,
        file_hard_cap=file_hard_cap, clean_output=clean_output,
        inject_files=inject_files, use_hard_cap=use_hard_cap,
    )
    (workdir / "roko.toml").write_text(config_toml)

    if inject_files:
        prompt = (
            f"# Issue\n\n{instance['problem_statement'].strip()}\n\n"
            "Respond with the patch only."
        )
    else:
        # No file injection → cram the files into the prompt itself so the
        # ablation is "same info, no harness-layer structure".
        bits = [f"# Issue\n\n{instance['problem_statement'].strip()}\n"]
        for f in existing:
            content = (workdir / f).read_text(errors="replace")
            bits.append(f"# File `{f}`\n\n```\n{content}\n```\n")
        bits.append("Respond with the patch only.")
        prompt = "\n".join(bits)

    res = subprocess.run(
        [roko_bin, "run", prompt, "--workdir", str(workdir)],
        capture_output=True, text=True,
    )
    # rc 0 = all gates pass, rc 1 = agent or gate failure. Both are expected
    # since we don't run any gates here (config has none).
    if res.returncode not in (0, 1):
        print(f"    roko run error rc={res.returncode}:\n{res.stderr}", flush=True)
        return None

    output_text = find_latest_agent_output(workdir / ".roko" / "signals.jsonl")
    if not output_text:
        print("    no AgentOutput signal found", flush=True)
        return None

    patch = extract_patch(output_text)
    if not patch:
        print(f"    no patch extracted (output was {len(output_text)} bytes)",
              flush=True)
        patch = ""

    print(f"    patch extracted: {len(patch)} bytes", flush=True)
    tag = f"roko-cli/{model_cmd}:{' '.join(model_args)}"
    if suffix:
        tag = f"{tag} [{suffix}]"
    return {
        "instance_id": instance_id,
        "model_name_or_path": tag,
        "model_patch": patch,
    }


def main() -> int:
    ap = argparse.ArgumentParser(
        description="SWE-bench-Lite runner for roko-cli (single-shot, oracle retrieval)",
    )
    ap.add_argument("--model", required=True,
                    help="Model tag (e.g. llama3.2:latest, gemma4:26b-moe-8k)")
    ap.add_argument("--backend", default="ollama",
                    help="Backend CLI command (default: ollama)")
    ap.add_argument("--dataset", default="princeton-nlp/SWE-bench_Lite",
                    help="HF dataset (default: SWE-bench_Lite)")
    ap.add_argument("--split", default="test")
    ap.add_argument("--limit", type=int, default=5,
                    help="How many instances to run (default: 5)")
    ap.add_argument("--offset", type=int, default=0,
                    help="Start from this index (default: 0)")
    ap.add_argument("--output", default="predictions.jsonl",
                    help="Output predictions file (default: predictions.jsonl)")
    ap.add_argument("--workdir-root", default="/tmp/roko-swe-workdirs",
                    help="Per-instance workdirs go here")
    ap.add_argument("--token-budget", type=int, default=20000)
    ap.add_argument("--timeout-ms", type=int, default=600_000,
                    help="Per-task model timeout (default: 10 min)")
    ap.add_argument("--file-hard-cap", type=int, default=4000,
                    help="Per-file token cap (default: 4000 tokens)")
    ap.add_argument("--roko-bin", default="roko",
                    help="Path to the roko binary (default: 'roko' from PATH)")
    ap.add_argument("--instance-ids", nargs="*",
                    help="Only run these specific instance_ids (overrides limit/offset)")
    # Ablation flags — turn off individual harness features to attribute value.
    ap.add_argument("--no-clean-output", action="store_true",
                    help="Disable ANSI + thinking-trace stripping")
    ap.add_argument("--no-file-injection", action="store_true",
                    help="Don't use [[prompt.files]] — cram files into the "
                         "prompt text instead (ablates the section abstraction)")
    ap.add_argument("--no-hard-cap", action="store_true",
                    help="Don't cap per-file tokens (let the composer budget "
                         "decide alone)")
    ap.add_argument("--minimal-role", action="store_true",
                    help="Use a 1-line role instead of the structured system role")
    ap.add_argument("--suffix", default="",
                    help="Append to model_name_or_path for A/B labeling "
                         "(e.g. 'ablate-clean', 'full-harness')")
    args = ap.parse_args()

    try:
        from datasets import load_dataset  # type: ignore
    except ImportError:
        print("ERROR: huggingface datasets is required: pip install datasets",
              file=sys.stderr)
        return 1

    # Verify roko binary exists
    try:
        subprocess.run([args.roko_bin, "--version"], capture_output=True, check=True)
    except (subprocess.CalledProcessError, FileNotFoundError):
        print(f"ERROR: cannot run '{args.roko_bin} --version' — "
              f"install the roko binary first (cargo install --path crates/roko-cli)",
              file=sys.stderr)
        return 1

    print(f"loading {args.dataset} ({args.split})...", flush=True)
    ds = load_dataset(args.dataset, split=args.split)
    print(f"dataset size: {len(ds)}", flush=True)

    work_root = Path(args.workdir_root)
    work_root.mkdir(parents=True, exist_ok=True)
    out_path = Path(args.output)
    if out_path.exists():
        print(f"NOTE: appending to existing {out_path} "
              f"(delete it first for a clean run)", flush=True)

    model_args = ["run", args.model] if args.backend == "ollama" else []
    model_cmd = args.backend

    # Select instances
    if args.instance_ids:
        wanted = set(args.instance_ids)
        instances = [ds[i] for i in range(len(ds)) if ds[i]["instance_id"] in wanted]
        missing = wanted - {inst["instance_id"] for inst in instances}
        if missing:
            print(f"WARNING: {len(missing)} unknown instance_ids: "
                  f"{sorted(missing)[:5]}...", file=sys.stderr)
    else:
        end = min(args.offset + args.limit, len(ds))
        instances = [ds[i] for i in range(args.offset, end)]

    clean_output = not args.no_clean_output
    inject_files = not args.no_file_injection
    use_hard_cap = not args.no_hard_cap
    minimal_role = args.minimal_role

    print(f"running {len(instances)} instances with {model_cmd} {' '.join(model_args)}",
          flush=True)
    print(
        f"  ablation: clean_output={clean_output} inject_files={inject_files} "
        f"hard_cap={use_hard_cap} minimal_role={minimal_role} "
        f"suffix={args.suffix or '(none)'}",
        flush=True,
    )

    ok = 0
    skipped = 0
    for inst in instances:
        pred = run_one(
            inst, args.roko_bin, model_cmd, model_args, work_root,
            token_budget=args.token_budget, timeout_ms=args.timeout_ms,
            file_hard_cap=args.file_hard_cap, clean_output=clean_output,
            inject_files=inject_files, use_hard_cap=use_hard_cap,
            minimal_role=minimal_role, suffix=args.suffix,
        )
        if pred is None:
            skipped += 1
            continue
        ok += 1
        with out_path.open("a") as f:
            f.write(json.dumps(pred) + "\n")

    run_id = f"{model_cmd}_{args.model}".replace("/", "_").replace(":", "_")
    print(
        f"\n=== done: {ok} predictions, {skipped} skipped → {out_path} ===",
        flush=True,
    )
    print("\nTo score with the official SWE-bench harness:", flush=True)
    print("  pip install swebench", flush=True)
    print("  python -m swebench.harness.run_evaluation \\", flush=True)
    print(f"    --predictions_path {out_path} \\", flush=True)
    print(f"    --dataset_name {args.dataset} \\", flush=True)
    print(f"    --run_id {run_id} \\", flush=True)
    print("    --max_workers 4", flush=True)
    print("\nRequires Docker: the harness runs each repo's tests inside a container.",
          flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
