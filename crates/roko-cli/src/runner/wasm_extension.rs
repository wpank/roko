//! Fuel-metered Tier-4 extension runtime.
//!
//! Modules are instantiated without WASI or any other imports. This leaves
//! them with no ambient filesystem, network, environment, or credential
//! authority. Hook exports use the `roko.extension.v1` JSON ABI:
//!
//! - manifest `config.hooks` explicitly lists every active hook;
//! - the module exports `memory` and `roko_alloc(i32) -> i32`;
//! - each declared hook has `(input_ptr: i32, input_len: i32) -> i64`;
//! - the return packs output pointer in the high 32 bits and length in the low;
//! - input is `{ "abi_version": 1, "hook": "...", "value": ... }` JSON;
//! - output is the hook's typed JSON value, bounded to 1 MiB.

use std::collections::HashSet;
use std::path::{Component, Path};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use roko_core::extension::{
    ActionDecision, Adjustment, AgentMessage, BudgetAction, CostUpdate, ErrorEvent, Extension,
    ExtensionLayer, ExtensionMeta, FilterDecision, GateEvent, InferenceRequest, InferenceResponse,
    Observation, RecoveryAction, ReflectionState, RetrievalResult, StoreEntry, ToolCallEvent,
    ToolDecision,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use wasmi::{Config, Engine, Instance, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

const DEFAULT_FUEL: u64 = 10_000_000;
const MAX_FUEL: u64 = 10_000_000;
const DEFAULT_MEMORY_MB: u64 = 256;
const MAX_MEMORY_MB: u64 = 256;
const MAX_MODULE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_HOOK_PAYLOAD_BYTES: usize = 1024 * 1024;

const HOOK_NAMES: [&str; 22] = [
    "on_init",
    "on_shutdown",
    "on_observe",
    "on_filter",
    "filter_input",
    "on_retrieve",
    "on_store",
    "pre_inference",
    "post_inference",
    "on_gate",
    "pre_action",
    "post_action",
    "on_tool_call",
    "on_message_send",
    "on_message_receive",
    "on_reflect",
    "on_cost_update",
    "on_error",
    "on_budget_exceeded",
    "on_tick_start",
    "on_tick_end",
    "on_slot_assigned",
];

// `on_slot_completed` makes 23 methods in the current Rust trait. The v2
// prose still calls it a 22-hook trait, so runtime validation follows code.
const EXTRA_HOOK_NAME: &str = "on_slot_completed";

type HookError = Box<dyn std::error::Error + Send + Sync>;

struct WasmRuntime {
    store: Store<StoreLimits>,
    instance: Instance,
    fuel: u64,
}

impl WasmRuntime {
    fn invoke_json(&mut self, export: &str, value: Value) -> Result<Value, String> {
        let function = self
            .instance
            .get_func(&self.store, export)
            .ok_or_else(|| format!("declared WASM hook `{export}` has no export"))?;
        self.store
            .set_fuel(self.fuel)
            .map_err(|error| format!("reset fuel for `{export}`: {error}"))?;

        let input = serde_json::to_vec(&json!({
            "abi_version": 1,
            "hook": export,
            "value": value,
        }))
        .map_err(|error| format!("serialize WASM hook `{export}` input: {error}"))?;
        if input.len() > MAX_HOOK_PAYLOAD_BYTES {
            return Err(format!(
                "WASM hook `{export}` input exceeds {MAX_HOOK_PAYLOAD_BYTES} bytes"
            ));
        }
        let input_len = i32::try_from(input.len())
            .map_err(|_| format!("WASM hook `{export}` input length overflow"))?;
        let memory = self
            .instance
            .get_memory(&self.store, "memory")
            .ok_or_else(|| "WASM JSON ABI requires exported `memory`".to_string())?;
        let alloc = self
            .instance
            .get_func(&self.store, "roko_alloc")
            .ok_or_else(|| "WASM JSON ABI requires exported `roko_alloc`".to_string())?
            .typed::<i32, i32>(&self.store)
            .map_err(|error| format!("WASM roko_alloc must be (i32) -> i32: {error}"))?;
        let input_ptr = alloc
            .call(&mut self.store, input_len)
            .map_err(|error| format!("WASM roko_alloc trapped for `{export}`: {error}"))?;
        let input_offset = usize::try_from(input_ptr)
            .map_err(|_| format!("WASM roko_alloc returned negative pointer for `{export}`"))?;
        memory
            .write(&mut self.store, input_offset, &input)
            .map_err(|error| format!("write WASM hook `{export}` input: {error}"))?;

        let function = function
            .typed::<(i32, i32), i64>(&self.store)
            .map_err(|error| format!("WASM hook `{export}` must be (i32, i32) -> i64: {error}"))?;
        let packed = function
            .call(&mut self.store, (input_ptr, input_len))
            .map_err(|error| format!("WASM hook `{export}` trapped: {error}"))?;
        let packed = packed as u64;
        let output_offset = usize::try_from(packed >> 32)
            .map_err(|_| format!("WASM hook `{export}` output pointer overflow"))?;
        let output_len = usize::try_from(packed & u64::from(u32::MAX))
            .map_err(|_| format!("WASM hook `{export}` output length overflow"))?;
        if output_len == 0 || output_len > MAX_HOOK_PAYLOAD_BYTES {
            return Err(format!(
                "WASM hook `{export}` output length must be within 1..={MAX_HOOK_PAYLOAD_BYTES}, got {output_len}"
            ));
        }
        let mut output = vec![0u8; output_len];
        memory
            .read(&self.store, output_offset, &mut output)
            .map_err(|error| format!("read WASM hook `{export}` output: {error}"))?;
        let output: Value = serde_json::from_slice(&output)
            .map_err(|error| format!("decode WASM hook `{export}` output JSON: {error}"))?;
        if let Some(error) = output.get("error").and_then(Value::as_str) {
            return Err(format!("WASM hook `{export}` returned error: {error}"));
        }
        Ok(output)
    }
}

/// Sandboxed extension backed by a WebAssembly module.
pub(super) struct WasmExtension {
    meta: ExtensionMeta,
    runtime: Arc<Mutex<WasmRuntime>>,
    timeout: Duration,
    declared_hooks: HashSet<String>,
}

impl WasmExtension {
    pub(super) fn load(
        meta: ExtensionMeta,
        manifest_path: &Path,
        config: &serde_json::Value,
        timeout: Duration,
    ) -> Result<Self, HookError> {
        let module = config
            .get("module")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                config
                    .pointer("/entrypoint/module")
                    .and_then(serde_json::Value::as_str)
            })
            .ok_or_else(|| "WASM extension config requires a string `module` path".to_string())?;
        let module_path = confined_module_path(manifest_path, module)?;
        let metadata = std::fs::metadata(&module_path)?;
        if metadata.len() > MAX_MODULE_BYTES {
            return Err(format!(
                "WASM module {} is {} bytes; maximum is {MAX_MODULE_BYTES}",
                module_path.display(),
                metadata.len()
            )
            .into());
        }

        let fuel = config
            .get("fuel")
            .and_then(serde_json::Value::as_u64)
            .or_else(|| {
                config
                    .pointer("/permissions/fuel")
                    .and_then(serde_json::Value::as_u64)
            })
            .unwrap_or(DEFAULT_FUEL);
        if fuel == 0 || fuel > MAX_FUEL {
            return Err(format!("WASM fuel must be within 1..={MAX_FUEL}, got {fuel}").into());
        }
        let memory_mb = config
            .get("memory_mb")
            .and_then(serde_json::Value::as_u64)
            .or_else(|| {
                config
                    .pointer("/permissions/memory_mb")
                    .and_then(serde_json::Value::as_u64)
            })
            .unwrap_or(DEFAULT_MEMORY_MB);
        if memory_mb == 0 || memory_mb > MAX_MEMORY_MB {
            return Err(format!(
                "WASM memory_mb must be within 1..={MAX_MEMORY_MB}, got {memory_mb}"
            )
            .into());
        }
        let memory_bytes = usize::try_from(memory_mb.saturating_mul(1024 * 1024))
            .map_err(|_| "WASM memory limit does not fit this platform")?;

        let mut engine_config = Config::default();
        engine_config.consume_fuel(true);
        let engine = Engine::new(&engine_config);
        let bytes = std::fs::read(&module_path)?;
        let module = Module::new(&engine, bytes.as_slice())?;
        let limits = StoreLimitsBuilder::new()
            .memory_size(memory_bytes)
            .memories(1)
            .tables(1)
            .instances(1)
            .trap_on_grow_failure(true)
            .build();
        let mut store = Store::new(&engine, limits);
        store.limiter(|limits| limits);
        store.set_fuel(fuel)?;

        // Intentionally empty: no WASI and no host imports means no ambient
        // filesystem, network, environment, clock, or credential authority.
        let linker = Linker::<StoreLimits>::new(&engine);
        let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;
        let declared_hooks = declared_hooks(config)?;
        instance
            .get_memory(&store, "memory")
            .ok_or("WASM JSON ABI requires exported `memory`")?;
        instance
            .get_func(&store, "roko_alloc")
            .ok_or("WASM JSON ABI requires exported `roko_alloc`")?
            .typed::<i32, i32>(&store)
            .map_err(|error| format!("WASM roko_alloc must be (i32) -> i32: {error}"))?;
        for hook in &declared_hooks {
            instance
                .get_func(&store, hook)
                .ok_or_else(|| {
                    format!("manifest declares WASM hook `{hook}` but export is absent")
                })?
                .typed::<(i32, i32), i64>(&store)
                .map_err(|error| {
                    format!("WASM hook `{hook}` must be (i32, i32) -> i64: {error}")
                })?;
        }

        Ok(Self {
            meta,
            runtime: Arc::new(Mutex::new(WasmRuntime {
                store,
                instance,
                fuel,
            })),
            timeout,
            declared_hooks,
        })
    }

    async fn invoke_value(
        &self,
        export: &'static str,
        input: Value,
    ) -> Result<Option<Value>, HookError> {
        if !self.declared_hooks.contains(export) {
            return Ok(None);
        }
        let runtime = Arc::clone(&self.runtime);
        let task = tokio::task::spawn_blocking(move || {
            runtime
                .lock()
                .map_err(|_| "WASM runtime mutex is poisoned".to_string())?
                .invoke_json(export, input)
        });
        match tokio::time::timeout(self.timeout, task).await {
            Ok(Ok(Ok(value))) => Ok(Some(value)),
            Ok(Ok(Err(error))) => Err(error.into()),
            Ok(Err(error)) => Err(format!("WASM hook worker failed: {error}").into()),
            Err(_) => Err(format!(
                "WASM hook `{export}` exceeded {}ms wall-clock limit",
                self.timeout.as_millis()
            )
            .into()),
        }
    }

    async fn invoke_void<T: Serialize + ?Sized>(
        &self,
        export: &'static str,
        input: &T,
    ) -> Result<(), HookError> {
        let input = serde_json::to_value(input)?;
        if let Some(output) = self.invoke_value(export, input).await?
            && !output.is_null()
        {
            return Err(format!("WASM hook `{export}` must return JSON null").into());
        }
        Ok(())
    }

    async fn transformed<T>(&self, export: &'static str, input: &T) -> Result<Option<T>, HookError>
    where
        T: Serialize + DeserializeOwned,
    {
        let input = serde_json::to_value(input)?;
        self.invoke_value(export, input)
            .await?
            .map(serde_json::from_value)
            .transpose()
            .map_err(Into::into)
    }
}

fn declared_hooks(config: &Value) -> Result<HashSet<String>, HookError> {
    let hooks = config
        .get("hooks")
        .or_else(|| config.pointer("/entrypoint/hooks"))
        .and_then(Value::as_array)
        .ok_or("WASM extension config requires a non-empty `hooks` array")?;
    if hooks.is_empty() {
        return Err("WASM extension config requires at least one declared hook".into());
    }
    let valid = HOOK_NAMES
        .into_iter()
        .chain(std::iter::once(EXTRA_HOOK_NAME))
        .collect::<HashSet<_>>();
    let mut declared = HashSet::new();
    for hook in hooks {
        let hook = hook
            .as_str()
            .ok_or("WASM hook declarations must be strings")?;
        if !valid.contains(hook) {
            return Err(format!("unsupported WASM extension hook `{hook}`").into());
        }
        if !declared.insert(hook.to_string()) {
            return Err(format!("duplicate WASM extension hook `{hook}`").into());
        }
    }
    Ok(declared)
}

fn confined_module_path(
    manifest_path: &Path,
    configured: &str,
) -> Result<std::path::PathBuf, HookError> {
    let relative = Path::new(configured);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(format!(
            "WASM module path must stay within the extension directory: {configured}"
        )
        .into());
    }
    let root = manifest_path
        .parent()
        .ok_or_else(|| "extension manifest has no parent directory".to_string())?
        .canonicalize()?;
    let module_path = root.join(relative).canonicalize()?;
    if !module_path.starts_with(&root) || !module_path.is_file() {
        return Err(format!(
            "WASM module path escapes the extension directory or is not a file: {}",
            module_path.display()
        )
        .into());
    }
    Ok(module_path)
}

