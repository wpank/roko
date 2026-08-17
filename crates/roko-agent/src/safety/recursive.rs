//! Non-widening delegation and mandatory recursive-safety validation.
//!
//! Meta-agents may only receive authority that is contained by both their
//! parent's grant and their own role. Every activation and role morph is
//! evaluated by the canonical five-head Graph from `roko-graph`.

use std::collections::BTreeSet;

use roko_core::corrigibility::{
    ActionContext, CorrigibilityDecision, CorrigibilityHead, HeadVerdict,
};
use roko_core::{AgentRole, ToolPermissions};
use roko_graph::cells::CorrigibilityPipelineGraph;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::lifecycle::{
    MAX_META_AGENT_DEPTH, MAX_META_AGENT_FANOUT, MAX_META_AGENT_LINEAGE_COST_USD,
    MAX_META_AGENT_RETRIES,
};

/// Longest lifetime accepted for a newly checked meta-agent grant.
pub const MAX_META_AGENT_GRANT_TTL_SECS: u64 = 30 * 24 * 60 * 60;

/// Recursive authority that a parent may delegate to descendants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnAuthority {
    /// Remaining descendant generations below this agent.
    pub remaining_depth: u32,
    /// Maximum direct children this agent may propose.
    pub max_children: u32,
    /// Maximum rejected child proposals before creation stops.
    pub max_retries: u32,
}

/// Complete authority envelope for a meta-agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaAgentGrant {
    /// Runtime tool capability bits.
    pub tools: ToolPermissions,
    /// Named data scopes, with `*` meaning all scopes.
    #[serde(default)]
    pub data_scopes: BTreeSet<String>,
    /// Exact outbound hosts, with `*` meaning all hosts.
    #[serde(default)]
    pub network_hosts: BTreeSet<String>,
    /// Maximum cumulative cost attributable to this agent.
    pub max_cost_usd: f64,
    /// Trusted unix-second expiry; meta grants are never perpetual.
    pub expires_at: Option<u64>,
    /// Authority this agent may delegate further.
    pub spawn: SpawnAuthority,
}

/// Evidence that the canonical monitor executed all five heads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecursiveSafetyEvidence {
    /// Ordered, fail-closed decision from the fixed Graph.
    pub decision: CorrigibilityDecision,
    /// Number of Graph nodes that reached a terminal execution state.
    pub evaluated_nodes: usize,
}

/// Recursive-safety validation failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RecursiveSafetyError {
    /// A grant exceeds an absolute contract bound.
    #[error("meta-agent grant exceeds an absolute safety bound")]
    InvalidGrant,
    /// Child tool bits exceed the parent or role ceiling.
    #[error("child tool authority exceeds {0} ceiling")]
    ToolWidening(&'static str),
    /// A child requested a data scope not covered by its parent.
    #[error("child data scope `{0}` is not covered by the parent")]
    DataWidening(String),
    /// A child requested a network host not covered by its parent.
    #[error("child network host `{0}` is not covered by the parent")]
    NetworkWidening(String),
    /// A child requested a larger or invalid cost grant.
    #[error("child cost grant is invalid or exceeds the parent")]
    CostWidening,
    /// The child grant is expired, perpetual, or outlives its parent.
    #[error("child grant expiry is absent, expired, or exceeds the parent")]
    GrantExpiry,
    /// Descendant spawning authority was not reduced.
    #[error("child spawn authority exceeds the parent")]
    SpawnWidening,
    /// The canonical Graph could not produce a valid decision.
    #[error("canonical corrigibility graph failed closed: {0}")]
    MonitorFailure(String),
    /// One of the ordered heads vetoed activation.
    #[error("canonical corrigibility graph vetoed activation: {0}")]
    CorrigibilityVeto(String),
}

/// Validate a child grant against its parent and role ceilings.
pub fn validate_delegation(
    parent: &MetaAgentGrant,
    child_role: AgentRole,
    child: &MetaAgentGrant,
) -> Result<(), RecursiveSafetyError> {
    validate_delegation_at(parent, child_role, child, unix_now())
}

