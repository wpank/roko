"""`vb refine`: the spec refiner R-v1 (`specops/refine.py`, S07 §4.3) on a live model, ledgered (task 3236).

    vb refine --experiment REFINE-COST --line BL15 --allow-network --max-cost-usd 0.40 [--tasks 20] [--seed 3236] \
              [--arm cheap_direct] [--model gpt-oss-120b] [--budget-per-task 0.02] [--max-tokens 4096] \
              [--results DIR] [--run-id ID] [--key-file PATH] [--provider-url URL] [--proxy] [--dry-run]
    vb refine --summary RUN_DIR [--bundle DIR]

S07.4 asks what the refiner costs on a real cheap model: its mean ledgered cost must stay at or below $0.02 a task on
a 20-task live sample. `specops/refine.py` takes any `refine.Model`; its tests pass a stub. This module is the live
one and the run that measures it.

**The sample** (3236's plan). `--tasks` band C or D tasks under the linter in force (`speclint.LINTER`, static mode),
half from the active plans (`plans/`, nothing under an `archive` directory) and half from speclint's golden fixtures
(`speclint/fixtures/`), each half the first of its pool when ordered by sha256 of (`--seed`, plan path, task id). The
sample is the same on every host for the same corpus and seed, and the run's manifest lists it with each task's spec
hash, so a later corpus change cannot hide what ran.

**The model** (`LiveModel`). Each round is one chat call: a system prompt with S07 §4.1's additive fields and the
refiner's rules, and the spec as JSON with its linter score and the rules below full credit. The reply must be one
JSON object, `{"additions": [{"field", "item", "source"}]}` (`parse_additions`); an item of the wrong type for its
field is dropped and counted, and a field outside `refine.ADDITIVE_FIELDS` is kept, so that `refine`'s own guards
reject the round as they would any model's. A retryable provider error is retried twice (`Retry-After`, else 2^n
seconds, at most 30). The task's budget (`--budget-per-task`, S07 §4.3's $0.02, `roko.toml`'s `refine_budget_usd`)
holds every call: before each one, the calls so far (a call whose cost is unknown counts at its worst case) plus this
call's worst case (its bytes plus `PREAMBLE_TOKENS` of input at the input rate, `--max-tokens` of output) must fit,
or the model proposes nothing and the task stops on `budget`. So no task can cost more than its budget.

**Fences**, those of a billed `vb run` call, in the same order. A non-loopback provider needs `--allow-network` and
`--max-cost-usd`, and a model without a price row is refused, before anything is written. The provider's key is read
from the driver-only key file (`--key-file`, `$VB_KEY_FILE`, `~/.config/viabilitybench/keys`; `secret.load_keys`)
into this process, and only the metering proxy (`faultproxy`) sends it: the chat client gets the proxy's loopback URL
and no key, and the run refuses to start while the environment holds a provider key (`secret.key_exposures`). The
proxy forwards only plain chat completions with `--model` (its policy), and holds each task to the input its budget
could buy at the input rate. Every task reserves its budget on `--line` before its first call (`Ledger.reserve`), so
a task that could take the line, its experiment's cap or the programme stop past their caps never starts, and a
task that would pass `--max-cost-usd` never starts either. Each task that made a call appends one ledger row, priced
from the provider's reported usage by the reported model's row (`ledger.price`), as the direct loop's attempts do, or
by the requested model's when none reported one: calls that all ended in an HTTP error billed zero tokens, which cost
$0, not an unknown amount charged at the task's reservation (the direct loop's bug-2a5566). No
agent runs here, and nothing runs the spec's commands: there is no workdir, no agent environment, and no benchmark
secret, so neither the secret file nor the tripwire is involved, and nothing is written next to the key file.

**Outputs** in `$VB_RESULTS/<experiment>/<run_id>/` (default `~/.roko-bench/viability`): `manifest.json` (the config,
its hash, the sample, the proxy's policy), `refine.jsonl` (one `vb.refine_record/1` per task: the scores and bands
before and after, each round's `spec.refined` record, what the model proposed each round and whether it was kept, why
refinement stopped, the calls, the usage and the ledgered cost, and the proxy's meter for the task), `ledger.jsonl` and
`reservations.jsonl`, `proxy.jsonl`, `transcripts.jsonl` (the model's replies, never committed), `errors.jsonl` and
`summary.md`.

**The summary** (`summarize`, `--summary RUN_DIR`): tasks, rounds, mean, median and max cost a task, the score change,
the hard fails before and after (a round that raises the score is kept even when it adds one), a row per task, and the
line `within the $<budget> cap: yes|no`: yes when every task of the sample ran, the model
answered each of them at least once (a task whose every call failed measured nothing), and every task's ledgered cost
is known and at most the per-task budget, so the mean is too. `--bundle DIR` writes it with the run's `manifest.json`, `refine.jsonl`
and `ledger.jsonl` (no transcripts or proxy log), the bundle committed under `reports/refiner-cost/`.

Exit status: 0 when every task of the sample got a record, 1 when the run stopped early (see `errors.jsonl`), 2 for a
usage, configuration or admission error, before anything is written.

API:
    LiveModel(chat, *, model, price_row, budget_usd, ctx, max_tokens=MAX_TOKENS, timeout_s=180.0, linter=...,
              max_rounds=refine.MAX_ROUNDS, sleep=time.sleep)          # a refine.Model
    LiveModel: .calls, .usage, .usage_unknown, .model_reported, .spent_bound_usd, .stop, .proposals, .transcript;
               .reported_usage() -> dict | None; .messages(spec, round_number) -> list[dict]
    parse_additions(text) -> (additions, dropped, problem)
    Candidate(group, ctx, task, record); select(root, *, count, seed, linter=speclint.LINTER, groups=GROUPS)
    summarize(run_dir) -> str; write_bundle(run_dir, out_dir) -> Path
    add_arguments(parser); cmd_refine(vb, args) -> int                   # vb.py wires `vb refine`
    PROMPT_VERSION, PROMPT_SHA256, SYSTEM_PROMPT, ROUND_MESSAGE, GROUPS
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import math
import os
import re
import secrets
import shutil
import statistics
import sys
import time
from collections import Counter
from collections.abc import Callable
from dataclasses import asdict, dataclass
from pathlib import Path
from types import ModuleType

import archive
import faultproxy
import harness
import layout
import ledger
import provider
import records
import secret

if str(layout.VB_ROOT) not in sys.path:
    sys.path.insert(0, str(layout.VB_ROOT))
from specops import refine  # noqa: E402  (refine puts speclint/ on sys.path)

import speclint  # noqa: E402

PROMPT_VERSION = "refine-live-2"
DEFAULT_EXPERIMENT = "REFINE-COST"
DEFAULT_LINE = "BL15"  # experiments/budget.toml: decision 5's $0.40 for this sample
DEFAULT_ARM = "cheap_direct"  # whose [providers.cerebras] endpoint serves gpt-oss-120b
DEFAULT_MODEL = "gpt-oss-120b"
DEFAULT_TASKS = 20
DEFAULT_SEED = 3236
BUDGET_PER_TASK_USD = 0.02  # S07 §4.3: a refinement costs at most $0.02 a task (`[spec_quality] refine_budget_usd`)
MAX_TOKENS = 4096  # a round's output, reasoning included (gpt-oss-120b's `reasoning_in_output`)
RETRIES = 2
MAX_RETRY_WAIT_S = 30.0
PREAMBLE_TOKENS = 256  # what a provider may put before the first message (a harmony system header), as mini_loop
MESSAGE_OVERHEAD_TOKENS = 16  # chat-template tokens a message may add beyond its bytes, as mini_loop
BANDS = ("C", "D")
# The sample's two halves: (group, its corpus relative to the repository root).
GROUPS = (("plans", "plans"), ("fixtures", "benchmarks/viabilitybench/speclint/fixtures"))
RECORDS = "refine.jsonl"
TRANSCRIPTS = "transcripts.jsonl"
SUMMARY = "summary.md"
PROXY_LOG = "proxy.jsonl"
BUNDLED = ("manifest.json", RECORDS, "ledger.jsonl")  # what --bundle copies beside summary.md
CAP_LINE = "within the ${cap} cap: {verdict}"  # the summary's verdict line, which task 3236's verify reads
STRING_FIELDS = ("acceptance", "non_goals", "assumptions", "context.symbols")
TABLE_FIELDS = {"verify": "command", "context.read_files": "path"}  # each item's required key
FENCE = re.compile(r"```(?:json)?[ \t]*\r?\n(.*?)\r?\n[ \t]*```", re.DOTALL)

SYSTEM_PROMPT = """\
You refine task specs for a coding agent. A spec is a JSON object. You may only ADD items to six fields, and each
addition must say where it comes from.

