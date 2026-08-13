//! `roko trigger` -- manage trigger bindings.
//!
//! Trigger bindings live as JSON files in `.roko/triggers/`. Each binding
//! connects an event source (cron, webhook, signal, etc.) to a graph that
//! is executed when the trigger fires.

use anyhow::{Context as _, Result};
use clap::Subcommand;
use roko_core::trigger::{TriggerBinding, TriggerEvent, TriggerKind, TriggerSource};

use crate::*;

/// Trigger management subcommands.
#[derive(Debug, Subcommand)]
pub enum TriggerCmd {
    /// List all trigger bindings from `.roko/triggers/`.
    List {
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Show details of a named trigger binding.
    Show {
        /// Trigger binding name (file stem in `.roko/triggers/`).
        name: String,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Create a new trigger binding.
    Create {
        /// Trigger binding name (used as file stem).
        name: String,
        /// Trigger kind: webhook, cron, signal, manual.
        #[arg(long)]
        kind: String,
        /// Graph reference to fire when the trigger activates.
        #[arg(long)]
        graph: String,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Manually fire a trigger, publishing a TriggerEvent.
    Fire {
        /// Trigger binding name to fire.
        name: String,
        /// JSON payload to include in the trigger event.
        #[arg(long, default_value = "{}")]
        payload: String,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

pub(crate) async fn cmd_trigger(cli: &Cli, cmd: TriggerCmd) -> Result<i32> {
    match cmd {
        TriggerCmd::List { workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_list(cli, &wd)
        }
        TriggerCmd::Show { name, workdir } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_show(cli, &wd, &name)
        }
        TriggerCmd::Create {
            name,
            kind,
            graph,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_create(cli, &wd, &name, &kind, &graph)
        }
        TriggerCmd::Fire {
            name,
            payload,
            workdir,
        } => {
            let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
            cmd_fire(cli, &wd, &name, &payload).await
        }
    }
}

/// Directory where trigger bindings are stored.
fn triggers_dir(workdir: &Path) -> PathBuf {
    workdir.join(".roko").join("triggers")
}

/// Load a single trigger binding from its JSON file.
fn load_binding(path: &Path) -> Result<TriggerBinding> {
    let data = std::fs::read_to_string(path)
        .with_context(|| format!("read trigger binding {}", path.display()))?;
    serde_json::from_str(&data).with_context(|| format!("parse trigger binding {}", path.display()))
}

/// Load all trigger bindings from the triggers directory.
fn load_all_bindings(workdir: &Path) -> Result<Vec<TriggerBinding>> {
    let dir = triggers_dir(workdir);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut bindings = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "json")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        match load_binding(&entry.path()) {
            Ok(binding) => bindings.push(binding),
            Err(e) => {
                eprintln!("warning: skipping {}: {e}", entry.path().display());
            }
        }
    }
    Ok(bindings)
}

fn cmd_list(cli: &Cli, workdir: &Path) -> Result<i32> {
    let bindings = load_all_bindings(workdir)?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&bindings)?);
        return Ok(EXIT_SUCCESS);
    }

    if bindings.is_empty() {
        println!("No trigger bindings found.");
        println!("Create one with: roko trigger create <name> --kind <kind> --graph <graph>");
        return Ok(EXIT_SUCCESS);
    }

    println!(
        "{:<20} {:<12} {:<30} {}",
        "NAME", "KIND", "GRAPH", "ENABLED"
    );
    println!("{}", "-".repeat(70));

    for b in &bindings {
        let kind_label = trigger_kind_label(&b.kind);
        println!(
            "{:<20} {:<12} {:<30} {}",
            b.name,
            kind_label,
            b.graph,
            if b.enabled { "yes" } else { "no" }
        );
    }
    println!("\n{} trigger(s)", bindings.len());

    Ok(EXIT_SUCCESS)
}

