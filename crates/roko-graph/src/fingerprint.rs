//! Stable identity for execution-relevant Graph definitions.

use std::collections::BTreeMap;

use roko_core::ContentHash;
use serde::Serialize;
use serde_json::Value;

use crate::types::{ExecutionClass, Graph, GraphPolicy};

const FINGERPRINT_SCHEMA_VERSION: u32 = 1;

/// Domain tag hashed into every [`plan_graph_fingerprint`].
const PLAN_FINGERPRINT_SCHEMA: &str = "roko.plan-graph@1";

/// `[[task]]` fields roko rewrites in a plan file; they never change what a
/// task does.
const RUNTIME_TASK_FIELDS: &[&str] = &["status"];

/// `[meta]` fields that change how a plan executes: `max_parallel` bounds its
/// concurrency and `skip_enrichment` changes how every task is dispatched. The
/// other fields are counters roko rewrites (`total`, `done`, `status`) or
/// descriptive labels.
const EXECUTION_META_FIELDS: &[&str] = &["max_parallel", "skip_enrichment"];

/// Compute a stable BLAKE3 identity for the execution-relevant parts of a Graph.
///
/// Node and edge insertion order and metadata-label map order do not affect the
/// result. Checkpoint callers use this identity to reject replay after graph
/// definition or policy drift. Converted plans use [`plan_graph_fingerprint`]
/// instead, but checkpoints written before it existed recorded this value, so
/// [`legacy_graph_execution_fingerprint`] must keep reproducing it.
pub fn graph_execution_fingerprint(graph: &Graph) -> Result<String, serde_json::Error> {
    #[derive(Serialize)]
    struct Fingerprint<'a> {
        schema: u32,
        name: &'a str,
        description: &'a Option<String>,
        version: &'a Option<String>,
        labels: BTreeMap<&'a str, &'a str>,
        policy: &'a crate::types::GraphPolicy,
        nodes: Vec<&'a crate::types::Node>,
        edges: Vec<&'a crate::types::Edge>,
    }

    let mut nodes: Vec<_> = graph.inner.node_weights().collect();
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut edges: Vec<_> = graph.inner.edge_weights().collect();
    edges.sort_by(|left, right| {
        left.from
            .cmp(&right.from)
            .then_with(|| left.to.cmp(&right.to))
            .then_with(|| format!("{:?}", left.condition).cmp(&format!("{:?}", right.condition)))
    });
    let identity = Fingerprint {
        schema: FINGERPRINT_SCHEMA_VERSION,
        name: &graph.metadata.name,
        description: &graph.metadata.description,
        version: &graph.metadata.version,
        labels: graph
            .metadata
            .labels
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
            .collect(),
        policy: &graph.policy,
        nodes,
        edges,
    };
    let encoded = serde_json::to_vec(&identity)?;
    Ok(ContentHash::of(&encoded).to_hex())
}

/// The identity plan checkpoints recorded before [`plan_graph_fingerprint`].
///
/// It hashes the converted graph's node configs, which embed the running
/// binary's serialization of every task, so any release that adds a task field
/// changes it. Checkpoint hosts accept it only to migrate checkpoints written
/// by older binaries.
pub fn legacy_graph_execution_fingerprint(graph: &Graph) -> Result<String, serde_json::Error> {
    graph_execution_fingerprint(graph)
}

/// What an operator authored for one plan, read from its `tasks.toml`.
///
/// This is the input of [`plan_graph_fingerprint`]: the fields the plan file
/// states, not the defaults and derived values a roko release fills in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthoredPlan {
    /// `[meta]` fields that change execution, keyed by name.
    pub settings: BTreeMap<String, Value>,
    /// Each `[[task]]` table keyed by task id, without runtime-mutable fields.
    pub tasks: BTreeMap<String, Value>,
}

