//! Trigger protocol types for E31.
//!
//! This module provides the types needed to evaluate and manage triggers at runtime:
//!
//! - [`TriggerKind`] — discriminant enum matching [`crate::manifest::TriggerDef`] variants.
//! - [`TriggerEntry`] — a registered trigger with stable id, action, and enabled flag.
//! - [`TriggerEvaluation`] — the result of evaluating a trigger against the current state.
//! - [`TriggerRegistry`] — a collection of active triggers that can be iterated and evaluated.
//!
//! # Design
//!
//! `TriggerDef` (in [`crate::manifest`]) is the *on-disk* schema written by plugin authors in
//! `plugin.toml`. `TriggerEntry` is the *runtime* handle that pairs a `TriggerDef` with an
//! assigned `id`, an `action` (plan slug or shell command to execute), and an `enabled` flag.
//!
//! `TriggerRegistry` owns the collection and exposes:
//!
//! 1. `register` — add a new entry.
//! 2. `evaluate_signal` — check which signal-match triggers should fire for a given signal kind.
//! 3. `evaluate_cron` — check which cron triggers are due at the given wall-clock time.
//! 4. `fire_manual` — mark a manual trigger as fired, returning its action.

use crate::manifest::TriggerDef;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── TriggerKind ─────────────────────────────────────────────────────────────

/// Discriminant for the kind of condition a trigger watches.
///
/// This mirrors the variants of [`TriggerDef`] but is a simple `Copy` enum
/// suitable for use as a map key or in match arms without carrying payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerKind {
    /// Fires on a cron schedule.
    Cron,
    /// Fires when a signal with a matching kind is observed on the bus.
    SignalMatch,
    /// Fires when a watched filesystem path changes.
    FileWatch,
    /// Fires when an inbound HTTP webhook is received.
    Webhook,
    /// Fires only when explicitly invoked by a user or operator.
    Manual,
}

impl TriggerKind {
    /// Return the [`TriggerKind`] for the given [`TriggerDef`].
    #[must_use]
    pub fn from_def(def: &TriggerDef) -> Self {
        match def {
            TriggerDef::Cron { .. } => Self::Cron,
            TriggerDef::SignalMatch { .. } => Self::SignalMatch,
            TriggerDef::FileWatch { .. } => Self::FileWatch,
            TriggerDef::Webhook { .. } => Self::Webhook,
            TriggerDef::Manual { .. } => Self::Manual,
        }
    }
}

// ─── TriggerAction ───────────────────────────────────────────────────────────

/// What to execute when a trigger fires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TriggerAction {
    /// Execute a named plan by slug (e.g. `"plans/my-plan/"`).
    PlanSlug {
        /// Plan slug or directory path, passed to `roko plan run`.
        slug: String,
    },
    /// Execute a shell command.
    Command {
        /// Shell command string, executed via `sh -c`.
        command: String,
    },
}

impl TriggerAction {
    /// Convenience constructor for a plan-slug action.
    #[must_use]
    pub fn plan(slug: impl Into<String>) -> Self {
        Self::PlanSlug { slug: slug.into() }
    }

    /// Convenience constructor for a command action.
    #[must_use]
    pub fn command(cmd: impl Into<String>) -> Self {
        Self::Command {
            command: cmd.into(),
        }
    }
}

// ─── TriggerEntry ────────────────────────────────────────────────────────────

/// A registered trigger with a stable ID, definition, action, and enabled flag.
///
/// Created by calling [`TriggerRegistry::register`]. Immutable once registered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerEntry {
    /// Stable identifier for this trigger instance.
    pub id: String,
    /// The kind of condition this trigger watches.
    pub kind: TriggerKind,
    /// The underlying definition carrying the trigger's parameters.
    pub def: TriggerDef,
    /// What to do when the trigger fires.
    pub action: TriggerAction,
    /// Whether this trigger is active. Disabled triggers are never evaluated.
    pub enabled: bool,
    /// Optional source plugin name for debugging.
    pub plugin: Option<String>,
}

impl TriggerEntry {
    /// Create a new entry (always starts enabled).
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        def: TriggerDef,
        action: TriggerAction,
        plugin: Option<String>,
    ) -> Self {
        let kind = TriggerKind::from_def(&def);
        Self {
            id: id.into(),
            kind,
            def,
            action,
            enabled: true,
            plugin,
        }
    }
}

// ─── TriggerEvaluation ───────────────────────────────────────────────────────