fn cmd_show(cli: &Cli, workdir: &Path, name: &str) -> Result<i32> {
    let path = triggers_dir(workdir).join(format!("{name}.json"));
    if !path.exists() {
        if cli.json {
            println!(
                "{}",
                serde_json::json!({"error": format!("trigger '{name}' not found")})
            );
        } else {
            eprintln!("trigger '{name}' not found at {}", path.display());
        }
        return Ok(EXIT_FAILURE);
    }

    let binding = load_binding(&path)?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&binding)?);
        return Ok(EXIT_SUCCESS);
    }

    println!("Trigger: {}", binding.name);
    println!("  kind:        {}", trigger_kind_label(&binding.kind));
    println!("  graph:       {}", binding.graph);
    println!("  enabled:     {}", binding.enabled);
    println!("  concurrency: {}", concurrency_label(&binding.concurrency));

    match &binding.kind {
        TriggerKind::Cron(c) => {
            println!("  expression:  {}", c.expression);
            if let Some(tz) = &c.timezone {
                println!("  timezone:    {tz}");
            }
        }
        TriggerKind::Webhook(w) => {
            println!("  path:        {}", w.path);
            if let Some(method) = &w.method {
                println!("  method:      {method}");
            }
        }
        TriggerKind::FileWatch(fw) => {
            println!("  watch_path:  {}", fw.path.display());
            let events: Vec<_> = fw.events.iter().map(|e| format!("{e:?}")).collect();
            println!("  events:      {}", events.join(", "));
            if let Some(g) = &fw.glob {
                println!("  glob:        {g}");
            }
        }
        TriggerKind::Bus(b) => {
            println!("  topic:       {}", b.topic);
        }
        TriggerKind::ChainEvent(ce) => {
            println!("  chain_id:    {}", ce.chain_id);
            println!("  contract:    {}", ce.contract);
            println!("  event_sig:   {}", ce.event_signature);
        }
        TriggerKind::Manual => {
            println!("  (manually fired)");
        }
        TriggerKind::SignalPattern(sp) => {
            println!("  description: {}", sp.description);
            println!("  required:    {}", sp.required_kinds.join(", "));
            println!("  window_secs: {}", sp.window_secs);
        }
    }

    if let Some(space) = &binding.space {
        println!("  space:       {space}");
    }
    if binding.filter.is_some() {
        println!("  filter:      (configured)");
    }
    if binding.auth.is_some() {
        println!("  auth:        (configured)");
    }

    Ok(EXIT_SUCCESS)
}

fn cmd_create(cli: &Cli, workdir: &Path, name: &str, kind: &str, graph: &str) -> Result<i32> {
    let trigger_kind = parse_trigger_kind(kind)?;
    let binding = TriggerBinding::new(name, trigger_kind, graph);

    let dir = triggers_dir(workdir);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("create triggers directory {}", dir.display()))?;

    let path = dir.join(format!("{name}.json"));
    if path.exists() {
        if cli.json {
            println!(
                "{}",
                serde_json::json!({"error": format!("trigger '{name}' already exists")})
            );
        } else {
            eprintln!("trigger '{name}' already exists at {}", path.display());
        }
        return Ok(EXIT_FAILURE);
    }

    let json = serde_json::to_string_pretty(&binding).context("serialize trigger binding")?;
    std::fs::write(&path, &json)
        .with_context(|| format!("write trigger binding to {}", path.display()))?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&binding)?);
    } else {
        println!("Created trigger '{name}' at {}", path.display());
        println!("  kind:  {kind}");
        println!("  graph: {graph}");
    }

    Ok(EXIT_SUCCESS)
}

async fn cmd_fire(cli: &Cli, workdir: &Path, name: &str, payload_str: &str) -> Result<i32> {
    let path = triggers_dir(workdir).join(format!("{name}.json"));
    if !path.exists() {
        if cli.json {
            println!(
                "{}",
                serde_json::json!({"error": format!("trigger '{name}' not found")})
            );
        } else {
            eprintln!("trigger '{name}' not found at {}", path.display());
        }
        return Ok(EXIT_FAILURE);
    }

    let binding = load_binding(&path)?;
    if !binding.enabled {
        if cli.json {
            println!(
                "{}",
                serde_json::json!({"error": format!("trigger '{name}' is disabled")})
            );
        } else {
            eprintln!("trigger '{name}' is disabled; enable it first");
        }
        return Ok(EXIT_FAILURE);
    }

    let payload: serde_json::Value =
        serde_json::from_str(payload_str).context("parse --payload as JSON")?;

    let trace_id = format!("manual-{}", uuid::Uuid::new_v4());
    let event = TriggerEvent::new(
        name.to_string(),
        payload,
        TriggerSource::Manual {
            user: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "unknown".to_string()),
        },
        trace_id.clone(),
    );

    // Persist the event to .roko/triggers/events/ for the trigger engine to pick up.
    let events_dir = triggers_dir(workdir).join("events");
    std::fs::create_dir_all(&events_dir)
        .with_context(|| format!("create trigger events directory {}", events_dir.display()))?;

    let event_path = events_dir.join(format!("{name}-{trace_id}.json"));
    let event_json = serde_json::to_string_pretty(&event).context("serialize trigger event")?;
    std::fs::write(&event_path, &event_json)
        .with_context(|| format!("write trigger event to {}", event_path.display()))?;

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&event)?);
    } else {
        println!("Fired trigger '{name}'");
        println!("  trace_id: {trace_id}");
        println!("  graph:    {}", binding.graph);
        println!("  event:    {}", event_path.display());
    }

    Ok(EXIT_SUCCESS)
}

