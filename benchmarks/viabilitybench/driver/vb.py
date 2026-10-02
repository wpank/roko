#!/usr/bin/env python3
"""`vb`, the ViabilityBench driver (S08 §5.7): `run`, `estimate`, `materialize`, `campaign`, `ledger` and `report`.

    vb run --experiment PILOT-A --stream pilot --arm cheap_direct --model gpt-oss-120b --seeds 1-3 \
           --allow-network --max-cost-usd 10 [--line BL0] [--limit N] [--proxy] [--disturbance SPEC.toml] \
           [--transcripts] [--keep-workdirs]
    vb estimate --stream pilot --arm cheap_direct --model gpt-oss-120b --seeds 1-3
    vb materialize --stream pilot --instance F1-l1-0001 --out DIR
    vb campaign --manifest experiments/pilot_a.toml --dry-run        # an experiment's blocks (campaign.py)

`vb run` runs every (task, seed) of a stream on one arm and one model, in a fresh workdir under `$VB_WORK` (default
`~/vb-work/<run_id>/`), outside the repository. For each one it: materializes the task (`materialize`); runs the
arm's runner module (`harness`: the arm file's `runner`, or its `harness` id with "-" read as "_", so a new arm adds
files and never edits this one); commits the final tree as c_i without touching the agent's repo and archives it
(`archive`); labels c_i by VS-census (`census`); and appends a `vb.run_record/1` row (`records`). Ledger rows
(`ledger`) are appended by the runner as attempts end. Everything lands in `$VB_RESULTS/<experiment>/<run_id>/`
(default `~/.roko-bench/viability`): `manifest.json`, `order-<seed>.json`, `records.jsonl`, `ledger.jsonl`,
`archives/`, `private/` (task manifests and pristine bundles, never an agent's), `errors.jsonl` and, with
`--transcripts`, `transcripts/`. Stream positions in records are 1-based.

**Network admission** (from `scripts/dev_benchmark.py`'s `execute`, W10 rec 14). A provider whose base URL is not a
loopback address is a network provider. `vb run` calls one only with both `--allow-network` and an explicit
`--max-cost-usd`, and the check runs before anything else: no directory, no workdir, no request. The budget is then
enforced task by task rather than on the whole run's worst case: a task starts only while the ledger's spend plus
the most one task can cost under the arm's caps (priced from the snapshot) still fits, so the run stops early
instead of overspending. dev_benchmark refuses when the whole run's worst case exceeds the budget; here that would
need a $17 flag for a pilot budgeted at $10 (60 cheap_direct runs at $0.29 worst case each, against about $0.02
typical), and the flag would then no longer cap anything. `vb estimate` and the run manifest show the whole-run
worst case. A model without a price row cannot be bounded and is refused. `--provider-url` with a loopback URL (such
as `stub_provider`'s) runs offline, with neither flag. A loopback URL in front of a network provider would skip
admission, so `--provider-url` never names a proxy started by hand: `vb run` starts its own (below).

**The metering proxy** (`faultproxy.py`, S08 T13). A billed run on a network provider always goes through it, and
`--proxy` sends any other run through it too, such as an offline one on a stub. Once the run is admitted, `vb run`
starts the proxy inside the driver's process with one upstream per provider its models_allow rows use (3311: a
ladder's rungs may span several), each the arm's own provider or the `--provider-url` that overrides all of them.
The runners reach a model only through it: they get its loopback URL and no key, and it sends the provider's key. It logs every call to `<run_dir>/proxy.jsonl`, and before each task the driver sets its task to the
task key, `<instance_id>.s<seed>`; the Roko arm finds its rows by that key (`run_roko`). Admission judges the
provider's own URL, never the proxy's, so a network provider behind the loopback proxy still needs both flags. Before
each task the driver also sets the task's caps in the proxy: the arm's input cap, the per-attempt cap for a runner
whose `PROXY_CAPS` name it (Roko has none of its own), and the fault profile, which is `clean` unless a disturbance
covers the task. A network provider without an `api_key_env` (a subscription CLI signs in by itself) cannot go
through the proxy, which drops a client's own credentials.

**Disturbances** (`--disturbance SPEC.toml`, `disturb.py`, gap-15bb83). A `vb.disturbance/1` spec applies S08 §4.6's
hooks for H6 to stream positions: `provider_fault` (a proxy fault profile), `budget_cut` (the task's caps scaled),
`harder_mix` (the harder instances first from a position on), `convention_flip` (a family's other latent),
`flaky_verify` (the arm's visible checks fail at random) and `model_swap` (the proxy serves another model than the
pin, which the model checks then accept as a declared swap). A spec the run cannot apply is refused before anything
runs. The config and so `config_hash` carry the spec, each record's `stream.perturbations_active` names the hooks
that covered its task, each attempt's `fault_injected` names the fault the proxy injected into its calls, and its
`model_swapped` says whether the swap's model served it. In a run with `flaky_verify`, every task gets the
visible-verify wrapper (`vb_verify`) in its agent's `.vb-bin/`, and the arm's visible checks go through it: each
record's `visible` says how many ran (`verify_runs`) and which ones the wrapper failed (`flakes`, `flake_injected`).

The benchmark secret reaches only `hidden.py`, in the census, as a file path: `--secret-file`, else
`$VB_SECRET_FILE`, else `~/.config/viabilitybench/secret` (mode 0600; `driver/secret.py init` makes one). Before the
first task, `vb run` refuses a secret file that breaks its rules and a secret that roko or an agent could inherit, and
from then on every agent environment is checked against it (`secret.preflight`, `agent_env`).

**Provider keys** (bug-979a06) live in a driver-only key file: `--key-file`, else `$VB_KEY_FILE`, else
`~/.config/viabilitybench/keys` (`driver/secret.py keys` checks it). They never live in the driver's environment,
which every agent can read (`ps -E`, `/proc/<pid>/environ`). A proxied run reads the key its endpoint names into the
driver's memory for the proxy, the only sender of a key; a loopback `--provider-url` and a CLI that signs in by itself
need none. `vb run` refuses to start while its environment holds any arm's `api_key_env` or a loaded key. While the
tasks run, the secret file and the key file sit at mode 000, and the census reports any change to them as
`leak_suspected` (the tripwire, gap-308373, `secret.tripwire`). Once its checks have passed, `vb run` starts itself
again with its environment cut to an allowlist (`agent_env.exec_scrubbed`, bug-32eb77). No other credential of the
operator's (`GITHUB_TOKEN`, a key no arm names) reaches an agent through the driver.

Exit status: 0 when every (task, seed) got a record, 1 when some did not (see `errors.jsonl`), 2 for a usage,
configuration or admission error.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import importlib
import json
import os
import re
import secrets
import shutil
import stat
import sys
import tempfile
import tomllib
from collections.abc import Iterable, Mapping
from dataclasses import asdict, dataclass
from pathlib import Path
from types import ModuleType

import agent_env
import archive
import campaign
import caps
import census
import disturb
import faultproxy
import harness
import layout
import ledger
import materialize
import provider
import records
import secret
import vb_verify
from common import hmac_seed, knobs, repo

DRIVER_VERSION = "vb-driver-1.0.0"
DEFAULT_SECRET_FILE = Path("~/.config/viabilitybench/secret")
DEFAULT_RESULTS = Path("~/.roko-bench/viability")
DEFAULT_WORK = Path("~/vb-work")
ID_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9._-]{0,63}")
SKIP_IN_SUITE_HASH = ("__pycache__", ".pytest_cache", ".venv")
PROXY_LOG = "proxy.jsonl"  # in the run directory, where run_roko reads it


class DriverError(RuntimeError):
    """A usage, configuration or admission error: `vb` exits 2 before running anything."""


@dataclass(frozen=True)
class Stream:
    id: str
    path: Path
    families: dict[str, Path]
    instances: list[str]
    spec_variant: str

    def family_dir(self, instance_id: str) -> Path:
        return self.families[knobs.parse_instance_id(instance_id)[0]]

    def order(self, seed: int) -> list[str]:
        """The stream's fixed order for one run seed (S08 §4.7): a keyed shuffle, the same on every host."""
        return hmac_seed.surface_stream("stream", f"{self.id}/order/{seed}").shuffled(self.instances)


