//! Agent metamorphosis and role switching.

use std::collections::HashMap;

use async_trait::async_trait;
use roko_core::corrigibility::ActionContext;
use roko_core::{AgentRole, Body, Context, Provenance, Signal};

use crate::agent::{Agent, AgentResult, derived_output};
use crate::introspection::AgentIdentity;
use crate::safety::{RecursiveSafetyEvidence, RecursiveSafetyMonitor, intersect_tools};

/// A role-shaping profile that can be updated at runtime.
#[derive(Debug, Clone, PartialEq)]
pub struct RoleProfile {
    /// Role this profile applies to.
    pub role: AgentRole,
    /// How clearly the role should expose its reasoning.
    pub clarity: f32,
    /// How differentiated the role should be from adjacent roles.
    pub differentiation: f32,
    /// How aligned the role is with the current plan.
    pub alignment: f32,
}

impl RoleProfile {
    /// Build a default profile for a role.
    #[must_use]
    pub const fn new(role: AgentRole) -> Self {
        Self {
            role,
            clarity: 0.5,
            differentiation: 0.5,
            alignment: 0.5,
        }
    }
}

/// Error returned when a morph is not allowed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MorphError {
    /// The role transition is not present in the allowlist.
    #[error("role transition {from} -> {to} is not allowed")]
    TransitionDenied { from: AgentRole, to: AgentRole },
    /// The mandatory canonical safety Graph failed or vetoed the morph.
    #[error("role morph failed recursive safety validation: {0}")]
    Safety(String),
    /// A prior role morph must be rolled back before another can begin.
    #[error("role morph rollback is still pending")]
    RollbackPending,
    /// No prior role morph is available to roll back.
    #[error("no role morph is available to roll back")]
    NoRollback,
}

/// An [`Agent`] wrapper that can change roles during a run.
pub struct MorphableAgent {
    inner: Box<dyn Agent>,
    identity: AgentIdentity,
    profile: RoleProfile,
    allowed_transitions: HashMap<AgentRole, Vec<AgentRole>>,
    system_prompt: String,
    name: String,
    last_safety_evidence: Option<RecursiveSafetyEvidence>,
    previous_identity: Option<AgentIdentity>,
}

