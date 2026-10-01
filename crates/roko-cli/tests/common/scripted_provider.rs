//! The scripted fake Claude CLI of the golden-path canaries C1–C8
//! (gap-3aa9cb).
//!
//! [`ScriptedProvider::install`] writes a POSIX `sh` script, `claude`, into a
//! directory, together with the [`Script`] it plays. A canary names the script
//! as a `claude_cli` provider's `command` in `roko.toml`.
//!
//! On each call the provider:
//!
//! 1. reads its prompt on stdin and finds the task the call is for: the one id
//!    on the `Task: <id>: ` lines of the prompt and its arguments (roko's
//!    system prompt names its task there);
//! 2. numbers the call and the task's attempt, and logs the call in
//!    `calls/<n>/` ([`Call`]);
//! 3. plays the task's [`Turn`] for that attempt. A task's last turn repeats;
//!    a call naming no single task, or a task without turns of its own, plays
//!    the script's default turn. A turn applies its edits (relative to the
//!    agent's working directory unless absolute), holds, stays silent, prints
//!    its output and exits with its status.
//!
//! `@TASK@` and `@MODEL@` in an edit's path and in the output stand for the
//! call's task and the `--model` it asked for (`claude-sonnet-4-6` when it
//! named none). Each turn is written out as plain files under `turns/`, so the
//! script needs no JSON parser; `script.json` records the [`Script`].

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;

use super::write_executable;

/// The provider. See the module documentation.
const PROVIDER: &str = r#"#!/bin/sh
# The golden-path canaries' scripted fake Claude CLI, written with the turns it
# plays by crates/roko-cli/tests/common/scripted_provider.rs.
set -eu
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
prompt=$(cat)

model=
previous=
for arg in "$@"; do
  if [ "$previous" = --model ]; then
    model=$arg
  fi
  previous=$arg
done
served=${model:-claude-sonnet-4-6}

# The task the call is for: the one id on its `Task: <id>: ` lines.
ids=$({ printf '%s\n' "$prompt"; printf '%s\n' "$@"; } | sed -n 's/^Task: \([^ :]*\): .*/\1/p' | sort -u)
task=
if [ -n "$ids" ] && [ "$(printf '%s\n' "$ids" | grep -c .)" -eq 1 ]; then
  task=$ids
fi

# Number the call, and the task's attempt: the first number no earlier call
# took (mkdir is atomic, so calls running at once never share one).
n=1
until mkdir "$dir/calls/$n" 2>/dev/null; do
  n=$((n + 1))
  [ "$n" -le 100000 ] || exit 70
done
call=$dir/calls/$n
attempt=0
if [ -n "$task" ]; then
  mkdir -p "$dir/attempts/$task"
  attempt=1
  until mkdir "$dir/attempts/$task/$attempt" 2>/dev/null; do
    attempt=$((attempt + 1))
    [ "$attempt" -le 100000 ] || exit 70
  done
fi
printf '%s\n' "$prompt" > "$call/prompt"
printf '%s\0' "$@" > "$call/argv"
env > "$call/env"
pwd > "$call/cwd"
printf '%s\n' "$ids" > "$call/ids"
printf '%s\n' "$task" > "$call/task"
printf '%s\n' "$attempt" > "$call/attempt"
printf '%s\n' "$model" > "$call/model"

turn=$dir/turns/default
if [ -n "$task" ] && [ -f "$dir/turns/task-$task/count" ]; then
  last=$(cat "$dir/turns/task-$task/count")
  if [ "$attempt" -lt "$last" ]; then
    turn=$dir/turns/task-$task/$attempt
  else
    turn=$dir/turns/task-$task/$last
  fi
fi
[ -z "$task" ] || printf 'start %s\n' "$task" >> "$dir/events"

# The file at $1 with the call's task and model filled in.
fill() {
  sed -e "s|@TASK@|$task|g" -e "s|@MODEL@|$served|g" "$1"
}