/// Parse a user-supplied kind string into a `TriggerKind`.
///
/// For kinds that require additional configuration (cron expression, webhook
/// path, etc.), we create a minimal placeholder that the user can edit in the
/// generated JSON file.
fn parse_trigger_kind(kind: &str) -> Result<TriggerKind> {
    match kind.to_lowercase().as_str() {
        "webhook" => Ok(TriggerKind::Webhook(roko_core::trigger::WebhookTrigger {
            method: Some("POST".to_string()),
            path: "/hook/placeholder".to_string(),
        })),
        "cron" => Ok(TriggerKind::Cron(roko_core::trigger::CronTrigger {
            expression: "0 * * * *".to_string(),
            timezone: None,
        })),
        "signal" => Ok(TriggerKind::SignalPattern(
            roko_core::trigger::SignalPatternTrigger {
                description: "Signal pattern trigger".to_string(),
                required_kinds: vec![],
                window_secs: 60,
            },
        )),
        "manual" => Ok(TriggerKind::Manual),
        "bus" => Ok(TriggerKind::Bus(roko_core::trigger::BusTrigger {
            topic: "*".to_string(),
        })),
        "filewatch" | "file_watch" | "file-watch" => Ok(TriggerKind::FileWatch(
            roko_core::trigger::FileWatchTrigger {
                path: PathBuf::from("."),
                events: vec![roko_core::trigger::FileWatchEvent::Any],
                glob: None,
            },
        )),
        "chain" | "chain_event" | "chain-event" => Ok(TriggerKind::ChainEvent(
            roko_core::trigger::ChainEventTrigger {
                chain_id: 1,
                contract: "0x0000000000000000000000000000000000000000".to_string(),
                event_signature: "Transfer(address,address,uint256)".to_string(),
            },
        )),
        other => anyhow::bail!(
            "unknown trigger kind '{other}'; valid kinds: webhook, cron, signal, manual, bus, filewatch, chain"
        ),
    }
}

/// Human-readable label for a `TriggerKind`.
fn trigger_kind_label(kind: &TriggerKind) -> &'static str {
    match kind {
        TriggerKind::Cron(_) => "cron",
        TriggerKind::Webhook(_) => "webhook",
        TriggerKind::FileWatch(_) => "filewatch",
        TriggerKind::Bus(_) => "bus",
        TriggerKind::ChainEvent(_) => "chain",
        TriggerKind::Manual => "manual",
        TriggerKind::SignalPattern(_) => "signal",
    }
}

