//! Runtime five-stage cognitive immune Pipeline Graph.
//!
//! This module turns the pure `roko-core` immune transform into five typed,
//! independently executable Graph Cells. Every stage accepts exactly one
//! versioned state variant and rejects missing, malformed, skipped, duplicated,
//! or reordered state. The fixed Graph therefore fails closed even if a caller
//! attempts to reuse a later Cell outside its canonical predecessor chain.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use roko_core::{
    AnomalyScore, Body, ContentHash, ImmuneAssessment, ImmuneContainment, ImmunePerception,
    ImmunePipeline, ImmunePipelineResult, ImmuneValidation, Kind, ProtocolId, Signal, TypeSchema,
    error::Result,
};
use serde::{Deserialize, Serialize};

use crate::cell::{Cell, CellContext, CellVersion};
use crate::engine::{GraphEngine, GraphOutput};
use crate::registry::CellRegistry;
use crate::types::{Edge, ExecutionClass, Graph, GraphError, GraphMetadata, GraphPolicy, Node};

/// Registry key for stage 1.
pub const IMMUNE_PERCEPTION_CELL_TYPE: &str = "security.immune.perception";
/// Registry key for stage 2.
pub const IMMUNE_ASSESSMENT_CELL_TYPE: &str = "security.immune.assessment";
/// Registry key for stage 3.
pub const IMMUNE_CONTAINMENT_CELL_TYPE: &str = "security.immune.containment";
/// Registry key for stage 4.
pub const IMMUNE_VALIDATION_CELL_TYPE: &str = "security.immune.validation";
/// Registry key for stage 5.
pub const IMMUNE_ESCALATION_CELL_TYPE: &str = "security.immune.escalation";

/// Versioned state carried between the five immune Cells.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "stage", content = "state", rename_all = "snake_case")]
pub enum ImmuneCellState {
    /// Trust-boundary input awaiting perception.
    Request {
        /// Exact Signal under review.
        target: ContentHash,
        /// Immutable anomaly evidence supplied by the boundary detector.
        anomaly: AnomalyScore,
        /// Proposed containment effect scope.
        affected_signals: Vec<ContentHash>,
    },
    /// Stage 1 output.
    Perceived {
        /// Target-bound anomaly evidence.
        perception: ImmunePerception,
        /// Proposed containment effect scope, carried without mutation.
        affected_signals: Vec<ContentHash>,
    },
    /// Stage 2 output.
    Assessed {
        /// Deterministic severity assessment.
        assessment: ImmuneAssessment,
        /// Proposed containment effect scope, carried without mutation.
        affected_signals: Vec<ContentHash>,
    },
    /// Stage 3 output.
    Contained(ImmuneContainment),
    /// Stage 4 output.
    Validated(ImmuneValidation),
    /// Stage 5 output.
    Complete(ImmunePipelineResult),
}

impl ImmuneCellState {
    fn stage_name(&self) -> &'static str {
        match self {
            Self::Request { .. } => "request",
            Self::Perceived { .. } => "perceived",
            Self::Assessed { .. } => "assessed",
            Self::Contained(_) => "contained",
            Self::Validated(_) => "validated",
            Self::Complete(_) => "complete",
        }
    }
}

type ImmuneResultTrace = Arc<Mutex<Option<ImmunePipelineResult>>>;

fn state_schema() -> TypeSchema {
    TypeSchema::JsonSchema("roko.security.immune_cell_state.v1".to_string())
}

fn decode_single_state(input: Vec<Signal>, cell_name: &str) -> Result<(Signal, ImmuneCellState)> {
    if input.len() != 1 {
        return Err(roko_core::RokoError::Invalid(format!(
            "{cell_name} requires exactly one typed input Signal, received {}",
            input.len()
        )));
    }
    let Some(parent) = input.into_iter().next() else {
        return Err(roko_core::RokoError::Invalid(format!(
            "{cell_name} input disappeared after length validation"
        )));
    };
    let state = parent.body.as_json::<ImmuneCellState>()?;
    Ok((parent, state))
}

fn encode_state(parent: &Signal, state: &ImmuneCellState) -> Result<Vec<Signal>> {
    Ok(vec![
        Signal::builder(Kind::Custom("roko.security.immune.pipeline".to_string()))
            .body(Body::from_json(state)?)
            .lineage([parent.id])
            .tag("immune_stage", state.stage_name())
            .build(),
    ])
}

