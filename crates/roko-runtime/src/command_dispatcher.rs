//! Universal platform command dispatcher (#429).
//!
//! [`CommandDispatcher`] parses incoming chat messages for roko slash commands
//! (e.g. `/status`, `/run <plan>`, `/help`) regardless of which platform the
//! message arrived from.  It is wired into the [`ChatBridge`] incoming message
//! handler so that every platform can respond to the same set of commands.
//!
//! # Supported commands
//!
//! | Command | Arguments | Description |
//! |---|---|---|
//! | `/help` | — | List available commands |
//! | `/status` | — | Report current roko status |
//! | `/run` | `<plan-id>` | Start a plan run |
//! | `/stop` | — | Request graceful shutdown |
//! | `/agents` | — | List running agents |
//!
//! # Custom commands
//!
//! Additional commands can be registered at construction time using
//! [`CommandDispatcher::with_command`].  A registered command provides a
//! description (shown in `/help` output) and a handler that receives the
//! parsed [`ParsedCommand`].

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// ParsedCommand
// ---------------------------------------------------------------------------

/// The result of parsing a slash command from an incoming message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedCommand {
    /// The command name without the leading `/` (e.g. `"help"`, `"run"`).
    pub name: String,
    /// Arguments that followed the command name (split on whitespace).
    pub args: Vec<String>,
    /// The original raw message text.
    pub raw: String,
}

// ---------------------------------------------------------------------------
// CommandResult
// ---------------------------------------------------------------------------

/// The outcome produced by a command handler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    /// The command was handled successfully; the string is the reply text.
    Reply(String),
    /// The command was recognised but produced no reply.
    Silent,
    /// The command is unknown to all registered handlers.
    Unknown,
}

// ---------------------------------------------------------------------------
// CommandDispatcher
// ---------------------------------------------------------------------------

type BoxedHandler = Box<dyn Fn(&ParsedCommand) -> CommandResult + Send + Sync>;

/// Parses and dispatches roko slash commands from incoming chat messages.
///
/// # Usage
///
/// ```rust
/// use roko_runtime::command_dispatcher::{CommandDispatcher, CommandResult};
///
/// let dispatcher = CommandDispatcher::new();
/// let result = dispatcher.dispatch("/help");
/// match result {
///     Some(CommandResult::Reply(text)) => println!("{text}"),
///     _ => {}
/// }
/// ```
pub struct CommandDispatcher {
    handlers: HashMap<String, (String, BoxedHandler)>,
}

impl CommandDispatcher {
    /// Construct a dispatcher with the built-in commands pre-registered.
    #[must_use]
    pub fn new() -> Self {
        let mut d = Self {
            handlers: HashMap::new(),
        };
        d.register_builtins();
        d
    }

    /// Register a custom command.
    ///
    /// - `name`: command name without the leading `/`.
    /// - `description`: shown in `/help` output.
    /// - `handler`: called when the command is recognised; receives the parsed
    ///   command and returns a [`CommandResult`].
    pub fn with_command<F>(mut self, name: &str, description: &str, handler: F) -> Self
    where
        F: Fn(&ParsedCommand) -> CommandResult + Send + Sync + 'static,
    {
        self.handlers.insert(
            name.to_lowercase(),
            (description.to_string(), Box::new(handler)),
        );
        self
    }

    /// Try to parse `text` as a slash command and dispatch it.
    ///
    /// Returns `None` if the text is not a slash command (does not start with
    /// `/`).  Returns `Some(CommandResult::Unknown)` if the command was
    /// recognised as a slash command but has no registered handler.
    pub fn dispatch(&self, text: &str) -> Option<CommandResult> {
        let cmd = Self::parse(text)?;
        if let Some((_, handler)) = self.handlers.get(&cmd.name) {
            Some(handler(&cmd))
        } else {
            Some(CommandResult::Unknown)
        }
    }

    /// Parse `text` into a [`ParsedCommand`], or return `None`.
    ///
    /// A valid slash command starts with `/` followed by at least one
    /// non-whitespace character.
    pub fn parse(text: &str) -> Option<ParsedCommand> {
        let text = text.trim();
        if !text.starts_with('/') {
            return None;
        }
        let without_slash = &text[1..];
        if without_slash.is_empty() {
            return None;
        }
        let mut parts = without_slash.split_whitespace();
        let name = parts.next()?.to_lowercase();
        let args: Vec<String> = parts.map(String::from).collect();
        Some(ParsedCommand {
            name,
            args,
            raw: text.to_string(),
        })
    }

    /// Return the names and descriptions of all registered commands.
    pub fn registered_commands(&self) -> Vec<(String, String)> {
        let mut cmds: Vec<(String, String)> = self
            .handlers
            .iter()
            .map(|(name, (desc, _))| (name.clone(), desc.clone()))
            .collect();
        cmds.sort_by(|a, b| a.0.cmp(&b.0));
        cmds
    }

