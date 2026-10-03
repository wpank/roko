//! The schema rung (9124): JSON, JSONL and CSV artefacts must match a schema.
//!
//! A cleaned dataset or a JSON report can be malformed and still pass every
//! command rung its author wrote. [`check_schema`] checks each artefact a
//! `schema` rung names: a JSON or JSONL file against the rung's schema as a
//! JSON Schema, and a CSV file against it as a table schema. Its verdict
//! lists the first [`MAX_VIOLATIONS`] violations, each at its JSON pointer
//! or CSV row, as feedback for the next attempt.
//!
//! The JSON Schema check covers the draft 2020-12 vocabulary that data
//! contracts use: `type`, `enum`, `const`, `properties`, `required`,
//! `additionalProperties`, `items`, `minItems`, `maxItems`, `minLength`,
//! `maxLength`, `pattern`, `minimum`, `maximum`, `exclusiveMinimum`,
//! `exclusiveMaximum`, `minProperties`, `maxProperties`, `allOf`, `anyOf`,
//! `oneOf`, `not` and local `$ref`s, with annotations (`title`, `format`,
//! `$defs` and the like) ignored. A schema that uses any other keyword
//! leaves the rung skipped, never passed, so a constraint it cannot check
//! never counts as met. It adds no dependency.
//!
//! The table schema is a subset of Frictionless Table Schema: `fields`, each
//! with a `name`, a `type` among `string`, `integer`, `number`, `boolean` and
//! `date` (`YYYY-MM-DD`), and `constraints.required`; and a `primaryKey`, a
//! field name or a list of them, whose values must be present and unique.

use std::collections::HashMap;

use regex::Regex;
use roko_core::Verdict;
use serde_json::{Map, Value};

/// The most violations a verdict lists.
pub const MAX_VIOLATIONS: usize = 20;

/// How deep a schema may nest, or refer to itself, before the check stops.
const MAX_DEPTH: usize = 64;

/// Keywords that annotate a JSON Schema and constrain nothing.
const ANNOTATIONS: &[&str] = &[
    "$schema",
    "$id",
    "$anchor",
    "$comment",
    "$defs",
    "definitions",
    "title",
    "description",
    "default",
    "examples",
    "deprecated",
    "readOnly",
    "writeOnly",
    "format",
];

/// The schema rung `gate`'s verdict on `artefacts`, each a path and its
/// text, against the schema file `schema_path`, whose text is
/// `schema_text`. It fails, listing the violations, when an artefact does not
/// match; it is skipped when the schema uses a keyword the check does not
/// cover; it passes otherwise.
#[must_use]
pub fn check_schema(
    gate: &str,
    schema_path: &str,
    schema_text: &str,
    artefacts: &[(String, String)],
) -> Verdict {
    let schema: Value = match serde_json::from_str(schema_text) {
        Ok(schema) => schema,
        Err(error) => {
            return Verdict::fail(gate, format!("schema {schema_path} is not JSON: {error}"));
        }
    };
    let mut violations = Vec::new();
    for (path, text) in artefacts {
        let checked = match extension(path).as_str() {
            "json" => check_json_text(&schema, path, text, &mut violations),
            "jsonl" | "ndjson" => check_json_lines(&schema, path, text, &mut violations),
            "csv" => check_table(&schema, path, text, &mut violations),
            _ => {
                violations.push(format!("{path}: not a JSON, JSONL or CSV file"));
                Ok(())
            }
        };
        if let Err(unsupported) = checked {
            let reason = format!(
                "schema {schema_path} uses {unsupported}, which the schema rung does not check"
            );
            return Verdict::skip(gate, reason);
        }
    }
    if violations.is_empty() {
        let mut verdict = Verdict::pass(gate);
        verdict.reason = format!("{} artefacts match {schema_path}", artefacts.len());
        return verdict;
    }
    let shown: Vec<&str> = violations
        .iter()
        .take(MAX_VIOLATIONS)
        .map(String::as_str)
        .collect();
    let reason = format!(
        "{} violations of {schema_path}, the first: {}",
        violations.len(),
        shown[0]
    );
    Verdict::fail(gate, reason).with_detail(shown.join("\n"))
}