impl std::fmt::Debug for MorphableAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MorphableAgent")
            .field("identity", &self.identity)
            .field("profile", &self.profile)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl MorphableAgent {
    /// Create a new morphable wrapper.
    #[must_use]
    pub fn new(inner: Box<dyn Agent>, identity: AgentIdentity) -> Self {
        let profile = RoleProfile::new(identity.role);
        let system_prompt = system_prompt_for(identity.role, &profile);
        let name = format!("{}[{}]", inner.name(), identity.role.label());
        Self {
            inner,
            identity,
            profile,
            allowed_transitions: default_transition_matrix(),
            system_prompt,
            name,
            last_safety_evidence: None,
            previous_identity: None,
        }
    }

    /// Override the allowlist of transitions.
    #[must_use]
    pub fn with_transitions(
        mut self,
        allowed_transitions: HashMap<AgentRole, Vec<AgentRole>>,
    ) -> Self {
        self.allowed_transitions = allowed_transitions;
        self
    }

    /// Current role.
    #[must_use]
    pub const fn role(&self) -> AgentRole {
        self.identity.role
    }

    /// Current identity snapshot.
    #[must_use]
    pub const fn identity(&self) -> &AgentIdentity {
        &self.identity
    }

    /// Current role profile.
    #[must_use]
    pub const fn profile(&self) -> &RoleProfile {
        &self.profile
    }

    /// The current system-prompt augmentation used for morphing.
    #[must_use]
    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    /// Evidence from the mandatory Graph evaluation for the last morph.
    #[must_use]
    pub const fn last_safety_evidence(&self) -> Option<&RecursiveSafetyEvidence> {
        self.last_safety_evidence.as_ref()
    }

    /// Attempt to morph into a new role.
    pub async fn morph(&mut self, new_role: AgentRole) -> Result<(), MorphError> {
        if self.previous_identity.is_some() {
            return Err(MorphError::RollbackPending);
        }
        if !self.can_morph_to(new_role) {
            return Err(MorphError::TransitionDenied {
                from: self.identity.role,
                to: new_role,
            });
        }

        let prior_identity = self.identity.clone();
        let safety = RecursiveSafetyMonitor
            .validate_action(
                format!(
                    "bounded role morph from {} to {}",
                    self.identity.role.label(),
                    new_role.label()
                ),
                morph_action_context(true),
            )
            .await
            .map_err(|error| MorphError::Safety(error.to_string()))?;

        self.identity.role = new_role;
        self.identity.model_tier = new_role.model_tier();
        // A morph may reduce authority to the target role's defaults, but it
        // must never re-enable a capability lost earlier in the lineage.
        self.identity.capabilities =
            intersect_tools(self.identity.capabilities, new_role.tool_permissions());
        self.profile.role = new_role;
        self.system_prompt = system_prompt_for(new_role, &self.profile);
        self.name = format!("{}[{}]", self.inner.name(), new_role.label());
        self.last_safety_evidence = Some(safety);
        self.previous_identity = Some(prior_identity);
        Ok(())
    }

    /// Roll back the most recent role morph to its exact prior authorization.
    pub async fn rollback_morph(&mut self) -> Result<(), MorphError> {
        let prior_identity = self
            .previous_identity
            .clone()
            .ok_or(MorphError::NoRollback)?;
        let safety = RecursiveSafetyMonitor
            .validate_action(
                format!(
                    "rollback bounded role morph from {} to {}",
                    self.identity.role.label(),
                    prior_identity.role.label()
                ),
                morph_action_context(true),
            )
            .await
            .map_err(|error| MorphError::Safety(error.to_string()))?;
        self.identity = prior_identity;
        self.profile.role = self.identity.role;
        self.system_prompt = system_prompt_for(self.identity.role, &self.profile);
        self.name = format!("{}[{}]", self.inner.name(), self.identity.role.label());
        self.last_safety_evidence = Some(safety);
        self.previous_identity = None;
        Ok(())
    }

    fn can_morph_to(&self, new_role: AgentRole) -> bool {
        role_transition_allowed_in(&self.allowed_transitions, self.identity.role, new_role)
    }

    fn augment_input(&self, input: &Signal) -> Signal {
        if self.system_prompt.is_empty() {
            return input.clone();
        }

        let text = input
            .body
            .as_text()
            .ok()
            .map(|body| format!("{}\n\n{}", self.system_prompt, body))
            .unwrap_or_else(|| self.system_prompt.clone());
        derived_output(input, input.kind.clone(), Body::text(text))
            .provenance(Provenance::agent(self.name()))
            .tag("role", self.identity.role.label())
            .build()
    }
}

/// Whether the canonical role-transition policy permits a morph.
#[must_use]
pub fn role_transition_allowed(from: AgentRole, to: AgentRole) -> bool {
    role_transition_allowed_in(&default_transition_matrix(), from, to)
}

fn role_transition_allowed_in(
    transitions: &HashMap<AgentRole, Vec<AgentRole>>,
    from: AgentRole,
    to: AgentRole,
) -> bool {
    from == to
        || transitions
            .get(&from)
            .is_some_and(|roles| roles.contains(&to))
}

fn morph_action_context(rollback_available: bool) -> ActionContext {
    ActionContext {
        // The caller explicitly invokes morph/rollback; it is not autonomous.
        autonomy_level: Some("assist".to_string()),
        // True only because this wrapper installs an exact one-step rollback.
        reversible: Some(rollback_available),
        // Neither operation can replace or configure the fixed monitor.
        modifies_audit: Some(false),
        // Role and capability intersections are deterministic typed values.
        outputs_verifiable: Some(true),
        // The explicit morph request is the operation's assigned task.
        on_task: Some(true),
    }
}