macro_rules! define_immune_cell {
    (
        $cell:ident,
        $id:literal,
        $name:literal,
        $protocol:expr,
        |$pipeline:ident, $state:ident| $transition:block
    ) => {
        /// One fixed stage of the cognitive immune Pipeline Graph.
        #[derive(Clone)]
        pub struct $cell {
            pipeline: ImmunePipeline,
            result_trace: Option<ImmuneResultTrace>,
            input_schema: TypeSchema,
            output_schema: TypeSchema,
        }

        impl $cell {
            fn new(pipeline: ImmunePipeline) -> Self {
                Self {
                    pipeline,
                    result_trace: None,
                    input_schema: state_schema(),
                    output_schema: state_schema(),
                }
            }
        }

        impl Default for $cell {
            fn default() -> Self {
                Self::new(ImmunePipeline::default())
            }
        }

        #[async_trait::async_trait]
        impl Cell for $cell {
            fn cell_id(&self) -> &'static str {
                $id
            }

            fn cell_name(&self) -> &'static str {
                $name
            }

            fn cell_version(&self) -> CellVersion {
                (1, 0, 0)
            }

            fn protocols(&self) -> Vec<ProtocolId> {
                vec![$protocol]
            }

            fn input_schema(&self) -> Option<&TypeSchema> {
                Some(&self.input_schema)
            }

            fn output_schema(&self) -> Option<&TypeSchema> {
                Some(&self.output_schema)
            }

            async fn execute(&self, input: Vec<Signal>, _ctx: &CellContext) -> Result<Vec<Signal>> {
                let (parent, $state) = decode_single_state(input, $name)?;
                let $pipeline = &self.pipeline;
                let next = $transition;
                if let ImmuneCellState::Complete(result) = &next
                    && let Some(trace) = &self.result_trace
                {
                    *trace.lock() = Some(result.clone());
                }
                encode_state(&parent, &next)
            }
        }
    };
}

define_immune_cell!(
    ImmunePerceptionCell,
    "immune-perception",
    "ImmunePerception",
    ProtocolId::Observe,
    |pipeline, state| {
        match state {
            ImmuneCellState::Request {
                target,
                anomaly,
                affected_signals,
            } => ImmuneCellState::Perceived {
                perception: pipeline.perceive(target, anomaly),
                affected_signals,
            },
            other => {
                return Err(roko_core::RokoError::Invalid(format!(
                    "ImmunePerception expected request state, got {}",
                    other.stage_name()
                )));
            }
        }
    }
);

define_immune_cell!(
    ImmuneAssessmentCell,
    "immune-assessment",
    "ImmuneAssessment",
    ProtocolId::Score,
    |pipeline, state| {
        match state {
            ImmuneCellState::Perceived {
                perception,
                affected_signals,
            } => ImmuneCellState::Assessed {
                assessment: pipeline.assess(perception),
                affected_signals,
            },
            other => {
                return Err(roko_core::RokoError::Invalid(format!(
                    "ImmuneAssessment expected perceived state, got {}",
                    other.stage_name()
                )));
            }
        }
    }
);

define_immune_cell!(
    ImmuneContainmentCell,
    "immune-containment",
    "ImmuneContainment",
    ProtocolId::React,
    |pipeline, state| {
        match state {
            ImmuneCellState::Assessed {
                assessment,
                affected_signals,
            } => ImmuneCellState::Contained(pipeline.respond(assessment, affected_signals)),
            other => {
                return Err(roko_core::RokoError::Invalid(format!(
                    "ImmuneContainment expected assessed state, got {}",
                    other.stage_name()
                )));
            }
        }
    }
);

define_immune_cell!(
    ImmuneValidationCell,
    "immune-validation",
    "ImmuneValidation",
    ProtocolId::Verify,
    |pipeline, state| {
        match state {
            ImmuneCellState::Contained(containment) => {
                ImmuneCellState::Validated(pipeline.validate(containment))
            }
            other => {
                return Err(roko_core::RokoError::Invalid(format!(
                    "ImmuneValidation expected contained state, got {}",
                    other.stage_name()
                )));
            }
        }
    }
);

define_immune_cell!(
    ImmuneEscalationCell,
    "immune-escalation",
    "ImmuneEscalation",
    ProtocolId::Route,
    |pipeline, state| {
        match state {
            ImmuneCellState::Validated(validation) => {
                ImmuneCellState::Complete(pipeline.escalate(validation))
            }
            other => {
                return Err(roko_core::RokoError::Invalid(format!(
                    "ImmuneEscalation expected validated state, got {}",
                    other.stage_name()
                )));
            }
        }
    }
);

/// Result of one runtime execution of the five-stage immune Graph.
#[derive(Debug, Clone)]
pub struct ImmuneGraphOutput {
    /// Validated containment and escalation decision.
    pub result: ImmunePipelineResult,
    /// Per-stage Graph execution evidence.
    pub graph: GraphOutput,
}