/// `path`'s extension, in lower case.
fn extension(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// Check a JSON file's text against `schema`.
fn check_json_text(
    schema: &Value,
    path: &str,
    text: &str,
    violations: &mut Vec<String>,
) -> Result<(), String> {
    match serde_json::from_str::<Value>(text) {
        Ok(instance) => {
            let mut found = Vec::new();
            JsonSchema { root: schema }.check(schema, &instance, "", 0, &mut found)?;
            violations.extend(found.into_iter().map(|found| format!("{path}#{found}")));
        }
        Err(error) => violations.push(format!("{path}: not JSON: {error}")),
    }
    Ok(())
}

/// Check each line of a JSONL file against `schema`.
fn check_json_lines(
    schema: &Value,
    path: &str,
    text: &str,
    violations: &mut Vec<String>,
) -> Result<(), String> {
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let number = index + 1;
        match serde_json::from_str::<Value>(line) {
            Ok(instance) => {
                let mut found = Vec::new();
                JsonSchema { root: schema }.check(schema, &instance, "", 0, &mut found)?;
                let at = |found: String| format!("{path}:{number}#{found}");
                violations.extend(found.into_iter().map(at));
            }
            Err(error) => violations.push(format!("{path}:{number}: not JSON: {error}")),
        }
    }
    Ok(())
}

/// One JSON Schema document, which local `$ref`s resolve against.
struct JsonSchema<'a> {
    root: &'a Value,
}

impl JsonSchema<'_> {
    /// Check `instance`, at JSON pointer `pointer`, against `schema`, adding
    /// each violation to `found` as `<pointer>: <what is wrong>`. `Err` names
    /// a keyword the check does not cover.
    fn check(
        &self,
        schema: &Value,
        instance: &Value,
        pointer: &str,
        depth: usize,
        found: &mut Vec<String>,
    ) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err("references that nest too deep".to_string());
        }
        let keywords = match schema {
            Value::Bool(true) => return Ok(()),
            Value::Bool(false) => {
                found.push(format!("{pointer}: no value is allowed here"));
                return Ok(());
            }
            Value::Object(keywords) => keywords,
            _ => return Err("a schema that is neither an object nor a boolean".to_string()),
        };
        for (keyword, value) in keywords {
            if ANNOTATIONS.contains(&keyword.as_str()) {
                continue;
            }
            match keyword.as_str() {
                "$ref" => {
                    let target = value
                        .as_str()
                        .and_then(|reference| reference.strip_prefix('#'))
                        .and_then(|fragment| self.root.pointer(fragment))
                        .ok_or_else(|| format!("the `$ref` {value}, outside the schema file"))?;
                    self.check(target, instance, pointer, depth + 1, found)?;
                }
                "type" => check_type(value, instance, pointer, found)?,
                "enum" => {
                    let options = value.as_array().ok_or("an `enum` that is not a list")?;
                    if !options.iter().any(|option| same_json(option, instance)) {
                        found.push(format!("{pointer}: {instance} is not one of {value}"));
                    }
                }
                "const" => {
                    if !same_json(value, instance) {
                        found.push(format!("{pointer}: {instance} is not {value}"));
                    }
                }
                "properties" | "additionalProperties" => {
                    if keyword == "properties" || !keywords.contains_key("properties") {
                        self.check_properties(keywords, instance, pointer, depth, found)?;
                    }
                }
                "required" => {
                    let names = value.as_array().ok_or("a `required` that is not a list")?;
                    if let Value::Object(members) = instance {
                        for name in names.iter().filter_map(Value::as_str) {
                            if !members.contains_key(name) {
                                found.push(format!("{pointer}: `{name}` is required"));
                            }
                        }
                    }
                }
                "items" => {
                    if value.is_array() {
                        return Err("`items` as a list".to_string());
                    }
                    if let Value::Array(items) = instance {
                        for (index, item) in items.iter().enumerate() {
                            let at = format!("{pointer}/{index}");
                            self.check(value, item, &at, depth + 1, found)?;
                        }
                    }
                }
                "minItems" | "maxItems" => {
                    if let Value::Array(items) = instance {
                        check_count(keyword, value, items.len(), "items", pointer, found)?;
                    }
                }
                "minLength" | "maxLength" => {
                    if let Value::String(text) = instance {
                        let length = text.chars().count();
                        check_count(keyword, value, length, "characters", pointer, found)?;
                    }
                }
                "minProperties" | "maxProperties" => {
                    if let Value::Object(members) = instance {
                        let count = members.len();
                        check_count(keyword, value, count, "properties", pointer, found)?;
                    }
                }
                "pattern" => {
                    let pattern = value.as_str().ok_or("a `pattern` that is not text")?;
                    let regex = Regex::new(pattern).map_err(|error| {
                        format!("the `pattern` {pattern}, which is not a regex: {error}")
                    })?;
                    if let Value::String(text) = instance
                        && !regex.is_match(text)
                    {
                        found.push(format!("{pointer}: \"{text}\" does not match /{pattern}/"));
                    }
                }
                "minimum" | "maximum" | "exclusiveMinimum" | "exclusiveMaximum" => {
                    check_bound(keyword, value, instance, pointer, found)?;
                }
                "allOf" | "anyOf" | "oneOf" => {
                    let schemas = value
                        .as_array()
                        .ok_or_else(|| format!("an `{keyword}` that is not a list"))?;
                    self.check_combination(keyword, schemas, instance, pointer, depth, found)?;
                }
                "not" => {
                    let mut inner = Vec::new();
                    self.check(value, instance, pointer, depth + 1, &mut inner)?;
                    if inner.is_empty() {
                        found.push(format!("{pointer}: matches the schema `not` excludes"));
                    }
                }
                other => return Err(format!("the keyword `{other}`")),
            }
        }
        Ok(())
    }

    /// Check an object `instance` against `properties` and
    /// `additionalProperties` in `keywords`.
    fn check_properties(
        &self,
        keywords: &Map<String, Value>,
        instance: &Value,
        pointer: &str,
        depth: usize,
        found: &mut Vec<String>,
    ) -> Result<(), String> {
        let Value::Object(members) = instance else {
            return Ok(());
        };
        let empty = Map::new();
        let properties = match keywords.get("properties") {
            None => &empty,
            Some(Value::Object(properties)) => properties,
            Some(_) => return Err("a `properties` that is not an object".to_string()),
        };
        for (name, member) in members {
            let at = format!("{pointer}/{}", name.replace('~', "~0").replace('/', "~1"));
            match (properties.get(name), keywords.get("additionalProperties")) {
                (Some(schema), _) | (None, Some(schema)) => {
                    self.check(schema, member, &at, depth + 1, found)?;
                }
                (None, None) => {}
            }
        }
        Ok(())
    }

    /// Check `instance` against `allOf`, `anyOf` or `oneOf` (`keyword`) of
    /// `schemas`.
    fn check_combination(
        &self,
        keyword: &str,
        schemas: &[Value],
        instance: &Value,
        pointer: &str,
        depth: usize,
        found: &mut Vec<String>,
    ) -> Result<(), String> {
        if keyword == "allOf" {
            for schema in schemas {
                self.check(schema, instance, pointer, depth + 1, found)?;
            }
            return Ok(());
        }
        let mut matching = 0;
        for schema in schemas {
            let mut inner = Vec::new();
            self.check(schema, instance, pointer, depth + 1, &mut inner)?;
            if inner.is_empty() {
                matching += 1;
            }
        }
        if keyword == "anyOf" && matching == 0 {
            found.push(format!("{pointer}: matches none of `anyOf`'s schemas"));
        } else if keyword == "oneOf" && matching != 1 {
            found.push(format!(
                "{pointer}: matches {matching} of `oneOf`'s schemas, not one"
            ));
        }
        Ok(())
    }
}

