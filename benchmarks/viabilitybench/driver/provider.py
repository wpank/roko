"""The model interface the direct loop calls, and its one implementation: an OpenAI-compatible chat client.

Every model call in the driver goes through `ChatProvider.complete`. `OpenAICompatible` speaks the chat-completions
protocol with stdlib `urllib` to a base URL: a real provider (Cerebras, OpenAI), or a loopback server such as
`stub_provider`'s scripted fake model. A loopback base URL is offline. Anything else is a network provider, which
`vb run` admits only with both `--allow-network` and `--max-cost-usd` (`vb.admit`).

Usage comes back as the provider reports it (`Usage`), or None when the response has none, which makes that call's
cost unknown. The API key is read from the environment variable the arm names, in the driver's process only; agent
processes never see it (`agent_env`). Proxies from the environment are ignored for loopback URLs.

A failed call raises `ProviderError`. `retryable` says whether trying again may help (429, 5xx, a dropped
connection); `billed_unknown` says whether the provider may have billed it (the request may have reached the model
and no usage came back), in which case its cost is unknown rather than $0.

API:
    Endpoint(provider, base_url, api_key_env=None, max_tokens_param="max_completion_tokens", timeout_s=180.0)
    Endpoint.offline -> bool
    Usage(prompt_tokens, completion_tokens, cached_tokens=0, reasoning_tokens=0)
    Completion(content, model_reported, usage, finish_reason)
    ProviderError(message, *, retryable, billed_unknown, retry_after=None)
    ChatProvider: complete(messages, *, model, max_tokens, timeout_s) -> Completion      # a Protocol
    OpenAICompatible(endpoint)
    is_loopback(url: str) -> bool
"""

from __future__ import annotations

import ipaddress
import json
import os
import urllib.error
import urllib.parse
import urllib.request
from collections.abc import Sequence
from dataclasses import dataclass
from typing import Protocol

MAX_RETRY_AFTER_S = 30.0


class ProviderError(RuntimeError):
    """A model call failed; see the module docstring for `retryable` and `billed_unknown`."""

    def __init__(self, message: str, *, retryable: bool, billed_unknown: bool, retry_after: float | None = None):
        super().__init__(message)
        self.retryable = retryable
        self.billed_unknown = billed_unknown
        self.retry_after = retry_after


@dataclass(frozen=True)
class Endpoint:
    provider: str
    base_url: str
    api_key_env: str | None = None
    max_tokens_param: str = "max_completion_tokens"
    timeout_s: float = 180.0

    @property
    def offline(self) -> bool:
        return is_loopback(self.base_url)


@dataclass(frozen=True)
class Usage:
    prompt_tokens: int
    completion_tokens: int
    cached_tokens: int = 0
    reasoning_tokens: int = 0


@dataclass(frozen=True)
class Completion:
    content: str
    model_reported: str | None
    usage: Usage | None
    finish_reason: str | None


class ChatProvider(Protocol):
    def complete(self, messages: Sequence[dict], *, model: str, max_tokens: int, timeout_s: float) -> Completion:
        ...