/// Runtime wrapper that executes the cognitive immune transform as five Cells.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ImmunePipelineGraph {
    pipeline: ImmunePipeline,
}

impl ImmunePipelineGraph {
    /// Construct a runtime Graph with explicit normalized thresholds.
    #[must_use]
    pub fn new(quarantine_threshold: f64, critical_threshold: f64) -> Self {
        Self {
            pipeline: ImmunePipeline::new(quarantine_threshold, critical_threshold),
        }
    }

    /// Execute all five typed stages in the canonical order.
    pub async fn screen(
        &self,
        target: ContentHash,
        anomaly: AnomalyScore,
        affected_signals: Vec<ContentHash>,
    ) -> std::result::Result<ImmuneGraphOutput, GraphError> {
        let trace = Arc::new(Mutex::new(None));
        let graph = immune_pipeline_graph()?;
        let registry = traced_registry(self.pipeline, Arc::clone(&trace));
        let state = ImmuneCellState::Request {
            target,
            anomaly,
            affected_signals,
        };
        let input = Signal::builder(Kind::Custom(
            "roko.security.immune.screening_request".to_string(),
        ))
        .body(
            Body::from_json(&state).map_err(|error| GraphError::NodeFailed {
                node_id: "immune-perception".to_string(),
                reason: error.to_string(),
            })?,
        )
        .build();

        let output = GraphEngine::new(graph, registry)
            .with_root_inputs(vec![input])
            .execute(&CellContext::new())
            .await?;
        if !output.success {
            return Err(GraphError::NodeFailed {
                node_id: "immune-pipeline".to_string(),
                reason: output.summary(),
            });
        }
        let result = trace.lock().clone().ok_or_else(|| GraphError::NodeFailed {
            node_id: "immune-pipeline".to_string(),
            reason: "all five stages completed without a captured immune result".to_string(),
        })?;
        Ok(ImmuneGraphOutput {
            result,
            graph: output,
        })
    }
}

/// Build the canonical linear five-stage immune Graph.
pub fn immune_pipeline_graph() -> std::result::Result<Graph, GraphError> {
    let mut graph = Graph::new(GraphMetadata {
        name: "immune-pipeline".to_string(),
        description: Some("Five-stage cognitive immune safety pipeline".to_string()),
        version: Some("1.0.0".to_string()),
        labels: HashMap::from([
            ("security.fail_closed".to_string(), "true".to_string()),
            ("security.effect_free".to_string(), "true".to_string()),
        ]),
    });
    graph.policy = GraphPolicy {
        max_concurrent_nodes: 1,
        ..GraphPolicy::default()
    };

    let stages = [
        ("immune-perception", IMMUNE_PERCEPTION_CELL_TYPE),
        ("immune-assessment", IMMUNE_ASSESSMENT_CELL_TYPE),
        ("immune-containment", IMMUNE_CONTAINMENT_CELL_TYPE),
        ("immune-validation", IMMUNE_VALIDATION_CELL_TYPE),
        ("immune-escalation", IMMUNE_ESCALATION_CELL_TYPE),
    ];
    for (id, cell_type) in stages {
        graph.add_node(Node {
            id: id.to_string(),
            cell_type: cell_type.to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec!["state".to_string()],
            outputs: vec!["state".to_string()],
            execution_class: ExecutionClass::Workflow,
        })?;
    }
    for (from, to) in [
        ("immune-perception", "immune-assessment"),
        ("immune-assessment", "immune-containment"),
        ("immune-containment", "immune-validation"),
        ("immune-validation", "immune-escalation"),
    ] {
        graph.add_edge(Edge {
            from: from.to_string(),
            to: to.to_string(),
            condition: None,
        })?;
    }
    Ok(graph)
}

/// Register all five immutable immune-stage Cell factories.
pub fn register_immune_cells(registry: &mut CellRegistry) {
    registry.register(IMMUNE_PERCEPTION_CELL_TYPE, |_| {
        Box::new(ImmunePerceptionCell::default())
    });
    registry.register(IMMUNE_ASSESSMENT_CELL_TYPE, |_| {
        Box::new(ImmuneAssessmentCell::default())
    });
    registry.register(IMMUNE_CONTAINMENT_CELL_TYPE, |_| {
        Box::new(ImmuneContainmentCell::default())
    });
    registry.register(IMMUNE_VALIDATION_CELL_TYPE, |_| {
        Box::new(ImmuneValidationCell::default())
    });
    registry.register(IMMUNE_ESCALATION_CELL_TYPE, |_| {
        Box::new(ImmuneEscalationCell::default())
    });
}