/// Whether two JSON values are the same, numbers by value (`1` is `1.0`).
fn same_json(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left.as_f64() == right.as_f64(),
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len() && left.iter().zip(right).all(|(l, r)| same_json(l, r))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .all(|(key, l)| right.get(key).is_some_and(|r| same_json(l, r)))
        }
        _ => left == right,
    }
}

/// Check `instance` against `type`, one type name or a list of them.
fn check_type(
    value: &Value,
    instance: &Value,
    pointer: &str,
    found: &mut Vec<String>,
) -> Result<(), String> {
    let names: Vec<&str> = match value {
        Value::String(name) => vec![name.as_str()],
        Value::Array(names) => names.iter().filter_map(Value::as_str).collect(),
        _ => return Err("a `type` that is neither a name nor a list".to_string()),
    };
    let mut matches = false;
    for name in &names {
        matches |= match *name {
            "null" => instance.is_null(),
            "boolean" => instance.is_boolean(),
            "object" => instance.is_object(),
            "array" => instance.is_array(),
            "string" => instance.is_string(),
            "number" => instance.is_number(),
            "integer" => instance
                .as_f64()
                .is_some_and(|number| number.fract() == 0.0),
            other => return Err(format!("the type `{other}`")),
        };
    }
    if !matches {
        found.push(format!("{pointer}: {instance} is not of type {value}"));
    }
    Ok(())
}

