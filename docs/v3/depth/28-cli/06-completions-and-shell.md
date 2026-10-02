# 28.06 -- Completions and Shell Integration

> Depth file for [28-CLI.md](../../28-CLI.md).

---

## Shell Completion Generation

Roko generates shell-specific completion scripts using a custom recursive
completion tree built from the clap command structure. Three shells are
supported: bash, zsh, and fish.

### Usage

```bash
# Generate for your shell
roko completions bash
roko completions zsh
roko completions fish
```

### Installation

#### Zsh

```bash
# Option 1: Add to fpath
roko completions zsh > ~/.zfunc/_roko
# Add to .zshrc: fpath=(~/.zfunc $fpath); autoload -Uz compinit; compinit

# Option 2: Oh My Zsh
roko completions zsh > ~/.oh-my-zsh/completions/_roko
```

#### Bash

```bash
# System-wide
roko completions bash | sudo tee /etc/bash_completion.d/roko > /dev/null

# Per-user
roko completions bash > ~/.local/share/bash-completion/completions/roko
```

#### Fish

```bash
roko completions fish > ~/.config/fish/completions/roko.fish
```

## Completion Architecture

Roko does **not** use `clap_complete` for completion generation. Instead, it
builds a custom `CompletionNode` tree from the clap command structure and
generates shell-native completion scripts with two completion strategies:

### Dynamic Completion (Primary)

Each generated script attempts dynamic completion first via a hidden `__complete`
subcommand:

```bash
candidates="$(roko __complete --shell bash --path "$cmd_path" --current "$cur" 2>/dev/null)"
```

This enables workspace-aware completions that resolve at runtime:
- Plan names from `plans/` directory
- Agent names from the agent registry
- Model and provider names from `roko.toml`

### Static Fallback (Offline)

If the `__complete` subcommand fails (e.g., roko binary not found, workspace
not initialized), the scripts fall back to a compiled-in case tree generated
from `collect_all_subcommand_paths()`:

```bash
case "$prev" in
    plan)
        COMPREPLY=( $(compgen -W "list show create run generate ..." -- "$cur") )
        return 0
        ;;
    config)
        COMPREPLY=( $(compgen -W "init show path edit set doctor ..." -- "$cur") )
        return 0
        ;;
esac
```

This ensures completions work even offline or before the workspace is initialized.

## Global Flag Completion

The `completion_flag_words()` function extracts all `--long` flag names from the
clap command tree:

```rust
pub(crate) fn completion_flag_words() -> Vec<String> {
    let mut command = Cli::command();
    command.build();
    command.get_arguments()
        .filter_map(|arg| arg.get_long().map(|l| format!("--{l}")))
        .collect()
}
```

All global and subcommand-specific flags complete:

```
roko run --<TAB>  -> --model, --effort, --role, --json, --quiet, ...
roko plan run --<TAB> -> --engine, --resume-plan
```

## What Gets Completed

### Subcommands

Tab completion works for the full recursive command tree:

```
roko pl<TAB>   -> plan
roko plan <TAB> -> list, show, create, run, generate, ...
roko config p<TAB> -> providers, plugins, preset
roko config providers <TAB> -> list, health, test, available, discover, ...
```

### Value Hints

Enum flags complete with valid values:

```
roko --effort <TAB>  -> low, medium, high, max
roko run --complexity <TAB> -> trivial, simple, standard, complex
roko doctor <TAB> -> disk, network, clean
```

### File Path Arguments

Path arguments use filesystem completion:

```
roko plan run plans/<TAB>  -> plans/my-plan/, plans/auth-refactor/
roko --repo <TAB>          -> (directory completion)
```

## Value Enum Types

Custom types use `clap::ValueEnum` for automatic completion of valid values:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Effort {
    Low,
    Medium,
    High,
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DoComplexity {
    #[value(alias = "mechanical")]
    Trivial,
    Simple,
    #[value(alias = "standard")]
    Medium,
    #[value(alias = "architectural")]
    Complex,
}
```

Aliases (e.g., `mechanical` for `Trivial`) are also completed.

## Fish-Specific Features

Fish completions include dynamic completions for workspace items using
`commandline` parsing:

```fish
# Dynamic completions for workspace values.
complete -c roko -n "__roko_needs_plan_name" -f -a "(ls plans/ 2>/dev/null)"
```

## Environment Variables

The `roko config env` command lists all recognized environment variables:

```bash
$ roko config env
ANTHROPIC_API_KEY     API key for Anthropic provider
OPENAI_API_KEY        API key for OpenAI-compatible providers
ROKO_CONFIG           Path to config file override
ROKO_LOG              Tracing filter directive
ROKO_SERVE_PORT       HTTP server port (default: 6677)
ROKO_SERVE_BIND       HTTP server bind address
...
```

## Shell Aliases

Useful shell aliases for frequent operations:

```bash
# .zshrc / .bashrc
alias rk='roko'
alias rkr='roko run'
alias rks='roko status'
alias rkp='roko plan'
alias rkdash='roko dashboard'
alias rkdoc='roko doctor'
```

## One-Shot Mode

The positional `[prompt]` argument enables quick one-shot execution without
a subcommand:

```bash
# These are equivalent:
roko "Fix the import statement"
roko run "Fix the import statement"
```

This works well with shell history and aliases.

## Pipe Mode

The CLI detects when stdin is not a terminal and enters pipe mode:

```rust
if !std::io::stdin().is_terminal() {
    // Read prompt from stdin pipe
}
```

This enables pipeline usage:

```bash
echo "Fix the typo" | roko
cat prompt.txt | roko --json
```

## Dev Helper: `dev.sh fast`

For local development with a prebuilt binary, the `dev.sh` wrapper provides
a bounded fast-path:

```bash
./dev.sh fast plans/my-plan
```

This skips warmup/cleanup, requires each task to define a `verify` command,
and captures a private evidence bundle. Not appropriate for safety, auth,
persistence, migration, or payment changes.

## Interactive REPL

`roko chat` provides a persistent interactive session:

```bash
$ roko chat
roko> explain the Signal type
The Signal type is the primary protocol noun...

roko> /plan generate an auth module
Generating plan...

roko> /exit
```

The REPL supports:
- Multi-line input
- Command history (via readline)
- Slash commands (`/plan`, `/research`, `/exit`)
- Session persistence

## Source

- `crates/roko-cli/src/main.rs` -- CLI definition with clap derives
- `crates/roko-cli/src/commands/util.rs` -- Completion tree builder and generators
- `crates/roko-cli/src/chat.rs` -- Interactive REPL
- `crates/roko-cli/src/pipe.rs` -- Pipe mode detection