/// Validate delegation using a caller-supplied trusted unix-second observation.
pub fn validate_delegation_at(
    parent: &MetaAgentGrant,
    child_role: AgentRole,
    child: &MetaAgentGrant,
    observed_at: u64,
) -> Result<(), RecursiveSafetyError> {
    validate_delegation_structure(parent, child_role, child)?;
    match (parent.expires_at, child.expires_at) {
        (Some(parent_expiry), Some(child_expiry))
            if parent_expiry > observed_at
                && child_expiry > observed_at
                && child_expiry <= observed_at.saturating_add(MAX_META_AGENT_GRANT_TTL_SECS)
                && child_expiry <= parent_expiry => {}
        _ => return Err(RecursiveSafetyError::GrantExpiry),
    }
    Ok(())
}

/// Validate immutable delegation structure while permitting historical expiry.
pub fn validate_delegation_structure(
    parent: &MetaAgentGrant,
    child_role: AgentRole,
    child: &MetaAgentGrant,
) -> Result<(), RecursiveSafetyError> {
    if !grant_is_bounded(parent) || !grant_is_bounded(child) {
        return Err(RecursiveSafetyError::InvalidGrant);
    }
    if !tools_are_subset(child.tools, parent.tools) {
        return Err(RecursiveSafetyError::ToolWidening("parent"));
    }
    if !tools_are_subset(child.tools, child_role.tool_permissions()) {
        return Err(RecursiveSafetyError::ToolWidening("role"));
    }
    validate_named_subset(&parent.data_scopes, &child.data_scopes)
        .map_err(RecursiveSafetyError::DataWidening)?;
    validate_named_subset(&parent.network_hosts, &child.network_hosts)
        .map_err(RecursiveSafetyError::NetworkWidening)?;
    if !parent.max_cost_usd.is_finite()
        || !child.max_cost_usd.is_finite()
        || child.max_cost_usd < 0.0
        || child.max_cost_usd > parent.max_cost_usd
    {
        return Err(RecursiveSafetyError::CostWidening);
    }
    if parent.spawn.remaining_depth == 0
        || child.spawn.remaining_depth >= parent.spawn.remaining_depth
        || child.spawn.max_children > parent.spawn.max_children
        || child.spawn.max_retries > parent.spawn.max_retries
    {
        return Err(RecursiveSafetyError::SpawnWidening);
    }
    Ok(())
}

fn grant_is_bounded(grant: &MetaAgentGrant) -> bool {
    grant.spawn.remaining_depth <= MAX_META_AGENT_DEPTH
        && grant.spawn.max_children <= MAX_META_AGENT_FANOUT
        && grant.spawn.max_retries <= MAX_META_AGENT_RETRIES
        && grant.max_cost_usd.is_finite()
        && grant.max_cost_usd >= 0.0
        && grant.max_cost_usd <= MAX_META_AGENT_LINEAGE_COST_USD
}

/// Intersect tool permissions without ever enabling a previously disabled bit.
#[must_use]
pub const fn intersect_tools(left: ToolPermissions, right: ToolPermissions) -> ToolPermissions {
    ToolPermissions {
        read: left.read && right.read,
        write: left.write && right.write,
        exec: left.exec && right.exec,
        git: left.git && right.git,
        network: left.network && right.network,
    }
}

/// Fixed monitor used at every meta-agent activation and morph boundary.
#[derive(Debug, Clone, Copy, Default)]
pub struct RecursiveSafetyMonitor;

impl RecursiveSafetyMonitor {
    /// Validate delegation and execute the canonical five-head Graph.
    pub async fn validate_activation(
        &self,
        parent: &MetaAgentGrant,
        child_role: AgentRole,
        child: &MetaAgentGrant,
        action_description: impl Into<String>,
        context: ActionContext,
        observed_at: u64,
    ) -> Result<RecursiveSafetyEvidence, RecursiveSafetyError> {
        validate_delegation_at(parent, child_role, child, observed_at)?;
        self.validate_action(action_description, context).await
    }