#[async_trait::async_trait]
impl Extension for WasmExtension {
    fn name(&self) -> &str {
        &self.meta.name
    }

    fn layer(&self) -> ExtensionLayer {
        self.meta.layer
    }

    fn meta(&self) -> ExtensionMeta {
        self.meta.clone()
    }

    async fn on_init(&mut self) -> Result<(), HookError> {
        self.invoke_void("on_init", &Value::Null).await
    }

    async fn on_shutdown(&mut self) -> Result<(), HookError> {
        self.invoke_void("on_shutdown", &Value::Null).await
    }

    async fn on_observe(&self, observation: &mut Observation) -> Result<(), HookError> {
        if let Some(updated) = self.transformed("on_observe", observation).await? {
            *observation = updated;
        }
        Ok(())
    }

    async fn on_filter(&self, observations: &mut Vec<Observation>) -> Result<(), HookError> {
        if let Some(updated) = self.transformed("on_filter", observations).await? {
            *observations = updated;
        }
        Ok(())
    }

    async fn filter_input(&self, message: &mut AgentMessage) -> Result<FilterDecision, HookError> {
        let Some(output) = self
            .invoke_value("filter_input", serde_json::to_value(&*message)?)
            .await?
        else {
            return Ok(FilterDecision::Pass);
        };
        parse_filter_decision(output)
    }