@dataclass(frozen=True)
class Run:
    """What every task of one `vb run` shares."""

    args: argparse.Namespace
    plan: Plan
    runner: ModuleType
    arm: dict  # plan.arm, with every [providers.*] base_url proxied when this run is proxied (3311)
    endpoint: provider.Endpoint  # what the runners call: the plan's endpoint, or the proxy's in front of it
    chat: provider.ChatProvider
    book: ledger.Ledger
    secret_file: Path
    run_dir: Path
    work_dir: Path
    run_id: str
    config_hash: str
    head: tuple[str, bool]
    suite: dict
    proxy: faultproxy.FaultProxy | None = None
    disturbances: tuple[disturb.Disturbance, ...] = ()  # the H6 hooks this run applies (disturb.py)


@dataclass(frozen=True)
class Plan:
    arm: dict
    stream: Stream
    model: str
    seeds: list[int]
    instances: list[str]
    snapshot: ledger.Snapshot
    endpoint: provider.Endpoint
    caps: caps.Caps
    worst_task_usd: float | None
    # 3311: one endpoint per provider the arm's models_allow uses (keyed by provider name), for a ladder's several
    # rungs; holds exactly `{endpoint.provider: endpoint}` for a one-model arm.
    endpoints: dict[str, provider.Endpoint]

    @property
    def runs(self) -> int:
        return len(self.instances) * len(self.seeds)

    @property
    def worst_case_usd(self) -> float | None:
        return None if self.worst_task_usd is None else self.worst_task_usd * self.runs

    def summary(self) -> dict:
        return {"arm": self.arm["arm"]["id"], "stream": self.stream.id, "model": self.model, "seeds": self.seeds,
                "tasks": len(self.instances), "runs": self.runs, "provider": self.endpoint.provider,
                "base_url": self.endpoint.base_url, "network": not self.endpoint.offline,
                "price_snapshot_id": self.snapshot.id, "worst_task_usd": self.worst_task_usd,
                "worst_case_usd": self.worst_case_usd, "caps": asdict(self.caps)}


def main(argv: list[str] | None = None) -> int:
    forwarded = sys.argv[1:] if argv is None else argv
    if forwarded[:1] == ["report"]:
        return run_report(forwarded[1:])
    args = _parser().parse_args(argv)
    args.argv = ["vb", *(sys.argv[1:] if argv is None else argv)]
    args.own_process = argv is None  # a script, not a call: `vb run` may start itself again (`agent_env.exec_scrubbed`)
    try:
        return args.handler(args)
    except (DriverError, caps.CapError, ledger.PriceError, campaign.CampaignError) as err:
        print(f"vb: {err}", file=sys.stderr)
        return 2


def run_report(argv: list[str]) -> int:
    """`vb report` (S08 §5.7) is `analysis/report.py`, which parses its own flags: argparse cannot pass them through."""
    sys.path.insert(0, str(layout.VB_ROOT / "analysis"))
    return importlib.import_module("report").main(argv)