i=1
while [ -d "$turn/edits/$i" ]; do
  path=$(fill "$turn/edits/$i/path")
  mkdir -p "$(dirname -- "$path")"
  if [ -e "$turn/edits/$i/append" ]; then
    cat "$turn/edits/$i/text" >> "$path"
  else
    cat "$turn/edits/$i/text" > "$path"
  fi
  i=$((i + 1))
done

if [ -f "$turn/hold/while" ]; then
  hold=$(cat "$turn/hold/while")
  if [ -e "$hold" ]; then
    : > "$(cat "$turn/hold/signal")"
    ticks=$(($(cat "$turn/hold/max_secs") * 10))
    while [ -e "$hold" ] && [ "$ticks" -gt 0 ]; do
      sleep 0.1
      ticks=$((ticks - 1))
    done
  fi
fi

silent=$(cat "$turn/silent_secs")
[ "$silent" = 0 ] || sleep "$silent"
[ -z "$task" ] || printf 'end %s\n' "$task" >> "$dir/events"
fill "$turn/stdout"
exit "$(cat "$turn/exit_code")"
"#;

/// What a [`Turn`] prints.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Output {
    /// Stream-json: an assistant message with `text`, then a successful
    /// result. `model` is the slug both name (`None`: the model the call asked
    /// for).
    Reply {
        text: String,
        model: Option<String>,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
    },
    /// Lines that are not JSON.
    Malformed,
    /// A reply whose text is `bytes` long.
    Overlong { bytes: usize },
    /// Nothing at all.
    Nothing,
}

/// A file a [`Turn`] writes, or appends to.
#[derive(Clone, Debug, Serialize)]
pub struct Edit {
    path: String,
    append: bool,
    text: String,
}

/// While `while_exists` exists, a [`Turn`] creates `signal` and waits for it
/// to go, for at most `max_secs`.
#[derive(Clone, Debug, Serialize)]
pub struct Hold {
    while_exists: PathBuf,
    signal: PathBuf,
    max_secs: u64,
}

/// What the provider does on one call.
#[derive(Clone, Debug, Serialize)]
pub struct Turn {
    edits: Vec<Edit>,
    hold: Option<Hold>,
    silent_secs: f64,
    output: Output,
    exit_code: i32,
}

impl Turn {
    /// A turn that changes nothing, replies `done` at once on the model the
    /// call asked for (usage 10 in and 5 out, $0.001), and exits 0.
    pub fn reply() -> Self {
        Self {
            edits: Vec::new(),
            hold: None,
            silent_secs: 0.0,
            output: Output::Reply {
                text: "done".to_string(),
                model: None,
                input_tokens: 10,
                output_tokens: 5,
                cost_usd: 0.001,
            },
            exit_code: 0,
        }
    }

    /// Write `text` to `path`, creating its directory.
    pub fn write(mut self, path: &str, text: &str) -> Self {
        self.edits.push(Edit {
            path: path.to_string(),
            append: false,
            text: text.to_string(),
        });
        self
    }

    /// Append `text` to `path`, creating it.
    pub fn append(mut self, path: &str, text: &str) -> Self {
        self.edits.push(Edit {
            path: path.to_string(),
            append: true,
            text: text.to_string(),
        });
        self
    }

    /// While `path` exists, create `signal` and wait, for at most
    /// `max_secs`, for `path` to go. The edits come first.
    pub fn hold_while(mut self, path: &Path, signal: &Path, max_secs: u64) -> Self {
        self.hold = Some(Hold {
            while_exists: path.to_path_buf(),
            signal: signal.to_path_buf(),
            max_secs,
        });
        self
    }

    /// Print nothing for `secs` seconds before the output.
    pub fn silent_for(mut self, secs: f64) -> Self {
        self.silent_secs = secs;
        self
    }

    /// Reply `text`.
    pub fn text(mut self, text: &str) -> Self {
        if let Output::Reply { text: reply, .. } = &mut self.output {
            *reply = text.to_string();
        }
        self
    }

    /// Reply as `slug`, whatever model the call asked for.
    pub fn model(mut self, slug: &str) -> Self {
        if let Output::Reply { model, .. } = &mut self.output {
            *model = Some(slug.to_string());
        }
        self
    }

