"""Emit a runnable one-task Roko plan and the workspace `roko.toml` for one (task, seed) (S08 §4.9, T11; B §2.3).

`emit(spec, workspace)` writes two files into the task's workdir, which is also Roko's workspace (`--repo`):

- `plans/<slug>/tasks.toml`: `meta.total = 1`, `max_parallel = 1` and an explicit `skip_enrichment`; one implementer
  task whose description is the spec text every arm gets, with `files` = the manifest's `files_in_scope`,
  `max_retries` (2: Roko's ≤ 3 attempts), `model_hint` = the pinned model, and exactly one `[[task.verify]]`: the
  manifest's visible check (several are joined with `&&`). In a run with `flaky_verify` the check runs through the
  visible-verify wrapper, as `'<vb-verify>' '<check>'` (`PlanSpec.verify_wrapper`, `vb_verify`), in the verify step
  and the gate rung alike. The slug is opaque (`vb-<sha256(key)[:10]>`) and the title is the spec's first heading,
  so Roko's prompt holds nothing the direct arm's task message does not.
- `roko.toml`: one provider and one model, the pinned one (its key equals its slug), with no fallback models, so
  nothing can fail over (bug-35379d); the routing ladder off (`[routing.ladder] enabled = false`), so the pin is
  the only routing input and `plan validate` has no ladder for the task's `model_hint` to bypass (PLAN_041,
  bug-a05c53); explicit `[[gates.rungs]]` holding only the visible check, so no path that
  reads rungs falls back to `cargo check` (bug-1410e8); the per-attempt turn cap in `[pipeline.<tier>]`; a budget at
  the arm's dollar cap, priced at the snapshot's rates rather than roko.toml's defaults (S08 D9), whose per-dispatch
  reservation (`max_turn_usd`, a tenth of it, with one agent) satisfies Roko's config invariants and still leaves
  room for every retry; retries fixed at
  `max_retries`; force-accept, cargo fix, replanning, playbook refresh and dreams off. `plan run` runs a workspace's
  required rungs after each task's own `[[task.verify]]` steps, but not a rung whose command a step already runs
  (gap-3506f1), so the visible check, which is both the verify step and the rung, runs once per attempt.

Agents see verify commands (A4), so no hidden check goes into either file: every command in them is one of the
manifest's visible commands, which the spec states, and `emit` refuses to write a file that holds a canary. The
authoring rules (memory `roko_plan_authoring_constraints.md`) are checked here too: no `depends_on_plan`, no
`read_files`, a non-empty `files` without glob characters, and scalars before sub-tables. Strings are written as
escaped TOML basic strings and each file is parsed back before it is written.

`emit` refuses a workspace that already has `roko.toml`, `plans/` or `.roko/`: the runner removes those after the
run, before the driver exports c_i, so they must be Roko's alone.

API:
    PlanSpec(...)                                   # frozen; see the fields
    emit(spec: PlanSpec, workspace: Path) -> Emitted              # raises PlanEmitError
    Emitted(slug, plan_dir, tasks_path, config_path, tasks_text, config_text, visible_command)
    plan_slug(key: str) -> str; SCAFFOLDING; TASK_ID; TEMPLATE_VERSION; TEMPLATE_SHA256
"""

from __future__ import annotations

import hashlib
import math
import re
import shlex
import tomllib
from dataclasses import dataclass
from pathlib import Path, PurePosixPath

import layout  # noqa: F401 (puts families/ on sys.path for common)
from common import canary

TEMPLATE_VERSION = "planemit-2"
TASK_ID = "T01"
SCAFFOLDING = ("roko.toml", "plans", ".roko")  # what Roko's run adds to the workspace; the runner removes it
ROLE = "implementer"
MAX_FILES = 32
GLOB = re.compile(r"[*?\[\]]")
MODEL_KEY = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]*")

TASKS_TEMPLATE = """\
# Emitted by the ViabilityBench driver ({version}) for one benchmark task. Roko's agent sees this file and its
# verify command; the benchmark's hidden checks never enter it.
[meta]
plan = {slug}
total = 1
done = 0
status = "ready"
max_parallel = 1
skip_enrichment = {skip_enrichment}

[[task]]
id = {task_id}
title = {title}
description = {description}
status = "ready"
role = {role}
tier = {tier}
files = {files}
depends_on = []
max_retries = {max_retries}
model_hint = {model}

[[task.verify]]
phase = "test"
command = {visible}
fail_msg = "the task's visible check failed"
"""

