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
  `max_retries`; the task in the shared working tree (`[runner] worktree_per_task = false`), so Roko's edits land in
  the workdir the driver checks and exports, not on a batch branch (gap-4ec59f made per-task worktrees the default);
  force-accept, cargo fix, replanning, playbook refresh and dreams off; `[learning] frozen` (gap-b001ca, decision
  2218), true unless `PlanSpec.learning_frozen` says otherwise. `plan run` runs a workspace's
  required rungs after each task's own `[[task.verify]]` steps, but not a rung whose command a step already runs
  (gap-3506f1), so the visible check, which is both the verify step and the rung, runs once per attempt.

**Ladder mode** (3310; decision 3302, D11). A spec with `rungs` emits the cheap-model ladder instead of one pinned
model, for the routed Roko arms (`roko_ladder`, `roko_plan`, `roko_full`): one provider table per provider the rungs
use and one model table per rung (its key is its slug, quoted in TOML), `[routing.ladder] enabled = true` with those
rungs in order and every tier starting on the arm's start rung, `fallback_models = []`, and no frontier rung
(`FRONTIER_MODELS`). The task carries no `model_hint`, so `plan validate --strict` has nothing for PLAN_041 to flag;
the runner must not pass `--model` either, which would pin a model past the ladder. The rest of roko.toml (gates,
budget, turns, runner) is the pinned mode's, `[learning]`'s `frozen` value aside (below). `_check` accepts it only
when every rung's model is in the arm's allowlist (`allow`) in the allowlist's order, each once, and the start rung
is a rung whose model is `model`. Without rungs, the files are byte for byte the pinned mode's (`testdata/planemit/`).

**Freezing learning** (gap-b001ca, decision 2218). `[learning] frozen` is `true` unless `PlanSpec.learning_frozen`
is set explicitly: a pinned-mode spec (no `rungs`) freezes by default, since G2's frozen-loop census
(`analysis/gates.py::_frozen_loops`) reads `ablation_flags` for exactly `roko_fixed` and `fr_claude`, today's only
pinned Roko arms; a routed arm's ladder is itself a learning mechanism, so a ladder-mode spec (`roko_ladder`,
`roko_plan`, `roko_full`) does not freeze by default. `learning_frozen=True`/`False` overrides either default, for
an arm that must learn despite running pinned, or must not despite being routed; no arm needs this today.
`config.learning.frozen` (`crates/roko-core/src/config/learning.rs`) is what a real run's S01 manifest reads into
`ablation_flags = ["learning_frozen"]` when it is set.