    /// Report `input` and `output` tokens, costing `cost_usd`.
    pub fn usage(mut self, input: u64, output: u64, cost_usd: f64) -> Self {
        if let Output::Reply {
            input_tokens,
            output_tokens,
            cost_usd: cost,
            ..
        } = &mut self.output
        {
            (*input_tokens, *output_tokens, *cost) = (input, output, cost_usd);
        }
        self
    }

    /// Print `output` instead of a reply.
    pub fn output(mut self, output: Output) -> Self {
        self.output = output;
        self
    }

    /// Exit with `code`.
    pub fn exit_code(mut self, code: i32) -> Self {
        self.exit_code = code;
        self
    }

    /// The bytes the turn prints, with `@MODEL@` standing for the model the
    /// call asked for.
    fn stdout(&self) -> String {
        let reply = |text: &str, model: Option<&str>, input: u64, output: u64, cost: f64| {
            let model = model.unwrap_or("@MODEL@");
            let usage = serde_json::json!({ "input_tokens": input, "output_tokens": output });
            let assistant = serde_json::json!({
                "type": "assistant",
                "message": {
                    "id": "msg_1",
                    "model": model,
                    "content": [{ "type": "text", "text": text }],
                    "usage": usage,
                },
            });
            let result = serde_json::json!({
                "type": "result",
                "subtype": "success",
                "session_id": "scripted",
                "model": model,
                "result": text,
                "total_cost_usd": cost,
                "usage": usage,
                "is_error": false,
            });
            format!("{assistant}\n{result}\n")
        };
        match &self.output {
            Output::Reply {
                text,
                model,
                input_tokens,
                output_tokens,
                cost_usd,
            } => reply(
                text,
                model.as_deref(),
                *input_tokens,
                *output_tokens,
                *cost_usd,
            ),
            Output::Malformed => "{\"type\":\"assistant\",\"message\":\nnot json\n".to_string(),
            Output::Overlong { bytes } => reply(&"x".repeat(*bytes), None, 10, 5, 0.001),
            Output::Nothing => String::new(),
        }
    }

    /// Lay the turn out in `dir` for the provider.
    fn write_to(&self, dir: &Path) {
        for (index, edit) in self.edits.iter().enumerate() {
            let edit_dir = dir.join("edits").join((index + 1).to_string());
            write_file(&edit_dir.join("path"), &edit.path);
            write_file(&edit_dir.join("text"), &edit.text);
            if edit.append {
                write_file(&edit_dir.join("append"), "");
            }
        }
        if let Some(hold) = &self.hold {
            let hold_dir = dir.join("hold");
            write_file(
                &hold_dir.join("while"),
                &hold.while_exists.display().to_string(),
            );
            write_file(&hold_dir.join("signal"), &hold.signal.display().to_string());
            write_file(&hold_dir.join("max_secs"), &hold.max_secs.to_string());
        }
        write_file(&dir.join("silent_secs"), &self.silent_secs.to_string());
        write_file(&dir.join("stdout"), &self.stdout());
        write_file(&dir.join("exit_code"), &self.exit_code.to_string());
    }
}

/// The turns a [`ScriptedProvider`] plays.
#[derive(Clone, Debug, Serialize)]
pub struct Script {
    tasks: BTreeMap<String, Vec<Turn>>,
    default: Turn,
}

impl Default for Script {
    fn default() -> Self {
        Self::new()
    }
}

impl Script {
    /// A script whose every call plays [`Turn::reply`].
    pub fn new() -> Self {
        Self {
            tasks: BTreeMap::new(),
            default: Turn::reply(),
        }
    }

    /// Task `id`'s turns, one per attempt; the last repeats.
    pub fn task(mut self, id: &str, turns: impl IntoIterator<Item = Turn>) -> Self {
        let turns: Vec<Turn> = turns.into_iter().collect();
        assert!(!turns.is_empty(), "task {id} needs a turn");
        self.tasks.insert(id.to_string(), turns);
        self
    }

    /// The turn of a call that names no single task, or a task without turns
    /// of its own.
    pub fn otherwise(mut self, turn: Turn) -> Self {
        self.default = turn;
        self
    }
}