CONFIG_TEMPLATE = """\
# Emitted by the ViabilityBench driver ({version}) for one benchmark run: one provider, one pinned model, no
# fallbacks and no routing ladder, explicit gate rungs that run only the visible check, and Roko's learning loops
# held off.
config_version = 2
schema_version = 2

[agent]
default_model = {model}

[providers.{provider_key}]
kind = {provider_kind}
base_url = {base_url}
api_key_env = {api_key_env}

[models.{model_key}]
provider = {provider}
slug = {model}
context_window = {context_window}
max_output = {max_output}
tool_format = "openai_json"
{rates}
[routing]
fallback_models = []

[routing.ladder]
enabled = false

[gates]
cargo_fix_enabled = false
max_review_cycles = 0
adaptive_min_retries = {max_retries}
adaptive_max_retries = {max_retries}

[[gates.rungs]]
name = "visible"
command = {visible}
timeout_secs = {verify_timeout_s}
required = true

[pipeline.{tier_key}]
max_turns = {max_turns}

[conductor]
max_agents = 1

[budget]
max_plan_usd = {usd_cap}
max_task_retry_usd = {usd_cap}
max_turn_usd = {turn_usd}

[learning]
auto_playbook_refresh = false
replan_on_gate_failure = false
dream_on_completion = false

[learning.dreams]
trigger_on_plan_complete = false
"""

TEMPLATE_SHA256 = hashlib.sha256((TASKS_TEMPLATE + "\0" + CONFIG_TEMPLATE).encode()).hexdigest()


class PlanEmitError(ValueError):
    """The plan cannot be emitted without breaking an authoring rule or leaking something hidden."""


@dataclass(frozen=True)
class PlanSpec:
    key: str  # "<instance_id>.s<seed>"
    spec_text: str  # the task text every arm gets
    files: tuple[str, ...]  # the manifest's files_in_scope
    visible: tuple[str, ...]  # the manifest's visible_verify commands
    model: str  # the pinned model: its key and slug in roko.toml
    provider: str  # the provider table name, e.g. "cerebras"
    base_url: str
    api_key_env: str
    price_row: dict | None  # the snapshot row: Roko's own budget guard uses its rates
    usd_cap: float  # the arm's dollar cap per task
    provider_kind: str = "openai_compat"
    context_window: int = 128_000
    max_output: int = 8192
    max_retries: int = 2
    max_turns: int = 12
    tier: str = "focused"
    skip_enrichment: bool = True
    verify_timeout_s: int = 120
    verify_wrapper: str | None = None  # the visible-verify wrapper's path, in a run with flaky_verify (vb_verify)


@dataclass(frozen=True)
class Emitted:
    slug: str
    plan_dir: Path
    tasks_path: Path
    config_path: Path
    tasks_text: str
    config_text: str
    visible_command: str


def plan_slug(key: str) -> str:
    """An opaque plan id: the instance id names the ladder level, which the direct arm's agent never sees."""
    return "vb-" + hashlib.sha256(key.encode("utf-8")).hexdigest()[:10]


def emit(spec: PlanSpec, workspace: Path) -> Emitted:
    workspace = Path(workspace).absolute()
    taken = [name for name in SCAFFOLDING if (workspace / name).exists() or (workspace / name).is_symlink()]
    if taken:
        raise PlanEmitError(f"the task tree already has {', '.join(taken)}; Roko's files would mix with the agent's")
    files = _files(spec.files)
    visible = _visible(spec.visible)
    if spec.verify_wrapper:
        visible = f"{shlex.quote(spec.verify_wrapper)} {shlex.quote(visible)}"
    for name, value in (("model", spec.model), ("provider", spec.provider), ("tier", spec.tier)):
        if not MODEL_KEY.fullmatch(value):
            raise PlanEmitError(f"{name} {value!r} cannot be a roko.toml key")
    if not 0 <= spec.max_retries <= 5 or spec.max_turns < 1 or spec.verify_timeout_s < 1:
        raise PlanEmitError("max_retries must be 0-5, and max_turns and the verify timeout at least 1")
    if not (spec.usd_cap > 0 and math.isfinite(spec.usd_cap)):
        raise PlanEmitError(f"the dollar cap must be a finite number above 0, not {spec.usd_cap!r}")
    slug = plan_slug(spec.key)
    tasks_text = TASKS_TEMPLATE.format(
        version=TEMPLATE_VERSION, slug=_s(slug), skip_enrichment=_b(spec.skip_enrichment), task_id=_s(TASK_ID),
        title=_s(_title(spec.spec_text)), description=_s(spec.spec_text.strip() + "\n"), role=_s(ROLE),
        tier=_s(spec.tier), files="[" + ", ".join(_s(path) for path in files) + "]", max_retries=spec.max_retries,
        model=_s(spec.model), visible=_s(visible))
    rates = ""
    if spec.price_row:  # Roko's budget guard prices with these; the driver re-prices from the snapshot anyway
        rates = (f"cost_input_per_m = {float(spec.price_row['input'])!r}\n"
                 f"cost_output_per_m = {float(spec.price_row['output'])!r}\n")
    config_text = CONFIG_TEMPLATE.format(
        version=TEMPLATE_VERSION, model=_s(spec.model), model_key=spec.model, provider_key=spec.provider,
        provider=_s(spec.provider), provider_kind=_s(spec.provider_kind), base_url=_s(spec.base_url),
        api_key_env=_s(spec.api_key_env), context_window=spec.context_window, max_output=spec.max_output, rates=rates,
        max_retries=spec.max_retries, visible=_s(visible), verify_timeout_s=spec.verify_timeout_s,
        tier_key=spec.tier, max_turns=spec.max_turns, usd_cap=round(spec.usd_cap, 6),
        turn_usd=max(round(spec.usd_cap / 10, 6), 1e-06))
    _check(tasks_text, config_text, spec, slug, files, visible)
    plan_dir = workspace / "plans" / slug
    plan_dir.mkdir(parents=True)
    tasks_path, config_path = plan_dir / "tasks.toml", workspace / "roko.toml"
    tasks_path.write_text(tasks_text, encoding="utf-8")
    config_path.write_text(config_text, encoding="utf-8")
    return Emitted(slug=slug, plan_dir=plan_dir, tasks_path=tasks_path, config_path=config_path,
                   tasks_text=tasks_text, config_text=config_text, visible_command=visible)


