"""`vb capture-errors`: each arm provider's own error answer, captured through the metering proxy and masked, as a
fixture (gap-db8dab).

    vb capture-errors --allow-network --max-cost-usd 0.05 [--provider cerebras ...] [--out DIR] \
                      [--experiment ERROR-CAPTURE] [--line BL0] [--results DIR] [--run-id ID] [--key-file PATH] \
                      [--provider-url NAME=URL] [--dry-run]

bug-2224eb made an injected `http_5xx` or `rate_limit` fault answer in its upstream's own error shape and words
(`faultproxy.UPSTREAM_ERRORS`) and with the headers of the upstream's last forwarded answer, and gap-db8dab asks for
captures of the providers' own errors to check those templates against.

**What it sends.** None of the four arm providers documents a way to get a 429 or a 5xx on request (`DOCS`, checked
2026-10-18), and provoking one, a burst past a rate limit, would abuse the API. So each capture is the provider's
error for a request it refuses before any model runs: one chat completion naming a model that does not exist
(`PROBE_MODEL`, a neutral name, since its words may come back in the body), with a two-word prompt and an output cap
of one token (`PROBE_MAX_TOKENS`). Nothing is generated, so nothing is billed. The answer shows the provider's error
envelope (its keys and their order, compact or indented JSON, the content type) and the headers it sends with an
error, which are what a fault that comes before the upstream's first forwarded answer has to carry.

**Fences**, those of a billed `vb run` call. A non-loopback provider needs `--allow-network` and `--max-cost-usd`, and
the sum of the calls' worst cases must fit it. The keys are read from the driver-only key file (`--key-file`,
`$VB_KEY_FILE`, `~/.config/viabilitybench/keys`; `secret.load_keys`) into this process, and only the metering proxy
sends them; the run refuses to start while the environment holds a provider key (`secret.key_exposures`). The proxy
forwards only the probe's model. Each call reserves its worst case on `--line` before it is sent (default BL0, the
pilot's line, whose Pilot B runs the `provider_fault` rows these templates serve), priced at the provider's most
expensive snapshot row, since the probe's model has none; its ledger row books the error answer at $0 (no usage
comes back with an error), or what a provider that served it reports. Requests carry the driver's User-Agent
(`USER_AGENT`), which Cerebras's edge accepts where urllib's default gets a 403 (bug-7b1b7f).

**What it stores.** For each provider, `<out>/<provider>.json` (default `driver/testdata/provider_errors/`), a
`vb.provider_error/1` fixture: the probe, the status, the content type, the body as text with its JSON format
(`compact`, `spaced` or `indent:<n>`, and whether it ends in a newline), and the headers of the answer that a fault
would carry (`faultproxy.NOT_MIRRORED` left out), in order. Request and trace ids and Cloudflare's ray
(`faultproxy.PER_ANSWER`), and account, project and user ids (`ACCOUNT`) are drawn anew in the same form
(`faultproxy._redrawn`), in the headers and wherever the body repeats them, as are ids the body holds of its own; a
cookie's token is drawn anew in every word but its numbers (`_cookie`). No real id is stored, and a fixture that would
still hold one, or a key, is refused. The run directory
(`$VB_RESULTS/<experiment>/<run_id>/`) keeps the manifest, the ledger and the proxy's log, which records statuses and
usage, never a header or a body.

Exit status: 0 when every provider's fixture was written, 1 when one was not (see `errors.jsonl`), 2 for a usage,
configuration or admission error, before anything is written or sent.

API:
    PROBE_MODEL, PROBE_MAX_TOKENS, USER_AGENT, PROVIDERS, DOCS, ACCOUNT, SCHEMA
    probe_body(endpoint) -> bytes; mask(headers, body) -> (headers, body, masked_names); body_format(text) -> dict
    add_arguments(parser); cmd_capture(vb, args) -> int                 # vb.py wires `vb capture-errors`
"""

from __future__ import annotations

import argparse
import datetime as dt
import http.client
import json
import os
import re
import secrets
import sys
import urllib.parse
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