/// Check a count (items, characters, properties) against `minItems` and its
/// like (`keyword`), whose bound is `value`.
fn check_count(
    keyword: &str,
    value: &Value,
    count: usize,
    unit: &str,
    pointer: &str,
    found: &mut Vec<String>,
) -> Result<(), String> {
    let bound = value
        .as_u64()
        .ok_or_else(|| format!("a `{keyword}` that is not a whole number"))?;
    let count = count as u64;
    if keyword.starts_with("min") && count < bound {
        found.push(format!("{pointer}: {count} {unit}, fewer than {bound}"));
    } else if keyword.starts_with("max") && count > bound {
        found.push(format!("{pointer}: {count} {unit}, more than {bound}"));
    }
    Ok(())
}

/// Check a number `instance` against `minimum` and its like (`keyword`).
fn check_bound(
    keyword: &str,
    value: &Value,
    instance: &Value,
    pointer: &str,
    found: &mut Vec<String>,
) -> Result<(), String> {
    let bound = value
        .as_f64()
        .ok_or_else(|| format!("a `{keyword}` that is not a number"))?;
    let Some(number) = instance.as_f64() else {
        return Ok(());
    };
    let (holds, relation) = match keyword {
        "minimum" => (number >= bound, "at least"),
        "maximum" => (number <= bound, "at most"),
        "exclusiveMinimum" => (number > bound, "above"),
        _ => (number < bound, "below"),
    };
    if !holds {
        found.push(format!("{pointer}: {instance} is not {relation} {bound}"));
    }
    Ok(())
}

/// One column of a table schema.
struct TableField {
    name: String,
    kind: String,
    required: bool,
}

/// Check a CSV file's text against `schema` as a table schema.
fn check_table(
    schema: &Value,
    path: &str,
    text: &str,
    violations: &mut Vec<String>,
) -> Result<(), String> {
    let fields: Vec<TableField> = schema
        .get("fields")
        .and_then(Value::as_array)
        .ok_or("a table schema without `fields`")?
        .iter()
        .map(|field| TableField {
            name: field["name"].as_str().unwrap_or_default().to_string(),
            kind: field["type"].as_str().unwrap_or("string").to_string(),
            required: field["constraints"]["required"].as_bool().unwrap_or(false),
        })
        .collect();
    if let Some(field) = fields.iter().find(|field| !valid_cell(&field.kind, None)) {
        return Err(format!("the field type `{}`", field.kind));
    }
    let primary_key: Vec<&str> = match &schema["primaryKey"] {
        Value::String(name) => vec![name.as_str()],
        Value::Array(names) => names.iter().filter_map(Value::as_str).collect(),
        _ => Vec::new(),
    };

    let records = parse_csv(text);
    let Some((header, rows)) = records.split_first() else {
        violations.push(format!("{path}: no header row"));
        return Ok(());
    };
    let column = |name: &str| header.iter().position(|cell| cell.trim() == name);
    for field in fields.iter().filter(|field| field.required) {
        if column(&field.name).is_none() {
            violations.push(format!("{path}: column `{}` is missing", field.name));
        }
    }
    let mut keys: HashMap<Vec<String>, usize> = HashMap::new();
    for (index, row) in rows.iter().enumerate() {
        // The header is row 1.
        let number = index + 2;
        let cell = |name: &str| {
            column(name)
                .and_then(|at| row.get(at))
                .map_or("", |cell| cell.trim())
        };
        for field in &fields {
            let value = cell(&field.name);
            if value.is_empty() {
                if field.required && column(&field.name).is_some() {
                    violations.push(format!("{path}: row {number}, `{}` is empty", field.name));
                }
            } else if !valid_cell(&field.kind, Some(value)) {
                let kind = &field.kind;
                violations.push(format!(
                    "{path}: row {number}, `{}` is \"{value}\", not of type {kind}",
                    field.name
                ));
            }
        }
        if primary_key.is_empty() {
            continue;
        }
        let key: Vec<String> = primary_key
            .iter()
            .map(|name| cell(name).to_string())
            .collect();
        if key.iter().any(String::is_empty) {
            violations.push(format!(
                "{path}: row {number}, the primary key is incomplete"
            ));
        } else if let Some(first) = keys.insert(key, number) {
            violations.push(format!(
                "{path}: row {number} repeats row {first}'s primary key"
            ));
        }
    }
    Ok(())
}

