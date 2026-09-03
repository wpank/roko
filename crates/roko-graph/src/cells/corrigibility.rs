//! Fixed five-Cell corrigibility verification Graph.
//!
//! The live safety layer owns the immutable verifier implementations in
//! `roko-core`. This module hosts each verifier as an independently addressable
//! Graph Cell and fixes their topology to Deference -> Switch -> Truth ->
//! Impact -> Task. Conditional edges stop the Graph after the first veto, while
//! [`CorrigibilityCellState`] independently rejects skipped or reordered heads.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use roko_core::corrigibility::{
    ActionContext, CorrigibilityCellState, CorrigibilityDecision, VerifyDeference, VerifyHead,
    VerifyImpact, VerifySwitch, VerifyTask, VerifyTruth,
};
use roko_core::{Body, Kind, ProtocolId, Signal, TypeSchema, error::Result};

use crate::cell::{Cell, CellContext, CellVersion};
use crate::engine::{GraphEngine, GraphOutput};
use crate::registry::CellRegistry;
use crate::types::{
    Edge, EdgeCondition, ExecutionClass, Graph, GraphError, GraphMetadata, GraphPolicy, Node,
};

/// Registry key for the highest-priority Deference verifier.
pub const VERIFY_DEFERENCE_CELL_TYPE: &str = "security.verify.deference";
/// Registry key for the Switch verifier.
pub const VERIFY_SWITCH_CELL_TYPE: &str = "security.verify.switch";
/// Registry key for the Truth verifier.
pub const VERIFY_TRUTH_CELL_TYPE: &str = "security.verify.truth";
/// Registry key for the Impact verifier.
pub const VERIFY_IMPACT_CELL_TYPE: &str = "security.verify.impact";
/// Registry key for the lowest-priority Task verifier.
pub const VERIFY_TASK_CELL_TYPE: &str = "security.verify.task";

type DecisionTrace = Arc<Mutex<Option<CorrigibilityCellState>>>;

fn state_schema() -> TypeSchema {
    TypeSchema::JsonSchema("roko.security.corrigibility_cell_state.v1".to_string())
}

fn decode_single_state(
    input: Vec<Signal>,
    cell_name: &str,
) -> Result<(Signal, CorrigibilityCellState)> {
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
    let state = parent.body.as_json::<CorrigibilityCellState>()?;
    Ok((parent, state))
}

fn execute_head(
    verifier: &dyn VerifyHead,
    input: Vec<Signal>,
    cell_name: &str,
    trace: Option<&DecisionTrace>,
) -> Result<Vec<Signal>> {
    let (parent, mut state) = decode_single_state(input, cell_name)?;
    state.verify_with(verifier)?;
    if let Some(trace) = trace {
        *trace.lock() = Some(state.clone());
    }
    let body = Body::from_json(&state)?;
    Ok(vec![
        Signal::builder(Kind::GateVerdict)
            .body(body)
            .lineage([parent.id])
            .tag(
                "corrigibility_head",
                format!("{:?}", verifier.head()).to_lowercase(),
            )
            .tag("allowed", state.allowed.to_string())
            .build(),
    ])
}

macro_rules! define_verify_cell {
    ($cell:ident, $verifier:ty, $id:literal, $name:literal) => {
        /// Independently hosted verifier for one immutable corrigibility head.
        #[derive(Clone, Default)]
        pub struct $cell {
            verifier: $verifier,
            trace: Option<DecisionTrace>,
            input_schema: Option<TypeSchema>,
            output_schema: Option<TypeSchema>,
        }

        impl $cell {
            fn traced(trace: DecisionTrace) -> Self {
                Self {
                    trace: Some(trace),
                    input_schema: Some(state_schema()),
                    output_schema: Some(state_schema()),
                    ..Self::default()
                }
            }

            fn standalone() -> Self {
                Self {
                    input_schema: Some(state_schema()),
                    output_schema: Some(state_schema()),
                    ..Self::default()
                }
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
                vec![ProtocolId::Verify]
            }

            fn input_schema(&self) -> Option<&TypeSchema> {
                self.input_schema.as_ref()
            }

            fn output_schema(&self) -> Option<&TypeSchema> {
                self.output_schema.as_ref()
            }

            async fn execute(&self, input: Vec<Signal>, _ctx: &CellContext) -> Result<Vec<Signal>> {
                execute_head(&self.verifier, input, $name, self.trace.as_ref())
            }
        }
    };
}