SCHEMA = "vb.provider_error/1"
PROBE_MODEL = "nonexistent-model"
PROBE_MESSAGES = [{"role": "user", "content": "Say OK."}]
PROBE_MAX_TOKENS = 1
USER_AGENT = "vb-driver/1.0"  # the driver's own, as provider.USER_AGENT (bug-7b1b7f)
PROVIDERS = ("cerebras", "zai", "moonshot", "openai")  # the arm providers that faultproxy.UPSTREAM_ERRORS speaks for
# Each provider's error documentation, read on 2026-10-18: none documents a way to get a 429 or a 5xx on request.
DOCS = {
    "cerebras": "https://inference-docs.cerebras.ai/support/error",
    "zai": "https://docs.z.ai/api-reference/api-code",
    "moonshot": "https://platform.kimi.ai/docs/api/errors",
    "openai": "https://developers.openai.com/api/docs/guides/error-codes",
}
# Header names whose values identify the account or may name the request, beside faultproxy.PER_ANSWER: OpenAI's
# organization and project, Moonshot's Msh-Uid (its user id), and a deployment canary number.
ACCOUNT = re.compile(r"organi[sz]ation|project|account|user|(?:^|-)uid$|canary", re.IGNORECASE)
BODY_IDS = re.compile(r"(?:^|_)id$|request_?id|trace", re.IGNORECASE)  # body keys whose string values are ids
DEFAULT_EXPERIMENT = "ERROR-CAPTURE"
DEFAULT_LINE = "BL0"
DEFAULT_OUT = layout.DRIVER_DIR / "testdata" / "provider_errors"
PROXY_LOG = "proxy.jsonl"
PREAMBLE_TOKENS = 256  # what a provider may put before the first message, as faultproxy bounds a call's input


def probe_body(endpoint: provider.Endpoint) -> bytes:
    """The probe's request: the model that does not exist, two words, one token of output."""
    return json.dumps({"model": PROBE_MODEL, "messages": PROBE_MESSAGES,
                       endpoint.max_tokens_param: PROBE_MAX_TOKENS}).encode("utf-8")


def mask(headers: list[tuple[str, str]], body: bytes) -> tuple[list[tuple[str, str]], bytes, list[str]]:
    """`headers` and `body` with every id drawn anew in its own form (module docstring, "What it stores"), and the
    names of the headers that were masked. A value the body repeats is replaced there by the same new value."""
    masked: list[str] = []
    swaps: dict[str, str] = {}
    out = []
    for name, value in headers:
        if name.lower() == "set-cookie":
            new = _cookie(value)
            swaps[value] = new
            masked.append(name.lower())
            out.append((name, new))
        elif faultproxy.PER_ANSWER.search(name) or ACCOUNT.search(name):
            new = faultproxy._redrawn(value, cookie=False)
            swaps[value] = new
            masked.append(name.lower())
            out.append((name, new))
        else:
            out.append((name, value))
    text = body.decode("utf-8", "replace")
    try:
        parsed = json.loads(text)
    except ValueError:
        parsed = None
    for key, value in _walk(parsed):
        if BODY_IDS.search(key) and isinstance(value, str) and value and value not in swaps:
            swaps[value] = faultproxy._redrawn(value, cookie=False)
    for old, new in sorted(swaps.items(), key=lambda pair: -len(pair[0])):
        if old:
            text = text.replace(old, new)
    return out, text.encode("utf-8"), masked


def _cookie(value: str) -> str:
    """A cookie with every run of its token that is not digits alone drawn anew in its own alphabet: stricter than
    `faultproxy._redrawn`, which keeps a short word so that a fault's cookie reads like a real one. The name, the
    attributes and the token's numbers (its timestamps) stay."""
    name, equals, rest = value.partition("=")
    token, semicolon, attributes = rest.partition(";")

    def draw(run: re.Match) -> str:
        text = run.group(0)
        if text.isdigit():
            return text
        for alphabet in ("0123456789abcdef", "0123456789ABCDEF"):  # a hex token stays hex, as faultproxy draws it
            if set(text) <= set(alphabet):
                return "".join(secrets.choice(alphabet) for _ in text)
        return "".join(secrets.choice("0123456789" if char.isdigit() else "abcdefghijklmnopqrstuvwxyz" if char.islower()
                                      else "ABCDEFGHIJKLMNOPQRSTUVWXYZ") for char in text)

    return name + equals + re.sub(r"[A-Za-z0-9]+", draw, token) + semicolon + attributes


def _walk(value: object, key: str = "") -> list[tuple[str, object]]:
    """Every (key, value) pair of a parsed JSON document, nested ones included."""
    found: list[tuple[str, object]] = []
    if isinstance(value, dict):
        for name, item in value.items():
            found.append((str(name), item))
            found += _walk(item, str(name))
    elif isinstance(value, list):
        for item in value:
            found += _walk(item, key)
    return found


