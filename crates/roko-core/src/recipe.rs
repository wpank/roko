//! Pure-data scoring recipes represented as validated DAGs.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::{Result, RokoError};

/// A named graph of deterministic scoring operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    /// Stable filesystem-safe identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Monotonic persisted version.
    pub version: u32,
    /// Scoring nodes.
    #[serde(default)]
    pub nodes: Vec<RecipeNode>,
    /// Directed data-flow edges.
    #[serde(default)]
    pub edges: Vec<RecipeEdge>,
    /// Required input feed identifiers.
    #[serde(default)]
    pub input_feeds: Vec<String>,
    /// Optional JSON schema for the final result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
}

/// One scoring operation in a recipe.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeNode {
    /// Unique node identifier.
    pub id: String,
    /// Pure operation performed by this node.
    pub operation: ScoreOp,
    /// Numeric operation parameters.
    #[serde(default)]
    pub params: HashMap<String, f64>,
}

/// Directed value routing between an input/node and another node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeEdge {
    /// Input feed or node identifier.
    pub from: String,
    /// Destination node identifier.
    pub to: String,
    /// Optional object field extracted from the source value.
    #[serde(default)]
    pub field: String,
}

/// Built-in pure score transformations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreOp {
    /// Weighted mean; parameter names may identify source ids and values are weights.
    WeightedAverage,
    /// Normalize using `min` and `max` parameters.
    Normalize,
    /// Emit one or zero using a `threshold` parameter.
    Threshold,
    /// Standardize using `mean` and `stddev` parameters.
    ZScore,
    /// Clamp using `min` and `max` parameters.
    Clamp,
    /// Linear mapping using `from_min`, `from_max`, `to_min`, `to_max`.
    Rescale,
    /// Reserved extension name; the pure evaluator rejects unknown implementations.
    Custom(String),
}

impl Recipe {
    /// Validate identifiers, edge references, finite parameters, and acyclicity.
    #[must_use]
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.id.is_empty() {
            errors.push("recipe id must not be empty".to_string());
        }
        let ids = self
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<HashSet<_>>();
        if ids.len() != self.nodes.len() {
            errors.push("recipe node ids must be unique".to_string());
        }
        let inputs = self
            .input_feeds
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        if inputs.len() != self.input_feeds.len() {
            errors.push("input feed ids must be unique".to_string());
        }
        for node in &self.nodes {
            if node.params.values().any(|value| !value.is_finite()) {
                errors.push(format!("node {} has a non-finite parameter", node.id));
            }
        }
        for edge in &self.edges {
            if !ids.contains(edge.to.as_str()) {
                errors.push(format!("edge destination does not exist: {}", edge.to));
            }
            if !ids.contains(edge.from.as_str()) && !inputs.contains(edge.from.as_str()) {
                errors.push(format!("edge source does not exist: {}", edge.from));
            }
        }
        if self.topological_order().is_err() {
            errors.push("recipe graph contains a cycle".to_string());
        }
        errors
    }

    /// Evaluate the graph synchronously and return its single sink value (or
    /// an object keyed by sink id when the recipe has multiple sinks).
    pub fn evaluate(&self, inputs: &HashMap<String, Value>) -> Result<Value> {
        let validation = self.validate();
        if !validation.is_empty() {
            return Err(RokoError::invalid(validation.join("; ")));
        }
        for input in &self.input_feeds {
            if !inputs.contains_key(input) {
                return Err(RokoError::invalid(format!("missing recipe input: {input}")));
            }
        }
        let order = self.topological_order()?;
        let nodes = self
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect::<HashMap<_, _>>();
        let mut values = inputs.clone();
        for node_id in order {
            let node = nodes[node_id.as_str()];
            let incoming = self
                .edges
                .iter()
                .filter(|edge| edge.to == node.id)
                .map(|edge| {
                    let value = values.get(&edge.from).ok_or_else(|| {
                        RokoError::invalid(format!("missing edge value: {}", edge.from))
                    })?;
                    let selected = if edge.field.is_empty() {
                        value
                    } else {
                        value.get(&edge.field).ok_or_else(|| {
                            RokoError::invalid(format!(
                                "missing field {} on {}",
                                edge.field, edge.from
                            ))
                        })?
                    };
                    Ok((edge.from.as_str(), numeric(selected)?))
                })
                .collect::<Result<Vec<_>>>()?;
            let output = evaluate_node(node, &incoming)?;
            values.insert(node.id.clone(), json!(output));
        }
        let sinks = self
            .nodes
            .iter()
            .filter(|node| !self.edges.iter().any(|edge| edge.from == node.id))
            .collect::<Vec<_>>();
        match sinks.as_slice() {
            [] => Err(RokoError::invalid("recipe has no output node")),
            [sink] => Ok(values[&sink.id].clone()),
            _ => Ok(Value::Object(
                sinks
                    .iter()
                    .map(|sink| (sink.id.clone(), values[&sink.id].clone()))
                    .collect(),
            )),
        }
    }

    fn topological_order(&self) -> Result<Vec<String>> {
        let ids = self
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<HashSet<_>>();
        let mut incoming = ids
            .iter()
            .map(|id| (id.clone(), 0_usize))
            .collect::<HashMap<_, _>>();
        for edge in &self.edges {
            if ids.contains(&edge.from) && ids.contains(&edge.to) {
                *incoming.entry(edge.to.clone()).or_default() += 1;
            }
        }
        let mut ready = incoming
            .iter()
            .filter(|(_, count)| **count == 0)
            .map(|(id, _)| id.clone())
            .collect::<VecDeque<_>>();
        let mut order = Vec::with_capacity(ids.len());
        while let Some(id) = ready.pop_front() {
            order.push(id.clone());
            for edge in self
                .edges
                .iter()
                .filter(|edge| edge.from == id && ids.contains(&edge.to))
            {
                let count = incoming.get_mut(&edge.to).expect("validated node id");
                *count -= 1;
                if *count == 0 {
                    ready.push_back(edge.to.clone());
                }
            }
        }
        if order.len() == ids.len() {
            Ok(order)
        } else {
            Err(RokoError::invalid("recipe graph contains a cycle"))
        }
    }
}