define_verify_cell!(
    VerifyDeferenceCell,
    VerifyDeference,
    "verify-deference",
    "VerifyDeference"
);
define_verify_cell!(
    VerifySwitchCell,
    VerifySwitch,
    "verify-switch",
    "VerifySwitch"
);
define_verify_cell!(VerifyTruthCell, VerifyTruth, "verify-truth", "VerifyTruth");
define_verify_cell!(
    VerifyImpactCell,
    VerifyImpact,
    "verify-impact",
    "VerifyImpact"
);
define_verify_cell!(VerifyTaskCell, VerifyTask, "verify-task", "VerifyTask");

/// Result of one runtime execution of the fixed corrigibility Graph.
#[derive(Debug, Clone)]
pub struct CorrigibilityGraphOutput {
    /// Ordered decision captured from the last head that actually executed.
    pub decision: CorrigibilityDecision,
    /// Graph execution evidence, including condition-skipped lower heads.
    pub graph: GraphOutput,
}

/// Runtime wrapper for the fixed, non-reorderable five-Cell verification Graph.
#[derive(Debug, Clone, Copy, Default)]
pub struct CorrigibilityPipelineGraph;

impl CorrigibilityPipelineGraph {
    /// Execute the five Verify Cells through [`GraphEngine`].
    ///
    /// A veto is a successful safety decision: its lower-priority conditional
    /// routes are not selected. Malformed state, missing Cells, or execution
    /// errors fail closed as [`GraphError::NodeFailed`].
    pub async fn evaluate(
        &self,
        action_description: impl Into<String>,
        context: ActionContext,
    ) -> std::result::Result<CorrigibilityGraphOutput, GraphError> {
        let trace = Arc::new(Mutex::new(None));
        let graph = corrigibility_pipeline_graph()?;
        let registry = traced_registry(Arc::clone(&trace));
        let state = CorrigibilityCellState::new(action_description, context);
        let input = Signal::builder(Kind::Custom(
            "roko.security.corrigibility.proposal".to_string(),
        ))
        .body(
            Body::from_json(&state).map_err(|error| GraphError::NodeFailed {
                node_id: "verify-deference".to_string(),
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
                node_id: "corrigibility-pipeline".to_string(),
                reason: output.summary(),
            });
        }
        let state = trace.lock().clone().ok_or_else(|| GraphError::NodeFailed {
            node_id: "corrigibility-pipeline".to_string(),
            reason: "no Verify Cell produced a decision".to_string(),
        })?;
        if state.allowed && state.verdicts.len() != 5 {
            return Err(GraphError::NodeFailed {
                node_id: "corrigibility-pipeline".to_string(),
                reason: "allowed decision did not execute all five Verify Cells".to_string(),
            });
        }

        Ok(CorrigibilityGraphOutput {
            decision: state.decision(),
            graph: output,
        })
    }
}