    async fn on_retrieve(&self, results: &mut RetrievalResult) -> Result<(), HookError> {
        if let Some(updated) = self.transformed("on_retrieve", results).await? {
            *results = updated;
        }
        Ok(())
    }

    async fn on_store(&self, entry: &mut StoreEntry) -> Result<(), HookError> {
        if let Some(updated) = self.transformed("on_store", entry).await? {
            *entry = updated;
        }
        Ok(())
    }

    async fn pre_inference(&self, request: &mut InferenceRequest) -> Result<(), HookError> {
        if let Some(updated) = self.transformed("pre_inference", request).await? {
            *request = updated;
        }
        Ok(())
    }

    async fn post_inference(&self, response: &mut InferenceResponse) -> Result<(), HookError> {
        if let Some(updated) = self.transformed("post_inference", response).await? {
            *response = updated;
        }
        Ok(())
    }

    async fn on_gate(&self, event: &mut GateEvent) -> Result<(), HookError> {
        if let Some(updated) = self.transformed("on_gate", event).await? {
            *event = updated;
        }
        Ok(())
    }

    async fn pre_action(&self, event: &ToolCallEvent) -> Result<ActionDecision, HookError> {
        let Some(output) = self
            .invoke_value("pre_action", serde_json::to_value(event)?)
            .await?
        else {
            return Ok(ActionDecision::Proceed);
        };
        parse_action_decision(output)
    }