def body_format(text: str) -> dict:
    """How a JSON body is laid out: `compact` (no spaces), `spaced` (json.dumps' default), `indent:<n>`, or `other`;
    and whether it ends in a newline. An empty or non-JSON body is `other`."""
    stripped = text.rstrip("\n")
    try:
        value = json.loads(stripped)
    except ValueError:
        return {"layout": "other", "newline": text.endswith("\n")}
    candidates = {"compact": json.dumps(value, separators=(",", ":"), ensure_ascii=False),
                  "spaced": json.dumps(value, ensure_ascii=False),
                  **{f"indent:{n}": json.dumps(value, indent=n, ensure_ascii=False) for n in (2, 4)}}
    layout_ = next((name for name, rendered in candidates.items() if rendered == stripped), "other")
    return {"layout": layout_, "newline": text.endswith("\n")}


def add_arguments(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--provider", action="append", choices=PROVIDERS,
                        help="capture this provider (repeatable; default: all four)")
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT, help="where the fixtures go (default: "
                                                                      "driver/testdata/provider_errors)")
    parser.add_argument("--experiment", default=DEFAULT_EXPERIMENT, help=f"default: {DEFAULT_EXPERIMENT}")
    parser.add_argument("--line", default=DEFAULT_LINE, help=f"the budget line (default: {DEFAULT_LINE})")
    parser.add_argument("--run-id")
    parser.add_argument("--allow-network", action="store_true", help="allow calls to a non-loopback provider")
    parser.add_argument("--max-cost-usd", type=_positive, help="required with --allow-network")
    parser.add_argument("--key-file", type=Path, help=f"provider keys, NAME=value; default: ${secret.KEYS_FILE_ENV}, "
                                                      f"then {secret.KEYS_DEFAULT_PATH}")
    parser.add_argument("--results", type=Path, help="default: $VB_RESULTS, then ~/.roko-bench/viability")
    parser.add_argument("--price-snapshot", default=ledger.DEFAULT_SNAPSHOT)
    parser.add_argument("--provider-url", action="append", default=[], metavar="NAME=URL",
                        help="override a provider's base URL (repeatable); a loopback URL runs offline, with no key")
    parser.add_argument("--dry-run", action="store_true", help="print what would be sent; send nothing")