def admit(plan: Plan, *, allow_network: bool, max_cost_usd: float | None) -> None:
    """Fail closed before any network call (the module docstring has the rule). It judges `plan.endpoint`, the
    provider's own URL: `--proxy` never changes it, so a loopback proxy in front of a network provider is admitted
    as the network provider it is."""
    if plan.endpoint.offline:
        return
    if not allow_network:
        raise DriverError(f"{plan.endpoint.base_url} is a network provider; inspect `vb estimate` and pass "
                          "--allow-network explicitly")
    if max_cost_usd is None:
        raise DriverError("network runs require an explicit --max-cost-usd budget")
    if plan.worst_task_usd is None:
        raise DriverError(f"{plan.model} has no row in {plan.snapshot.id}, so its cost cannot be bounded")
    if plan.worst_task_usd > max_cost_usd:
        raise DriverError(f"one task can cost up to ${plan.worst_task_usd:.2f} under the arm's caps, more than "
                          f"--max-cost-usd ${max_cost_usd:.2f}")


def make_plan(args: argparse.Namespace) -> Plan:
    arm = load_arm(args.arm)
    stream = load_stream(args.stream)
    allowed = arm["arm"]["models_allow"]
    if args.model not in allowed:
        raise DriverError(f"arm {arm['arm']['id']} allows {', '.join(allowed)}, not {args.model}")
    snapshot = ledger.load_snapshot(args.price_snapshot)
    provider_url = getattr(args, "provider_url", None)
    row = snapshot.row(args.model)
    endpoint = _endpoint(arm, row, provider_url)
    arm_caps = caps.Caps.from_table(arm.get("caps", {}))
    instances = stream.instances[:args.limit] if args.limit else stream.instances
    # 3311: a multi-model arm routes several models through the proxy, one endpoint per provider its models_allow
    # rows name, and prices admission and reservations at the most expensive rung, so they stay conservative. A
    # one-model arm has one row, so `rows`, `endpoints` and `worst_task_usd` below are exactly the single-model values.
    rows = [snapshot.row(name) for name in allowed]
    endpoints = {one.provider: one for one in (_endpoint(arm, row_, provider_url) for row_ in rows)}
    worst_per_rung = [caps.worst_task_usd(arm_caps, row_) for row_ in rows]
    worst_task_usd = None if any(one is None for one in worst_per_rung) else max(worst_per_rung)
    return Plan(arm=arm, stream=stream, model=args.model, seeds=parse_seeds(args.seeds), instances=instances,
                snapshot=snapshot, endpoint=endpoint, caps=arm_caps, worst_task_usd=worst_task_usd,
                endpoints=endpoints)


def load_arm(name: str) -> dict:
    path = _config_path(name, layout.ARMS_DIR)
    doc = _toml(path)
    arm = doc.get("arm")
    if doc.get("schema_version") != "vb.arm/1" or not isinstance(arm, dict):
        raise DriverError(f"{path}: not a vb.arm/1 file with an [arm] table")
    for key, kind in (("id", str), ("harness", str), ("models_allow", list), ("line", str), ("billed", bool)):
        if not isinstance(arm.get(key), kind):
            raise DriverError(f"{path}: [arm] {key} must be a {kind.__name__}")
    for name_, table in doc.get("providers", {}).items():
        if not isinstance(table, dict) or not isinstance(table.get("base_url"), str):
            raise DriverError(f"{path}: [providers.{name_}] needs a base_url")
    doc["path"] = str(path)
    return doc


def load_stream(name: str) -> Stream:
    path = _config_path(name, layout.STREAMS_DIR)
    doc = _toml(path)
    table = doc.get("stream")
    if doc.get("schema_version") != "vb.stream/1" or not isinstance(table, dict):
        raise DriverError(f"{path}: not a vb.stream/1 file with a [stream] table")
    families = {family: (layout.VB_ROOT / directory).resolve()
                for family, directory in table.get("families", {}).items()}
    instances = table.get("instances", [])
    if not instances or len(set(instances)) != len(instances):
        raise DriverError(f"{path}: instances must be a non-empty list without repeats")
    for instance in instances:
        try:
            family = knobs.parse_instance_id(instance)[0]
        except ValueError as err:
            raise DriverError(f"{path}: {err}") from None
        if family not in families:
            raise DriverError(f"{path}: {instance}'s family {family} has no directory in [stream] families")
    return Stream(id=table["id"], path=path, families=families, instances=list(instances),
                  spec_variant=table.get("spec_variant", "precise"))


def parse_seeds(text: str) -> list[int]:
    """"1-3" -> [1, 2, 3]; "1,3" -> [1, 3]."""
    seeds: list[int] = []
    for part in text.split(","):
        low, _, high = part.strip().partition("-")
        if not low.isdigit() or (high and not high.isdigit()):
            raise DriverError(f"bad --seeds {text!r}: use forms like 1-3 or 1,3")
        seeds += range(int(low), int(high or low) + 1)
    if not seeds or len(set(seeds)) != len(seeds):
        raise DriverError(f"bad --seeds {text!r}: empty or repeated")
    return seeds


def cmd_estimate(args: argparse.Namespace) -> int:
    print(json.dumps(make_plan(args).summary(), indent=2))
    return 0


def cmd_campaign(args: argparse.Namespace) -> int:
    """`vb campaign` (campaign.py), which gets this module rather than importing a second copy of it."""
    return campaign.cmd_campaign(sys.modules[__name__], args)


def cmd_materialize(args: argparse.Namespace) -> int:
    stream = load_stream(args.stream)
    if args.instance not in stream.instances:
        raise DriverError(f"{args.instance} is not in stream {stream.id}")
    out = Path(args.out).absolute()
    private = Path(args.private).absolute() if args.private else out.with_name(out.name + ".private")
    try:
        done = materialize.materialize(family_dir=stream.family_dir(args.instance), instance_id=args.instance,
                                       workdir=out, private_dir=private, spec_variant=stream.spec_variant)
    except (materialize.MaterializeError, repo.RepoError) as err:
        print(f"vb: {err}", file=sys.stderr)
        return 1
    print(json.dumps({"workdir": str(done.workdir), "private_dir": str(done.private_dir),
                      "manifest": str(done.manifest_path), "pristine": done.pristine.as_json()}, indent=2))
    return 0


