# roko-agent

Agent backends for Roko — async executors that take a prompt signal and emit an output signal.

## Install

```toml
[dependencies]
roko-agent = { path = "../roko-agent" }
roko-core = { path = "../roko-core" }
```

## Why a dedicated trait?

The six core Roko traits (`Substrate`, `Scorer`, `Gate`, `Router`, `Composer`, `Policy`) capture composition, verification, and decision-making. An `Agent` is different: it is an async executor with potentially long-running side effects — subprocess management, file edits, LLM API calls, tool use.

Rather than contort an agent into a `Gate` or `Composer`, Roko adds `Agent` as a capability extension. The core stays clean; agent impls live here.

## Trait

```rust
#[async_trait]
trait Agent: Send + Sync {
    async fn run(&self, input: &Signal, ctx: &Context) -> AgentResult;
    fn name(&self) -> &str;
}

struct AgentResult {
    pub output: Signal,          // the agent's primary output
    pub trace: Vec<Signal>,      // intermediate steps (tool calls, thoughts)
    pub success: bool,
    pub usage: Usage,            // tokens / time / cost
}
```

## Implementations

| Agent | Behavior |
| --- | --- |
| `MockAgent` | Deterministic scripted responses for tests |
| `ExecAgent` | Spawns any CLI that reads stdin, writes stdout |

## ExecAgent

The lowest-common-denominator LLM integration — works with `ollama run`, `mods`, `llm`, `aichat`, `claude`, or even `cat` for smoke-testing:

```rust
use roko_agent::{Agent, ExecAgent};
use roko_core::{Body, Context, Kind, Signal};

let agent = ExecAgent::new("ollama", vec!["run".into(), "llama3".into()])
    .with_timeout_ms(60_000);
let prompt = Signal::builder(Kind::Prompt).body(Body::text("Write a haiku.")).build();
let result = agent.run(&prompt, &Context::now()).await;
assert!(result.success);
```

The prompt's text body is piped to the subprocess's stdin; stdout is captured as an `AgentOutput` signal whose lineage points at the input.

## Usage tracking

`Usage` records input/output token counts, wall-clock time, and (when the backend reports it) cost in micro-USD. Downstream `Policy` impls can feed this into budgets and kill-switches.

## Future backends

`ClaudeAgent`, `CodexAgent`, `CursorAgent`, `OllamaAgent` — wrappers with backend-specific retries, tool-use protocols, and streaming. `ExecAgent` covers every stdin/stdout-compatible CLI in the meantime.