def cmd_capture(vb: ModuleType, args: argparse.Namespace) -> int:
    """`vb capture-errors` (module docstring). `vb` is the driver module, whose checks and errors this reuses."""
    names = list(dict.fromkeys(args.provider or PROVIDERS))
    overrides = _overrides(vb, args.provider_url, names)
    for value, flag in ((args.experiment, "--experiment"), (args.run_id or "x", "--run-id")):
        if not vb.ID_RE.fullmatch(value):
            raise vb.DriverError(f"{flag} must match {vb.ID_RE.pattern}")
    if vb.campaign.requires_lock(args.experiment):
        raise vb.DriverError(f"experiment {args.experiment} runs only under the pre-registration lock")
    snapshot = ledger.load_snapshot(args.price_snapshot)
    endpoints = {name: _endpoint(vb, snapshot, name, overrides.get(name)) for name in names}
    worst = {name: _worst_usd(snapshot, endpoint) for name, endpoint in endpoints.items()}
    plan = {"experiment_id": args.experiment, "line": args.line, "probe_model": PROBE_MODEL,
            "max_tokens": PROBE_MAX_TOKENS, "user_agent": USER_AGENT, "price_snapshot_id": snapshot.id,
            "worst_case_usd": round(sum(worst.values()), 8), "out": str(args.out),
            "providers": [{"provider": name, "base_url": endpoint.base_url, "api_key_env": endpoint.api_key_env,
                           "network": not endpoint.offline, "worst_case_usd": round(worst[name], 8),
                           "request": json.loads(probe_body(endpoint)), "docs": DOCS[name]}
                          for name, endpoint in endpoints.items()]}
    if args.dry_run:
        print(json.dumps(plan, indent=2))
        return 0
    if any(not endpoint.offline for endpoint in endpoints.values()):  # vb.admit's rule, before anything is sent
        if not args.allow_network:
            raise vb.DriverError("a provider is on the network; inspect `vb capture-errors --dry-run` and pass "
                                 "--allow-network explicitly")
        if args.max_cost_usd is None:
            raise vb.DriverError("network runs require an explicit --max-cost-usd budget")
        if sum(worst.values()) > args.max_cost_usd:
            raise vb.DriverError(f"the calls could cost up to ${sum(worst.values()):.6f}, more than --max-cost-usd "
                                 f"${args.max_cost_usd:.2f}")
    keys = vb._provider_keys(args.key_file, endpoints.values())
    exposed = secret.key_exposures(keys)
    if exposed:
        raise vb.DriverError("refusing to run, a provider key sits where it could leak: " + "; ".join(exposed))
    out_dir = Path(args.out).expanduser().absolute()
    results_root = vb._outside_repo(args.results or os.environ.get("VB_RESULTS") or vb.DEFAULT_RESULTS, "--results")
    run_id = args.run_id or f"capture-{dt.datetime.now(dt.UTC):%Y%m%d}-{secrets.token_hex(3)}"
    run_dir = results_root / args.experiment / run_id
    if run_dir.exists():
        raise vb.DriverError(f"{run_dir} already exists; pick another --run-id")
    book = ledger.Ledger(run_dir / "ledger.jsonl", line=args.line, experiment_id=args.experiment, run_id=run_id,
                         price_snapshot_id=snapshot.id, root=results_root)
    refusal = book.refusal(sum(worst.values()))
    if refusal:
        raise vb.DriverError(f"budget: {refusal}")
    run_dir.mkdir(mode=0o700, parents=True)
    try:
        proxy = faultproxy.FaultProxy([faultproxy.Upstream.from_endpoint(one) for one in endpoints.values()],
                                      log_path=run_dir / PROXY_LOG, snapshot=snapshot, keys=keys,
                                      models=[PROBE_MODEL]).start()
    except (faultproxy.ProxyError, OSError) as err:
        archive.remove_tree(run_dir)  # made a moment ago; nothing has been sent
        raise vb.DriverError(f"the metering proxy cannot start: {err}") from None
    head = records.harness_state()
    _write_json(run_dir / "manifest.json", {
        "schema_version": "vb.capture_manifest/1", "experiment_id": args.experiment, "run_id": run_id,
        "started_at": harness.utc_now(), "argv": getattr(args, "argv", ["vb", "capture-errors"]),
        "harness_sha": head[0], "dirty": head[1], "proxy": {"log": PROXY_LOG, "policy": proxy.policy()},
        **{key: value for key, value in plan.items() if key != "out"}})
    written = 0
    out_dir.mkdir(parents=True, exist_ok=True)
    try:
        for name, endpoint in endpoints.items():
            attempt_key = f"{run_id}/{name}:1"
            try:
                book.reserve(attempt_key, worst[name])
            except ledger.BudgetError as err:
                _log_error(run_dir, attempt_key, "budget", err)
                break
            proxy.configure(task=attempt_key)
            written += _capture_one(name, endpoint, proxy=proxy, book=book, snapshot=snapshot, keys=keys,
                                    attempt_key=attempt_key, reserved=worst[name], out_dir=out_dir, run_dir=run_dir)
    finally:
        proxy.close()
        print(f"vb: {written}/{len(endpoints)} fixtures in {out_dir}; run {run_dir}", file=sys.stderr)
    return 0 if written == len(endpoints) else 1


def _capture_one(name: str, endpoint: provider.Endpoint, *, proxy: faultproxy.FaultProxy, book: ledger.Ledger,
                 snapshot: ledger.Snapshot, keys: secret.ProviderKeys | None, attempt_key: str, reserved: float,
                 out_dir: Path, run_dir: Path) -> bool:
    """Send one provider's probe through the proxy, book it, and write its masked fixture; whether it was written."""
    url = urllib.parse.urlsplit(proxy.base_url(name) + "/chat/completions")
    connection = http.client.HTTPConnection(url.hostname, url.port, timeout=endpoint.timeout_s)
    status, content_type, data, error = None, None, b"", None
    try:
        connection.request("POST", url.path, body=probe_body(endpoint),
                           headers={"Content-Type": "application/json", "Accept": "application/json",
                                    "User-Agent": USER_AGENT})
        response = connection.getresponse()
        status, content_type, data = response.status, response.getheader("Content-Type"), response.read()
    except (OSError, http.client.HTTPException) as err:
        error = f"{type(err).__name__}: {err}"
    finally:
        connection.close()
    payload = _json_object(data)
    usage = None  # unknown: no answer came back, or a served call reported no usage
    if status is not None and status != 200:
        usage = ledger.vb_usage(0, 0)  # an error answer reports no usage, and is not billed
    elif status == 200 and (used := provider._usage(payload.get("usage"))) is not None:
        usage = ledger.vb_usage(used.prompt_tokens, used.completion_tokens, used.cached_tokens, used.reasoning_tokens)
    priced = snapshot.row(payload.get("model")) or _priciest(snapshot, name)
    book.append(attempt_key=attempt_key, provider=name, model_reported=payload.get("model") or None,
                usage=usage, cost=ledger.price(usage, priced), billed=True, reserved_usd=reserved)
    if error or status is None:
        _log_error(run_dir, attempt_key, "send", error or "no answer")
        return False
    headers, body, masked = mask(proxy.answered_headers(name), data)
    fixture = {
        "schema_version": SCHEMA, "provider": name, "captured_at": harness.utc_now(),
        "kind": "unknown_model" if status in (400, 404) else f"http_{status}",
        "why": (f"{name} documents no way to get a 429 or a 5xx on request ({DOCS[name]}, read 2026-10-18), and "
                "provoking one would abuse the API, so this is its error for a model that does not exist; it shows "
                "the error envelope and the headers sent with an error, not the words of a 429 or a 5xx"),
        "request": {"path": "/chat/completions", "model": PROBE_MODEL, "max_tokens_param": endpoint.max_tokens_param,
                    "max_tokens": PROBE_MAX_TOKENS, "messages": len(PROBE_MESSAGES), "user_agent": USER_AGENT},
        "status": status, "content_type": content_type, "body": body.decode("utf-8", "replace"),
        "format": body_format(body.decode("utf-8", "replace")),
        "headers": [[header, value] for header, value in headers], "masked": sorted(set(masked)),
    }
    text = json.dumps(fixture, indent=2, ensure_ascii=False) + "\n"
    originals = [value for (header, value), (_, new) in zip(proxy.answered_headers(name), headers) if value != new]
    leaked = [one for one in (*(keys or {}).values(), *originals) if one and one in text]
    if leaked:  # never write a key or a real id; the reason names neither
        _log_error(run_dir, attempt_key, "mask", f"the fixture would still hold {len(leaked)} secret or id value(s)")
        return False
    (out_dir / f"{name}.json").write_text(text, encoding="utf-8")
    print(f"vb: {name}: HTTP {status} {content_type}; {len(headers)} header(s), {len(set(masked))} masked; "
          f"body {fixture['format']['layout']}", file=sys.stderr)
    return True