/// Build the canonical five-node conditional verification Graph.
pub fn corrigibility_pipeline_graph() -> std::result::Result<Graph, GraphError> {
    let mut graph = Graph::new(GraphMetadata {
        name: "corrigibility-pipeline".to_string(),
        description: Some("Fixed five-head lexicographic Verify pipeline".to_string()),
        version: Some("1.0.0".to_string()),
        labels: HashMap::from([
            ("security.non_modifiable".to_string(), "true".to_string()),
            ("security.fail_closed".to_string(), "true".to_string()),
        ]),
    });
    graph.policy = GraphPolicy {
        max_concurrent_nodes: 1,
        ..GraphPolicy::default()
    };

    let stages = [
        ("verify-deference", VERIFY_DEFERENCE_CELL_TYPE),
        ("verify-switch", VERIFY_SWITCH_CELL_TYPE),
        ("verify-truth", VERIFY_TRUTH_CELL_TYPE),
        ("verify-impact", VERIFY_IMPACT_CELL_TYPE),
        ("verify-task", VERIFY_TASK_CELL_TYPE),
    ];
    for (id, cell_type) in stages {
        graph.add_node(Node {
            id: id.to_string(),
            cell_type: cell_type.to_string(),
            config: toml::Value::Table(toml::map::Map::new()),
            inputs: vec!["state".to_string()],
            outputs: vec!["state".to_string(), "allowed".to_string()],
            execution_class: ExecutionClass::Workflow,
        })?;
    }
    for (from, to) in [
        ("verify-deference", "verify-switch"),
        ("verify-switch", "verify-truth"),
        ("verify-truth", "verify-impact"),
        ("verify-impact", "verify-task"),
    ] {
        graph.add_edge(Edge {
            from: from.to_string(),
            to: to.to_string(),
            condition: Some(EdgeCondition::OutputEquals {
                key: "allowed".to_string(),
                value: "true".to_string(),
            }),
        })?;
    }
    Ok(graph)
}

/// Register the five Verify Cell factories in an existing Graph registry.
///
/// Each cell receives a [`CellDescriptor`] with the corrigibility state schema
/// so edge validation can check type compatibility without constructing Cells.
pub fn register_corrigibility_cells(registry: &mut CellRegistry) {
    use crate::registry::CellDescriptor;

    let schema = state_schema();
    for cell_type in [
        VERIFY_DEFERENCE_CELL_TYPE,
        VERIFY_SWITCH_CELL_TYPE,
        VERIFY_TRUTH_CELL_TYPE,
        VERIFY_IMPACT_CELL_TYPE,
        VERIFY_TASK_CELL_TYPE,
    ] {
        let desc = CellDescriptor::new(
            cell_type,
            (1, 0, 0),
            Some(schema.clone()),
            Some(schema.clone()),
        );
        match cell_type {
            t if t == VERIFY_DEFERENCE_CELL_TYPE => {
                registry.register_with_descriptor(t, desc, |_| {
                    Box::new(VerifyDeferenceCell::standalone())
                });
            }
            t if t == VERIFY_SWITCH_CELL_TYPE => {
                registry.register_with_descriptor(t, desc, |_| {
                    Box::new(VerifySwitchCell::standalone())
                });
            }
            t if t == VERIFY_TRUTH_CELL_TYPE => {
                registry.register_with_descriptor(t, desc, |_| {
                    Box::new(VerifyTruthCell::standalone())
                });
            }
            t if t == VERIFY_IMPACT_CELL_TYPE => {
                registry.register_with_descriptor(t, desc, |_| {
                    Box::new(VerifyImpactCell::standalone())
                });
            }
            t if t == VERIFY_TASK_CELL_TYPE => {
                registry.register_with_descriptor(t, desc, |_| {
                    Box::new(VerifyTaskCell::standalone())
                });
            }
            _ => unreachable!(),
        }
    }
}

fn traced_registry(trace: DecisionTrace) -> CellRegistry {
    let mut registry = CellRegistry::new();
    let stage_trace = Arc::clone(&trace);
    registry.register(VERIFY_DEFERENCE_CELL_TYPE, move |_| {
        Box::new(VerifyDeferenceCell::traced(Arc::clone(&stage_trace)))
    });
    let stage_trace = Arc::clone(&trace);
    registry.register(VERIFY_SWITCH_CELL_TYPE, move |_| {
        Box::new(VerifySwitchCell::traced(Arc::clone(&stage_trace)))
    });
    let stage_trace = Arc::clone(&trace);
    registry.register(VERIFY_TRUTH_CELL_TYPE, move |_| {
        Box::new(VerifyTruthCell::traced(Arc::clone(&stage_trace)))
    });
    let stage_trace = Arc::clone(&trace);
    registry.register(VERIFY_IMPACT_CELL_TYPE, move |_| {
        Box::new(VerifyImpactCell::traced(Arc::clone(&stage_trace)))
    });
    registry.register(VERIFY_TASK_CELL_TYPE, move |_| {
        Box::new(VerifyTaskCell::traced(Arc::clone(&trace)))
    });
    registry
}