    async fn post_action(&self, event: &ToolCallEvent) -> Result<(), HookError> {
        self.invoke_void("post_action", event).await
    }

    async fn on_tool_call(&self, event: &ToolCallEvent) -> Result<ToolDecision, HookError> {
        let Some(output) = self
            .invoke_value("on_tool_call", serde_json::to_value(event)?)
            .await?
        else {
            return Ok(ToolDecision::Allow);
        };
        parse_tool_decision(output)
    }

    async fn on_message_send(&self, message: &mut AgentMessage) -> Result<(), HookError> {
        if let Some(updated) = self.transformed("on_message_send", message).await? {
            *message = updated;
        }
        Ok(())
    }

    async fn on_message_receive(&self, message: &mut AgentMessage) -> Result<(), HookError> {
        if let Some(updated) = self.transformed("on_message_receive", message).await? {
            *message = updated;
        }
        Ok(())
    }

    async fn on_reflect(&self, state: &ReflectionState) -> Result<Vec<Adjustment>, HookError> {
        let Some(output) = self
            .invoke_value("on_reflect", serde_json::to_value(state)?)
            .await?
        else {
            return Ok(Vec::new());
        };
        Ok(serde_json::from_value(output)?)
    }

    async fn on_cost_update(&self, cost: &CostUpdate) -> Result<(), HookError> {
        self.invoke_void("on_cost_update", cost).await
    }

