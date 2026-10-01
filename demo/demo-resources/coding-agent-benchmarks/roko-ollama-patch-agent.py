#!/usr/bin/env python3
"""Command adapter for `roko bench swe --agent-mode command`.

The benchmark harness sends one instance JSON object on stdin and expects a
unified diff on stdout. This adapter runs `roko run` against an isolated copy of
the benchmark repo, then prints the resulting `git diff`.

The harness keeps its own test command and grading tests from agents, so the
agent checks its edits with `--validate-cmd` when given, or else with the repo's
visible unittest tests. With neither, the adapter exits with an error: a check
that cannot fail is worse than none.
"""

import argparse
import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parents[2]


def run(cmd, cwd=None, input_text=None, timeout=120):
    return subprocess.run(
        cmd,
        cwd=cwd,
        input=input_text,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )


def file_context(root):
    chunks = []
    for path in sorted(root.iterdir()):
        if not path.is_file() or path.name == "roko.toml":
            continue
        try:
            text = path.read_text()
        except UnicodeDecodeError:
            continue
        chunks.append(f"### {path.name}\n```text\n{text}\n```")
    return "\n\n".join(chunks)


def query_knowledge(roko_bin, knowledge_workdir, topic):
    if not knowledge_workdir:
        return ""
    result = run(
        [
            str(Path(roko_bin).resolve()),
            "knowledge",
            "query",
            topic,
            "--workdir",
            str(Path(knowledge_workdir).resolve()),
        ],
        timeout=30,
    )
    if result.returncode != 0:
        sys.stderr.write(result.stderr)
        return ""
    return result.stdout.strip()


def validation_command(explicit, workdir):
    """The shell command that checks the agent's edits, or None if there is none.

    The benchmark hides its test command, so this is the caller's command or
    the repo's own visible `test*.py` files run through unittest.
    """
    if explicit:
        return explicit
    if any(workdir.glob("test*.py")):
        return f"{shlex.quote(sys.executable)} -m unittest discover"
    return None


def write_roko_config(root, model, validate_cmd):
    escaped_test = json.dumps(["-lc", validate_cmd])
    root.joinpath("roko.toml").write_text(
        f"""[agent]
command = "ollama"
model = "{model}"
timeout_ms = 120000
bare_mode = true
clean_output = true

[prompt]
token_budget = 6000
role = "implementer"

[[gate]]
kind = "shell"
program = "sh"
args = {escaped_test}
timeout_ms = 60000
"""
    )


def build_prompt(instance, workdir, validate_cmd, mode, roko_bin, knowledge_workdir):
    problem = instance.get("problem_statement", "")
    prompt = (
        "Fix this small repository so the benchmark test passes. "
        "Edit the implementation files directly. Do not modify tests unless the problem explicitly asks for it.\n\n"
        f"Problem:\n{problem}\n\n"
        f"Validation command:\n{validate_cmd}\n"
        "The benchmark grades the patch with its own tests, which you cannot see.\n"
    )
    if mode in {"context", "neuro"}:
        context = file_context(workdir)
        if context:
            prompt += "\nRelevant repository context:\n" + context
    if mode == "neuro":
        topic = f"benchmark code repair {problem}"
        knowledge = query_knowledge(roko_bin, knowledge_workdir, topic)
        if knowledge:
            prompt += "\n\nRelevant learned knowledge:\n" + knowledge
        prompt += (
            "\n\nBenchmark guidance:\n"
            "- Make the smallest implementation change that satisfies the stated failing behavior.\n"
            "- Preserve public function names and test files.\n"
            "- Run or reason against the validation command before stopping.\n"
        )
    return prompt


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["minimal", "context", "neuro"], default="context")
    parser.add_argument(
        "--model",
        default=os.environ.get("ROKO_OLLAMA_MODEL", "llama3.2:latest"),
    )
    parser.add_argument(
        "--roko-bin",
        default=os.environ.get("ROKO_BIN") or os.environ.get("ROKO") or str(REPO_ROOT / "target/debug/roko"),
    )
    parser.add_argument(
        "--knowledge-workdir",
        default=os.environ.get("ROKO_KNOWLEDGE_WORKDIR", str(REPO_ROOT)),
        help="Workdir whose .roko/neuro store should be queried in neuro mode.",
    )
    parser.add_argument(
        "--validate-cmd",
        default=os.environ.get("ROKO_BENCH_VALIDATE_CMD"),
        help="Shell command that checks the agent's edits. Defaults to the repo's visible "
        "unittest tests; the benchmark's own test command is hidden from agents.",
    )
    args = parser.parse_args()

    instance = json.load(sys.stdin)
    source = Path(instance["repo_path"]).resolve()

    with tempfile.TemporaryDirectory(prefix="roko-bench-agent-") as tmp:
        workdir = Path(tmp) / "repo"
        shutil.copytree(source, workdir)
        validate_cmd = validation_command(args.validate_cmd, workdir)
        if validate_cmd is None:
            sys.stderr.write(
                f"no validation command for {instance['instance_id']}: the benchmark hides "
                "its tests, the repo has no visible test*.py, and no --validate-cmd was given\n"
            )
            sys.exit(2)
        run(["git", "init", "-q"], cwd=workdir)
        run(["git", "add", "."], cwd=workdir)
        write_roko_config(workdir, args.model, validate_cmd)

        prompt = build_prompt(
            instance, workdir, validate_cmd, args.mode, args.roko_bin, args.knowledge_workdir
        )
        result = run(
            [
                str(Path(args.roko_bin).resolve()),
                "--config",
                str(workdir / "roko.toml"),
                "--repo",
                str(workdir),
                "--quiet",
                "run",
                prompt,
            ],
            cwd=workdir,
            timeout=180,
        )
        if result.returncode != 0:
            sys.stderr.write(result.stderr)

        diff = run(["git", "diff", "--no-ext-diff", "--", "."], cwd=workdir).stdout
        sys.stdout.write(diff)


if __name__ == "__main__":
    main()