#[cfg(test)]
mod tests {
    use roko_core::corrigibility::{CorrigibilityHead, HeadVerdict};

    use crate::engine::NodeStatus;

    use super::*;

    #[test]
    fn default_registry_hosts_all_five_verify_cells() {
        let registry = crate::engine::default_registry();
        for cell_type in [
            VERIFY_DEFERENCE_CELL_TYPE,
            VERIFY_SWITCH_CELL_TYPE,
            VERIFY_TRUTH_CELL_TYPE,
            VERIFY_IMPACT_CELL_TYPE,
            VERIFY_TASK_CELL_TYPE,
        ] {
            assert!(registry.contains(cell_type), "missing {cell_type}");
        }
    }

    #[tokio::test]
    async fn all_five_verify_cells_execute_in_priority_order() {
        let output = CorrigibilityPipelineGraph
            .evaluate(
                "write a bounded patch",
                ActionContext {
                    autonomy_level: Some("assist".to_string()),
                    reversible: Some(true),
                    modifies_audit: Some(false),
                    outputs_verifiable: Some(true),
                    on_task: Some(true),
                },
            )
            .await
            .expect("fixed Graph should allow safe action");

        assert!(output.decision.is_allowed());
        assert_eq!(
            output
                .decision
                .verdicts
                .iter()
                .map(|(head, _)| *head)
                .collect::<Vec<_>>(),
            CorrigibilityHead::all_in_order()
        );
        assert!(
            output
                .graph
                .node_results
                .iter()
                .all(|result| result.status == NodeStatus::Complete)
        );
    }

    #[tokio::test]
    async fn first_veto_condition_skips_all_lower_priority_cells() {
        let output = CorrigibilityPipelineGraph
            .evaluate(
                "disable audit logging",
                ActionContext {
                    modifies_audit: Some(true),
                    outputs_verifiable: Some(false),
                    reversible: Some(false),
                    on_task: Some(false),
                    ..ActionContext::default()
                },
            )
            .await
            .expect("a policy veto is a successful safety decision");

        assert_eq!(
            output.decision.first_veto(),
            Some((
                CorrigibilityHead::Switch,
                "action modifies audit/logging infrastructure, reducing human oversight"
            ))
        );
        assert_eq!(output.decision.verdicts.len(), 2);
        assert!(matches!(
            output.decision.verdicts.last(),
            Some((CorrigibilityHead::Switch, HeadVerdict::Veto(_)))
        ));
        assert_eq!(output.graph.node_results[0].status, NodeStatus::Complete);
        assert_eq!(output.graph.node_results[1].status, NodeStatus::Complete);
        assert!(
            output.graph.node_results[2..]
                .iter()
                .all(|result| result.status == NodeStatus::ConditionSkipped)
        );
    }

    #[tokio::test]
    async fn reordered_verify_cell_fails_closed() {
        let state = CorrigibilityCellState::new("action", ActionContext::default());
        let input = Signal::builder(Kind::Custom("proposal".to_string()))
            .body(Body::from_json(&state).expect("serialize state"))
            .build();
        let error = VerifyTruthCell::standalone()
            .execute(vec![input], &CellContext::new())
            .await
            .expect_err("Truth cannot run before Deference and Switch");
        assert!(error.to_string().contains("order violation"));
    }

    #[tokio::test]
    async fn malformed_verify_input_fails_closed() {
        let input = Signal::builder(Kind::Task)
            .body(Body::text("not typed JSON"))
            .build();
        assert!(
            VerifyDeferenceCell::standalone()
                .execute(vec![input], &CellContext::new())
                .await
                .is_err()
        );
    }
}