**The overlay** (3360). A spec with `overlay` appends extra tables after the rest of roko.toml, once per table so a
later field never duplicates one: `gate_mode` -> `[spec_quality] mode` (D14); `audit_floor` -> `[audit] enabled =
true`, `eps_floor` (D13, locked at 0.05 or more); `routing_mode`/`routing_policy` -> `[self_model] mode`/`policy`
(S04's cross-fitted, verify-then-escalate predictor, D11 policy (a) `lcb_aci`; no separate fold key: cross-fitting is
`lcb_aci`'s own training, not a roko.toml knob); `holdout` -> `[homeostasis] holdout` (D10). `resolve_overlay` first
refuses a key this build has no mechanism for (`SUPPORTED_OVERLAY_KEYS`), then resolves `homeostasis = "if_live"` to
`[homeostasis] mode = "on"` or `"shadow"` by `HOMEOSTASIS_LOOP`'s current loop census (`campaign.census_report`/
`is_loop_live`, the same check 3359's requires_live reads; a test fakes it with `monkeypatch.setattr(campaign,
"census_report", ...)`, never a real binary). An empty overlay (`roko_fixed`, `roko_ladder`) appends nothing, so
their files stay byte for byte what they were before this task (`testdata/planemit/`, `TEMPLATE_SHA256`).

Agents see verify commands (A4), so no hidden check goes into either file: every command in them is one of the
manifest's visible commands, which the spec states, and `emit` refuses to write a file that holds a canary. The
authoring rules (memory `roko_plan_authoring_constraints.md`) are checked here too: no `depends_on_plan`, no
`read_files`, a non-empty `files` without glob characters, and scalars before sub-tables. Strings are written as
escaped TOML basic strings and each file is parsed back before it is written.

`emit` refuses a workspace that already has `roko.toml`, `plans/` or `.roko/`: the runner removes those after the
run, before the driver exports c_i, so they must be Roko's alone.

API:
    PlanSpec(...)                                   # frozen; see the fields
    Rung(name, model, provider, base_url, api_key_env, price_row=None, ...)      # one rung of ladder mode
    emit(spec: PlanSpec, workspace: Path) -> Emitted              # raises PlanEmitError
    Emitted(slug, plan_dir, tasks_path, config_path, tasks_text, config_text, visible_command)
    plan_slug(key: str) -> str; SCAFFOLDING; TASK_ID; TIERS; FRONTIER_MODELS
    TEMPLATE_VERSION, TEMPLATE_SHA256                 # the pinned mode's, provider_kind openai_compat
    LADDER_VERSION, LADDER_TEMPLATE_SHA256            # ladder mode's
    CLAUDE_CLI_TEMPLATE_SHA256                        # the pinned mode's, provider_kind claude_cli (3318)
    SUPPORTED_OVERLAY_KEYS, HOMEOSTASIS_LOOP          # the overlay (3360)
    resolve_overlay(overlay: Mapping[str, object]) -> dict        # raises PlanEmitError on an unknown key
    is_frozen(spec: PlanSpec) -> bool                 # [learning] frozen's resolved value (gap-b001ca)
"""

from __future__ import annotations

import hashlib
import math
import re
import shlex
import tomllib
from collections.abc import Mapping
from dataclasses import dataclass, field
from pathlib import Path, PurePosixPath

import campaign  # the overlay's homeostasis=if_live census check (3360): census_report, is_loop_live
import layout  # noqa: F401 (puts families/ on sys.path for common)
from common import canary

TEMPLATE_VERSION = "planemit-4"
LADDER_VERSION = "planemit-ladder-1"
TIERS = ("mechanical", "focused", "integrative", "architectural")  # Roko's task tiers, each with a start rung
# The frontier models of S09 §4.2's arms (fd_claude, fd_claude_lite, fd_codex, fd_api, the optional Fable arm) and of
# roko.toml's `top` rung: a ladder of the cheap pool never holds one (decision 3302).
FRONTIER_MODELS = frozenset({"claude-opus-5-5", "claude-sonnet-5", "claude-sonnet", "claude-fable-5-1", "gpt-5.4",
                             "gpt-5.5"})
BARE_KEY = re.compile(r"[A-Za-z0-9_-]+")
TASK_ID = "T01"
SCAFFOLDING = ("roko.toml", "plans", ".roko")  # what Roko's run adds to the workspace; the runner removes it
ROLE = "implementer"
MAX_FILES = 32
GLOB = re.compile(r"[*?\[\]]")
MODEL_KEY = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]*")
# The overlay (module docstring, 3360): the keys this build knows how to emit, and the loop its "if_live" reads.
SUPPORTED_OVERLAY_KEYS = frozenset({"gate_mode", "audit_floor", "routing_mode", "routing_policy", "holdout",
                                    "homeostasis"})
HOMEOSTASIS_LOOP = "L-M1"  # S06's own loop id (S09 §9.5: roko_full includes S06 only when the census says LIVE)

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

CONFIG_HEAD = """\
# Emitted by the ViabilityBench driver ({version}) for one benchmark run: one provider, one pinned model, no
# fallbacks and no routing ladder, explicit gate rungs that run only the visible check, the task in the shared
# working tree, and Roko's learning loops held off.
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

"""

CONFIG_TAIL = """\
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

[runner]
worktree_per_task = false

[budget]
max_plan_usd = {usd_cap}
max_task_retry_usd = {usd_cap}
max_turn_usd = {turn_usd}

[learning]
frozen = {frozen}
auto_playbook_refresh = false
dream_on_completion = false

[learning.dreams]
trigger_on_plan_complete = false
"""

CONFIG_TEMPLATE = CONFIG_HEAD + CONFIG_TAIL

# A claude_cli provider (3318, fr_claude): no base_url or api_key_env, since the CLI signs in by itself on the
# subscription; `command` and the Anthropic tool format in place of openai_compat's.
CLAUDE_CLI_CONFIG_HEAD = """\
# Emitted by the ViabilityBench driver ({version}) for one benchmark run on the Claude CLI's subscription: one
# pinned model, no fallbacks and no routing ladder, explicit gate rungs that run only the visible check, the task in
# the shared working tree, and Roko's learning loops held off.
config_version = 2
schema_version = 2

[agent]
default_model = {model}

[providers.{provider_key}]
kind = "claude_cli"
command = "claude"

[models.{model_key}]
provider = {provider}
slug = {model}
context_window = {context_window}
max_output = {max_output}
tool_format = "anthropic_blocks"

[routing]
fallback_models = []

[routing.ladder]
enabled = false

"""
CLAUDE_CLI_CONFIG_TEMPLATE = CLAUDE_CLI_CONFIG_HEAD + CONFIG_TAIL

LADDER_TASKS_TEMPLATE = TASKS_TEMPLATE.replace("model_hint = {model}\n", "")  # the ladder picks the model

LADDER_HEAD = """\
# Emitted by the ViabilityBench driver ({version}) for one benchmark run on the cheap-model ladder: a
# provider table per provider and a model table per rung, the routing ladder on with these rungs alone (no frontier
# rung) and no fallbacks, explicit gate rungs that run only the visible check, the task in the shared working tree,
# and Roko's learning loops held off.
config_version = 2
schema_version = 2

[agent]
default_model = {start_model}
{providers}{models}
[routing]
fallback_models = []

[routing.ladder]
enabled = true
rungs = [
{rungs}]
start = {start}

"""

LADDER_CONFIG_TEMPLATE = LADDER_HEAD + CONFIG_TAIL

PROVIDER_TABLE = """
[providers.{key}]
kind = {kind}
base_url = {base_url}
api_key_env = {api_key_env}
"""

MODEL_TABLE = """
[models.{key}]
provider = {provider}
slug = {model}
context_window = {context_window}
max_output = {max_output}
tool_format = "openai_json"
{rates}"""

TEMPLATE_SHA256 = hashlib.sha256((TASKS_TEMPLATE + "\0" + CONFIG_TEMPLATE).encode()).hexdigest()
LADDER_TEMPLATE_SHA256 = hashlib.sha256("\0".join(
    [LADDER_TASKS_TEMPLATE, LADDER_CONFIG_TEMPLATE, PROVIDER_TABLE, MODEL_TABLE]).encode()).hexdigest()
CLAUDE_CLI_TEMPLATE_SHA256 = hashlib.sha256((TASKS_TEMPLATE + "\0" + CLAUDE_CLI_CONFIG_TEMPLATE).encode()).hexdigest()


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
    # Ladder mode (module docstring): the rungs, cheapest first; the start rung's name; the arm's models_allow.
    rungs: tuple[Rung, ...] = ()
    start: str | None = None
    allow: tuple[str, ...] = ()
    # The overlay (module docstring, 3360): extra mechanism tables, empty for every arm but roko_full.
    overlay: Mapping[str, object] = field(default_factory=dict)
    # Freezing learning (module docstring, gap-b001ca): None resolves from `rungs` (is_frozen); True/False is an
    # explicit override, for an arm that needs the opposite of its mode's default.
    learning_frozen: bool | None = None


@dataclass(frozen=True)
class Rung:
    """One rung of ladder mode: a model, its key and slug in roko.toml, on its provider's endpoint."""

    name: str  # what `start` and escalation refer to, e.g. "cheap"
    model: str
    provider: str
    base_url: str  # the metering proxy's URL for this provider in a proxied run (3311)
    api_key_env: str
    price_row: dict | None = None
    provider_kind: str = "openai_compat"
    context_window: int = 128_000
    max_output: int = 8192


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
    overlay = resolve_overlay(spec.overlay)
    frozen = is_frozen(spec)
    if spec.rungs:
        tasks_text, config_text = _ladder_texts(spec, slug, files, visible, frozen)
        config_text += _overlay_text(overlay)
        _check_ladder(tasks_text, config_text, spec, slug, files, visible, overlay)
        return _write(workspace, slug, tasks_text, config_text, visible)
    tasks_text = TASKS_TEMPLATE.format(
        version=TEMPLATE_VERSION, slug=_s(slug), skip_enrichment=_b(spec.skip_enrichment), task_id=_s(TASK_ID),
        title=_s(_title(spec.spec_text)), description=_s(spec.spec_text.strip() + "\n"), role=_s(ROLE),
        tier=_s(spec.tier), files="[" + ", ".join(_s(path) for path in files) + "]", max_retries=spec.max_retries,
        model=_s(spec.model), visible=_s(visible))
    common = dict(max_retries=spec.max_retries, visible=_s(visible), verify_timeout_s=spec.verify_timeout_s,
                 tier_key=spec.tier, max_turns=spec.max_turns, usd_cap=round(spec.usd_cap, 6),
                 turn_usd=max(round(spec.usd_cap / 10, 6), 1e-06), model=_s(spec.model), model_key=spec.model,
                 provider_key=spec.provider, provider=_s(spec.provider), context_window=spec.context_window,
                 max_output=spec.max_output, frozen=_b(frozen))
    if spec.provider_kind == "claude_cli":  # 3318: the CLI signs in by itself, no base_url or api_key_env
        config_text = CLAUDE_CLI_CONFIG_TEMPLATE.format(version=TEMPLATE_VERSION, **common)
    else:
        config_text = CONFIG_TEMPLATE.format(version=TEMPLATE_VERSION, provider_kind=_s(spec.provider_kind),
                                             base_url=_s(spec.base_url), api_key_env=_s(spec.api_key_env),
                                             rates=_rates(spec.price_row), **common)
    config_text += _overlay_text(overlay)
    _check(tasks_text, config_text, spec, slug, files, visible, overlay)
    return _write(workspace, slug, tasks_text, config_text, visible)


def _write(workspace: Path, slug: str, tasks_text: str, config_text: str, visible: str) -> Emitted:
    plan_dir = workspace / "plans" / slug
    plan_dir.mkdir(parents=True)
    tasks_path, config_path = plan_dir / "tasks.toml", workspace / "roko.toml"
    tasks_path.write_text(tasks_text, encoding="utf-8")
    config_path.write_text(config_text, encoding="utf-8")
    return Emitted(slug=slug, plan_dir=plan_dir, tasks_path=tasks_path, config_path=config_path,
                   tasks_text=tasks_text, config_text=config_text, visible_command=visible)


def _ladder_texts(spec: PlanSpec, slug: str, files: list[str], visible: str, frozen: bool) -> tuple[str, str]:
    """Ladder mode's tasks.toml and roko.toml (module docstring)."""
    for rung in spec.rungs:
        for name, value in (("rung name", rung.name), ("model", rung.model), ("provider", rung.provider)):
            if not MODEL_KEY.fullmatch(value):
                raise PlanEmitError(f"{name} {value!r} cannot be a roko.toml key")
    tasks_text = LADDER_TASKS_TEMPLATE.format(
        version=LADDER_VERSION, slug=_s(slug), skip_enrichment=_b(spec.skip_enrichment), task_id=_s(TASK_ID),
        title=_s(_title(spec.spec_text)), description=_s(spec.spec_text.strip() + "\n"), role=_s(ROLE),
        tier=_s(spec.tier), files="[" + ", ".join(_s(path) for path in files) + "]", max_retries=spec.max_retries,
        visible=_s(visible))
    providers = {}
    for rung in spec.rungs:  # one table per provider, from its first rung
        providers.setdefault(rung.provider, PROVIDER_TABLE.format(
            key=_key(rung.provider), kind=_s(rung.provider_kind), base_url=_s(rung.base_url),
            api_key_env=_s(rung.api_key_env)))
    models = "".join(MODEL_TABLE.format(
        key=_key(rung.model), provider=_s(rung.provider), model=_s(rung.model), context_window=rung.context_window,
        max_output=rung.max_output, rates=_rates(rung.price_row)) for rung in spec.rungs)
    start = "{ " + ", ".join(f"{tier} = {_s(spec.start or '')}" for tier in TIERS) + " }"
    config_text = LADDER_CONFIG_TEMPLATE.format(
        version=LADDER_VERSION, start_model=_s(spec.model), providers="".join(providers.values()), models=models,
        rungs="".join(f"  {{ name = {_s(rung.name)}, model = {_s(rung.model)} }},\n" for rung in spec.rungs),
        start=start, max_retries=spec.max_retries, visible=_s(visible), verify_timeout_s=spec.verify_timeout_s,
        tier_key=spec.tier, max_turns=spec.max_turns, usd_cap=round(spec.usd_cap, 6),
        turn_usd=max(round(spec.usd_cap / 10, 6), 1e-06), frozen=_b(frozen))
    return tasks_text, config_text


def resolve_overlay(overlay: Mapping[str, object]) -> dict:
    """`overlay` (an arm's `[overlay]` table) with `homeostasis: "if_live"` resolved to `"on"` or `"shadow"` by
    `HOMEOSTASIS_LOOP`'s current loop census (module docstring). Raises PlanEmitError for a key outside
    `SUPPORTED_OVERLAY_KEYS`: this build has no mechanism to emit it."""
    unknown = set(overlay) - SUPPORTED_OVERLAY_KEYS
    if unknown:
        raise PlanEmitError(f"this build has no mechanism for overlay key(s) {', '.join(sorted(unknown))}")
    resolved = dict(overlay)
    if resolved.get("homeostasis") == "if_live":
        report = campaign.census_report()
        rows = {row.get("loop"): row for row in report.get("rows", []) if isinstance(row, dict)}
        resolved["homeostasis"] = "on" if campaign.is_loop_live(rows.get(HOMEOSTASIS_LOOP, {})) else "shadow"
    return resolved


def is_frozen(spec: PlanSpec) -> bool:
    """`[learning] frozen`'s resolved value for `spec` (module docstring, gap-b001ca): `spec.learning_frozen`
    when the arm overrides it explicitly, else pinned mode (no `rungs`) freezes and ladder mode does not."""
    return (not spec.rungs) if spec.learning_frozen is None else spec.learning_frozen


def _overlay_text(resolved: Mapping[str, object]) -> str:
    """The `[spec_quality]`, `[audit]`, `[self_model]` and `[homeostasis]` tables `resolved` (`resolve_overlay`'s
    own output) requests, one table per mechanism so a later field never writes `[homeostasis]` twice. Empty
    when `resolved` is empty (every arm but roko_full, module docstring)."""
    if not resolved:
        return ""
    tables: dict[str, list[str]] = {}
    if "gate_mode" in resolved:
        tables.setdefault("spec_quality", []).append(f"mode = {_s(resolved['gate_mode'])}")
    if "audit_floor" in resolved:
        tables.setdefault("audit", []).append("enabled = true")
        tables["audit"].append(f"eps_floor = {float(resolved['audit_floor'])!r}")
    if "routing_mode" in resolved:
        tables.setdefault("self_model", []).append(f"mode = {_s(resolved['routing_mode'])}")
    if "routing_policy" in resolved:
        tables.setdefault("self_model", []).append(f"policy = {_s(resolved['routing_policy'])}")
    if "holdout" in resolved:
        tables.setdefault("homeostasis", []).append(f"holdout = {float(resolved['holdout'])!r}")
    if "homeostasis" in resolved:
        tables.setdefault("homeostasis", []).append(f"mode = {_s(resolved['homeostasis'])}")
    return "".join(f"\n[{name}]\n" + "\n".join(lines) + "\n" for name, lines in tables.items())


def _check_overlay(config: dict, resolved: Mapping[str, object]) -> None:
    """The overlay's own tables, parsed back (module docstring); empty `resolved` means none of them exist."""
    if not resolved:
        if {"spec_quality", "audit", "self_model", "homeostasis"} & set(config):
            raise PlanEmitError("an empty overlay must emit no spec_quality, audit, self_model or homeostasis table")
        return
    if "gate_mode" in resolved and config.get("spec_quality", {}).get("mode") != resolved["gate_mode"]:
        raise PlanEmitError("[spec_quality].mode must match the overlay's gate_mode")
    if "audit_floor" in resolved and (config.get("audit", {}).get("enabled") is not True or
                                      config["audit"].get("eps_floor") != resolved["audit_floor"]):
        raise PlanEmitError("[audit] must be enabled with the overlay's audit_floor as eps_floor")
    for key, overlay_key in (("mode", "routing_mode"), ("policy", "routing_policy")):
        if overlay_key in resolved and config.get("self_model", {}).get(key) != resolved[overlay_key]:
            raise PlanEmitError(f"[self_model].{key} must match the overlay's {overlay_key}")
    if "holdout" in resolved and config.get("homeostasis", {}).get("holdout") != resolved["holdout"]:
        raise PlanEmitError("[homeostasis].holdout must match the overlay's holdout")
    if "homeostasis" in resolved and config.get("homeostasis", {}).get("mode") != resolved["homeostasis"]:
        raise PlanEmitError("[homeostasis].mode must match the overlay's resolved homeostasis")


def _check_ladder(tasks_text: str, config_text: str, spec: PlanSpec, slug: str, files: list[str],
                  visible: str, overlay: Mapping[str, object]) -> None:
    """Ladder mode's rules (module docstring), on the parsed files."""
    models = [rung.model for rung in spec.rungs]
    names = [rung.name for rung in spec.rungs]
    frontier = sorted(set(models) & FRONTIER_MODELS)
    if frontier:
        raise PlanEmitError(f"a ladder of the cheap pool holds no frontier model, not {', '.join(frontier)}")
    if len(set(models)) != len(models) or len(set(names)) != len(names):
        raise PlanEmitError("each rung needs a model and a name of its own")
    outside = [model for model in models if model not in spec.allow]
    if outside:
        raise PlanEmitError(f"rung model(s) {', '.join(outside)} are not in the arm's models_allow")
    if models != [model for model in spec.allow if model in models]:
        raise PlanEmitError(f"the rungs {', '.join(models)} are not in the order of the arm's models_allow, cheapest "
                            "first")
    start = next((rung for rung in spec.rungs if rung.name == spec.start), None)
    if start is None or start.model != spec.model:
        raise PlanEmitError(f"the start rung {spec.start!r} must be a rung, and its model {spec.model!r}")
    for text in (tasks_text, config_text):
        if canary.find(text):
            raise PlanEmitError("refusing to write a canary into Roko's files")
    try:
        plan, config = tomllib.loads(tasks_text), tomllib.loads(config_text)
    except tomllib.TOMLDecodeError as err:
        raise PlanEmitError(f"emitted TOML does not parse: {err}") from None
    [task] = plan["task"]
    if plan["meta"]["plan"] != slug or "model_hint" in task or (task["role"], task["files"], task["max_retries"]) != (
            ROLE, files, spec.max_retries):
        raise PlanEmitError("the emitted ladder task does not round-trip, or it pins a model")
    commands = [step["command"] for step in task["verify"]] + [rung["command"] for rung in config["gates"]["rungs"]]
    ladder = config["routing"]["ladder"]
    if commands != [visible, visible] or list(config["models"]) != models or \
            list(config["providers"]) != list(dict.fromkeys(rung.provider for rung in spec.rungs)) or \
            config["routing"]["fallback_models"] or ladder.get("enabled") is not True or \
            ladder["rungs"] != [{"name": rung.name, "model": rung.model} for rung in spec.rungs] or \
            ladder["start"] != dict.fromkeys(TIERS, spec.start) or config["agent"]["default_model"] != spec.model:
        raise PlanEmitError("roko.toml must hold exactly the rungs' providers and models, the ladder on with those "
                            "rungs in order from the start rung, and no fallbacks")
    if config["runner"] != {"worktree_per_task": False}:
        raise PlanEmitError("roko.toml must run the task in the shared working tree, where the driver reads it")
    if config["learning"]["frozen"] is not is_frozen(spec):
        raise PlanEmitError("[learning].frozen must match the resolved frozen value (gap-b001ca)")
    _check_overlay(config, overlay)


def _check(tasks_text: str, config_text: str, spec: PlanSpec, slug: str, files: list[str], visible: str,
          overlay: Mapping[str, object]) -> None:
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
    if config["runner"] != {"worktree_per_task": False}:
        raise PlanEmitError("roko.toml must run the task in the shared working tree, where the driver reads it")
    if config["learning"]["frozen"] is not is_frozen(spec):
        raise PlanEmitError("[learning].frozen must match the resolved frozen value (gap-b001ca)")
    _check_overlay(config, overlay)


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


def _rates(price_row: dict | None) -> str:
    """A model table's rates: Roko's budget guard prices with these; the driver re-prices from the snapshot anyway."""
    if not price_row:
        return ""
    return (f"cost_input_per_m = {float(price_row['input'])!r}\n"
            f"cost_output_per_m = {float(price_row['output'])!r}\n")


def _key(name: str) -> str:
    """A TOML key: bare when it can be, else quoted, so a slug's dots stay in one key (glm-4.7)."""
    return name if BARE_KEY.fullmatch(name) else _s(name)