class OpenAICompatible:
    """POST {base_url}/chat/completions, one request per call; no retries here (the loop owns them)."""

    def __init__(self, endpoint: Endpoint) -> None:
        self.endpoint = endpoint
        handlers = [urllib.request.ProxyHandler({})] if endpoint.offline else []
        self._opener = urllib.request.build_opener(*handlers)

    def complete(self, messages: Sequence[dict], *, model: str, max_tokens: int, timeout_s: float) -> Completion:
        endpoint = self.endpoint
        headers = {"Content-Type": "application/json", "Accept": "application/json"}
        key = os.environ.get(endpoint.api_key_env) if endpoint.api_key_env else None
        if key:
            headers["Authorization"] = f"Bearer {key}"
        elif not endpoint.offline:
            raise ProviderError(f"{endpoint.api_key_env or 'the API key variable'} is not set for {endpoint.provider}",
                                retryable=False, billed_unknown=False)
        body = {"model": model, "messages": list(messages), endpoint.max_tokens_param: max_tokens}
        request = urllib.request.Request(endpoint.base_url.rstrip("/") + "/chat/completions",
                                         data=json.dumps(body).encode("utf-8"), headers=headers, method="POST")
        try:
            with self._opener.open(request, timeout=max(timeout_s, 1.0)) as response:
                payload = json.loads(response.read())
        except urllib.error.HTTPError as err:
            detail = err.read(500).decode("utf-8", "replace").strip()
            raise ProviderError(f"HTTP {err.code} from {endpoint.provider}: {detail}",
                                retryable=err.code == 429 or err.code >= 500, billed_unknown=False,
                                retry_after=_retry_after(err.headers.get("Retry-After"))) from None
        except (urllib.error.URLError, OSError, ValueError) as err:  # timeouts are OSErrors; bad JSON a ValueError
            raise ProviderError(f"{type(err).__name__} from {endpoint.provider}: {err}", retryable=True,
                                billed_unknown=True) from None
        return parse_completion(payload)


def parse_completion(payload: object) -> Completion:
    """A Completion from a chat-completions response body; raises ProviderError if it has no choice."""
    if not isinstance(payload, dict) or not isinstance(payload.get("choices"), list) or not payload["choices"]:
        raise ProviderError("the response has no choices", retryable=True, billed_unknown=True)
    choice = payload["choices"][0] if isinstance(payload["choices"][0], dict) else {}
    message = choice.get("message") if isinstance(choice.get("message"), dict) else {}
    content = message.get("content")
    if isinstance(content, list):  # content parts
        content = "".join(part.get("text", "") for part in content if isinstance(part, dict))
    model = payload.get("model")
    return Completion(content=content if isinstance(content, str) else "",
                      model_reported=model if isinstance(model, str) and model else None,
                      usage=_usage(payload.get("usage")), finish_reason=choice.get("finish_reason"))


def is_loopback(url: str) -> bool:
    """Whether `url` points at this machine (localhost or a loopback address): no network, no spend."""
    host = urllib.parse.urlsplit(url).hostname or ""
    if host == "localhost":
        return True
    try:
        return ipaddress.ip_address(host).is_loopback
    except ValueError:
        return False


def _usage(raw: object) -> Usage | None:
    if not isinstance(raw, dict):
        return None
    prompt, completion = raw.get("prompt_tokens"), raw.get("completion_tokens")
    if not all(isinstance(value, int) and not isinstance(value, bool) and value >= 0 for value in (prompt, completion)):
        return None
    reasoning = _count(raw.get("completion_tokens_details"), "reasoning_tokens")
    return Usage(prompt_tokens=prompt, completion_tokens=completion, cached_tokens=min(_cached(raw), prompt),
                 reasoning_tokens=reasoning)


def _cached(raw: dict) -> int:
    """The cache reads inside `prompt_tokens`: `prompt_tokens_details.cached_tokens` (OpenAI, Cerebras, Z.ai), or a
    top-level `cached_tokens` (Moonshot; Roko's `translate/openai.rs` reads both). The nested count wins when a
    response has one, as in `faultproxy.usage_classes`, so the driver's ledger and the proxy's meter agree."""
    details = raw.get("prompt_tokens_details")
    nested = isinstance(details, dict) and details.get("cached_tokens") is not None
    return _count(details if nested else raw, "cached_tokens")


def _count(details: object, name: str) -> int:
    value = details.get(name) if isinstance(details, dict) else None
    return value if isinstance(value, int) and not isinstance(value, bool) and value > 0 else 0


def _retry_after(value: str | None) -> float | None:
    try:
        return min(max(float(value), 0.0), MAX_RETRY_AFTER_S) if value else None
    except ValueError:
        return None