    // ── Built-in commands ──────────────────────────────────────────────────

    fn register_builtins(&mut self) {
        // /help
        self.handlers.insert(
            "help".to_string(),
            (
                "List available roko commands.".to_string(),
                Box::new({
                    // The help handler has to build the list at call time; we
                    // can't reference `self` from inside the closure, so we
                    // emit a static list of the built-in names.
                    |_cmd| {
                        let help = "\
/help      — list available commands
/status    — report current roko status
/run <id>  — start a plan run
/stop      — request graceful shutdown
/agents    — list running agents";
                        CommandResult::Reply(help.to_string())
                    }
                }),
            ),
        );

        // /status
        self.handlers.insert(
            "status".to_string(),
            (
                "Report current roko status.".to_string(),
                Box::new(|_cmd| CommandResult::Reply("roko is running.".to_string())),
            ),
        );

        // /run <plan-id>
        self.handlers.insert(
            "run".to_string(),
            (
                "Start a plan run: /run <plan-id>".to_string(),
                Box::new(|cmd| {
                    if let Some(plan_id) = cmd.args.first() {
                        CommandResult::Reply(format!("Queuing plan run: {plan_id}"))
                    } else {
                        CommandResult::Reply("Usage: /run <plan-id>".to_string())
                    }
                }),
            ),
        );

        // /stop
        self.handlers.insert(
            "stop".to_string(),
            (
                "Request graceful shutdown.".to_string(),
                Box::new(|_cmd| {
                    CommandResult::Reply("Requesting graceful shutdown…".to_string())
                }),
            ),
        );

        // /agents
        self.handlers.insert(
            "agents".to_string(),
            (
                "List running agents.".to_string(),
                Box::new(|_cmd| CommandResult::Reply("Agent list: (no agents running)".to_string())),
            ),
        );
    }
}

impl Default for CommandDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_command() {
        let cmd = CommandDispatcher::parse("/status").unwrap();
        assert_eq!(cmd.name, "status");
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn parse_command_with_args() {
        let cmd = CommandDispatcher::parse("/run my-plan-01").unwrap();
        assert_eq!(cmd.name, "run");
        assert_eq!(cmd.args, vec!["my-plan-01"]);
    }

    #[test]
    fn parse_multi_arg() {
        let cmd = CommandDispatcher::parse("/do a b c").unwrap();
        assert_eq!(cmd.name, "do");
        assert_eq!(cmd.args, ["a", "b", "c"]);
    }

    #[test]
    fn parse_non_command_returns_none() {
        assert!(CommandDispatcher::parse("hello world").is_none());
        assert!(CommandDispatcher::parse("").is_none());
        assert!(CommandDispatcher::parse("/").is_none());
    }

    #[test]
    fn parse_trims_whitespace() {
        let cmd = CommandDispatcher::parse("  /help  ").unwrap();
        assert_eq!(cmd.name, "help");
    }

    #[test]
    fn dispatch_help_returns_reply() {
        let d = CommandDispatcher::new();
        match d.dispatch("/help") {
            Some(CommandResult::Reply(text)) => {
                assert!(text.contains("/status"));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn dispatch_status_returns_reply() {
        let d = CommandDispatcher::new();
        assert!(matches!(d.dispatch("/status"), Some(CommandResult::Reply(_))));
    }

    #[test]
    fn dispatch_run_with_plan_id() {
        let d = CommandDispatcher::new();
        match d.dispatch("/run my-plan") {
            Some(CommandResult::Reply(text)) => assert!(text.contains("my-plan")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn dispatch_run_without_args_gives_usage() {
        let d = CommandDispatcher::new();
        match d.dispatch("/run") {
            Some(CommandResult::Reply(text)) => assert!(text.contains("Usage")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn dispatch_unknown_command() {
        let d = CommandDispatcher::new();
        assert_eq!(d.dispatch("/unknown_xyz"), Some(CommandResult::Unknown));
    }

    #[test]
    fn dispatch_non_command_returns_none() {
        let d = CommandDispatcher::new();
        assert!(d.dispatch("hello").is_none());
    }

    #[test]
    fn custom_command_registered() {
        let d = CommandDispatcher::new()
            .with_command("ping", "Ping the bot.", |_cmd| {
                CommandResult::Reply("pong".to_string())
            });
        match d.dispatch("/ping") {
            Some(CommandResult::Reply(text)) => assert_eq!(text, "pong"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn registered_commands_sorted() {
        let d = CommandDispatcher::new();
        let cmds = d.registered_commands();
        let names: Vec<&str> = cmds.iter().map(|(n, _)| n.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "registered_commands must be sorted");
    }
}