/// Human-readable label for a `ConcurrencyPolicy`.
fn concurrency_label(policy: &roko_core::trigger::ConcurrencyPolicy) -> String {
    match policy {
        roko_core::trigger::ConcurrencyPolicy::Queue { max_depth } => match max_depth {
            Some(d) => format!("queue (max {d})"),
            None => "queue (unbounded)".to_string(),
        },
        roko_core::trigger::ConcurrencyPolicy::Skip => "skip".to_string(),
        roko_core::trigger::ConcurrencyPolicy::CancelRunning => "cancel_running".to_string(),
        roko_core::trigger::ConcurrencyPolicy::Parallel { max_concurrent } => {
            match max_concurrent {
                Some(c) => format!("parallel (max {c})"),
                None => "parallel (unbounded)".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_trigger_kind_webhook() {
        let kind = parse_trigger_kind("webhook").unwrap();
        assert!(matches!(kind, TriggerKind::Webhook(_)));
    }

    #[test]
    fn parse_trigger_kind_cron() {
        let kind = parse_trigger_kind("cron").unwrap();
        assert!(matches!(kind, TriggerKind::Cron(_)));
    }

    #[test]
    fn parse_trigger_kind_signal() {
        let kind = parse_trigger_kind("signal").unwrap();
        assert!(matches!(kind, TriggerKind::SignalPattern(_)));
    }

    #[test]
    fn parse_trigger_kind_manual() {
        let kind = parse_trigger_kind("manual").unwrap();
        assert!(matches!(kind, TriggerKind::Manual));
    }

    #[test]
    fn parse_trigger_kind_bus() {
        let kind = parse_trigger_kind("bus").unwrap();
        assert!(matches!(kind, TriggerKind::Bus(_)));
    }

    #[test]
    fn parse_trigger_kind_filewatch_variants() {
        for variant in &["filewatch", "file_watch", "file-watch"] {
            let kind = parse_trigger_kind(variant).unwrap();
            assert!(
                matches!(kind, TriggerKind::FileWatch(_)),
                "failed for variant: {variant}"
            );
        }
    }

    #[test]
    fn parse_trigger_kind_chain_variants() {
        for variant in &["chain", "chain_event", "chain-event"] {
            let kind = parse_trigger_kind(variant).unwrap();
            assert!(
                matches!(kind, TriggerKind::ChainEvent(_)),
                "failed for variant: {variant}"
            );
        }
    }

    #[test]
    fn parse_trigger_kind_unknown_fails() {
        assert!(parse_trigger_kind("unknown").is_err());
    }

    #[test]
    fn parse_trigger_kind_case_insensitive() {
        assert!(parse_trigger_kind("Webhook").is_ok());
        assert!(parse_trigger_kind("CRON").is_ok());
        assert!(parse_trigger_kind("Manual").is_ok());
    }

    #[test]
    fn trigger_kind_labels() {
        assert_eq!(trigger_kind_label(&TriggerKind::Manual), "manual");
        assert_eq!(
            trigger_kind_label(&TriggerKind::Cron(roko_core::trigger::CronTrigger {
                expression: "* * * * *".into(),
                timezone: None,
            })),
            "cron"
        );
    }

    #[test]
    fn concurrency_labels() {
        assert_eq!(
            concurrency_label(&roko_core::trigger::ConcurrencyPolicy::Skip),
            "skip"
        );
        assert_eq!(
            concurrency_label(&roko_core::trigger::ConcurrencyPolicy::Queue { max_depth: Some(5) }),
            "queue (max 5)"
        );
        assert_eq!(
            concurrency_label(&roko_core::trigger::ConcurrencyPolicy::Queue { max_depth: None }),
            "queue (unbounded)"
        );
        assert_eq!(
            concurrency_label(&roko_core::trigger::ConcurrencyPolicy::CancelRunning),
            "cancel_running"
        );
        assert_eq!(
            concurrency_label(&roko_core::trigger::ConcurrencyPolicy::Parallel {
                max_concurrent: Some(3)
            }),
            "parallel (max 3)"
        );
        assert_eq!(
            concurrency_label(&roko_core::trigger::ConcurrencyPolicy::Parallel {
                max_concurrent: None
            }),
            "parallel (unbounded)"
        );
    }

    #[test]
    fn load_binding_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let binding = TriggerBinding::new("test-trigger", TriggerKind::Manual, "plans/test.toml");
        let path = tmp.path().join("test-trigger.json");
        let json = serde_json::to_string_pretty(&binding).unwrap();
        std::fs::write(&path, &json).unwrap();

        let loaded = load_binding(&path).unwrap();
        assert_eq!(loaded.name, "test-trigger");
        assert_eq!(loaded.graph, "plans/test.toml");
        assert!(loaded.enabled);
    }

    #[test]
    fn load_all_bindings_from_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let bindings = load_all_bindings(tmp.path()).unwrap();
        assert!(bindings.is_empty());
    }

    #[test]
    fn load_all_bindings_skips_non_json() {
        let tmp = tempfile::tempdir().unwrap();
        let triggers = tmp.path().join(".roko").join("triggers");
        std::fs::create_dir_all(&triggers).unwrap();

        // Write a JSON trigger file.
        let binding = TriggerBinding::new("good", TriggerKind::Manual, "plans/g.toml");
        std::fs::write(
            triggers.join("good.json"),
            serde_json::to_string(&binding).unwrap(),
        )
        .unwrap();

        // Write a non-JSON file that should be skipped.
        std::fs::write(triggers.join("readme.txt"), "not a trigger").unwrap();

        let loaded = load_all_bindings(tmp.path()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "good");
    }
}