def cmd_run(args: argparse.Namespace) -> int:
    plan = make_plan(args)
    admit(plan, allow_network=args.allow_network, max_cost_usd=args.max_cost_usd)
    if args.max_cost_usd is not None and (plan.worst_case_usd or 0) > args.max_cost_usd:
        print(f"vb: the worst case of {plan.runs} runs is ${plan.worst_case_usd:.2f}; the run stops before any task "
              f"that could take spend past ${args.max_cost_usd:.2f}", file=sys.stderr)
    # A billed network run always meters through the proxy, which alone sends the provider's key (module docstring),
    # read from the driver-only key file (bug-979a06).
    proxied = args.proxy or (plan.arm["arm"]["billed"] and not plan.endpoint.offline)
    keys = _provider_keys(args.key_file, plan.endpoints.values()) if proxied else None
    if proxied and not plan.endpoint.offline and keys is None:
        raise DriverError(f"the metering proxy sends {plan.endpoint.provider}'s key, but the arm names no api_key_env, "
                          "since its client signs in by itself")
    for family, directory in sorted(plan.stream.families.items()):
        if any(knobs.parse_instance_id(i)[0] == family for i in plan.instances) and not all(
                (directory / name).is_file() for name in ("gen.py", "hidden.py")):
            raise DriverError(f"family {family} at {directory} needs gen.py and hidden.py")
    disturbances = _disturbances(args.disturbance, plan, proxied)
    secret_file = _secret_file(args.secret_file)
    runner = load_runner(plan.arm)
    results_root = _outside_repo(args.results or os.environ.get("VB_RESULTS") or DEFAULT_RESULTS, "--results")
    work_root = _outside_repo(args.work or os.environ.get("VB_WORK") or DEFAULT_WORK, "--work")
    if layout.within(work_root, results_root) or layout.within(results_root, work_root):
        raise DriverError("--work and --results must not contain each other")
    try:
        loaded = secret.preflight(secret_file, work_root=work_root, results_root=results_root, keys=keys)
    except secret.SecretError as err:
        raise DriverError(str(err)) from None
    if args.own_process:  # the checks passed on the operator's environment; now shed its credentials (bug-32eb77)
        agent_env.exec_scrubbed()
    _preflight(runner, plan)
    for value, flag in ((args.experiment, "--experiment"), (args.run_id or "x", "--run-id")):
        if not ID_RE.fullmatch(value):
            raise DriverError(f"{flag} must match {ID_RE.pattern}")
    run_id = args.run_id or f"vb-{dt.datetime.now(dt.UTC):%Y%m%d}-{secrets.token_hex(3)}"
    run_dir = results_root / args.experiment / run_id
    if run_dir.exists():
        raise DriverError(f"{run_dir} already exists; pick another --run-id")
    run_dir.mkdir(mode=0o700, parents=True)
    work_dir = work_root / run_id
    work_dir.mkdir(mode=0o700, parents=True, exist_ok=True)

    line = args.line or plan.arm["arm"]["line"]
    config = {"driver": DRIVER_VERSION, "arm": {k: v for k, v in plan.arm.items() if k != "path"},
              "model": plan.model, "endpoint": asdict(plan.endpoint), "caps": asdict(plan.caps),
              "stream": {"id": plan.stream.id, "instances": plan.instances, "spec_variant": plan.stream.spec_variant},
              "price_snapshot_id": plan.snapshot.id, "line": line,
              "prompt": {"version": getattr(runner, "PROMPT_VERSION", None),
                         "sha256": getattr(runner, "PROMPT_SHA256", None)},
              **({"disturbances": [one.as_json() for one in disturbances]} if disturbances else {})}
    config_hash = records.canonical_hash(config)
    head = records.harness_state()
    suite = {"id": "vb", "hash": _suite_hash(plan)}
    _write_json(run_dir / "manifest.json", {
        "schema_version": "vb.run_manifest/1", "experiment_id": args.experiment, "run_id": run_id,
        "started_at": harness.utc_now(), "argv": args.argv, "harness_sha": head[0],
        "dirty": head[1], "config_hash": config_hash, "config": config, "suite": suite,
        "offline": plan.endpoint.offline,
        "proxy": {"log": PROXY_LOG, "profile": faultproxy.Profile().as_json()} if proxied else None,
        "secret_file": str(secret_file), "work_dir": str(work_dir), **plan.summary()})
    book = ledger.Ledger(run_dir / "ledger.jsonl", line=line, experiment_id=args.experiment, run_id=run_id,
                         price_snapshot_id=plan.snapshot.id)
    proxy = _start_proxy(plan, run_dir, keys=keys) if proxied else None
    endpoint = proxy.endpoint(plan.endpoint) if proxy else plan.endpoint
    # 3311: hand the runner every provider's proxied URL through the arm dict's own [providers.*] tables, the only
    # channel a runner has for more than the one endpoint on `ctx.endpoint`; `api_key_env` names a key, not its
    # value, so it is unchanged (bug-979a06: the real key never reaches this dict either way).
    arm = plan.arm
    if proxy:
        tables = {name: ({**table, "base_url": proxy.base_url(name)} if name in proxy.upstreams else table)
                  for name, table in plan.arm.get("providers", {}).items()}
        arm = {**plan.arm, "providers": tables}
    run = Run(args=args, plan=plan, runner=runner, arm=arm, endpoint=endpoint, chat=provider.OpenAICompatible(endpoint),
              book=book, secret_file=secret_file, run_dir=run_dir, work_dir=work_dir, run_id=run_id,
              config_hash=config_hash, head=head, suite=suite, proxy=proxy, disturbances=disturbances)
    written = 0
    try:
        with secret.tripwire(loaded, keys):  # the secret file and the key file at mode 000 while the tasks run
            for seed in plan.seeds:
                order = disturb.reorder([i for i in plan.stream.order(seed) if i in plan.instances], disturbances,
                                        lambda instance: knobs.parse_instance_id(instance)[1])
                _write_json(run_dir / f"order-{seed}.json", {"stream": plan.stream.id, "seed": seed, "order": order})
                for position, instance_id in enumerate(order, 1):
                    refusal = book.refusal(plan.worst_task_usd if plan.arm["arm"]["billed"] else 0.0)
                    if refusal:  # S09 §4.6: the task could take billed spend past a budget cap (ledger.py)
                        _log_error(run_dir, f"{instance_id}.s{seed}", "budget", f"stopped before the task: {refusal}")
                        return 1
                    if args.max_cost_usd is not None and plan.worst_task_usd is not None and \
                            book.spent_bound_usd + plan.worst_task_usd > args.max_cost_usd:
                        _log_error(run_dir, f"{instance_id}.s{seed}", "budget", f"stopped: ${book.spent_bound_usd:.4f} "
                                   "spent; the next task could pass --max-cost-usd")
                        return 1
                    written += _run_one(run, instance_id, seed, {
                        "id": plan.stream.id, "position": position, "length": len(order),
                        "perturbations_active": disturb.kinds(disturbances, position)})
    except secret.SecretError as err:  # the tripwire could not be armed, so no task ran
        raise DriverError(str(err)) from None
    finally:
        if proxy:
            proxy.close()
        _cleanup(args, [work_dir])
        print(f"vb: {written}/{plan.runs} records in {run_dir}", file=sys.stderr)
    return 0 if written == plan.runs else 1