impl AuthoredPlan {
    /// Read the authored fields of a `tasks.toml` document.
    ///
    /// Task tables without a string `id` are skipped: such a plan does not
    /// load, so it never reaches a checkpoint.
    pub fn from_tasks_toml(content: &str) -> Result<Self, toml::de::Error> {
        let mut document: toml::Table = toml::from_str(content)?;
        let settings = match document.remove("meta") {
            Some(toml::Value::Table(meta)) => meta
                .into_iter()
                .filter(|(key, _)| EXECUTION_META_FIELDS.contains(&key.as_str()))
                .map(|(key, value)| (key, toml_to_json(value)))
                .collect(),
            _ => BTreeMap::new(),
        };
        let tasks = match document.remove("task") {
            Some(toml::Value::Array(tasks)) => tasks
                .into_iter()
                .filter_map(|task| {
                    let toml::Value::Table(mut task) = task else {
                        return None;
                    };
                    for field in RUNTIME_TASK_FIELDS {
                        task.remove(*field);
                    }
                    let id = task.get("id")?.as_str()?.to_string();
                    Some((id, toml_to_json(toml::Value::Table(task))))
                })
                .collect(),
            _ => BTreeMap::new(),
        };
        Ok(Self { settings, tasks })
    }
}

/// Compute a stable BLAKE3 identity for a converted plan graph.
///
/// A converted plan's node configs are generated by the running binary, so
/// hashing them (as [`graph_execution_fingerprint`] does) ties the identity to
/// the roko release. This hashes what the operator authored instead, together
/// with the structure converted from it: node ids, cell types and execution
/// classes, dependency edges, and the Graph policy fields that differ from
/// their defaults. Node configs, metadata labels and description, map key
/// order, and fields a later release adds with a default value do not affect
/// the result.
pub fn plan_graph_fingerprint(
    graph: &Graph,
    authored: &AuthoredPlan,
) -> Result<String, serde_json::Error> {
    #[derive(Serialize)]
    struct NodeIdentity<'a> {
        id: &'a str,
        cell_type: &'a str,
        execution_class: ExecutionClass,
    }

    #[derive(Serialize)]
    struct EdgeIdentity<'a> {
        from: &'a str,
        to: &'a str,
        condition: Value,
    }

    #[derive(Serialize)]
    struct Identity<'a> {
        schema: &'static str,
        name: &'a str,
        policy: Value,
        nodes: Vec<NodeIdentity<'a>>,
        edges: Vec<EdgeIdentity<'a>>,
        settings: &'a BTreeMap<String, Value>,
        tasks: &'a BTreeMap<String, Value>,
    }

    let mut nodes: Vec<_> = graph
        .inner
        .node_weights()
        .map(|node| NodeIdentity {
            id: &node.id,
            cell_type: &node.cell_type,
            execution_class: node.execution_class,
        })
        .collect();
    nodes.sort_by(|left, right| left.id.cmp(right.id));
    let mut edges = graph
        .inner
        .edge_weights()
        .map(|edge| {
            Ok(EdgeIdentity {
                from: &edge.from,
                to: &edge.to,
                condition: canonical_json(serde_json::to_value(&edge.condition)?),
            })
        })
        .collect::<Result<Vec<_>, serde_json::Error>>()?;
    edges.sort_by(|left, right| {
        left.from
            .cmp(right.from)
            .then_with(|| left.to.cmp(right.to))
            .then_with(|| left.condition.to_string().cmp(&right.condition.to_string()))
    });
    let identity = Identity {
        schema: PLAN_FINGERPRINT_SCHEMA,
        name: &graph.metadata.name,
        policy: non_default_policy(&graph.policy)?,
        nodes,
        edges,
        settings: &authored.settings,
        tasks: &authored.tasks,
    };
    let encoded = serde_json::to_vec(&identity)?;
    Ok(ContentHash::of(&encoded).to_hex())
}

