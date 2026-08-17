//! Deterministic routing and composition for event-oriented telemetry Lenses.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use toml::Value;

use crate::{LensScope, ObservableEvent, ObservableEventKind, Result, RokoError};

/// TOML representation of one `[[lenses]]` entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LensConfig {
    pub name: String,
    pub block: String,
    pub scope: String,
    #[serde(default)]
    pub params: BTreeMap<String, Value>,
}

/// A validated Lens entry used by the runtime routing table.
#[derive(Clone, Debug, PartialEq)]
pub struct LensRegistration {
    pub config: LensConfig,
    pub scope: LensScope,
    pub observes: Vec<ObservableEventKind>,
}

/// Registry of independent and chained telemetry Lenses.
///
/// Registration order is retained for raw-event fan-out, so stacked Lenses
/// receive the same event independently. Chain scheduling is computed
/// separately and deterministically with a topological sort.
#[derive(Clone, Debug, Default)]
pub struct LensRegistry {
    registrations: Vec<LensRegistration>,
    chains: BTreeMap<String, Vec<String>>,
}

impl LensRegistry {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            registrations: Vec::new(),
            chains: BTreeMap::new(),
        }
    }

    /// Register a Lens, deriving its event-family filters from the built-in
    /// block name. Unknown/plugin Lens blocks default to `All` and can use
    /// [`Self::register_with_observes`] to declare a narrower filter.
    pub fn register(&mut self, config: LensConfig) -> Result<()> {
        let observes = observes_for_block(&config.block);
        self.register_with_observes(config, observes)
    }

    /// Register a Lens with explicit event-family filters.
    ///
    /// Names are unique. Chained registrations are inserted atomically: if a
    /// new edge closes a cycle, both the registration and edge are rolled
    /// back and the error includes a deterministic cycle path.
    pub fn register_with_observes(
        &mut self,
        config: LensConfig,
        observes: Vec<ObservableEventKind>,
    ) -> Result<()> {
        validate_config(&config)?;
        if self
            .registrations
            .iter()
            .any(|registration| registration.config.name == config.name)
        {
            return Err(RokoError::config(format!(
                "duplicate lens name `{}`",
                config.name
            )));
        }

        let scope = parse_scope(&config.scope)?;
        let observes = normalize_observes(observes);
        let chain_from = match &scope {
            LensScope::Lens(upstream) => Some(upstream.clone()),
            _ => None,
        };
        let name = config.name.clone();

        self.registrations.push(LensRegistration {
            config,
            scope,
            observes,
        });
        if let Some(upstream) = &chain_from {
            let downstream = self.chains.entry(upstream.clone()).or_default();
            downstream.push(name.clone());
            downstream.sort_unstable();
        }

        if let Err(error) = self.chain_order() {
            self.registrations.pop();
            if let Some(upstream) = chain_from {
                let mut remove_entry = false;
                if let Some(downstream) = self.chains.get_mut(&upstream) {
                    downstream.retain(|candidate| candidate != &name);
                    remove_entry = downstream.is_empty();
                }
                if remove_entry {
                    self.chains.remove(&upstream);
                }
            }
            return Err(error);
        }

        Ok(())
    }

    /// Route a raw lifecycle event using the source carried by the event.
    ///
    /// Runtimes that know the source's containing Graph/Agent/Space should use
    /// [`Self::route_with_ancestry`] so named wider scopes can match too.
    #[must_use]
    pub fn route(&self, event: &ObservableEvent) -> Vec<&LensRegistration> {
        self.route_with_ancestry(event, std::slice::from_ref(&event.source_scope()))
    }

    /// Route an event with runtime-resolved source ancestry.
    ///
    /// `ancestry` may contain, for example, Cell, Graph, Agent, and Space
    /// scopes. Empty identifiers in a configured scope are family wildcards.
    #[must_use]
    pub fn route_with_ancestry(
        &self,
        event: &ObservableEvent,
        ancestry: &[LensScope],
    ) -> Vec<&LensRegistration> {
        self.registrations
            .iter()
            .filter(|registration| {
                !matches!(registration.scope, LensScope::Lens(_))
                    && observes_event(&registration.observes, event)
                    && (registration.scope == LensScope::Global
                        || ancestry
                            .iter()
                            .any(|source| registration.scope.matches_source(source)))
            })
            .collect()
    }

    /// Route one upstream Lens output to its directly chained consumers.
    ///
    /// Transitive consumers run only when their immediate upstream emits its
    /// own output, preserving the per-event-cycle ordering contract.
    #[must_use]
    pub fn route_lens_output(
        &self,
        upstream: &str,
        event: &ObservableEvent,
    ) -> Vec<&LensRegistration> {
        let Some(downstream) = self.chains.get(upstream) else {
            return Vec::new();
        };
        downstream
            .iter()
            .filter_map(|name| self.get(name))
            .filter(|registration| observes_event(&registration.observes, event))
            .collect()
    }

    /// Deterministic dependency-first ordering of all registered and
    /// forward-referenced Lens names.
    pub fn chain_order(&self) -> Result<Vec<String>> {
        topo_sort(&self.registrations, &self.chains)
    }

    /// Reject forward references that were not satisfied after all Graph Lens
    /// entries were registered.
    pub fn validate(&self) -> Result<()> {
        let registered = self
            .registrations
            .iter()
            .map(|registration| registration.config.name.as_str())
            .collect::<BTreeSet<_>>();
        let unresolved = self
            .chains
            .keys()
            .filter(|name| !registered.contains(name.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        if unresolved.is_empty() {
            self.chain_order().map(|_| ())
        } else {
            Err(RokoError::config(format!(
                "unresolved upstream lens{}: {}",
                if unresolved.len() == 1 { "" } else { "es" },
                unresolved.join(", ")
            )))
        }
    }

    #[must_use]
    pub fn registrations(&self) -> &[LensRegistration] {
        &self.registrations
    }

    #[must_use]
    pub const fn chains(&self) -> &BTreeMap<String, Vec<String>> {
        &self.chains
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&LensRegistration> {
        self.registrations
            .iter()
            .find(|registration| registration.config.name == name)
    }
}

/// Parse the configuration form of a Lens scope.
///
/// Bare `cell`, `graph`, `agent`, and `space` scopes become empty-identifier
/// family wildcards. Named forms use `kind:name`; `lens:name` must be named.
pub fn parse_scope(input: &str) -> Result<LensScope> {
    let input = input.trim();
    if input.is_empty() {
        return Err(RokoError::config("lens scope cannot be empty"));
    }

    let (kind, target) = input
        .split_once(':')
        .map_or((input, None), |(kind, target)| (kind, Some(target.trim())));
    let kind = kind.trim().to_ascii_lowercase();
    let target = target.unwrap_or("");
    if input.contains(':') && target.is_empty() {
        return Err(RokoError::config(format!(
            "lens scope `{input}` has an empty target"
        )));
    }

    match kind.as_str() {
        "global" if target.is_empty() => Ok(LensScope::Global),
        "cell" => Ok(LensScope::Cell(target.to_string())),
        "graph" => Ok(LensScope::Graph(target.to_string())),
        "agent" => Ok(LensScope::Agent(target.to_string())),
        "space" => Ok(LensScope::Space(target.to_string())),
        "lens" if !target.is_empty() => Ok(LensScope::Lens(target.to_string())),
        "lens" => Err(RokoError::config("a chained lens scope requires a name")),
        "global" => Err(RokoError::config("global lens scope cannot have a target")),
        _ => Err(RokoError::config(format!(
            "unknown lens scope `{input}`; expected global, cell, graph, agent, space, or lens:name"
        ))),
    }
}

/// Default event-family declarations for the built-in Lens catalog.
#[must_use]
pub fn observes_for_block(block: &str) -> Vec<ObservableEventKind> {
    use ObservableEventKind::{
        AgentLifecycle as Agent, All, CellLifecycle as Cell, ExtensionLifecycle as Extension,
        GraphLifecycle as Graph, MemoryLifecycle as Memory, SignalLifecycle as Signal,
        TriggerLifecycle as Trigger, VerifyLifecycle as Verify,
    };

    let block = block.to_ascii_lowercase();
    if block.contains("cost-lens") {
        vec![Cell, Graph, Agent]
    } else if block.contains("latency-lens") {
        vec![Cell, Graph]
    } else if block.contains("quality-lens") {
        vec![Verify, Signal]
    } else if block.contains("efficiency-lens") {
        vec![Cell, Agent]
    } else if block.contains("error-lens") {
        vec![Cell, Graph, Extension]
    } else if block.contains("drift-lens") {
        vec![Memory, Signal]
    } else if block.contains("budget-lens") {
        vec![Agent, Cell]
    } else if block.contains("trend-lens") || block.contains("anomaly-lens") {
        vec![Signal]
    } else if block.contains("usage-lens") {
        vec![Cell, Graph, Trigger]
    } else if block.contains("collective-intelligence-lens") || block.contains("c-factor-lens") {
        vec![Agent, Signal, Memory]
    } else {
        vec![All]
    }
}

fn validate_config(config: &LensConfig) -> Result<()> {
    if config.name.trim().is_empty() {
        return Err(RokoError::config("lens name cannot be empty"));
    }
    if config.name != config.name.trim() {
        return Err(RokoError::config(
            "lens name cannot have surrounding whitespace",
        ));
    }
    if config.block.trim().is_empty() {
        return Err(RokoError::config(format!(
            "lens `{}` has an empty block",
            config.name
        )));
    }
    Ok(())
}

fn normalize_observes(observes: Vec<ObservableEventKind>) -> Vec<ObservableEventKind> {
    if observes.is_empty() || observes.contains(&ObservableEventKind::All) {
        return vec![ObservableEventKind::All];
    }
    observes
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn observes_event(filters: &[ObservableEventKind], event: &ObservableEvent) -> bool {
    filters.iter().any(|filter| filter.matches(event))
}

fn topo_sort(
    registrations: &[LensRegistration],
    chains: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<String>> {
    let mut nodes = registrations
        .iter()
        .map(|registration| registration.config.name.clone())
        .collect::<BTreeSet<_>>();
    for (upstream, downstream) in chains {
        nodes.insert(upstream.clone());
        nodes.extend(downstream.iter().cloned());
    }

    let mut indegree = nodes
        .iter()
        .cloned()
        .map(|name| (name, 0_usize))
        .collect::<BTreeMap<_, _>>();
    for downstream in chains.values() {
        for name in downstream {
            *indegree.entry(name.clone()).or_default() += 1;
        }
    }

    let mut ready = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    let mut ordered = Vec::with_capacity(nodes.len());
    while let Some(name) = ready.iter().next().cloned() {
        ready.remove(&name);
        ordered.push(name.clone());
        if let Some(downstream) = chains.get(&name) {
            for child in downstream {
                let degree = indegree
                    .get_mut(child)
                    .expect("chain nodes populate the indegree map");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(child.clone());
                }
            }
        }
    }

    if ordered.len() == nodes.len() {
        Ok(ordered)
    } else {
        let cycle = find_cycle(&nodes, chains).unwrap_or_default();
        Err(RokoError::config(format!(
            "lens chain cycle detected: {}",
            cycle.join(" -> ")
        )))
    }
}

fn find_cycle(
    nodes: &BTreeSet<String>,
    chains: &BTreeMap<String, Vec<String>>,
) -> Option<Vec<String>> {
    fn visit(
        node: &str,
        chains: &BTreeMap<String, Vec<String>>,
        states: &mut BTreeMap<String, u8>,
        stack: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        states.insert(node.to_string(), 1);
        stack.push(node.to_string());
        for child in chains.get(node).into_iter().flatten() {
            match states.get(child).copied().unwrap_or(0) {
                0 => {
                    if let Some(cycle) = visit(child, chains, states, stack) {
                        return Some(cycle);
                    }
                }
                1 => {
                    let start = stack.iter().position(|entry| entry == child)?;
                    let mut cycle = stack[start..].to_vec();
                    cycle.push(child.clone());
                    return Some(cycle);
                }
                _ => {}
            }
        }
        stack.pop();
        states.insert(node.to_string(), 2);
        None
    }

    let mut states = BTreeMap::new();
    for node in nodes {
        if states.get(node).copied().unwrap_or(0) == 0
            && let Some(cycle) = visit(node, chains, &mut states, &mut Vec::new())
        {
            return Some(cycle);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(name: &str, block: &str, scope: &str) -> LensConfig {
        LensConfig {
            name: name.to_string(),
            block: block.to_string(),
            scope: scope.to_string(),
            params: BTreeMap::new(),
        }
    }

    fn graph_started(graph: &str) -> ObservableEvent {
        ObservableEvent::GraphStarted {
            graph: graph.to_string(),
            run: "run-1".to_string(),
            input_hash: "abc123".to_string(),
        }
    }

    #[test]
    fn lens_config_round_trips_toml_params() {
        let input = r#"
name = "cost-monitor"
block = "roko:cost-lens@^1.0"
scope = "graph:checkout"

[params]
interval = "60s"
budget_warn_pct = 0.8
"#;
        let parsed: LensConfig = toml::from_str(input).unwrap();
        assert_eq!(parsed.name, "cost-monitor");
        assert_eq!(parsed.params["interval"].as_str(), Some("60s"));
        assert_eq!(parsed.params["budget_warn_pct"].as_float(), Some(0.8));
        assert_eq!(
            toml::from_str::<LensConfig>(&toml::to_string(&parsed).unwrap()).unwrap(),
            parsed
        );
    }

    #[test]
    fn parse_scope_accepts_wildcards_and_named_scopes() {
        assert_eq!(parse_scope("global").unwrap(), LensScope::Global);
        assert_eq!(
            parse_scope("graph").unwrap(),
            LensScope::Graph(String::new())
        );
        assert_eq!(
            parse_scope("agent").unwrap(),
            LensScope::Agent(String::new())
        );
        assert_eq!(
            parse_scope("cell:worker").unwrap(),
            LensScope::Cell("worker".into())
        );
        assert_eq!(
            parse_scope("space:prod").unwrap(),
            LensScope::Space("prod".into())
        );
        assert_eq!(
            parse_scope(" LENS:cost-monitor ").unwrap(),
            LensScope::Lens("cost-monitor".into())
        );
        assert!(parse_scope("").is_err());
        assert!(parse_scope("lens").is_err());
        assert!(parse_scope("cell:").is_err());
        assert!(parse_scope("global:anywhere").is_err());
        assert!(parse_scope("component:worker").is_err());
    }

    #[test]
    fn stacking_routes_in_registration_order_with_scope_and_kind_filters() {
        let mut registry = LensRegistry::new();
        registry
            .register(config("cost", "roko:cost-lens@^1", "graph:checkout"))
            .unwrap();
        registry
            .register(config("latency", "roko:latency-lens@^1", "graph:checkout"))
            .unwrap();
        registry
            .register(config("quality", "roko:quality-lens@^1", "graph:checkout"))
            .unwrap();

        let routed = registry
            .route(&graph_started("checkout"))
            .into_iter()
            .map(|registration| registration.config.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(routed, ["cost", "latency"]);
        assert!(registry.route(&graph_started("other")).is_empty());
    }

    #[test]
    fn runtime_ancestry_enables_scope_widening_without_guessing_topology() {
        let mut registry = LensRegistry::new();
        registry
            .register_with_observes(
                config("graph-lens", "plugin:custom@1", "graph:checkout"),
                vec![ObservableEventKind::CellLifecycle],
            )
            .unwrap();
        registry
            .register_with_observes(
                config("space-lens", "plugin:custom@1", "space:prod"),
                vec![ObservableEventKind::CellLifecycle],
            )
            .unwrap();
        let event = ObservableEvent::CellCompleted {
            block: "compile".into(),
            run: "run-1".into(),
            duration_ms: 5,
            cost_usd: 0.01,
        };

        assert!(registry.route(&event).is_empty());
        let routed = registry.route_with_ancestry(
            &event,
            &[
                LensScope::Cell("compile".into()),
                LensScope::Graph("checkout".into()),
                LensScope::Agent("builder".into()),
                LensScope::Space("prod".into()),
            ],
        );
        assert_eq!(routed.len(), 2);
    }

    #[test]
    fn chain_order_is_dependency_first_and_deterministic() {
        let mut registry = LensRegistry::new();
        registry
            .register(config("cost", "roko:cost-lens@^1", "graph"))
            .unwrap();
        registry
            .register(config("trend", "roko:trend-lens@^1", "lens:cost"))
            .unwrap();
        registry
            .register(config("anomaly", "roko:anomaly-lens@^1", "lens:trend"))
            .unwrap();
        registry
            .register(config("alpha", "plugin:independent@1", "global"))
            .unwrap();

        assert_eq!(
            registry.chain_order().unwrap(),
            ["alpha", "cost", "trend", "anomaly"]
        );
        assert_eq!(registry.chains()["cost"], ["trend"]);
        assert_eq!(registry.chains()["trend"], ["anomaly"]);
        registry.validate().unwrap();
    }

    #[test]
    fn lens_outputs_route_only_to_direct_downstream_consumers() {
        let mut registry = LensRegistry::new();
        registry
            .register(config("cost", "roko:cost-lens@^1", "graph"))
            .unwrap();
        registry
            .register(config("trend", "roko:trend-lens@^1", "lens:cost"))
            .unwrap();
        registry
            .register(config("anomaly", "roko:anomaly-lens@^1", "lens:trend"))
            .unwrap();
        let output = ObservableEvent::SignalPruned("observation-signal".into());

        let from_cost = registry.route_lens_output("cost", &output);
        assert_eq!(from_cost.len(), 1);
        assert_eq!(from_cost[0].config.name, "trend");
        let from_trend = registry.route_lens_output("trend", &output);
        assert_eq!(from_trend.len(), 1);
        assert_eq!(from_trend[0].config.name, "anomaly");
        assert!(registry.route_lens_output("anomaly", &output).is_empty());
    }

    #[test]
    fn duplicate_names_and_cycles_are_rejected_atomically() {
        let mut registry = LensRegistry::new();
        registry
            .register(config("a", "plugin:a@1", "lens:b"))
            .unwrap();
        let duplicate = registry
            .register(config("a", "plugin:a@2", "global"))
            .unwrap_err();
        assert!(duplicate.to_string().contains("duplicate lens name `a`"));

        let cycle = registry
            .register(config("b", "plugin:b@1", "lens:a"))
            .unwrap_err();
        assert!(cycle.to_string().contains("a -> b -> a"));
        assert_eq!(registry.registrations().len(), 1);
        assert!(registry.get("b").is_none());
        assert_eq!(registry.chains()["b"], ["a"]);

        let unresolved = registry.validate().unwrap_err();
        assert!(
            unresolved
                .to_string()
                .contains("unresolved upstream lens: b")
        );
        registry
            .register(config("b", "plugin:b@1", "global"))
            .unwrap();
        registry.validate().unwrap();
        assert_eq!(registry.chain_order().unwrap(), ["b", "a"]);
    }
}