def _run_one(run: Run, instance_id: str, seed: int, stream_position: dict) -> bool:
    """One (task, seed): materialize, run, commit and archive c_i, label it, and append its record."""
    args, plan, run_dir, work_dir = run.args, run.plan, run.run_dir, run.work_dir
    key = f"{instance_id}.s{seed}"
    workdir, private = work_dir / key, run_dir / "private" / key
    homes = [work_dir / "_home" / key, work_dir / "_home" / f"{key}.census"]
    position = stream_position["position"]
    limits = disturb.scaled(plan.caps, run.disturbances, position)  # the arm's caps, cut by a budget_cut
    try:
        task = materialize.materialize(family_dir=plan.stream.family_dir(instance_id), instance_id=instance_id,
                                       workdir=workdir, private_dir=private, spec_variant=plan.stream.spec_variant,
                                       latent=disturb.latent(run.disturbances, position))
    except (materialize.MaterializeError, repo.RepoError, OSError, ValueError) as err:
        _log_error(run_dir, key, "materialize", err)
        _cleanup(args, [workdir])
        return False
    env = agent_env.build(home=homes[0])
    wrapper = _verify_wrapper(run, key, task.manifest, env, position)
    swap = disturb.swap(run.disturbances, position)  # the model the proxy serves in place of the pin, if any
    ctx = harness.TaskContext(
        experiment_id=args.experiment, run_id=run.run_id, arm=run.arm, model=plan.model, endpoint=run.endpoint,
        provider=run.chat, snapshot=plan.snapshot, caps=limits, ledger=run.book, billed=plan.arm["arm"]["billed"],
        instance_id=instance_id, seed=seed, key=key, workdir=workdir, spec_text=task.spec_text, agent_env=env,
        visible_verify=tuple(task.manifest["visible_verify"]), files_in_scope=tuple(task.manifest["files_in_scope"]),
        verify_wrapper=wrapper, model_swap=swap, deny=_agent_deny(run))
    if run.proxy:  # the proxy's rows for this task carry its key, which is how the Roko arm finds them
        held = getattr(run.runner, "PROXY_CAPS", ())  # the caps a runner's harness cannot hold itself
        run.proxy.configure(task=ctx.key, profile=disturb.profile(run.disturbances, position),
                            input_token_cap=limits.input_tokens_per_task,
                            attempt_input_cap=limits.input_tokens_per_attempt if "input_tokens_per_attempt" in held
                            else None, model_swap=swap)
    try:
        outcome = run.runner.run_task(ctx)
    except Exception as err:  # the runner owns its errors; this catches its bugs
        now = harness.utc_now()
        outcome = harness.TaskOutcome("infra_error", f"runner crashed: {type(err).__name__}: {err}", [], [], now, now)
    verify_log = vb_verify.read_log(wrapper) if wrapper else None
    if wrapper and verify_log is None:
        _log_error(run_dir, key, "verify wrapper", "its log is gone, so the record cannot count the visible checks")
    meter = _task_meter(run.proxy, key)
    _end_on_proxy_cap(outcome, meter)
    if run.proxy:
        _mark_faults(outcome.attempts, run_dir / PROXY_LOG, key)
    meter_usd = None if meter is None or meter["cost_unknown"] else meter["api_equiv_usd"]
    final = archived = None
    transcript_text = json.dumps(outcome.transcript, ensure_ascii=False)
    try:
        final = archive.commit_final(workdir, private / "final")
        archived = archive.archive_task(final, task.pristine, run_dir / "archives", key)
        result = census.run_census(final=final, manifest=task.manifest, manifest_path=task.manifest_path,
                                   pristine=task.pristine, family_dir=task.family_dir, secret_file=run.secret_file,
                                   completed=outcome.status == "completed", transcript_text=transcript_text,
                                   diff_text=archived.diff_text, scratch=private / "census",
                                   env=agent_env.build(home=homes[1]))
    except (archive.ArchiveError, repo.RepoError, OSError) as err:
        result = census.CensusResult(completion=int(outcome.status == "completed"), visible_clean=None, hidden=None,
                                     integrity=None, infra_error=f"archive or census failed: {err}")
    transcript_ref = None
    if args.transcripts:
        (run_dir / "transcripts").mkdir(mode=0o700, exist_ok=True)
        transcript_ref = f"transcripts/{key}.json"
        (run_dir / transcript_ref).write_text(transcript_text, encoding="utf-8")
    record = records.build(experiment_id=args.experiment, run_id=run.run_id, arm_id=plan.arm["arm"]["id"],
                           seed=seed, head=run.head, billed=plan.arm["arm"]["billed"], config_hash=run.config_hash,
                           snapshot_id=plan.snapshot.id,
                           suite={**run.suite, "generator_versions": {task.manifest["family"]:
                                                                  task.manifest["generator_version"]}},
                           stream=stream_position, materialized=task, outcome=outcome, result=result, final=final,
                           archived=archived, transcript_ref=transcript_ref, meter_usd=meter_usd,
                           verify_log=verify_log, model_swap=swap)
    if meter_usd is not None and record["costs"]["api_equiv_usd"] is not None:  # as `vb ledger reconcile` flags drift
        check = ledger._compare(record["costs"]["api_equiv_usd"], meter_usd, ledger.TOLERANCE)
        if check["flagged"]:
            _log_error(run_dir, key, "meter check", f"the ledger's ${check['ledger']} and the proxy's "
                       f"${check['export']} differ by more than ±{ledger.TOLERANCE:.0%}")
    try:
        records.append(run_dir / "records.jsonl", record)
    except records.RecordError as err:
        _log_error(run_dir, key, "record", err)
        return False
    finally:
        _cleanup(args, [workdir, *homes, private / "final", private / "census"])
    cost, flakes = record["costs"]["api_equiv_usd"], record["visible"]["flakes"]
    print(f"vb: {key} {record['execution']['status']} ({outcome.reason}) VS={record['vs']['label']} "
          f"turns={sum(a.turns for a in outcome.attempts)} cost={'unknown' if cost is None else f'${cost:.4f}'}"
          + ("" if meter_usd is None else f" meter=${meter_usd:.4f}") + (f" flakes={len(flakes)}" if flakes else ""),
          file=sys.stderr)
    return True


