//! Experiment enrichment Cell (`compose.experiment@1`).
//!
//! Provides prompt experiment assignment sections. This is an **optional**
//! enrichment provider. Experiment sections may modify or replace canonical
//! sections via experiment IDs.

use async_trait::async_trait;
use roko_core::error::Result;
use roko_core::{Body, Kind, Signal};

use crate::prompt::PromptSection;

use super::signals::{ComposeRequest, ComposeScope, ExperimentAssignment, cell_ids};

// ---------------------------------------------------------------------------
// Service trait (layer-safe)
// ---------------------------------------------------------------------------

/// Layer-safe handle for prompt experiment assignment.
pub trait ExperimentProvider: Send + Sync + 'static {
    /// Evaluate which experiments are active for the given context and
    /// return the corresponding prompt sections plus experiment IDs.
    fn assign_experiments(
        &self,
        scope: &ComposeScope,
    ) -> ExperimentResult;
}

/// Result of experiment evaluation.
pub struct ExperimentResult {
    /// Sections contributed by active experiments.
    pub sections: Vec<PromptSection>,
    /// Active experiment IDs.
    pub experiment_ids: Vec<String>,
    /// Non-fatal warnings.
    pub warnings: Vec<String>,
}

/// No-op provider used when no experiment framework is available.
#[derive(Debug, Clone, Copy)]
pub struct NoopExperimentProvider;

impl ExperimentProvider for NoopExperimentProvider {
    fn assign_experiments(&self, _scope: &ComposeScope) -> ExperimentResult {
        ExperimentResult {
            sections: Vec::new(),
            experiment_ids: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Cell implementation
// ---------------------------------------------------------------------------

/// Experiment enrichment Cell for the compose graph.
pub struct ExperimentCell<P: ExperimentProvider = NoopExperimentProvider> {
    provider: P,
}

impl<P: ExperimentProvider> ExperimentCell<P> {
    /// Create a new experiment cell backed by the given provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl Default for ExperimentCell<NoopExperimentProvider> {
    fn default() -> Self {
        Self::new(NoopExperimentProvider)
    }
}

#[async_trait]
impl<P: ExperimentProvider> roko_graph::Cell for ExperimentCell<P> {
    fn cell_id(&self) -> &str {
        cell_ids::EXPERIMENT
    }

    fn cell_name(&self) -> &str {
        "Compose Experiment Provider"
    }

    fn cell_version(&self) -> roko_graph::CellVersion {
        (1, 0, 0)
    }

    async fn execute(
        &self,
        input: Vec<Signal>,
        _ctx: &roko_graph::CellContext,
    ) -> Result<Vec<Signal>> {
        let request = extract_compose_request(&input)?;
        let scope = request.scope.clone();

        let result = self.provider.assign_experiments(&scope);

        let payload = ExperimentAssignment {
            scope,
            sections: result.sections,
            warnings: result.warnings,
            active_experiment_ids: result.experiment_ids,
        };

        let body = Body::from_json(&payload).map_err(|e| {
            roko_core::error::RokoError::Internal(format!(
                "experiment cell serialization: {e}"
            ))
        })?;
        let signal = Signal::builder(Kind::Context).body(body).build();
        Ok(vec![signal])
    }
}

fn extract_compose_request(input: &[Signal]) -> Result<ComposeRequest> {
    for signal in input {
        if let Ok(req) = signal.body.as_json::<ComposeRequest>() {
            return Ok(req);
        }
    }
    Err(roko_core::error::RokoError::Internal(
        "ExperimentCell: no ComposeRequest found in input signals".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::AgentRole;

    #[tokio::test]
    async fn noop_produces_no_experiments() {
        let cell = ExperimentCell::default();
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::Context).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        let payload: ExperimentAssignment = result[0].body.as_json().unwrap();
        assert!(payload.sections.is_empty());
        assert!(payload.active_experiment_ids.is_empty());
    }

    #[tokio::test]
    async fn custom_provider_experiment_ids() {
        struct TestProvider;
        impl ExperimentProvider for TestProvider {
            fn assign_experiments(&self, _scope: &ComposeScope) -> ExperimentResult {
                ExperimentResult {
                    sections: vec![PromptSection::new("exp_section", "Try approach B")
                        .with_section_id("experiment:exp-001:exp_section".into())],
                    experiment_ids: vec!["exp-001".into()],
                    warnings: Vec::new(),
                }
            }
        }

        let cell = ExperimentCell::new(TestProvider);
        let req = ComposeRequest::new("r1", "p1", "t1", AgentRole::Implementer);
        let body = Body::from_json(&req).unwrap();
        let signal = Signal::builder(Kind::Context).body(body).build();

        let result = cell
            .execute(vec![signal], &roko_graph::CellContext::new())
            .await
            .unwrap();
        let payload: ExperimentAssignment = result[0].body.as_json().unwrap();
        assert_eq!(payload.active_experiment_ids, vec!["exp-001"]);
        assert_eq!(payload.sections.len(), 1);
    }
}