    async fn on_error(&self, event: &ErrorEvent) -> Result<RecoveryAction, HookError> {
        let Some(output) = self
            .invoke_value("on_error", serde_json::to_value(event)?)
            .await?
        else {
            return Ok(RecoveryAction::Propagate);
        };
        parse_recovery_action(output)
    }

    async fn on_budget_exceeded(&self, cost: &CostUpdate) -> Result<BudgetAction, HookError> {
        let Some(output) = self
            .invoke_value("on_budget_exceeded", serde_json::to_value(cost)?)
            .await?
        else {
            return Ok(BudgetAction::Sleepwalk);
        };
        parse_budget_action(output)
    }

    async fn on_tick_start(&self, tick: u64) -> Result<(), HookError> {
        self.invoke_void("on_tick_start", &json!({ "tick": tick }))
            .await
    }

    async fn on_tick_end(&self, tick: u64) -> Result<(), HookError> {
        self.invoke_void("on_tick_end", &json!({ "tick": tick }))
            .await
    }

    async fn on_slot_assigned(&self, slot: &str, task: &Value) -> Result<(), HookError> {
        self.invoke_void("on_slot_assigned", &json!({ "slot": slot, "task": task }))
            .await
    }

    async fn on_slot_completed(&self, slot: &str, result: &Value) -> Result<(), HookError> {
        self.invoke_void(
            "on_slot_completed",
            &json!({ "slot": slot, "result": result }),
        )
        .await
    }
}

fn decision<'a>(value: &'a Value, hook: &str) -> Result<&'a str, HookError> {
    value
        .get("decision")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("WASM hook `{hook}` output requires string `decision`").into())
}

fn decision_text(value: &Value, hook: &str, field: &str) -> Result<String, HookError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("WASM hook `{hook}` decision requires string `{field}`").into())
}

fn parse_filter_decision(value: Value) -> Result<FilterDecision, HookError> {
    match decision(&value, "filter_input")? {
        "pass" => Ok(FilterDecision::Pass),
        "drop" => Ok(FilterDecision::Drop),
        "transform" => Ok(FilterDecision::Transform(serde_json::from_value(
            value
                .get("message")
                .cloned()
                .ok_or("WASM filter_input transform requires `message`")?,
        )?)),
        other => Err(format!("invalid WASM filter_input decision `{other}`").into()),
    }
}

fn parse_action_decision(value: Value) -> Result<ActionDecision, HookError> {
    match decision(&value, "pre_action")? {
        "proceed" => Ok(ActionDecision::Proceed),
        "block" => Ok(ActionDecision::Block(decision_text(
            &value,
            "pre_action",
            "reason",
        )?)),
        "rewrite" => Ok(ActionDecision::Rewrite(decision_text(
            &value,
            "pre_action",
            "replacement",
        )?)),
        other => Err(format!("invalid WASM pre_action decision `{other}`").into()),
    }
}

fn parse_tool_decision(value: Value) -> Result<ToolDecision, HookError> {
    match decision(&value, "on_tool_call")? {
        "allow" => Ok(ToolDecision::Allow),
        "deny" => Ok(ToolDecision::Deny(decision_text(
            &value,
            "on_tool_call",
            "reason",
        )?)),
        "rewrite" => Ok(ToolDecision::Rewrite(decision_text(
            &value,
            "on_tool_call",
            "replacement",
        )?)),
        other => Err(format!("invalid WASM on_tool_call decision `{other}`").into()),
    }
}

fn parse_recovery_action(value: Value) -> Result<RecoveryAction, HookError> {
    match decision(&value, "on_error")? {
        "propagate" => Ok(RecoveryAction::Propagate),
        "retry" => Ok(RecoveryAction::Retry),
        "skip" => Ok(RecoveryAction::Skip),
        "fallback" => Ok(RecoveryAction::Fallback(decision_text(
            &value, "on_error", "value",
        )?)),
        other => Err(format!("invalid WASM on_error decision `{other}`").into()),
    }
}