def _agent_deny(run: Run) -> tuple[Path, ...]:
    """What every agent process is denied (`TaskContext.deny`, `common.sandbox`): the secret file, every file the
    tripwire holds (the key file too), and the run's private task directories, every earlier task's included."""
    return tuple(dict.fromkeys([run.secret_file, *(wire.path for wire in secret.tripwires()), run.run_dir / "private"]))


def _verify_wrapper(run: Run, key: str, manifest: dict, env: dict[str, str], position: int) -> Path | None:
    """In a run with `flaky_verify`, the task's visible-verify wrapper, installed in the agent's `.vb-bin/` with the
    covering disturbance's p and seed (p = 0 at the positions it does not cover); None in any other run."""
    if not any(one.kind == "flaky_verify" for one in run.disturbances):
        return None
    p, seed = disturb.flake(run.disturbances, position)
    bin_dir = Path(env["PATH"].split(os.pathsep, 1)[0])  # agent_env's per-task .vb-bin/, which PATH starts with
    shell = shutil.which("bash", path=env["PATH"]) or "/bin/bash"  # the shell the direct loop would use
    return vb_verify.install(bin_dir, key=key, visible=manifest["visible_verify"], p=p, seed=seed, shell=shell)


def _preflight(runner: ModuleType, plan: Plan) -> None:
    """The runner's own start-up check, when it has one (`preflight`, harness.py), before the first task: the Roko
    arm's binary must accept the plans the arm emits (bug-a05c53). A refusal stops the run."""
    check = getattr(runner, "preflight", None)
    if not callable(check):
        return
    try:
        check(plan.arm, plan.model, plan.endpoint, plan.caps, plan.snapshot)
    except Exception as err:  # the runner owns its errors; any of them refuses the run
        raise DriverError(f"arm {plan.arm['arm']['id']}: {err}") from None


def load_runner(arm: dict) -> ModuleType:
    """The arm's runner module from driver/: `[arm] runner`, or the harness id with "-" read as "_"."""
    name = arm["arm"].get("runner") or arm["arm"]["harness"].replace("-", "_")
    if not re.fullmatch(r"[a-z_][a-z0-9_]*", name) or not (layout.DRIVER_DIR / f"{name}.py").is_file():
        raise DriverError(f"arm {arm['arm']['id']}: no runner module driver/{name}.py")
    module = importlib.import_module(name)
    if not callable(getattr(module, "run_task", None)):
        raise DriverError(f"driver/{name}.py has no run_task(ctx)")
    return module


def _endpoint(arm: dict, row: dict | None, provider_url: str | None) -> provider.Endpoint:
    providers = arm.get("providers", {})
    name = row["provider"] if row else (next(iter(providers)) if len(providers) == 1 else None)
    if name is None or (name not in providers and not provider_url):
        raise DriverError(f"arm {arm['arm']['id']} has no [providers.{name}] endpoint for this model")
    table = providers.get(name, {})
    # An overriding URL, such as a stub's, never gets the provider's key.
    keys = ("max_tokens_param", "timeout_s") if provider_url else ("api_key_env", "max_tokens_param", "timeout_s")
    fields = {key: table[key] for key in keys if key in table}
    return provider.Endpoint(provider=name, base_url=provider_url or table["base_url"], **fields)