fn evaluate_node(node: &RecipeNode, incoming: &[(&str, f64)]) -> Result<f64> {
    let first = || {
        incoming
            .first()
            .map(|(_, value)| *value)
            .ok_or_else(|| RokoError::invalid(format!("node {} has no input", node.id)))
    };
    let result = match &node.operation {
        ScoreOp::WeightedAverage => {
            if incoming.is_empty() {
                return Err(RokoError::invalid(format!("node {} has no input", node.id)));
            }
            let weighted = incoming
                .iter()
                .map(|(id, value)| (*value, node.params.get(*id).copied().unwrap_or(1.0)))
                .collect::<Vec<_>>();
            let weight_sum = weighted.iter().map(|(_, weight)| weight).sum::<f64>();
            if weight_sum == 0.0 {
                return Err(RokoError::invalid("weighted average has zero total weight"));
            }
            weighted
                .iter()
                .map(|(value, weight)| value * weight)
                .sum::<f64>()
                / weight_sum
        }
        ScoreOp::Normalize => scale(
            first()?,
            param(node, "min", 0.0),
            param(node, "max", 1.0),
            0.0,
            1.0,
        )?,
        ScoreOp::Threshold => {
            if first()? >= param(node, "threshold", 0.5) {
                1.0
            } else {
                0.0
            }
        }
        ScoreOp::ZScore => {
            let stddev = param(node, "stddev", 1.0);
            if stddev == 0.0 {
                return Err(RokoError::invalid("z-score stddev must be non-zero"));
            }
            (first()? - param(node, "mean", 0.0)) / stddev
        }
        ScoreOp::Clamp => {
            let minimum = param(node, "min", 0.0);
            let maximum = param(node, "max", 1.0);
            if minimum > maximum {
                return Err(RokoError::invalid("clamp minimum must not exceed maximum"));
            }
            first()?.clamp(minimum, maximum)
        }
        ScoreOp::Rescale => scale(
            first()?,
            param(node, "from_min", 0.0),
            param(node, "from_max", 1.0),
            param(node, "to_min", 0.0),
            param(node, "to_max", 1.0),
        )?,
        ScoreOp::Custom(name) => {
            return Err(RokoError::invalid(format!(
                "custom score operation is not registered: {name}"
            )));
        }
    };
    if result.is_finite() {
        Ok(result)
    } else {
        Err(RokoError::invalid(format!(
            "node {} produced a non-finite result",
            node.id
        )))
    }
}

fn param(node: &RecipeNode, key: &str, default: f64) -> f64 {
    node.params.get(key).copied().unwrap_or(default)
}
fn numeric(value: &Value) -> Result<f64> {
    value
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| RokoError::invalid("recipe input must be a finite number"))
}
fn scale(value: f64, from_min: f64, from_max: f64, to_min: f64, to_max: f64) -> Result<f64> {
    if from_max == from_min {
        return Err(RokoError::invalid("scale range must be non-zero"));
    }
    Ok(to_min + (value - from_min) * (to_max - to_min) / (from_max - from_min))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weighted_recipe() -> Recipe {
        Recipe {
            id: "blend".into(),
            name: "Blend".into(),
            version: 1,
            nodes: vec![RecipeNode {
                id: "score".into(),
                operation: ScoreOp::WeightedAverage,
                params: HashMap::from([("left".into(), 1.0), ("right".into(), 3.0)]),
            }],
            edges: vec![
                RecipeEdge {
                    from: "left".into(),
                    to: "score".into(),
                    field: String::new(),
                },
                RecipeEdge {
                    from: "right".into(),
                    to: "score".into(),
                    field: String::new(),
                },
            ],
            input_feeds: vec!["left".into(), "right".into()],
            output_schema: None,
        }
    }

    #[test]
    fn evaluates_weighted_average() {
        let value = weighted_recipe()
            .evaluate(&HashMap::from([
                ("left".into(), json!(2)),
                ("right".into(), json!(6)),
            ]))
            .unwrap();
        assert_eq!(value, json!(5.0));
    }

    #[test]
    fn validation_catches_cycle() {
        let mut recipe = weighted_recipe();
        recipe.nodes.push(RecipeNode {
            id: "other".into(),
            operation: ScoreOp::Clamp,
            params: HashMap::new(),
        });
        recipe.edges.push(RecipeEdge {
            from: "score".into(),
            to: "other".into(),
            field: String::new(),
        });
        recipe.edges.push(RecipeEdge {
            from: "other".into(),
            to: "score".into(),
            field: String::new(),
        });
        assert!(
            recipe
                .validate()
                .iter()
                .any(|error| error.contains("cycle"))
        );
    }
}
