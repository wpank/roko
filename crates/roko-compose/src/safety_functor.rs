//! Capability-level safety wrapper, structurally outside cross-cut arbitration.

use std::sync::Arc;

use async_trait::async_trait;
use roko_agent::safety::contract::AgentContract;
use roko_core::capabilities::{Capability, CapabilitySet};
use roko_core::{Body, Signal};

use crate::cross_cut::{CrossCutContext, CrossCutFunctor, CrossCutResult, EnrichedCell};

/// Capability and contract pre-filter that never participates in VCG.
pub struct SafetyFunctor {
    contract: AgentContract,
    grants: CapabilitySet,
}

impl SafetyFunctor {
    /// Construct from the role's active contract and capability grants.
    #[must_use]
    pub const fn new(contract: AgentContract, grants: CapabilitySet) -> Self {
        Self { contract, grants }
    }

    /// Active immutable contract.
    #[must_use]
    pub const fn contract(&self) -> &AgentContract {
        &self.contract
    }

    /// Active capability grant set.
    #[must_use]
    pub const fn grants(&self) -> CapabilitySet {
        self.grants
    }

    /// Build `F_safety . F_inner`, placing safety outside every other functor.
    #[must_use]
    pub fn wrap(
        self: Arc<Self>,
        inner: Vec<Arc<dyn CrossCutFunctor<CrossCutContext>>>,
    ) -> EnrichedCell {
        let mut functors: Vec<Arc<dyn CrossCutFunctor<CrossCutContext>>> = vec![self];
        functors.extend(inner);
        EnrichedCell::new(functors)
    }

    fn capability_allowed(&self, signal: &Signal) -> bool {
        required_capability_names(signal).into_iter().all(|name| {
            parse_capability(&name).is_some_and(|capability| self.grants.has(capability))
        })
    }

    fn contract_allowed(&self, signal: &Signal) -> bool {
        let taint_allowed = self
            .contract
            .check_taint_level(signal.provenance.effective_trust_origin())
            .is_ok();
        let tool_allowed = signal
            .tag("tool_name")
            .or_else(|| signal.tag("tool"))
            .is_none_or(|tool| self.contract.permits_tool(tool));
        taint_allowed && tool_allowed
    }
}

#[async_trait]
impl CrossCutFunctor for SafetyFunctor {
    fn name(&self) -> &str {
        "safety"
    }

    async fn pre_enrich(
        &self,
        input: Vec<Signal>,
        _ctx: &CrossCutContext,
    ) -> CrossCutResult<Vec<Signal>> {
        Ok(input
            .into_iter()
            .filter(|signal| {
                let allowed = self.capability_allowed(signal);
                if !allowed {
                    tracing::warn!(
                        signal_id = %signal.id,
                        "safety cross-cut filtered signal outside agent capability grants"
                    );
                }
                allowed
            })
            .collect())
    }

    async fn post_enrich(
        &self,
        output: Vec<Signal>,
        _ctx: &CrossCutContext,
    ) -> CrossCutResult<Vec<Signal>> {
        Ok(output
            .into_iter()
            .filter(|signal| {
                let allowed = self.contract_allowed(signal);
                if !allowed {
                    tracing::warn!(
                        signal_id = %signal.id,
                        contract_role = %self.contract.role,
                        "safety cross-cut filtered contract-violating output"
                    );
                }
                allowed
            })
            .collect())
    }

    fn should_short_circuit(&self) -> bool {
        false
    }
}

fn required_capability_names(signal: &Signal) -> Vec<String> {
    let mut names = signal
        .tag("required_capabilities")
        .or_else(|| signal.tag("requires_capability"))
        .into_iter()
        .flat_map(split_capabilities)
        .collect::<Vec<_>>();
    if let Body::Json(value) = &signal.body
        && let Some(required) = value
            .get("required_capabilities")
            .or_else(|| value.get("requires_capability"))
    {
        match required {
            serde_json::Value::String(name) => names.extend(split_capabilities(name)),
            serde_json::Value::Array(values) => names.extend(
                values
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .flat_map(split_capabilities),
            ),
            _ => names.push("<invalid>".into()),
        }
    }
    names
}

fn split_capabilities(names: &str) -> impl Iterator<Item = String> + '_ {
    names
        .split([',', ' ', ';'])
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

fn parse_capability(name: &str) -> Option<Capability> {
    match name.to_ascii_lowercase().as_str() {
        "read" | "read_fs" | "readfs" => Some(Capability::ReadFs),
        "write" | "write_fs" | "writefs" | "filesystem" => Some(Capability::WriteFs),
        "network" => Some(Capability::Network),
        "shell" | "execute" | "subprocess" => Some(Capability::Shell),
        "llm" => Some(Capability::Llm),
        "secret" | "secrets" => Some(Capability::Secrets),
        "bus" => Some(Capability::Bus),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use roko_core::extension::CamelTaintLevel;
    use roko_core::{Kind, Provenance};

    use super::*;

    #[tokio::test]
    async fn capability_filter_is_deny_by_default_and_always_active() {
        let safety = SafetyFunctor::new(
            AgentContract::permissive("tester"),
            CapabilitySet::from([Capability::ReadFs]),
        );
        let read = Signal::builder(Kind::Task)
            .tag("requires_capability", "read_fs")
            .build();
        let shell = Signal::builder(Kind::Task)
            .tag("requires_capability", "shell")
            .build();
        let unknown = Signal::builder(Kind::Task)
            .tag("requires_capability", "teleport")
            .build();

        let filtered = safety
            .pre_enrich(
                vec![read.clone(), shell, unknown],
                &CrossCutContext::default(),
            )
            .await
            .unwrap();

        assert_eq!(filtered, vec![read]);
        assert!(!safety.should_short_circuit());
    }

    #[tokio::test]
    async fn post_filter_enforces_contract_taint_and_tool_allowlist() {
        let mut contract = AgentContract::permissive("reader");
        contract.allowed_tools = Some(vec!["read_file".into()]);
        contract.max_taint_level = CamelTaintLevel::External;
        let safety = SafetyFunctor::new(contract, CapabilitySet::all());
        let allowed = Signal::builder(Kind::AgentOutput)
            .tag("tool_name", "read_file")
            .build();
        let forbidden_tool = Signal::builder(Kind::AgentOutput)
            .tag("tool_name", "bash")
            .build();
        let tainted = Signal::builder(Kind::AgentOutput)
            .provenance(Provenance::agent("test").with_trust_origin(CamelTaintLevel::Untrusted))
            .build();

        let filtered = safety
            .post_enrich(
                vec![allowed.clone(), forbidden_tool, tainted],
                &CrossCutContext::default(),
            )
            .await
            .unwrap();

        assert_eq!(filtered, vec![allowed]);
    }
}