/// The policy fields that differ from [`GraphPolicy::default`], so a policy
/// field added with a default value leaves existing identities unchanged.
fn non_default_policy(policy: &GraphPolicy) -> Result<Value, serde_json::Error> {
    let defaults = serde_json::to_value(GraphPolicy::default())?;
    Ok(match serde_json::to_value(policy)? {
        Value::Object(fields) => canonical_json(Value::Object(
            fields
                .into_iter()
                .filter(|(key, value)| defaults.get(key) != Some(value))
                .collect(),
        )),
        other => canonical_json(other),
    })
}

/// Rebuild `value` with every object's keys in sorted order, whatever map
/// ordering serde_json was compiled with.
fn canonical_json(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| (key, canonical_json(value)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(canonical_json).collect()),
        other => other,
    }
}

/// Convert an authored TOML value to canonical JSON.
fn toml_to_json(value: toml::Value) -> Value {
    match value {
        toml::Value::String(text) => Value::String(text),
        toml::Value::Integer(number) => Value::from(number),
        toml::Value::Float(number) => serde_json::Number::from_f64(number)
            .map_or_else(|| Value::String(number.to_string()), Value::Number),
        toml::Value::Boolean(flag) => Value::Bool(flag),
        toml::Value::Datetime(datetime) => Value::String(datetime.to_string()),
        toml::Value::Array(items) => Value::Array(items.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => canonical_json(Value::Object(
            table
                .into_iter()
                .map(|(key, value)| (key, toml_to_json(value)))
                .collect(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde::de::DeserializeOwned;

    use crate::convert::{PlanTaskInfo, plan_to_graph};
    use crate::types::{ExecutionClass, GraphMetadata, Node};

    use super::*;

    fn graph(config_value: i64) -> Graph {
        let mut graph = Graph::new(GraphMetadata {
            name: "stable".to_string(),
            labels: [
                ("z".to_string(), "last".to_string()),
                ("a".to_string(), "first".to_string()),
            ]
            .into_iter()
            .collect(),
            ..GraphMetadata::default()
        });
        graph
            .add_node(Node {
                id: "node".to_string(),
                cell_type: "noop".to_string(),
                config: toml::Value::Table(toml::map::Map::from_iter([(
                    "value".to_string(),
                    toml::Value::Integer(config_value),
                )])),
                inputs: Vec::new(),
                outputs: Vec::new(),
                execution_class: ExecutionClass::Workflow,
            })
            .expect("node");
        graph
    }

    #[test]
    fn fingerprint_is_stable_and_sensitive_to_execution_config() {
        let first = graph_execution_fingerprint(&graph(1)).expect("fingerprint");
        assert_eq!(
            first,
            graph_execution_fingerprint(&graph(1)).expect("fingerprint")
        );
        assert_ne!(
            first,
            graph_execution_fingerprint(&graph(2)).expect("fingerprint")
        );
    }

    const TASKS_TOML: &str = r#"
[meta]
plan = "stable"
max_parallel = 2
done = 0

[[task]]
id = "T1"
title = "Add the parser"
status = "ready"

[[task]]
id = "T2"
title = "Wire the parser"
depends_on = ["T1"]
"#;

    fn default_max_retries() -> u32 {
        3
    }

    /// A task schema as one release serializes it.
    #[derive(Serialize, Deserialize)]
    struct TaskV1 {
        id: String,
        title: String,
        #[serde(default)]
        status: String,
        #[serde(default)]
        depends_on: Vec<String>,
        #[serde(default = "default_max_retries")]
        max_retries: u32,
    }

    /// The next release: the same schema plus one defaulted field.
    #[derive(Serialize, Deserialize)]
    struct TaskV2 {
        id: String,
        title: String,
        #[serde(default)]
        status: String,
        #[serde(default)]
        depends_on: Vec<String>,
        #[serde(default = "default_max_retries")]
        max_retries: u32,
        #[serde(default)]
        review_required: bool,
    }

    /// Convert `content` the way the plan runner does, embedding each task's
    /// serialization through the schema `T`.
    fn converted<T: Serialize + DeserializeOwned>(content: &str, max_parallel: u32) -> Graph {
        #[derive(Deserialize)]
        struct TasksFile<T> {
            #[serde(rename = "task")]
            tasks: Vec<T>,
        }

        let file: TasksFile<T> = toml::from_str(content).expect("parse tasks");
        let tasks: Vec<(String, PlanTaskInfo)> = file
            .tasks
            .iter()
            .enumerate()
            .map(|(sequence, task)| {
                let json = serde_json::to_value(task).expect("serialize task");
                let depends_on = json["depends_on"]
                    .as_array()
                    .expect("depends_on")
                    .iter()
                    .map(|dep| dep.as_str().expect("dependency id").to_string())
                    .collect();
                let info = PlanTaskInfo {
                    title: json["title"].as_str().expect("title").to_string(),
                    description: None,
                    role: None,
                    tier: "focused".to_string(),
                    model_hint: None,
                    files: Vec::new(),
                    depends_on,
                    depends_on_plan: Vec::new(),
                    timeout_secs: 0,
                    max_retries: 3,
                    domain: None,
                    sequence,
                    full_config_json: json.clone(),
                };
                (json["id"].as_str().expect("id").to_string(), info)
            })
            .collect();
        plan_to_graph("stable", "/plans/stable", &tasks, max_parallel).expect("convert plan")
    }

    fn plan_fingerprint(content: &str, max_parallel: u32) -> String {
        let authored = AuthoredPlan::from_tasks_toml(content).expect("authored plan");
        plan_graph_fingerprint(&converted::<TaskV1>(content, max_parallel), &authored)
            .expect("fingerprint")
    }

    #[test]
    fn plan_fingerprint_survives_additive_task_schema_changes() {
        let before = converted::<TaskV1>(TASKS_TOML, 2);
        let after = converted::<TaskV2>(TASKS_TOML, 2);
        // The legacy identity hashes each release's task serialization, so a
        // new defaulted field changes it although the plan file did not.
        assert_ne!(
            legacy_graph_execution_fingerprint(&before).expect("fingerprint"),
            legacy_graph_execution_fingerprint(&after).expect("fingerprint")
        );

        let authored = AuthoredPlan::from_tasks_toml(TASKS_TOML).expect("authored plan");
        assert_eq!(
            plan_graph_fingerprint(&before, &authored).expect("fingerprint"),
            plan_graph_fingerprint(&after, &authored).expect("fingerprint")
        );
    }

    #[test]
    fn plan_fingerprint_ignores_runtime_fields_and_key_order() {
        let base = plan_fingerprint(TASKS_TOML, 2);
        let rewritten = TASKS_TOML
            .replace("status = \"ready\"", "status = \"done\"")
            .replace("done = 0", "done = 2\ntotal = 2\nstatus = \"complete\"");
        assert_eq!(base, plan_fingerprint(&rewritten, 2));
        let reordered = TASKS_TOML.replace(
            "id = \"T1\"\ntitle = \"Add the parser\"",
            "title = \"Add the parser\"\nid = \"T1\"",
        );
        assert_ne!(reordered, TASKS_TOML);
        assert_eq!(base, plan_fingerprint(&reordered, 2));
    }

    #[test]
    fn plan_fingerprint_tracks_authored_tasks_settings_and_policy() {
        let base = plan_fingerprint(TASKS_TOML, 2);
        for (changed, max_parallel) in [
            (TASKS_TOML.replace("Wire the parser", "Wire the lexer"), 2),
            (TASKS_TOML.replace("depends_on = [\"T1\"]\n", ""), 2),
            (
                TASKS_TOML.replace("[meta]", "[meta]\nskip_enrichment = true"),
                2,
            ),
            (TASKS_TOML.to_string(), 3),
        ] {
            assert_ne!(base, plan_fingerprint(&changed, max_parallel), "{changed}");
        }
    }
}