/// Whether `value` is a valid cell of type `kind`; with no value, whether
/// `kind` is a type the check knows.
fn valid_cell(kind: &str, value: Option<&str>) -> bool {
    let Some(value) = value else {
        return matches!(kind, "string" | "integer" | "number" | "boolean" | "date");
    };
    match kind {
        "integer" => value.parse::<i64>().is_ok(),
        "number" => value.parse::<f64>().is_ok_and(f64::is_finite),
        "boolean" => matches!(
            value,
            "true" | "True" | "TRUE" | "1" | "false" | "False" | "FALSE" | "0"
        ),
        "date" => chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok(),
        _ => true,
    }
}

/// The records of CSV `text`: comma-separated fields, which double quotes
/// may enclose, a doubled quote standing for one. Blank lines are skipped.
fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    field.push('"');
                    chars.next();
                }
                '"' => quoted = false,
                _ => field.push(c),
            }
            continue;
        }
        match c {
            '"' if field.is_empty() => quoted = true,
            ',' => record.push(std::mem::take(&mut field)),
            '\r' => {}
            '\n' => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records.retain(|record| !(record.len() == 1 && record[0].trim().is_empty()));
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPORT_SCHEMA: &str = r#"{
        "type": "object",
        "required": ["title", "score"],
        "properties": {
            "title": {"type": "string", "minLength": 1},
            "score": {"type": "number", "minimum": 0, "maximum": 1},
            "tags": {"type": "array", "items": {"type": "string"}}
        },
        "additionalProperties": false
    }"#;

    const TABLE_SCHEMA: &str = r#"{
        "fields": [
            {"name": "id", "type": "integer", "constraints": {"required": true}},
            {"name": "day", "type": "date", "constraints": {"required": true}},
            {"name": "note", "type": "string"}
        ],
        "primaryKey": "id"
    }"#;

    fn artefact(path: &str, text: &str) -> Vec<(String, String)> {
        vec![(path.to_string(), text.to_string())]
    }

    /// Assert that `verdict` failed and lists each of `expected`.
    fn assert_lists(verdict: &Verdict, expected: &[&str]) {
        assert!(!verdict.passed && !verdict.skipped, "{verdict:?}");
        let detail = verdict.detail.clone().unwrap_or_default();
        for violation in expected {
            assert!(detail.contains(violation), "{violation}:\n{detail}");
        }
    }

    /// 9124: an invalid JSON artefact fails with the JSON pointer of each
    /// violation; a valid one passes; a CSV missing a required column, or
    /// with a bad cell or a repeated primary key, fails at its row; and a
    /// schema keyword the check does not cover leaves the rung skipped.
    #[test]
    fn schema_rung_fails_invalid_json_artefact() {
        let invalid = artefact("report.json", r#"{"score": 1.5, "tags": ["a", 2], "x": 1}"#);
        let verdict = check_schema("rung[shape]", "report.schema", REPORT_SCHEMA, &invalid);
        assert_lists(
            &verdict,
            &[
                "report.json#: `title` is required",
                "report.json#/score: 1.5 is not at most 1",
                "report.json#/tags/1: 2 is not of type \"string\"",
                "report.json#/x: no value is allowed here",
            ],
        );

        let valid = artefact("report.json", r#"{"title": "T", "score": 0.5}"#);
        let verdict = check_schema("rung[shape]", "report.schema", REPORT_SCHEMA, &valid);
        assert!(verdict.passed, "{verdict:?}");

        let csv = artefact("data.csv", "id,note\n1,first\n1,again\n");
        let verdict = check_schema("rung[table]", "table.schema", TABLE_SCHEMA, &csv);
        assert_lists(
            &verdict,
            &[
                "data.csv: column `day` is missing",
                "data.csv: row 3 repeats row 2's primary key",
            ],
        );

        let csv = artefact("data.csv", "id,day\n1,2026-10-03\nx,yesterday\n");
        let verdict = check_schema("rung[table]", "table.schema", TABLE_SCHEMA, &csv);
        assert_lists(
            &verdict,
            &[
                "data.csv: row 3, `id` is \"x\", not of type integer",
                "data.csv: row 3, `day` is \"yesterday\", not of type date",
            ],
        );

        let unsupported = r#"{"type": "object", "dependentRequired": {"a": ["b"]}}"#;
        let verdict = check_schema("rung[shape]", "other.schema", unsupported, &valid);
        assert!(verdict.skipped && !verdict.passed, "{verdict:?}");
    }
}