#[async_trait]
impl Agent for MorphableAgent {
    async fn run(&self, input: &Signal, ctx: &Context) -> AgentResult {
        let wrapped_input = self.augment_input(input);
        let mut result = self.inner.run(&wrapped_input, ctx).await;
        let output_kind = result.output.kind.clone();
        let output_body = result.output.body.clone();
        result.output = derived_output(&result.output, output_kind, output_body)
            .provenance(Provenance::agent(self.name()))
            .tag("role", self.identity.role.label())
            .tag("temperament", self.identity.temperament.to_string())
            .build();
        result
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn supports_streaming(&self) -> bool {
        self.inner.supports_streaming()
    }
}

fn system_prompt_for(role: AgentRole, profile: &RoleProfile) -> String {
    format!(
        "You are now acting as {}. clarity={:.2}, differentiation={:.2}, alignment={:.2}.",
        role.label(),
        profile.clarity,
        profile.differentiation,
        profile.alignment
    )
}

fn default_transition_matrix() -> HashMap<AgentRole, Vec<AgentRole>> {
    use AgentRole::*;

    HashMap::from([
        (Implementer, vec![QuickReviewer, Auditor, Refactorer]),
        (QuickReviewer, vec![Auditor]),
        (Auditor, vec![Implementer, Critic]),
        (Strategist, vec![Implementer, Architect, Researcher]),
        (Researcher, vec![Strategist, Implementer, Auditor]),
        (Conductor, vec![Strategist, Implementer, Auditor]),
        (Refactorer, vec![Auditor, Implementer]),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockAgent;
    use roko_core::{Body, Context, Kind, Signal, Temperament};

    fn prompt(text: &str) -> Signal {
        Signal::builder(Kind::Prompt).body(Body::text(text)).build()
    }

    #[tokio::test]
    async fn morphable_agent_applies_role_tag() {
        let identity = AgentIdentity::new(AgentRole::Implementer, Temperament::Balanced);
        let agent = MorphableAgent::new(Box::new(MockAgent::reply("ok")), identity);
        let result = agent.run(&prompt("hi"), &Context::at(0)).await;
        assert_eq!(result.output.tag("role"), Some("implementer"));
    }

    #[tokio::test]
    async fn morph_rejects_disallowed_transition() {
        let identity = AgentIdentity::new(AgentRole::Implementer, Temperament::Balanced);
        let mut agent =
            MorphableAgent::new(Box::new(MockAgent::reply("ok")), identity).with_transitions(
                HashMap::from([(AgentRole::Implementer, vec![AgentRole::Auditor])]),
            );
        let err = agent.morph(AgentRole::Strategist).await.unwrap_err();
        assert!(matches!(err, MorphError::TransitionDenied { .. }));
    }

    #[tokio::test]
    async fn morph_rollback_restores_only_the_previously_authorized_identity() {
        let identity = AgentIdentity::new(AgentRole::Implementer, Temperament::Balanced);
        let mut agent = MorphableAgent::new(Box::new(MockAgent::reply("ok")), identity)
            .with_transitions(HashMap::from([
                (AgentRole::Implementer, vec![AgentRole::Auditor]),
                (AgentRole::Auditor, vec![AgentRole::Implementer]),
            ]));
        agent
            .morph(AgentRole::Auditor)
            .await
            .expect("reduce to auditor");
        assert!(!agent.identity().capabilities.write);
        assert!(!agent.identity().capabilities.exec);
        agent
            .rollback_morph()
            .await
            .expect("role rollback is available");
        assert!(agent.identity().capabilities.write);
        assert!(agent.identity().capabilities.exec);
        assert_eq!(
            agent
                .last_safety_evidence()
                .expect("mandatory monitor evidence")
                .decision
                .verdicts
                .len(),
            5
        );
    }

    #[tokio::test]
    async fn morph_monitor_vetoes_when_rollback_is_unavailable() {
        let error = RecursiveSafetyMonitor
            .validate_action("irreversible role morph", morph_action_context(false))
            .await
            .expect_err("Impact head must veto an irreversible morph");
        assert!(matches!(
            error,
            crate::safety::RecursiveSafetyError::CorrigibilityVeto(_)
        ));
    }
}