fn traced_registry(pipeline: ImmunePipeline, trace: ImmuneResultTrace) -> CellRegistry {
    let mut registry = CellRegistry::new();
    registry.register(IMMUNE_PERCEPTION_CELL_TYPE, move |_| {
        Box::new(ImmunePerceptionCell::new(pipeline))
    });
    registry.register(IMMUNE_ASSESSMENT_CELL_TYPE, move |_| {
        Box::new(ImmuneAssessmentCell::new(pipeline))
    });
    registry.register(IMMUNE_CONTAINMENT_CELL_TYPE, move |_| {
        Box::new(ImmuneContainmentCell::new(pipeline))
    });
    registry.register(IMMUNE_VALIDATION_CELL_TYPE, move |_| {
        Box::new(ImmuneValidationCell::new(pipeline))
    });
    registry.register(IMMUNE_ESCALATION_CELL_TYPE, move |_| {
        Box::new(ImmuneEscalationCell {
            result_trace: Some(Arc::clone(&trace)),
            ..ImmuneEscalationCell::new(pipeline)
        })
    });
    registry
}

#[cfg(test)]
mod tests {
    use roko_core::{QuarantineDecision, ResponseAction, ThreatSeverity};

    use crate::engine::NodeStatus;

    use super::*;

    #[test]
    fn default_registry_hosts_all_five_immune_cells() {
        let registry = crate::engine::default_registry();
        for cell_type in [
            IMMUNE_PERCEPTION_CELL_TYPE,
            IMMUNE_ASSESSMENT_CELL_TYPE,
            IMMUNE_CONTAINMENT_CELL_TYPE,
            IMMUNE_VALIDATION_CELL_TYPE,
            IMMUNE_ESCALATION_CELL_TYPE,
        ] {
            assert!(registry.contains(cell_type), "missing {cell_type}");
        }
    }

    fn hash(first: u8) -> ContentHash {
        let mut bytes = [0_u8; 32];
        bytes[0] = first;
        ContentHash(bytes)
    }

    #[tokio::test]
    async fn runtime_graph_executes_all_five_stages_with_pure_transform_parity() {
        let target = hash(1);
        let anomaly = AnomalyScore::from_score(0.82).with_dimension("lineage_gap", 0.9);
        let runtime = ImmunePipelineGraph::new(0.8, 0.95);
        let output = runtime
            .screen(target, anomaly.clone(), Vec::new())
            .await
            .expect("five-stage runtime Graph");
        let pure = ImmunePipeline::new(0.8, 0.95).run(target, anomaly, Vec::new());

        assert_eq!(output.result, pure);
        assert_eq!(
            output.result.validation.containment.assessment.severity,
            ThreatSeverity::High
        );
        assert_eq!(
            output.result.validation.containment.action,
            Some(ResponseAction::IsolateAgent)
        );
        assert!(output.result.escalation_required);
        assert_eq!(output.graph.node_results.len(), 5);
        assert!(
            output
                .graph
                .node_results
                .iter()
                .all(|result| result.status == NodeStatus::Complete)
        );
    }

    #[tokio::test]
    async fn collateral_scope_is_detected_before_escalation() {
        let output = ImmunePipelineGraph::default()
            .screen(hash(2), AnomalyScore::from_score(0.5), vec![hash(3)])
            .await
            .expect("runtime Graph");

        assert_eq!(
            output.result.validation.containment.decision,
            QuarantineDecision::Quarantine
        );
        assert!(!output.result.validation.collateral_safe);
        assert!(output.result.escalation_required);
    }

    #[tokio::test]
    async fn out_of_order_immune_stage_fails_closed() {
        let input = Signal::builder(Kind::Custom("immune".to_string()))
            .body(
                Body::from_json(&ImmuneCellState::Request {
                    target: hash(4),
                    anomaly: AnomalyScore::clean(),
                    affected_signals: Vec::new(),
                })
                .expect("serialize request"),
            )
            .build();
        let error = ImmuneValidationCell::default()
            .execute(vec![input], &CellContext::new())
            .await
            .expect_err("validation cannot skip the first three stages");
        assert!(error.to_string().contains("expected contained state"));
    }

    #[tokio::test]
    async fn malformed_immune_input_fails_closed() {
        let input = Signal::builder(Kind::Task)
            .body(Body::text("opaque"))
            .build();
        assert!(
            ImmunePerceptionCell::default()
                .execute(vec![input], &CellContext::new())
                .await
                .is_err()
        );
    }
}