Fields and the shape of each item:
- "acceptance": a string "AC<n>: <criterion>". A criterion is observable: an input and the output it gives, a file
  or state that must exist, an exit code, or a message. Number new criteria after the existing ones.
- "verify": an object {"phase": "test", "command": "<one shell command>", "covers": ["AC<n>"],
  "expect": "fail_on_base"}. The command runs a test runner scoped to one test file or test name, or asserts on
  the state the task leaves (grep -q, test -f, diff), and it fails until the task is done.
- "non_goals": a string naming something the task must not do or change.
- "assumptions": a string saying how you resolved something the spec leaves open.
- "context.read_files": an object {"path": "<a file that already exists in the repository now>", "why": "<what
  the implementer learns from it>"}. Never name a file only because the task's own "files" lists it: that file
  may not exist yet, since the task has not run.
- "context.symbols": a string "exact:<identifier>", a function or type defined in one of the read_files.

Rules:
- Never change or remove anything already in the spec, never touch "files", and never name a hidden test path
  (".vb/", any "hidden" directory, "task.json").
- Add only what the spec already implies through its title, goal, description, files and items. Never invent a
  requirement.
- Each new acceptance criterion needs a new verify step that lists it in "covers", in the same reply.
- A verify step that ends in "|| true" or "|| exit 0" always exits zero, so it checks nothing; never write one
  that way.
- Each addition needs a "source": the spec field, file or symbol it comes from, such as "description",
  "title" or "files: src/app.py".

Reply with one JSON object and nothing else:
{"additions": [{"field": "<one of the six fields>", "item": <a string or an object, as above>, "source": "<where>"}]}
Reply {"additions": []} when nothing useful can be added.
"""

ROUND_MESSAGE = """\
Round {round} of at most {max_rounds}. The spec linter ({linter}) scores this spec {score:.1f} of 100 (band {band}).
Rules below full credit: {weak}.

The spec:
{spec}
"""

PROMPT_SHA256 = hashlib.sha256("\0".join([PROMPT_VERSION, SYSTEM_PROMPT, ROUND_MESSAGE]).encode()).hexdigest()


class LiveModel:
    """`refine.Model` on a live chat model (module docstring, "The model"): one call per round, retried on a
    retryable error, never past the task's budget. `calls`, `usage` and `model_reported` are what the task's ledger row
    books; `stop` says why it proposed nothing when that was not the model's own answer; `proposals` holds what the
    model proposed each round, and `transcript` every reply and error."""

    def __init__(self, chat: provider.ChatProvider, *, model: str, price_row: dict, budget_usd: float,
                 ctx: speclint.PlanContext, max_tokens: int = MAX_TOKENS, timeout_s: float = 180.0,
                 linter: str = speclint.LINTER, max_rounds: int = refine.MAX_ROUNDS,
                 sleep: Callable[[float], None] = time.sleep) -> None:
        self.chat = chat
        self.model = model
        self.row = price_row
        self.budget_usd = budget_usd
        self.ctx = ctx
        self.max_tokens = max_tokens
        self.timeout_s = timeout_s
        self.linter = linter
        self.max_rounds = max_rounds
        self.sleep = sleep
        self.calls = 0
        self.replies = 0  # calls the model answered (the rest raised a provider error)
        self.usage = ledger.vb_usage(0, 0)
        self.usage_unknown = False  # some call may have been billed without reporting usage
        self.model_reported: str | None = None
        self.spent_bound_usd = 0.0  # known costs, plus the worst case of every call whose cost is unknown
        self.stop: str | None = None
        self.proposals: list[dict] = []
        self.transcript: list[dict] = []

    def reported_usage(self) -> dict | None:
        return None if self.usage_unknown else self.usage

    def messages(self, spec: dict, round_number: int) -> list[dict]:
        scored = speclint.score_task(spec, self.ctx, linter=self.linter)
        weak = [f"{rule} {speclint.RULE_NAMES[rule]} {value:.2f} (weight {speclint.WEIGHTS[rule]})"
                for rule, value in scored["rules"].items() if rule not in scored["excluded"] and value < 1.0]
        text = json.dumps(spec, indent=1, sort_keys=True, ensure_ascii=False, default=str)
        return [{"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": ROUND_MESSAGE.format(
                    round=round_number, max_rounds=self.max_rounds, linter=self.linter, score=scored["score"],
                    band=scored["band"], weak="; ".join(weak) or "none", spec=text)}]

    def __call__(self, spec: dict, round_number: int) -> refine.Proposal:
        messages = self.messages(spec, round_number)
        bound = PREAMBLE_TOKENS + sum(len(m["content"].encode("utf-8")) + MESSAGE_OVERHEAD_TOKENS for m in messages)
        worst = ledger.worst_case_usd(self.row, input_tokens=bound, output_tokens=self.max_tokens)
        failures = 0
        while True:
            if worst is None or round(self.spent_bound_usd + worst, 9) > self.budget_usd:
                return self._nothing(round_number, "budget", f"the next call could cost ${worst or 0:.6f}, and "
                                     f"${self.spent_bound_usd:.6f} of the task's ${self.budget_usd:.2f} is spent")
            self.calls += 1  # counted before the request, so a call that raises is still in the ledger row
            try:
                completion = self.chat.complete(messages, model=self.model, max_tokens=self.max_tokens,
                                                timeout_s=self.timeout_s)
                break
            except Exception as raised:  # every failed call is accounted for; a non-ProviderError is a client bug
                err = raised if isinstance(raised, provider.ProviderError) else provider.ProviderError(
                    f"{type(raised).__name__}: {raised}", retryable=False, billed_unknown=True)
                if err.billed_unknown:
                    self.usage_unknown = True
                    self.spent_bound_usd += worst
                self.transcript.append({"round": round_number, "event": "provider_error", "error": str(err)[:1000],
                                        "retryable": err.retryable, "billed_unknown": err.billed_unknown})
                failures += 1
                if not err.retryable or failures > RETRIES:
                    return self._nothing(round_number, "provider_error", str(err)[:300])
                self.sleep(min(err.retry_after or 2.0 ** failures, MAX_RETRY_WAIT_S))
        self.replies += 1
        cost = None
        if completion.usage is None:
            self.usage_unknown = True
            self.spent_bound_usd += worst
        else:
            used = completion.usage
            call_usage = ledger.vb_usage(used.prompt_tokens, used.completion_tokens, used.cached_tokens,
                                         used.reasoning_tokens)
            self.usage = ledger.add_usage(self.usage, call_usage)
            cost = ledger.price(call_usage, self.row).api_equiv_usd
            self.spent_bound_usd += cost
        self.model_reported = self.model_reported or completion.model_reported
        additions, dropped, problem = parse_additions(completion.content)
        self.transcript.append({"round": round_number, "event": "reply", "finish_reason": completion.finish_reason,
                                "model_reported": completion.model_reported, "cost_usd": cost,
                                "content": completion.content})
        if problem:
            return self._nothing(round_number, "unreadable_reply", f"{problem} (finish_reason "
                                 f"{completion.finish_reason})", cost)
        self.proposals.append({"round": round_number, "additions": [asdict(one) for one in additions],
                               "dropped": dropped, "cost_usd": cost})
        return refine.Proposal(model=completion.model_reported or self.model, additions=additions,
                               cost_usd=worst if cost is None else cost)

    def _nothing(self, round_number: int, kind: str, detail: str, cost: float | None = 0.0) -> refine.Proposal:
        """An empty proposal, which ends `refine`'s loop, with why: the task's `stop`."""
        self.stop = f"{kind}: {detail}"
        self.transcript.append({"round": round_number, "event": "stop", "kind": kind, "detail": detail})
        return refine.Proposal(model=self.model_reported or self.model, additions=[], cost_usd=cost or 0.0)


def parse_additions(text: str) -> tuple[list[refine.Addition], int, str | None]:
    """The additions a reply proposes, how many malformed entries were dropped, and why the reply could not be read
    (None when it could). An entry needs a string `field`; an item of the wrong type for one of the six fields (a
    criterion that is not a string, a step without a command) is dropped; a missing source becomes "" and a field
    outside the six is kept, so that `refine` rejects the round (module docstring)."""
    doc = _json_object(text or "")
    if doc is None or not isinstance(doc.get("additions"), list):
        return [], 0, "the reply holds no JSON object with an additions list"
    additions: list[refine.Addition] = []
    dropped = 0
    for entry in doc["additions"]:
        if not isinstance(entry, dict) or not isinstance(entry.get("field"), str):
            dropped += 1
            continue
        name, item, source = entry["field"].strip(), entry.get("item"), entry.get("source")
        if name == "context.read_files" and isinstance(item, str):
            item = {"path": item}  # speclint reads a bare string as a path
        if name in STRING_FIELDS:
            well_formed = isinstance(item, str) and bool(item.strip())
        elif name in TABLE_FIELDS:
            wanted = item.get(TABLE_FIELDS[name]) if isinstance(item, dict) else None
            well_formed = isinstance(wanted, str) and bool(wanted.strip())
        else:
            well_formed = True
        if not well_formed:
            dropped += 1
            continue
        additions.append(refine.Addition(field=name, item=item.strip() if isinstance(item, str) else item,
                                         source=source.strip() if isinstance(source, str) else ""))
    return additions, dropped, None


def _json_object(text: str) -> dict | None:
    """The first JSON object in a reply: all of it, a fenced block, or what lies between its first `{` and last `}`."""
    candidates = [text.strip(), *FENCE.findall(text)]
    start, end = text.find("{"), text.rfind("}")
    if 0 <= start < end:
        candidates.append(text[start:end + 1])
    for candidate in candidates:
        try:
            value = json.loads(candidate)
        except ValueError:
            continue
        if isinstance(value, dict):
            return value
    return None


@dataclass(frozen=True)
class Candidate:
    group: str  # a GROUPS name: "plans" or "fixtures"
    ctx: speclint.PlanContext
    task: dict
    record: dict  # its spec.quality record before refinement

    def entry(self, position: int) -> dict:
        """The manifest's line for this task."""
        return {"position": position, "group": self.group, "plan_path": self.ctx.plan_path,
                "plan_id": self.ctx.plan_id, "task_id": self.record["task_id"], "role": self.record["role"],
                "spec_hash": self.record["spec_hash"], "score": self.record["score"], "band": self.record["band"]}


def select(root: Path, *, count: int, seed: int, linter: str = speclint.LINTER,
           groups: tuple[tuple[str, str], ...] = GROUPS) -> list[Candidate]:
    """3236's sample (module docstring, "The sample"): `count` band C/D tasks, half from each group."""
    if count < len(groups) or count % len(groups):
        raise ValueError(f"--tasks must be a positive multiple of {len(groups)}, half from each of "
                         f"{', '.join(name for name, _ in groups)}")
    chosen: list[Candidate] = []
    for group, relative in groups:
        pairs, _ = speclint.plan_tasks(speclint.discover([root / relative]), root)  # a file that does not parse adds none
        pool = []
        for ctx, task in pairs:
            if ctx.archived:
                continue
            record = speclint.score_task(task, ctx, linter=linter)
            if record["band"] in BANDS:
                pool.append(Candidate(group, ctx, task, record))

        def draw(candidate: Candidate) -> str:
            return hashlib.sha256(f"{seed}\0{candidate.ctx.plan_path}\0{candidate.record['task_id']}".encode()
                                  ).hexdigest()

        wanted = count // len(groups)
        if len(pool) < wanted:
            raise ValueError(f"{relative} has {len(pool)} band {'/'.join(BANDS)} task(s) under {linter}, fewer than "
                             f"{wanted}")
        chosen += sorted(pool, key=draw)[:wanted]
    return chosen


def add_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--experiment", default=DEFAULT_EXPERIMENT, help=f"default: {DEFAULT_EXPERIMENT}")
    parser.add_argument("--run-id")
    parser.add_argument("--line", default=DEFAULT_LINE, help=f"the budget line (default: {DEFAULT_LINE})")
    parser.add_argument("--arm", default=DEFAULT_ARM, help=f"the arm whose provider endpoint serves the model (default: "
                                                           f"{DEFAULT_ARM})")
    parser.add_argument("--model", default=DEFAULT_MODEL, help=f"default: {DEFAULT_MODEL}")
    parser.add_argument("--tasks", type=int, default=DEFAULT_TASKS, help=f"the sample's size (default: {DEFAULT_TASKS})")
    parser.add_argument("--seed", type=int, default=DEFAULT_SEED, help=f"the sample's draw (default: {DEFAULT_SEED})")
    parser.add_argument("--budget-per-task", type=_positive, default=BUDGET_PER_TASK_USD,
                        help=f"the most one task's refinement may cost (default: ${BUDGET_PER_TASK_USD})")
    parser.add_argument("--max-tokens", type=int, default=MAX_TOKENS, help=f"each call's output cap (default: "
                                                                           f"{MAX_TOKENS})")
    parser.add_argument("--allow-network", action="store_true", help="allow calls to a non-loopback provider")
    parser.add_argument("--max-cost-usd", type=_positive, help="required with --allow-network")
    parser.add_argument("--proxy", action="store_true", help="route an offline run through the metering proxy too")
    parser.add_argument("--key-file", type=Path, help=f"provider keys, NAME=value; default: ${secret.KEYS_FILE_ENV}, "
                                                      f"then {secret.KEYS_DEFAULT_PATH}")
    parser.add_argument("--results", type=Path, help="default: $VB_RESULTS, then ~/.roko-bench/viability")
    parser.add_argument("--price-snapshot", default=ledger.DEFAULT_SNAPSHOT)
    parser.add_argument("--provider-url", help="override the provider's base URL; a loopback URL runs offline")
    parser.add_argument("--dry-run", action="store_true", help="print the sample and its worst case; call nothing")
    parser.add_argument("--summary", type=Path, metavar="RUN_DIR", help="print a finished run's summary instead")
    parser.add_argument("--bundle", type=Path, metavar="DIR", help="with --summary: write the summary bundle there")


def cmd_refine(vb: ModuleType, args: argparse.Namespace) -> int:
    """`vb refine` (module docstring). `vb` is the driver module, whose checks and errors this reuses."""
    if args.summary is not None:
        run_dir = args.summary.expanduser().absolute()
        if args.bundle is not None:
            print(write_bundle(run_dir, args.bundle))
        else:
            print(summarize(run_dir))
        return 0
    if args.bundle is not None:
        raise vb.DriverError("--bundle goes with --summary RUN_DIR")
    if vb.campaign.requires_lock(args.experiment):
        raise vb.DriverError(f"experiment {args.experiment} runs only under the pre-registration lock; the refiner's "
                             "cost sample is not one of those")
    for value, flag in ((args.experiment, "--experiment"), (args.run_id or "x", "--run-id")):
        if not vb.ID_RE.fullmatch(value):
            raise vb.DriverError(f"{flag} must match {vb.ID_RE.pattern}")
    arm = vb.load_arm(args.arm)
    if args.model not in arm["arm"]["models_allow"]:
        raise vb.DriverError(f"arm {arm['arm']['id']} allows {', '.join(arm['arm']['models_allow'])}, not {args.model}")
    snapshot = ledger.load_snapshot(args.price_snapshot)
    row = snapshot.row(args.model)
    if row is None:
        raise vb.DriverError(f"{args.model} has no row in {snapshot.id}, so its cost cannot be bounded")
    if args.max_tokens < 1:
        raise vb.DriverError("--max-tokens must be at least 1")
    endpoint = vb._endpoint(arm, row, args.provider_url)
    billed = arm["arm"]["billed"]
    budget = args.budget_per_task
    try:
        sample = select(layout.REPO_ROOT, count=args.tasks, seed=args.seed)
    except ValueError as err:
        raise vb.DriverError(str(err)) from None
    input_cap = max(1, math.floor(budget * ledger.PER_TOKENS / row["input"]))  # the input the budget could buy
    plan = {"experiment_id": args.experiment, "line": args.line, "arm": arm["arm"]["id"], "model": args.model,
            "provider": endpoint.provider, "base_url": endpoint.base_url, "network": not endpoint.offline,
            "price_snapshot_id": snapshot.id, "linter": speclint.LINTER, "seed": args.seed, "tasks": len(sample),
            "budget_per_task_usd": budget, "worst_case_usd": round(budget * len(sample), 6),
            "max_tokens": args.max_tokens, "proxy_input_token_cap": input_cap,
            "prompt": {"version": PROMPT_VERSION, "sha256": PROMPT_SHA256},
            "sample": [candidate.entry(position) for position, candidate in enumerate(sample, 1)]}
    if args.dry_run:
        print(json.dumps(plan, indent=2, ensure_ascii=False))
        return 0
    if not endpoint.offline:  # vb.admit's rule, before anything is written or called
        if not args.allow_network:
            raise vb.DriverError(f"{endpoint.base_url} is a network provider; inspect `vb refine --dry-run` and pass "
                                 "--allow-network explicitly")
        if args.max_cost_usd is None:
            raise vb.DriverError("network runs require an explicit --max-cost-usd budget")
        if budget > args.max_cost_usd:
            raise vb.DriverError(f"one task may cost up to its ${budget:.2f} budget, more than --max-cost-usd "
                                 f"${args.max_cost_usd:.2f}")
    proxied = args.proxy or (billed and not endpoint.offline)
    keys = vb._provider_keys(args.key_file, [endpoint]) if proxied else None
    if proxied and not endpoint.offline and keys is None:
        raise vb.DriverError(f"the metering proxy sends {endpoint.provider}'s key, but arm {arm['arm']['id']} names no "
                             "api_key_env")
    exposed = secret.key_exposures(keys)
    if exposed:
        raise vb.DriverError("refusing to run, a provider key sits where it could leak: " + "; ".join(exposed))
    results_root = vb._outside_repo(args.results or os.environ.get("VB_RESULTS") or vb.DEFAULT_RESULTS, "--results")
    run_id = args.run_id or f"refine-{dt.datetime.now(dt.UTC):%Y%m%d}-{secrets.token_hex(3)}"
    run_dir = results_root / args.experiment / run_id
    if run_dir.exists():
        raise vb.DriverError(f"{run_dir} already exists; pick another --run-id")
    book = ledger.Ledger(run_dir / "ledger.jsonl", line=args.line, experiment_id=args.experiment, run_id=run_id,
                         price_snapshot_id=snapshot.id, root=results_root)
    refusal = book.refusal(budget if billed else 0.0)
    if refusal:  # an unfunded line, or no room for one task: nothing is written
        raise vb.DriverError(f"budget: {refusal}")
    run_dir.mkdir(mode=0o700, parents=True)
    proxy = None
    if proxied:
        try:
            proxy = faultproxy.FaultProxy([faultproxy.Upstream.from_endpoint(endpoint)], log_path=run_dir / PROXY_LOG,
                                          snapshot=snapshot, keys=keys, models=[args.model],
                                          input_token_cap=input_cap).start()
        except (faultproxy.ProxyError, OSError) as err:
            archive.remove_tree(run_dir)  # made a moment ago; nothing has run
            raise vb.DriverError(f"the metering proxy cannot start: {err}") from None
    chat = provider.OpenAICompatible(proxy.endpoint(endpoint) if proxy else endpoint)
    config = {"driver": vb.DRIVER_VERSION, "runner": "refine_live", **{key: value for key, value in plan.items()
                                                                       if key != "sample"},
              "max_rounds": refine.MAX_ROUNDS, "retries": RETRIES,
              "sample": [entry["spec_hash"] for entry in plan["sample"]]}
    head = records.harness_state()
    _write_json(run_dir / "manifest.json", {
        "schema_version": "vb.refine_manifest/1", "experiment_id": args.experiment, "run_id": run_id,
        "started_at": harness.utc_now(), "argv": getattr(args, "argv", ["vb", "refine"]), "harness_sha": head[0],
        "dirty": head[1], "config_hash": records.canonical_hash(config), "config": config, "offline": endpoint.offline,
        "billed": billed, "proxy": {"log": PROXY_LOG, "policy": proxy.policy(), "input_token_cap": input_cap}
        if proxy else None, "sample": plan["sample"]})
    written = 0
    try:
        for position, candidate in enumerate(sample, 1):
            attempt_key = f"{run_id}/{position:02d}:1"
            refusal = book.refusal(budget if billed else 0.0)
            if refusal:
                _log_error(run_dir, attempt_key, "budget", f"stopped before the task: {refusal}")
                break
            if args.max_cost_usd is not None and round(book.spent_bound_usd + budget, 9) > args.max_cost_usd:
                _log_error(run_dir, attempt_key, "budget", f"stopped: ${book.spent_bound_usd:.4f} spent; the next "
                           "task could pass --max-cost-usd")
                break
            if proxy:
                proxy.configure(task=attempt_key)
            try:
                book.reserve(attempt_key, budget if billed else 0.0)
            except ledger.BudgetError as err:
                _log_error(run_dir, attempt_key, "budget", f"stopped before the task: {err}")
                break
            written += _refine_one(run_dir, book, snapshot, candidate, position, attempt_key, chat=chat, row=row,
                                   endpoint=endpoint, proxy=proxy, budget=budget, billed=billed, args=args)
    finally:
        if proxy:
            proxy.close()
        (run_dir / SUMMARY).write_text(summarize(run_dir) + "\n", encoding="utf-8")
        print(f"vb: {written}/{len(sample)} records in {run_dir}", file=sys.stderr)
    return 0 if written == len(sample) else 1


def _refine_one(run_dir: Path, book: ledger.Ledger, snapshot: ledger.Snapshot, candidate: Candidate, position: int,
                attempt_key: str, *, chat: provider.ChatProvider, row: dict, endpoint: provider.Endpoint,
                proxy: faultproxy.FaultProxy | None, budget: float, billed: bool, args: argparse.Namespace) -> bool:
    """Refine one task, book its calls on the ledger, and append its record; whether the record was written."""
    model = LiveModel(chat, model=args.model, price_row=row, budget_usd=budget, ctx=candidate.ctx,
                      max_tokens=args.max_tokens, timeout_s=endpoint.timeout_s)
    started, result, error = harness.utc_now(), None, None
    try:
        result = refine.refine(candidate.task, model, candidate.ctx)
    except Exception as err:  # a refiner bug; the task's calls are still booked below
        error = f"{type(err).__name__}: {err}"
    # Priced by the reported model, else the requested one: calls that all ended in an HTTP error report no model, but
    # their usage is known (zero tokens), so they cost $0, not an unknown amount charged at the reservation
    # (bug-2a5566 is the direct loop's same mistake). Usage that may have been billed unseen stays unknown.
    cost = ledger.price(model.reported_usage(), snapshot.row(model.model_reported or args.model)) if model.calls \
        else ledger.Cost(0.0, 0.0, "not_billed")
    ledger_row = None
    if model.calls:
        ledger_row = book.append(attempt_key=attempt_key, provider=endpoint.provider,
                                 model_reported=model.model_reported, usage=model.reported_usage(), cost=cost,
                                 billed=billed, reserved_usd=budget if billed else 0.0)
    else:
        book.release(attempt_key)
    meter = None
    if proxy:
        found = [one for one in proxy.state()["tasks"] if one["task"] == attempt_key]
        meter = found[0] if found else None
    after = speclint.score_task(result.spec, candidate.ctx) if result else None
    kept = len(result.rounds) if result else 0
    record = {
        "schema_version": "vb.refine_record/1", "experiment_id": book.experiment_id, "run_id": book.run_id,
        "position": position, "attempt_key": attempt_key, "group": candidate.group,
        "plan_path": candidate.ctx.plan_path, "plan_id": candidate.ctx.plan_id, "task_id": candidate.record["task_id"],
        "role": candidate.record["role"], "before": _scored(candidate.record), "after": _scored(after) if after else None,
        "rounds": [one.record() for one in result.rounds] if result else [],
        "proposals": [{**proposal, "kept": index < kept} for index, proposal in enumerate(model.proposals)],
        "stopped": model.stop or (result.stopped if result else "error"), "error": error, "calls": model.calls,
        "replies": model.replies,
        "model_requested": args.model, "model_reported": model.model_reported, "usage": model.reported_usage(),
        "api_equiv_usd": cost.api_equiv_usd, "billed_usd": ledger_row["billed_usd"] if ledger_row else 0.0,
        "cost_source": cost.source, "budget_usd": budget,
        "meter_usd": None if meter is None or meter["cost_unknown"] else round(meter["api_equiv_usd"], 9),
        "started_at": started, "finished_at": harness.utc_now()}
    _append_jsonl(run_dir / TRANSCRIPTS, {"attempt_key": attempt_key, "transcript": model.transcript})
    _append_jsonl(run_dir / RECORDS, record)
    if not records.same_model(args.model, model.model_reported):
        _log_error(run_dir, attempt_key, "model check", f"{model.model_reported} answered for {args.model}")
    if error:
        _log_error(run_dir, attempt_key, "refine", error)
    shown = "unknown" if cost.api_equiv_usd is None else f"${cost.api_equiv_usd:.4f}"
    print(f"vb: refine {position:02d} {candidate.group} {candidate.ctx.plan_path}::{candidate.record['task_id']} "
          f"{candidate.record['score']:.1f} {candidate.record['band']} -> "
          f"{after['score'] if after else float('nan'):.1f} {after['band'] if after else '?'} rounds={kept} "
          f"stopped={record['stopped'][:60]!r} calls={model.calls} cost={shown}"
          + ("" if record["meter_usd"] is None else f" meter=${record['meter_usd']:.4f}"), file=sys.stderr)
    return True


def _scored(record: dict) -> dict:
    return {"spec_hash": record["spec_hash"], "score": record["score"], "band": record["band"],
            "hard_fail": record["hard_fail"], "rules": record["rules"]}


def summarize(run_dir: Path) -> str:
    """The run's summary page (module docstring, "The summary"), from its manifest, records and ledger."""
    manifest = json.loads((run_dir / "manifest.json").read_text(encoding="utf-8"))
    config = manifest["config"]
    rows = _read_jsonl(run_dir / RECORDS)
    booked = _read_jsonl(run_dir / "ledger.jsonl")
    errors = _read_jsonl(run_dir / "errors.jsonl")
    budget = config["budget_per_task_usd"]
    costs = [row["api_equiv_usd"] for row in rows]
    known = [cost for cost in costs if cost is not None]
    unknown = len(costs) - len(known)
    unanswered = sum(1 for row in rows if not row.get("replies"))  # a task the model never answered measured nothing
    verdict = "yes" if (rows and len(rows) == config["tasks"] and not unknown and not unanswered
                        and max(known) <= budget) else "no"
    before = [row["before"]["score"] for row in rows]
    after = [row["after"]["score"] if row["after"] else row["before"]["score"] for row in rows]
    deltas = [b - a for a, b in zip(before, after)]
    kept = [len(row["rounds"]) for row in rows]
    stops = Counter(row["stopped"].split(":")[0] for row in rows)
    groups = Counter(row["group"] for row in rows)
    usage = {"tokens_in": 0, "tokens_cache_read": 0, "tokens_out": 0, "tokens_reasoning": 0}
    for row in rows:
        for key in usage:
            usage[key] += (row["usage"] or {}).get(key, 0)
    billed = sum(row["billed_usd"] or 0.0 for row in booked)
    meter = [row["meter_usd"] for row in rows if row["meter_usd"] is not None]

    def money(value: float | None) -> str:
        return "unknown" if value is None else f"${value:.4f}"

    bands_before = Counter(row["before"]["band"] for row in rows)
    bands_after = Counter((row["after"] or row["before"])["band"] for row in rows)
    route = "live" if config["network"] else "offline"
    route += ", through the metering proxy" if manifest["proxy"] else ""
    out = [
        "# Refiner cost sample (task 3236)",
        "",
        f"Run `{manifest['run_id']}` of experiment `{manifest['experiment_id']}`, started {manifest['started_at']}, "
        f"on budget line {config['line']}. The spec refiner R-v1 (`specops/refine.py`, at most {config['max_rounds']} "
        f"additive rounds) ran with the live adapter `driver/refine_live.py` (prompt `{config['prompt']['version']}`, "
        f"sha256 `{config['prompt']['sha256'][:12]}…`) on `{config['model']}` ({config['provider']}, {route}), priced "
        f"from `{config['price_snapshot_id']}`. Harness `{manifest['harness_sha'][:12]}`"
        f"{' (dirty)' if manifest['dirty'] else ''}; config `{manifest['config_hash'][:19]}…`.",
        "",
        f"- **Tasks:** {len(rows)} of {config['tasks']}, band C or D under `{config['linter']}` (static): "
        + ", ".join(f"{count} from {group}" for group, count in sorted(groups.items()))
        + f", drawn with seed {config['seed']} (the active plans under `plans/` and speclint's fixtures).",
        f"- **Rounds:** {sum(kept)} kept over {sum(1 for n in kept if n)} task(s); {sum(row['calls'] for row in rows)} "
        f"model call(s), {sum(row.get('replies', 0) for row in rows)} answered by the model; {unanswered} task(s) "
        "never answered. Why refinement stopped: "
        + ", ".join(f"{name} {n}" for name, n in stops.most_common()) + ".",
        "- **Cost a task** (the ledger's, at the reported usage): "
        f"mean {money(statistics.fmean(known) if known else None)}, "
        f"median {money(statistics.median(known) if known else None)}, max {money(max(known) if known else None)}; "
        f"{unknown} unknown. Total {money(sum(known) if known else None)} API-equivalent and ${billed:.4f} billed on "
        f"{config['line']} ({len(booked)} ledger row(s)); the proxy metered {money(sum(meter) if meter else None)} "
        "for the same calls.",
        f"- **Tokens:** {usage['tokens_in']:,} input, {usage['tokens_cache_read']:,} cached, {usage['tokens_out']:,} "
        f"output ({usage['tokens_reasoning']:,} of it reasoning).",
    ]
    if rows:
        out.append(f"- **Score change** (`{config['linter']}`, static): mean {statistics.fmean(before):.1f} -> "
                   f"{statistics.fmean(after):.1f}, mean change {statistics.fmean(deltas):+.1f}, largest "
                   f"{max(deltas):+.1f}; bands before {_bands(bands_before)}, after {_bands(bands_after)}.")
        gained = Counter(fail for row in rows for fail in set((row["after"] or row["before"])["hard_fail"])
                         - set(row["before"]["hard_fail"]))
        out.append(f"- **Hard fails** (S07 §4.2; they block whatever the score): "
                   f"{sum(1 for row in rows if row['before']['hard_fail'])} task(s) had one before, "
                   f"{sum(1 for row in rows if (row['after'] or row['before'])['hard_fail'])} after; added: "
                   + (", ".join(f"{fail} on {n} task(s)" for fail, n in sorted(gained.items())) or "none")
                   + ". A round is kept when the score does not drop (`refine`'s guard), even one that adds a hard "
                   "fail.")
    out.append(f"- **Per-task budget:** ${budget:.2f} (S07 §4.3), held before every call and reserved on the ledger "
               f"for each task; at most {config['max_tokens']} output tokens a call.")
    if errors:
        out.append(f"- **Run errors:** {len(errors)} (`errors.jsonl`): " + "; ".join(
            f"{one['task']}: {one['stage']}: {one['error'][:120]}" for one in errors[:3]))
    out += ["", CAP_LINE.format(cap=f"{budget:.2f}", verdict=verdict), "",
            "| # | From | Task | Score before | Score after | Hard fails | Rounds | Stopped | Calls | Cost |",
            "|---|---|---|---|---|---|---|---|---|---|"]
    for row in rows:
        final = row["after"] or row["before"]
        fails = f"{','.join(row['before']['hard_fail']) or '-'} -> {','.join(final['hard_fail']) or '-'}"
        out.append(f"| {row['position']} | {row['group']} | `{row['plan_path']}` {row['task_id']} | "
                   f"{row['before']['score']:.1f} {row['before']['band']} | {final['score']:.1f} {final['band']} | "
                   f"{fails} | {len(row['rounds'])} | {_cell(row['stopped'])} | {row['calls']} | "
                   f"{money(row['api_equiv_usd'])} |")
    out += ["", "Files: `manifest.json` (the config and the sample, with each task's spec hash), `refine.jsonl` (a "
            "`vb.refine_record/1` per task: scores before and after, each kept round's `spec.refined` record, what the "
            "model proposed, the calls, usage and cost) and `ledger.jsonl` (one row per task that called the model)."]
    return "\n".join(out)


def _bands(counts: Counter) -> str:
    return " ".join(f"{band} {counts[band]}" for band in "ABCD" if counts[band]) or "none"


def _cell(text: str) -> str:
    return text.replace("|", "/").replace("\n", " ")[:80]


def write_bundle(run_dir: Path, out_dir: Path) -> Path:
    """The summary bundle (module docstring): summary.md with the run's manifest, records and ledger; returns the
    summary's path."""
    out_dir = Path(out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    for name in BUNDLED:
        if (run_dir / name).is_file():
            shutil.copyfile(run_dir / name, out_dir / name)
    target = out_dir / SUMMARY
    target.write_text(summarize(run_dir) + "\n", encoding="utf-8")
    return target


def _read_jsonl(path: Path) -> list[dict]:
    try:
        return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    except FileNotFoundError:
        return []


def _append_jsonl(path: Path, value: dict) -> None:
    with open(path, "a", encoding="utf-8") as handle:
        handle.write(json.dumps(value, sort_keys=True, ensure_ascii=False) + "\n")


def _write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")


def _log_error(run_dir: Path, key: str, stage: str, err: object) -> None:
    _append_jsonl(run_dir / "errors.jsonl", {"ts": harness.utc_now(), "task": key, "stage": stage,
                                              "error": str(err)[:1000]})
    print(f"vb: {key}: {stage}: {str(err)[:300]}", file=sys.stderr)


def _positive(text: str) -> float:
    try:
        value = float(text)
    except ValueError:
        raise argparse.ArgumentTypeError("must be a number") from None
    if not value > 0 or not math.isfinite(value):
        raise argparse.ArgumentTypeError("must be a finite number above 0")
    return value