fn parse_budget_action(value: Value) -> Result<BudgetAction, HookError> {
    match decision(&value, "on_budget_exceeded")? {
        "sleepwalk" => Ok(BudgetAction::Sleepwalk),
        "stop" => Ok(BudgetAction::Stop),
        "request_more" => Ok(BudgetAction::RequestMore(
            value
                .get("microdollars")
                .and_then(Value::as_u64)
                .ok_or("WASM on_budget_exceeded request_more requires `microdollars`")?,
        )),
        other => Err(format!("invalid WASM on_budget_exceeded decision `{other}`").into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::extension::PackageTier;

    fn wasm_meta(name: &str) -> ExtensionMeta {
        ExtensionMeta {
            name: name.to_string(),
            layer: ExtensionLayer::Cognition,
            optional: false,
            depends_on: Vec::new(),
            soft_depends_on: Vec::new(),
            version: "1.0.0".to_string(),
            tier: PackageTier::Wasm,
        }
    }

    fn escaped_data(value: &str) -> String {
        value
            .as_bytes()
            .iter()
            .map(|byte| format!("\\{byte:02x}"))
            .collect()
    }

    fn write_module(root: &Path, wat: &str) -> std::path::PathBuf {
        let manifest = root.join("extension.toml");
        std::fs::write(&manifest, "# test manifest").unwrap();
        std::fs::write(root.join("hook.wasm"), wat::parse_str(wat).unwrap()).unwrap();
        manifest
    }

    #[tokio::test]
    async fn every_extension_trait_hook_executes_the_typed_memory_abi() {
        let tmp = tempfile::tempdir().unwrap();
        let hooks = HOOK_NAMES
            .into_iter()
            .chain(std::iter::once(EXTRA_HOOK_NAME))
            .collect::<Vec<_>>();
        let exports = hooks
            .iter()
            .map(|hook| {
                format!("(func (export \"{hook}\") (param i32 i32) (result i64) i64.const 4)")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let wat = format!(
            r#"(module
                (memory (export "memory") 1)
                (func (export "roko_alloc") (param i32) (result i32) i32.const 4096)
                (data (i32.const 0) "\6e\75\6c\6c")
                {exports})"#
        );
        let manifest = write_module(tmp.path(), &wat);
        let config = json!({
            "module": "hook.wasm",
            "memory_mb": 1,
            "fuel": 100_000,
            "hooks": hooks,
        });

        let extension = WasmExtension::load(
            wasm_meta("all-hooks"),
            &manifest,
            &config,
            Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(extension.declared_hooks.len(), 23);
        for hook in hooks {
            let output = extension
                .invoke_value(hook, Value::Null)
                .await
                .unwrap()
                .expect("hook was declared");
            assert!(output.is_null(), "{hook} did not return the fixture value");
        }
    }

    #[tokio::test]
    async fn typed_filter_decision_round_trips_through_wasm_memory() {
        let tmp = tempfile::tempdir().unwrap();
        let output = r#"{"decision":"drop"}"#;
        let wat = format!(
            r#"(module
                (memory (export "memory") 1)
                (func (export "roko_alloc") (param i32) (result i32) i32.const 4096)
                (data (i32.const 0) "{data}")
                (func (export "filter_input") (param i32 i32) (result i64)
                    i64.const {length}))"#,
            data = escaped_data(output),
            length = output.len(),
        );
        let manifest = write_module(tmp.path(), &wat);
        let config = json!({
            "module": "hook.wasm",
            "memory_mb": 1,
            "fuel": 100_000,
            "hooks": ["filter_input"],
        });
        let extension = WasmExtension::load(
            wasm_meta("filter"),
            &manifest,
            &config,
            Duration::from_secs(1),
        )
        .unwrap();
        let mut message = AgentMessage {
            from: "a".to_string(),
            to: "b".to_string(),
            payload: json!({"secret": true}),
        };

        assert!(matches!(
            extension.filter_input(&mut message).await.unwrap(),
            FilterDecision::Drop
        ));
    }

    #[test]
    fn declared_hook_without_export_is_rejected_at_load() {
        let tmp = tempfile::tempdir().unwrap();
        let manifest = write_module(
            tmp.path(),
            r#"(module
                (memory (export "memory") 1)
                (func (export "roko_alloc") (param i32) (result i32) i32.const 4096))"#,
        );
        let config = json!({
            "module": "hook.wasm",
            "memory_mb": 1,
            "fuel": 100_000,
            "hooks": ["on_init"],
        });
        let error = match WasmExtension::load(
            wasm_meta("missing"),
            &manifest,
            &config,
            Duration::from_secs(1),
        ) {
            Ok(_) => panic!("missing declared export must fail loading"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("export is absent"));
    }
}