/// The result of evaluating whether a single trigger should fire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TriggerEvaluation {
    /// ID of the evaluated trigger.
    pub trigger_id: String,
    /// Whether the trigger should fire given the current state.
    pub should_fire: bool,
    /// Reason for the decision — useful for debugging.
    pub reason: String,
    /// The action to execute if `should_fire` is true.
    pub action: Option<TriggerAction>,
}

impl TriggerEvaluation {
    /// Build a "fire" evaluation.
    #[must_use]
    pub fn fire(id: impl Into<String>, action: TriggerAction, reason: impl Into<String>) -> Self {
        Self {
            trigger_id: id.into(),
            should_fire: true,
            reason: reason.into(),
            action: Some(action),
        }
    }

    /// Build a "skip" evaluation.
    #[must_use]
    pub fn skip(id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            trigger_id: id.into(),
            should_fire: false,
            reason: reason.into(),
            action: None,
        }
    }
}

// ─── TriggerRegistry ─────────────────────────────────────────────────────────

/// Manages active triggers, allows registration, and evaluates firing conditions.
///
/// # Thread safety
///
/// `TriggerRegistry` is not `Sync` — wrap in `Arc<Mutex<TriggerRegistry>>` if you
/// need shared mutable access from multiple async tasks.
#[derive(Debug, Default)]
pub struct TriggerRegistry {
    /// Triggers indexed by their stable ID.
    entries: HashMap<String, TriggerEntry>,
}