def _check(tasks_text: str, config_text: str, spec: PlanSpec, slug: str, files: list[str], visible: str) -> None:
    """Parse both files back and hold them to the rules in the module docstring."""
    for text in (tasks_text, config_text):
        if canary.find(text):
            raise PlanEmitError("refusing to write a canary into Roko's files")
    try:
        plan, config = tomllib.loads(tasks_text), tomllib.loads(config_text)
    except tomllib.TOMLDecodeError as err:
        raise PlanEmitError(f"emitted TOML does not parse: {err}") from None
    meta, tasks = plan["meta"], plan["task"]
    [task] = tasks
    expected = {"plan": slug, "total": 1, "max_parallel": 1, "skip_enrichment": spec.skip_enrichment}
    if {key: meta.get(key) for key in expected} != expected:
        raise PlanEmitError(f"[meta] is {meta}, not {expected}")
    if (task["role"], task["files"], task["max_retries"], task["description"].strip()) != (
            ROLE, files, spec.max_retries, spec.spec_text.strip()):
        raise PlanEmitError("the emitted task does not round-trip")
    commands = [step["command"] for step in task["verify"]] + [rung["command"] for rung in config["gates"]["rungs"]]
    if commands != [visible, visible] or "depends_on_plan" in task or "context" in task:
        raise PlanEmitError("the emitted files hold a command other than the visible check, or extra context")
    if list(config["providers"]) != [spec.provider] or list(config["models"]) != [spec.model] or \
            config["routing"]["fallback_models"] or config["routing"]["ladder"] != {"enabled": False}:
        raise PlanEmitError("roko.toml must hold exactly the pinned provider and model, with no fallbacks and the "
                            "routing ladder off")


def _files(paths: tuple[str, ...]) -> list[str]:
    files = list(dict.fromkeys(paths))
    if not files or len(files) > MAX_FILES:
        raise PlanEmitError(f"an implementer task needs 1-{MAX_FILES} files, not {len(files)}")
    for path in files:
        pure = PurePosixPath(path)
        if not path or pure.is_absolute() or ".." in pure.parts or GLOB.search(path) or \
                pure.parts[0] in SCAFFOLDING:
            raise PlanEmitError(f"unsafe or globbed file path {path!r}")
    return files


def _visible(commands: tuple[str, ...]) -> str:
    commands = [command.strip() for command in commands]
    if not commands or not all(commands):
        raise PlanEmitError("a benchmark task needs at least one visible check")
    return " && ".join(f"( {command} )" if len(commands) > 1 else command for command in commands)


def _title(spec_text: str) -> str:
    """The spec's first Markdown heading, which every arm's task text already holds."""
    for line in spec_text.splitlines():
        if line.startswith("#") and line.lstrip("#").strip():
            return line.lstrip("#").strip()[:120]
    return "Complete the task"


def _s(value: str) -> str:
    """A TOML basic string: quotes, backslashes and every control character escaped."""
    out = ['"']
    for char in value:
        if char in '"\\':
            out.append("\\" + char)
        elif char == "\n":
            out.append("\\n")
        elif char == "\t":
            out.append("\\t")
        elif ord(char) < 0x20 or ord(char) == 0x7F:
            out.append(f"\\u{ord(char):04x}")
        else:
            out.append(char)
    return "".join(out) + '"'


def _b(value: bool) -> str:
    return "true" if value else "false"