/// One call the provider took, from its log.
#[derive(Clone, Debug)]
pub struct Call {
    /// The order in which the call started, from 1.
    pub number: usize,
    /// The task the call was for, when it named exactly one.
    pub task: Option<String>,
    /// Every task id on the call's `Task: <id>: ` lines.
    pub task_ids: Vec<String>,
    /// Which attempt at its task the call was, from 1 (0 without a task).
    pub attempt: u32,
    /// The `--model` the call asked for.
    pub model: Option<String>,
    pub prompt: String,
    pub argv: Vec<String>,
    /// Its environment, as `env` prints it.
    pub env: String,
    pub cwd: PathBuf,
}

/// A [`Script`] installed as a fake Claude CLI. See the module documentation.
#[derive(Clone, Debug)]
pub struct ScriptedProvider {
    dir: PathBuf,
}

impl ScriptedProvider {
    /// Install the provider in `dir`, which is created, to play `script`.
    pub fn install(dir: &Path, script: &Script) -> Self {
        for sub in ["calls", "attempts"] {
            fs::create_dir_all(dir.join(sub))
                .unwrap_or_else(|error| panic!("create {}: {error}", dir.join(sub).display()));
        }
        script.default.write_to(&dir.join("turns/default"));
        for (task, turns) in &script.tasks {
            let task_dir = dir.join("turns").join(format!("task-{task}"));
            for (index, turn) in turns.iter().enumerate() {
                turn.write_to(&task_dir.join((index + 1).to_string()));
            }
            write_file(&task_dir.join("count"), &turns.len().to_string());
        }
        let json = serde_json::to_string_pretty(script).expect("serialize the script");
        write_file(&dir.join("script.json"), &json);
        let provider = Self {
            dir: dir.to_path_buf(),
        };
        write_executable(&provider.command(), PROVIDER);
        provider
    }

    /// The provider's executable, for `roko.toml`.
    pub fn command(&self) -> PathBuf {
        self.dir.join("claude")
    }

    /// Where the provider logs its calls, one directory each.
    pub fn calls_dir(&self) -> PathBuf {
        self.dir.join("calls")
    }

    /// Every call so far, in the order they started.
    pub fn calls(&self) -> Vec<Call> {
        let mut numbers: Vec<usize> = fs::read_dir(self.calls_dir())
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter_map(|entry| entry.file_name().to_str()?.parse().ok())
                    .collect()
            })
            .unwrap_or_default();
        numbers.sort_unstable();
        numbers
            .into_iter()
            .map(|number| {
                let call = self.calls_dir().join(number.to_string());
                let read = |name: &str| fs::read_to_string(call.join(name)).unwrap_or_default();
                let line =
                    |name: &str| Some(read(name).trim().to_string()).filter(|s| !s.is_empty());
                Call {
                    number,
                    task: line("task"),
                    task_ids: read("ids").split_whitespace().map(String::from).collect(),
                    attempt: line("attempt")
                        .and_then(|attempt| attempt.parse().ok())
                        .unwrap_or(0),
                    model: line("model"),
                    prompt: read("prompt"),
                    argv: fs::read(call.join("argv"))
                        .unwrap_or_default()
                        .split(|byte| *byte == 0)
                        .filter(|arg| !arg.is_empty())
                        .map(|arg| String::from_utf8_lossy(arg).into_owned())
                        .collect(),
                    env: read("env"),
                    cwd: PathBuf::from(read("cwd").trim()),
                }
            })
            .collect()
    }

    /// The calls for `task`, in order.
    pub fn calls_for(&self, task: &str) -> Vec<Call> {
        self.calls()
            .into_iter()
            .filter(|call| call.task.as_deref() == Some(task))
            .collect()
    }

    /// `start <task>` and `end <task>` around each task's turn, in the order
    /// they happened.
    pub fn events(&self) -> Vec<String> {
        fs::read_to_string(self.dir.join("events"))
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }
}

fn write_file(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("create {}: {error}", parent.display()));
    }
    fs::write(path, text).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
}