impl TriggerRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new trigger. Replaces any existing entry with the same id.
    pub fn register(&mut self, entry: TriggerEntry) {
        self.entries.insert(entry.id.clone(), entry);
    }

    /// Enable or disable a trigger by id. Returns `true` if the id was found.
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Remove a trigger by id. Returns the removed entry if it existed.
    pub fn remove(&mut self, id: &str) -> Option<TriggerEntry> {
        self.entries.remove(id)
    }

    /// Return all registered entries (enabled and disabled).
    pub fn all(&self) -> impl Iterator<Item = &TriggerEntry> {
        self.entries.values()
    }

    /// Return all *enabled* entries.
    pub fn enabled(&self) -> impl Iterator<Item = &TriggerEntry> {
        self.entries.values().filter(|e| e.enabled)
    }

    /// Count of registered triggers (all, including disabled).
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the registry contains no triggers.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Evaluate which signal-match triggers should fire for the given signal kind.
    ///
    /// `signal_body` is the raw JSON body of the received signal (used for optional
    /// `filter_path` / `filter_value` evaluation).  Pass `None` to skip body
    /// filtering (all matching-kind triggers will fire).
    ///
    /// Returns one [`TriggerEvaluation`] per enabled `SignalMatch` trigger.
    #[must_use]
    pub fn evaluate_signal(
        &self,
        signal_kind: &str,
        signal_body: Option<&serde_json::Value>,
    ) -> Vec<TriggerEvaluation> {
        self.enabled()
            .filter(|e| e.kind == TriggerKind::SignalMatch)
            .map(|entry| {
                let TriggerDef::SignalMatch {
                    signal_kind: kind,
                    filter_path,
                    filter_value,
                    ..
                } = &entry.def
                else {
                    return TriggerEvaluation::skip(
                        &entry.id,
                        "internal: def mismatch (SignalMatch)",
                    );
                };

                if kind != signal_kind {
                    return TriggerEvaluation::skip(
                        &entry.id,
                        format!("signal kind `{signal_kind}` != expected `{kind}`"),
                    );
                }

                // Optional body filter.
                if let (Some(path), Some(expected)) = (filter_path.as_deref(), filter_value.as_deref()) {
                    if let Some(body) = signal_body {
                        let pointer = if path.starts_with('/') {
                            path.to_string()
                        } else {
                            format!("/{path}")
                        };
                        let actual = body.pointer(&pointer).and_then(|v| v.as_str());
                        match actual {
                            Some(val) if val == expected => {}
                            Some(val) => {
                                return TriggerEvaluation::skip(
                                    &entry.id,
                                    format!(
                                        "body filter mismatch at `{pointer}`: got `{val}`, expected `{expected}`"
                                    ),
                                );
                            }
                            None => {
                                return TriggerEvaluation::skip(
                                    &entry.id,
                                    format!("body filter path `{pointer}` not found"),
                                );
                            }
                        }
                    } else {
                        // Body filter requested but no body supplied — skip.
                        return TriggerEvaluation::skip(
                            &entry.id,
                            format!("body filter required at `{path}` but no body provided"),
                        );
                    }
                }

                TriggerEvaluation::fire(
                    &entry.id,
                    entry.action.clone(),
                    format!("signal kind matched: `{signal_kind}`"),
                )
            })
            .collect()
    }

    /// Evaluate which cron triggers are due at `now`.
    ///
    /// Parses each cron expression and returns an evaluation for every enabled `Cron`
    /// trigger whose *next scheduled fire* at or before `now`.  Callers are responsible
    /// for tracking which triggers have already been fired to avoid double-firing.
    ///
    /// Returns one [`TriggerEvaluation`] per enabled `Cron` trigger.
    #[must_use]
    pub fn evaluate_cron(&self, now: DateTime<Utc>) -> Vec<TriggerEvaluation> {
        use cron::Schedule;
        use std::str::FromStr;

        self.enabled()
            .filter(|e| e.kind == TriggerKind::Cron)
            .map(|entry| {
                let TriggerDef::Cron { expression, .. } = &entry.def else {
                    return TriggerEvaluation::skip(&entry.id, "internal: def mismatch (Cron)");
                };

                let schedule = match Schedule::from_str(expression) {
                    Ok(s) => s,
                    Err(e) => {
                        return TriggerEvaluation::skip(
                            &entry.id,
                            format!("invalid cron expression `{expression}`: {e}"),
                        );
                    }
                };

                // Check if the most recently scheduled occurrence is at or before `now`.
                // `upcoming` yields the next fire *after* the given time, so we check the
                // occurrence just prior to `now` by looking at what would have fired since
                // one tick ago (use `after` with `now - 1s` as a proxy).
                let one_second_ago = now - chrono::Duration::seconds(1);
                let next_after_past = schedule.after(&one_second_ago).next();
                let should_fire = next_after_past.is_some_and(|next| next <= now);

                if should_fire {
                    TriggerEvaluation::fire(
                        &entry.id,
                        entry.action.clone(),
                        format!("cron expression `{expression}` is due at {now}"),
                    )
                } else {
                    TriggerEvaluation::skip(
                        &entry.id,
                        format!("cron expression `{expression}` not due at {now}"),
                    )
                }
            })
            .collect()
    }

    /// Fire a manual trigger by id.
    ///
    /// Returns `Some(action)` if the trigger exists, is enabled, and is a `Manual` trigger.
    /// Returns `None` if the id is not found, disabled, or not a manual trigger.
    #[must_use]
    pub fn fire_manual(&self, id: &str) -> Option<TriggerAction> {
        let entry = self.entries.get(id)?;
        if !entry.enabled || entry.kind != TriggerKind::Manual {
            return None;
        }
        Some(entry.action.clone())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::TriggerDef;
    use serde_json::json;

    fn make_signal_match_entry(
        id: &str,
        kind: &str,
        filter_path: Option<&str>,
        filter_value: Option<&str>,
    ) -> TriggerEntry {
        TriggerEntry::new(
            id,
            TriggerDef::SignalMatch {
                signal_kind: kind.to_string(),
                filter_path: filter_path.map(str::to_string),
                filter_value: filter_value.map(str::to_string),
                description: None,
            },
            TriggerAction::plan("plans/demo/"),
            None,
        )
    }

    fn make_manual_entry(id: &str) -> TriggerEntry {
        TriggerEntry::new(
            id,
            TriggerDef::Manual {
                label: Some("Run manually".to_string()),
                description: None,
            },
            TriggerAction::command("echo fired"),
            None,
        )
    }

    #[test]
    fn trigger_kind_from_def() {
        assert_eq!(
            TriggerKind::from_def(&TriggerDef::Cron {
                expression: "0 * * * * *".to_string(),
                description: None,
            }),
            TriggerKind::Cron
        );
        assert_eq!(
            TriggerKind::from_def(&TriggerDef::SignalMatch {
                signal_kind: "github:push".to_string(),
                filter_path: None,
                filter_value: None,
                description: None,
            }),
            TriggerKind::SignalMatch
        );
        assert_eq!(
            TriggerKind::from_def(&TriggerDef::Manual {
                label: None,
                description: None,
            }),
            TriggerKind::Manual
        );
    }

    #[test]
    fn registry_register_and_len() {
        let mut reg = TriggerRegistry::new();
        assert!(reg.is_empty());
        reg.register(make_signal_match_entry("t1", "github:push", None, None));
        assert_eq!(reg.len(), 1);
        reg.register(make_manual_entry("t2"));
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn registry_set_enabled_and_disabled_are_skipped() {
        let mut reg = TriggerRegistry::new();
        reg.register(make_signal_match_entry("t1", "github:push", None, None));
        assert_eq!(reg.set_enabled("t1", false), true);
        assert_eq!(reg.set_enabled("missing", false), false);

        let evals = reg.evaluate_signal("github:push", None);
        assert!(evals.is_empty(), "disabled trigger should not be evaluated");
    }

    #[test]
    fn registry_remove() {
        let mut reg = TriggerRegistry::new();
        reg.register(make_signal_match_entry("t1", "github:push", None, None));
        let removed = reg.remove("t1");
        assert!(removed.is_some());
        assert!(reg.is_empty());
    }

    #[test]
    fn evaluate_signal_fires_on_matching_kind() {
        let mut reg = TriggerRegistry::new();
        reg.register(make_signal_match_entry("t1", "github:push", None, None));
        reg.register(make_signal_match_entry(
            "t2",
            "prd.plan_approved",
            None,
            None,
        ));

        // evaluate_signal returns an evaluation for every enabled SignalMatch trigger,
        // including those that don't fire (so the caller knows why each was skipped).
        let evals = reg.evaluate_signal("github:push", None);
        assert_eq!(evals.len(), 2);

        let firing: Vec<_> = evals.iter().filter(|e| e.should_fire).collect();
        assert_eq!(firing.len(), 1);
        assert_eq!(firing[0].trigger_id, "t1");
    }

    #[test]
    fn evaluate_signal_skips_non_matching_kind() {
        let mut reg = TriggerRegistry::new();
        reg.register(make_signal_match_entry("t1", "github:push", None, None));

        let evals = reg.evaluate_signal("slack:message", None);
        assert_eq!(evals.len(), 1);
        assert!(!evals[0].should_fire);
    }

    #[test]
    fn evaluate_signal_body_filter_matches() {
        let mut reg = TriggerRegistry::new();
        reg.register(make_signal_match_entry(
            "t1",
            "github:push",
            Some("/branch"),
            Some("main"),
        ));

        let body = json!({ "branch": "main", "repo": "roko" });
        let evals = reg.evaluate_signal("github:push", Some(&body));
        assert_eq!(evals.len(), 1);
        assert!(evals[0].should_fire, "filter should match");
    }

    #[test]
    fn evaluate_signal_body_filter_mismatches() {
        let mut reg = TriggerRegistry::new();
        reg.register(make_signal_match_entry(
            "t1",
            "github:push",
            Some("/branch"),
            Some("main"),
        ));

        let body = json!({ "branch": "develop" });
        let evals = reg.evaluate_signal("github:push", Some(&body));
        assert_eq!(evals.len(), 1);
        assert!(!evals[0].should_fire, "filter should not match");
    }

    #[test]
    fn evaluate_signal_body_filter_missing_path() {
        let mut reg = TriggerRegistry::new();
        reg.register(make_signal_match_entry(
            "t1",
            "github:push",
            Some("/branch"),
            Some("main"),
        ));

        let body = json!({ "ref": "refs/heads/main" });
        let evals = reg.evaluate_signal("github:push", Some(&body));
        assert!(!evals[0].should_fire, "missing path should not fire");
    }

    #[test]
    fn fire_manual_returns_action() {
        let mut reg = TriggerRegistry::new();
        reg.register(make_manual_entry("manual-1"));

        let action = reg.fire_manual("manual-1");
        assert!(action.is_some());
        assert_eq!(
            action.unwrap(),
            TriggerAction::Command {
                command: "echo fired".to_string()
            }
        );
    }

    #[test]
    fn fire_manual_returns_none_for_non_manual() {
        let mut reg = TriggerRegistry::new();
        reg.register(make_signal_match_entry("sm1", "github:push", None, None));

        assert!(
            reg.fire_manual("sm1").is_none(),
            "non-manual trigger should return None"
        );
    }

    #[test]
    fn fire_manual_returns_none_when_disabled() {
        let mut reg = TriggerRegistry::new();
        reg.register(make_manual_entry("manual-1"));
        reg.set_enabled("manual-1", false);

        assert!(
            reg.fire_manual("manual-1").is_none(),
            "disabled trigger should return None"
        );
    }

    #[test]
    fn trigger_action_constructors() {
        let plan_action = TriggerAction::plan("plans/demo/");
        assert_eq!(
            plan_action,
            TriggerAction::PlanSlug {
                slug: "plans/demo/".to_string()
            }
        );

        let cmd_action = TriggerAction::command("cargo test");
        assert_eq!(
            cmd_action,
            TriggerAction::Command {
                command: "cargo test".to_string()
            }
        );
    }

    #[test]
    fn trigger_entry_starts_enabled() {
        let entry = make_manual_entry("m1");
        assert!(entry.enabled);
        assert_eq!(entry.kind, TriggerKind::Manual);
    }
}