def _endpoint(vb: ModuleType, snapshot: ledger.Snapshot, name: str, override: str | None) -> provider.Endpoint:
    """Provider `name`'s endpoint as the first arm file (by name) that names it with a key gives it."""
    row = _priciest(snapshot, name)
    for path in sorted(layout.ARMS_DIR.glob("*.toml")):
        arm = vb.load_arm(str(path))
        table = arm.get("providers", {}).get(name)
        if isinstance(table, dict) and table.get("api_key_env"):
            return vb._endpoint(arm, row, override)
    raise vb.DriverError(f"no arm file names provider {name} with an api_key_env")


def _priciest(snapshot: ledger.Snapshot, name: str) -> dict:
    """The provider's most expensive snapshot row: what prices the probe's worst case, since its model has none."""
    rows = [row for row in snapshot.rows.values() if row["provider"] == name]
    if not rows:
        raise ledger.PriceError(f"{snapshot.id} has no row for provider {name}")
    return max(rows, key=lambda row: (row["input"], row["output"]))


def _worst_usd(snapshot: ledger.Snapshot, endpoint: provider.Endpoint) -> float:
    """The most the probe could cost: its bytes plus a preamble of input and its output cap, at the provider's
    priciest rates (`ledger.worst_case_usd` doubles the output rate where reasoning is billed apart)."""
    return ledger.worst_case_usd(_priciest(snapshot, endpoint.provider),
                                 input_tokens=len(probe_body(endpoint)) + PREAMBLE_TOKENS,
                                 output_tokens=PROBE_MAX_TOKENS)


def _overrides(vb: ModuleType, values: list[str], names: list[str]) -> dict[str, str]:
    found = {}
    for value in values:
        name, sep, url = value.partition("=")
        if not sep or name not in names or not url:
            raise vb.DriverError(f"--provider-url takes NAME=URL with NAME one of {', '.join(names)}, not {value!r}")
        found[name] = url
    return found


def _json_object(data: bytes) -> dict:
    try:
        value = json.loads(data)
    except ValueError:
        return {}
    return value if isinstance(value, dict) else {}


def _write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n", encoding="utf-8")


def _log_error(run_dir: Path, key: str, stage: str, err: object) -> None:
    line = json.dumps({"ts": harness.utc_now(), "task": key, "stage": stage, "error": str(err)[:1000]})
    with (run_dir / "errors.jsonl").open("a", encoding="utf-8") as handle:
        handle.write(line + "\n")
    print(f"vb: {key}: {stage}: {str(err)[:300]}", file=sys.stderr)


def _positive(text: str) -> float:
    try:
        value = float(text)
    except ValueError:
        raise argparse.ArgumentTypeError("must be a number") from None
    if not value > 0 or value == float("inf"):
        raise argparse.ArgumentTypeError("must be a finite number above 0")
    return value