def _start_proxy(plan: Plan, run_dir: Path, *, keys: Mapping[str, str] | None) -> faultproxy.FaultProxy:
    """The metering proxy in front of the plan's endpoint(s), logging to `<run_dir>/proxy.jsonl` (module docstring).
    One upstream per provider `plan.endpoints` names (3311: a ladder's several rungs may span providers). `keys`
    (`_provider_keys`) maps each upstream's `api_key_env` to its key: the proxy sends it, and the runners' endpoints
    name none."""
    try:
        upstreams = [faultproxy.Upstream.from_endpoint(endpoint) for endpoint in plan.endpoints.values()]
        return faultproxy.FaultProxy(upstreams, log_path=run_dir / PROXY_LOG, snapshot=plan.snapshot, keys=keys,
                                     input_token_cap=plan.caps.input_tokens_per_task).start()
    except (faultproxy.ProxyError, OSError) as err:
        archive.remove_tree(run_dir)  # made by this run a moment ago; nothing has run
        raise DriverError(f"the metering proxy cannot start: {err}") from None


def _provider_keys(value: Path | None, endpoints: Iterable[provider.Endpoint]) -> secret.ProviderKeys | None:
    """The key file's keys for every endpoint that names one (bug-979a06; 3311: a ladder needs one per provider its
    rungs use), read into the driver's memory for the proxy; None when none names one: an override URL never gets a
    key, and a CLI signs in by itself."""
    needed = sorted({endpoint.api_key_env for endpoint in endpoints if endpoint.api_key_env})
    if not needed:
        return None
    try:
        return secret.load_keys(secret.resolve_keys(value), need=needed)
    except secret.SecretError as err:  # also say where a key sits that belongs in the file
        raise DriverError("; ".join([str(err), *secret.key_exposures()])) from None


def _task_meter(proxy: faultproxy.FaultProxy | None, key: str) -> dict | None:
    """One task's totals in the proxy's meter (`FaultProxy.state`): all zero when no call of it reached the proxy,
    and None without a proxy."""
    if proxy is None:
        return None
    found = [meter for meter in proxy.state()["tasks"] if meter["task"] == key]
    return found[0] if found else {"task": key, "refused": 0, "cost_unknown": 0, "api_equiv_usd": 0.0}


def _end_on_proxy_cap(outcome: harness.TaskOutcome, meter: dict | None) -> None:
    """A task whose calls the proxy refused at its input-token cap ended on that cap, `aborted_cap` like one the
    direct loop's governor stops, when its runner reports it failed or in error. A completion or a timeout stands, and
    so does an outcome with no attempt (the runner crashed) or with an attempt its runner flagged (a model mismatch
    stays `infra_error`). The runner's own verdict stays in the transcript."""
    if not meter or not meter["refused"] or outcome.status not in ("failed", "infra_error") or not outcome.attempts \
            or any(getattr(attempt, "checks", None) for attempt in outcome.attempts):
        return
    outcome.transcript.append({"event": "input_token_cap", "refused_calls": meter["refused"],
                               "runner_status": outcome.status, "runner_reason": outcome.reason})
    outcome.status, outcome.reason = "aborted_cap", "input_token_cap"


def _mark_faults(attempts: list[harness.Attempt], log: Path, key: str) -> None:
    """Each attempt's `fault_injected` (S08 §4.6's ground truth): the first fault the proxy injected into its calls,
    which are the task's requests in order, `calls` of them each."""
    try:
        lines = log.read_text(encoding="utf-8").splitlines()
    except OSError:
        return
    rows = sorted((row for row in (json.loads(line) for line in lines if line.strip()) if row.get("task") == key),
                  key=lambda row: row.get("ordinal") or 0)
    start = 0
    for attempt in attempts:
        calls, start = rows[start:start + attempt.calls], start + attempt.calls
        attempt.fault_injected = next((row["fault_injected"] for row in calls if row.get("fault_injected")), None)


def _disturbances(path: Path | None, plan: Plan, proxied: bool) -> tuple[disturb.Disturbance, ...]:
    """The run's `--disturbance` spec (`disturb.py`), refused before anything runs when the run cannot apply it."""
    if path is None:
        return ()
    try:
        found = disturb.load(path)
    except disturb.DisturbanceError as err:
        raise DriverError(str(err)) from None
    for kind, what in (("provider_fault", "injects its faults"), ("model_swap", "swaps the model")):
        if not proxied and any(one.kind == kind for one in found):
            raise DriverError(f"{kind} {what} through the metering proxy: pass --proxy")
    for to in sorted({one.params["to"] for one in found if one.kind == "model_swap"}):
        if plan.snapshot.row(to) is None:
            raise DriverError(f"model_swap: the price snapshot {plan.snapshot.id} has no row for {to!r}, so the "
                              "swapped calls could not be priced")
    for latent in sorted({one.params["latent"] for one in found if one.kind == "convention_flip"}):
        for family, directory in sorted(plan.stream.families.items()):  # render one instance of each family
            instance = next((i for i in plan.instances if knobs.parse_instance_id(i)[0] == family), None)
            if instance is None:
                continue
            with tempfile.TemporaryDirectory(prefix="vb-latent-") as tmp:
                try:
                    materialize.materialize(family_dir=directory, instance_id=instance, workdir=Path(tmp) / "work",
                                            private_dir=Path(tmp) / "private", latent=latent)
                except (materialize.MaterializeError, repo.RepoError, OSError, ValueError) as err:
                    raise DriverError(f"convention_flip: family {family} cannot render latent {latent}: {err}") \
                        from None
    return tuple(found)


def _secret_file(value: Path | None) -> Path:
    path = Path(value or os.environ.get("VB_SECRET_FILE") or DEFAULT_SECRET_FILE).expanduser().absolute()
    try:
        info = path.lstat()
    except OSError:
        raise DriverError(f"no secret file at {path}; create one (mode 0600) or pass --secret-file") from None
    if not stat.S_ISREG(info.st_mode) or info.st_mode & 0o077:
        raise DriverError(f"the secret file {path} must be a regular file with mode 0600")
    if layout.within(path, layout.REPO_ROOT):
        raise DriverError("the secret file must live outside the repository")
    return path


