//! `roko recipe` -- inspect, validate, and execute persisted score recipes.

use std::collections::HashMap;

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use roko_core::RecipeStore;
use serde_json::Value;

use crate::*;

/// Recipe persistence and evaluation subcommands.
#[derive(Debug, Subcommand)]
pub enum RecipeCmd {
    /// List persisted recipe ids.
    List,
    /// Show a recipe definition.
    Show { id: String },
    /// Validate a recipe DAG.
    Validate { id: String },
    /// Evaluate a recipe with literal `key=value` inputs.
    Run {
        id: String,
        #[arg(long = "input", value_name = "KEY=VALUE")]
        inputs: Vec<String>,
    },
}

pub(crate) fn cmd_recipe(cli: &Cli, command: RecipeCmd) -> Result<i32> {
    let root = resolve_workdir(cli).join(".roko").join("recipes");
    let store = RecipeStore::new(root);
    match command {
        RecipeCmd::List => {
            let recipes = store.list()?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&recipes)?);
            } else if recipes.is_empty() {
                println!("(no recipes)");
            } else {
                for id in recipes {
                    println!("{id}");
                }
            }
        }
        RecipeCmd::Show { id } => {
            let recipe = store
                .load(&id)
                .with_context(|| format!("load recipe '{id}'"))?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&recipe)?);
            } else {
                println!("{}", toml::to_string_pretty(&recipe)?);
            }
        }
        RecipeCmd::Validate { id } => {
            let recipe = store
                .load(&id)
                .with_context(|| format!("load recipe '{id}'"))?;
            let errors = recipe.validate();
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({"id": id, "valid": errors.is_empty(), "errors": errors})
                );
            } else if errors.is_empty() {
                println!("recipe '{id}' is valid");
            } else {
                for error in &errors {
                    eprintln!("- {error}");
                }
            }
            if !errors.is_empty() {
                return Ok(EXIT_FAILURE);
            }
        }
        RecipeCmd::Run { id, inputs } => {
            let recipe = store
                .load(&id)
                .with_context(|| format!("load recipe '{id}'"))?;
            let inputs = parse_inputs(&inputs)?;
            let output = recipe.evaluate(&inputs)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("{output}");
            }
        }
    }
    Ok(EXIT_SUCCESS)
}

fn parse_inputs(entries: &[String]) -> Result<HashMap<String, Value>> {
    entries
        .iter()
        .map(|entry| {
            let Some((key, raw)) = entry.split_once('=') else {
                bail!("recipe input must be KEY=VALUE: {entry}")
            };
            if key.is_empty() {
                bail!("recipe input key must not be empty");
            }
            let value =
                serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()));
            Ok((key.to_string(), value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_literals_and_strings() {
        let values = parse_inputs(&["score=0.75".into(), "label=good".into()]).unwrap();
        assert_eq!(values["score"], serde_json::json!(0.75));
        assert_eq!(values["label"], serde_json::json!("good"));
    }
}