    /// Execute the canonical monitor for a non-delegating action such as morphing.
    pub async fn validate_action(
        &self,
        action_description: impl Into<String>,
        context: ActionContext,
    ) -> Result<RecursiveSafetyEvidence, RecursiveSafetyError> {
        let output = CorrigibilityPipelineGraph
            .evaluate(action_description, context)
            .await
            .map_err(|error| RecursiveSafetyError::MonitorFailure(error.to_string()))?;
        if let Some((_, reason)) = output.decision.first_veto() {
            return Err(RecursiveSafetyError::CorrigibilityVeto(reason.to_string()));
        }
        let expected = CorrigibilityHead::all_in_order()
            .into_iter()
            .map(|head| (head, HeadVerdict::Pass))
            .collect::<Vec<_>>();
        if output.decision.verdicts != expected || output.graph.node_results.len() != expected.len()
        {
            return Err(RecursiveSafetyError::MonitorFailure(
                "allowed activation did not execute the exact canonical five-head pipeline"
                    .to_string(),
            ));
        }
        Ok(RecursiveSafetyEvidence {
            decision: output.decision,
            evaluated_nodes: output.graph.node_results.len(),
        })
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn tools_are_subset(child: ToolPermissions, parent: ToolPermissions) -> bool {
    (!child.read || parent.read)
        && (!child.write || parent.write)
        && (!child.exec || parent.exec)
        && (!child.git || parent.git)
        && (!child.network || parent.network)
}

fn validate_named_subset(
    parent: &BTreeSet<String>,
    child: &BTreeSet<String>,
) -> Result<(), String> {
    if parent.contains("*") {
        return Ok(());
    }
    child
        .iter()
        .find(|item| !parent.contains(*item))
        .map_or(Ok(()), |item| Err(item.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant() -> MetaAgentGrant {
        MetaAgentGrant {
            tools: AgentRole::Implementer.tool_permissions(),
            data_scopes: BTreeSet::from(["workspace".to_string()]),
            network_hosts: BTreeSet::new(),
            max_cost_usd: 10.0,
            expires_at: Some(unix_now().saturating_add(3_600)),
            spawn: SpawnAuthority {
                remaining_depth: 3,
                max_children: 4,
                max_retries: 2,
            },
        }
    }

    #[test]
    fn delegation_rejects_each_widening_dimension() {
        let parent = grant();
        let mut child = parent.clone();
        child.spawn.remaining_depth = 2;
        child.tools.network = true;
        assert!(matches!(
            validate_delegation(&parent, AgentRole::Implementer, &child),
            Err(RecursiveSafetyError::ToolWidening(_))
        ));

        child.tools.network = false;
        child.data_scopes.insert("secrets".to_string());
        assert!(matches!(
            validate_delegation(&parent, AgentRole::Implementer, &child),
            Err(RecursiveSafetyError::DataWidening(_))
        ));

        child.data_scopes.remove("secrets");
        child.max_cost_usd = 11.0;
        assert_eq!(
            validate_delegation(&parent, AgentRole::Implementer, &child),
            Err(RecursiveSafetyError::CostWidening)
        );
    }

    #[tokio::test]
    async fn canonical_monitor_vetoes_a_recursive_safety_bypass() {
        let error = RecursiveSafetyMonitor
            .validate_action(
                "generated agent disables its safety monitor",
                ActionContext {
                    modifies_audit: Some(true),
                    reversible: Some(false),
                    outputs_verifiable: Some(false),
                    on_task: Some(false),
                    ..ActionContext::default()
                },
            )
            .await
            .expect_err("switch head must veto monitor removal");
        assert!(matches!(error, RecursiveSafetyError::CorrigibilityVeto(_)));
    }
}