def _outside_repo(value: Path | str, flag: str) -> Path:
    path = Path(value).expanduser().absolute()
    if layout.within(path, layout.REPO_ROOT) or layout.within(path, Path("~/.roko").expanduser()):
        raise DriverError(f"{flag} {path} must be outside the repository and ~/.roko")
    return path


def _suite_hash(plan: Plan) -> str:
    """sha256 over the files of the stream's families and the common library (caches excluded)."""
    digests = {}
    for name, directory in sorted({**plan.stream.families, "common": layout.FAMILIES_DIR / "common"}.items()):
        files = sorted(p for p in Path(directory).rglob("*") if p.is_file() and not p.is_symlink()
                       and not set(p.relative_to(directory).parts) & set(SKIP_IN_SUITE_HASH) and p.suffix != ".pyc")
        digests[name] = {p.relative_to(directory).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
    return records.canonical_hash(digests)


def _config_path(name: str, directory: Path) -> Path:
    path = Path(name) if name.endswith(".toml") or "/" in name else directory / f"{name}.toml"
    if not path.is_file():
        raise DriverError(f"no such file: {path}")
    return path.resolve()


def _toml(path: Path) -> dict:
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except (OSError, tomllib.TOMLDecodeError) as err:
        raise DriverError(f"{path}: {err}") from None


def _write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")


def _log_error(run_dir: Path, key: str, stage: str, err: object) -> None:
    line = json.dumps({"ts": harness.utc_now(), "task": key, "stage": stage, "error": str(err)[:1000]})
    with (run_dir / "errors.jsonl").open("a", encoding="utf-8") as handle:
        handle.write(line + "\n")
    print(f"vb: {key}: {stage} failed: {str(err)[:300]}", file=sys.stderr)


def _cleanup(args: argparse.Namespace, paths: list[Path]) -> None:
    if args.keep_workdirs:
        return
    for path in paths:
        archive.remove_tree(path)


def _positive_float(text: str) -> float:
    try:
        value = float(text)
    except ValueError:
        raise argparse.ArgumentTypeError("must be a number") from None
    if not value > 0 or value == float("inf"):
        raise argparse.ArgumentTypeError("must be a finite number above 0")
    return value


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="vb", description="The ViabilityBench driver (S08 §5.7).",
                                     allow_abbrev=False)
    commands = parser.add_subparsers(dest="command", required=True)

    def planned(sub: argparse.ArgumentParser) -> None:
        sub.add_argument("--stream", required=True, help="a stream name in streams/, or a path to one")
        sub.add_argument("--arm", required=True, help="an arm name in arms/, or a path to one")
        sub.add_argument("--model", required=True)
        sub.add_argument("--seeds", default="1", help="run seeds, e.g. 1-3 or 1,3 (default 1)")
        sub.add_argument("--limit", type=int, default=0, help="run only the stream's first N instances")
        sub.add_argument("--price-snapshot", default=ledger.DEFAULT_SNAPSHOT)
        sub.add_argument("--provider-url", help="override the provider's base URL; a loopback URL runs offline")

    run = commands.add_parser("run", help="run a stream on one arm and model", allow_abbrev=False)
    planned(run)
    run.add_argument("--experiment", required=True)
    run.add_argument("--run-id")
    run.add_argument("--line", help="budget line for the ledger (default: the arm's)")
    run.add_argument("--allow-network", action="store_true", help="allow calls to a non-loopback provider")
    run.add_argument("--max-cost-usd", type=_positive_float, help="required with --allow-network")
    run.add_argument("--proxy", action="store_true",
                     help="route an offline or unbilled run through the metering proxy (faultproxy.py), as every "
                          "billed network run is; its log is proxy.jsonl")
    run.add_argument("--disturbance", type=Path, help="a vb.disturbance/1 spec: the H6 hooks to apply, and the "
                     "stream positions they cover (driver/disturb.py)")
    run.add_argument("--secret-file", type=Path, help="default: $VB_SECRET_FILE, then " + str(DEFAULT_SECRET_FILE))
    run.add_argument("--key-file", type=Path, help=f"provider keys, NAME=value; default: ${secret.KEYS_FILE_ENV}, then "
                     f"{secret.KEYS_DEFAULT_PATH}")
    run.add_argument("--results", type=Path, help="default: $VB_RESULTS, then " + str(DEFAULT_RESULTS))
    run.add_argument("--work", type=Path, help="default: $VB_WORK, then " + str(DEFAULT_WORK))
    run.add_argument("--transcripts", action="store_true", help="keep transcripts in the run directory")
    run.add_argument("--keep-workdirs", action="store_true", help="keep workdirs and census exports")
    run.set_defaults(handler=cmd_run)

    estimate = commands.add_parser("estimate", help="print the plan and its worst-case cost", allow_abbrev=False)
    planned(estimate)
    estimate.set_defaults(handler=cmd_estimate)
    ledger.add_parser(commands, DEFAULT_RESULTS)  # vb ledger report|reconcile (S09 E2)

    mat = commands.add_parser("materialize", help="render one instance as the driver would", allow_abbrev=False)
    mat.add_argument("--stream", required=True)
    mat.add_argument("--instance", required=True)
    mat.add_argument("--out", required=True)
    mat.add_argument("--private", help="where the manifest and pristine bundle go (default: OUT.private)")
    mat.set_defaults(handler=cmd_materialize)
    commands.add_parser("report", help="metrics.json and bundle checks (analysis/report.py; see vb report --help)")
    run_campaign = commands.add_parser("campaign", help="validate, estimate and run an experiment manifest",
                                       allow_abbrev=False)
    campaign.add_arguments(run_campaign)
    run_campaign.set_defaults(handler=cmd_campaign)
    return parser


if __name__ == "__main__":
    sys.exit(main())
