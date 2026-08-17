//! Signed extension registry package contract.
//!
//! The relay publisher and CLI installer share this module so checksum,
//! manifest identity, capability, dependency, and signature validation cannot
//! drift across the wire boundary.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use base64::Engine as _;
use ed25519_dalek::{Signature, Signer as _, SigningKey, Verifier as _, VerifyingKey};
use roko_core::extension::{ExtensionManifest, PackageTier};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use wasmi::{Engine, ExternType, Module, core::ValType};

/// Current signed registry envelope schema.
pub const REGISTRY_SCHEMA_VERSION: u32 = 2;
const LEGACY_REGISTRY_SCHEMA_VERSION: u32 = 1;
/// Maximum decoded package bytes accepted by publisher and installer.
pub const MAX_PACKAGE_BYTES: usize = 64 * 1024 * 1024;
/// Maximum package file count accepted by publisher and installer.
pub const MAX_PACKAGE_FILES: usize = 1_024;
const MAX_WASM_MODULE_BYTES: usize = 32 * 1024 * 1024;
const MAX_WASM_FUEL: u64 = 10_000_000;
const MAX_WASM_MEMORY_MB: u64 = 256;
const WASM_HOOK_NAMES: [&str; 23] = [
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
    "on_slot_completed",
];

/// One file embedded in a registry package.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryFile {
    /// Safe package-relative UTF-8 path.
    pub path: String,
    /// Standard-base64 file contents.
    pub content_base64: String,
}

/// Immutable, publisher-signed extension package returned by the relay.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryPackage {
    /// Wire schema version.
    pub schema_version: u32,
    /// Extension identity.
    pub name: String,
    /// Immutable semantic version.
    pub version: String,
    /// Deterministic SHA-256 package checksum.
    pub sha256: String,
    /// Normalized capabilities mirrored from the manifest.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Normalized hard dependencies mirrored from the manifest.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Canonical semver requirements keyed by dependency name.
    ///
    /// Schema-v1 packages omit this map and treat every dependency as `*`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub dependency_requirements: BTreeMap<String, String>,
    /// Authenticated publisher identity.
    pub publisher: String,
    /// Standard-base64 Ed25519 public key.
    pub publisher_public_key: String,
    /// Standard-base64 Ed25519 signature over the canonical envelope.
    pub signature: String,
    /// Checksum-anchored package files.
    pub files: Vec<RegistryFile>,
}

/// A complete, conflict-free dependency resolution returned by the relay.
/// Packages are ordered dependencies-first, with the requested root last.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedRegistryGraph {
    /// Requested root extension name.
    pub root: String,
    /// Selected root semantic version.
    pub root_version: String,
    /// Authenticated packages in dependency-first installation order.
    pub packages: Vec<RegistryPackage>,
}

/// Fully validated package contents and manifest.
#[derive(Debug)]
pub struct ValidatedRegistryPackage {
    /// Decoded safe package files.
    pub files: BTreeMap<PathBuf, Vec<u8>>,
    /// Parsed and validated extension manifest.
    pub manifest: ExtensionManifest,
    /// Publisher key that verified the signature.
    pub verifying_key: VerifyingKey,
}

/// Build and sign a package from an extension directory.
pub fn build_signed_package(
    extension_dir: &Path,
    publisher: &str,
    signing_key: &SigningKey,
) -> Result<RegistryPackage, String> {
    validate_registry_publisher(publisher)?;
    let files = collect_package_files(extension_dir)?;
    let manifest = manifest_from_files(&files)?;
    validate_publishable_manifest(&manifest, &files)?;
    let capabilities = manifest_capabilities(&manifest)?;
    let dependencies = normalized_dependencies(&manifest.depends_on, &manifest.name)?;
    let dependency_requirements = normalized_dependency_requirements(
        &manifest.depends_on,
        &manifest.dependency_requirements,
        &manifest.name,
        false,
    )?;
    let sha256 = package_checksum(&files)?;
    let mut package = RegistryPackage {
        schema_version: REGISTRY_SCHEMA_VERSION,
        name: manifest.name,
        version: manifest.version,
        sha256,
        capabilities: capabilities.into_iter().collect(),
        dependencies: dependencies.into_iter().collect(),
        dependency_requirements,
        publisher: publisher.to_string(),
        publisher_public_key: base64::engine::general_purpose::STANDARD
            .encode(signing_key.verifying_key().as_bytes()),
        signature: String::new(),
        files: encode_package_files(files)?,
    };
    package.signature = base64::engine::general_purpose::STANDARD
        .encode(signing_key.sign(&signature_payload(&package)?).to_bytes());
    Ok(package)
}

/// Verify every integrity and identity field in a signed registry package.
pub fn validate_signed_package(
    package: &RegistryPackage,
) -> Result<ValidatedRegistryPackage, String> {
    if !matches!(
        package.schema_version,
        LEGACY_REGISTRY_SCHEMA_VERSION | REGISTRY_SCHEMA_VERSION
    ) {
        return Err(format!(
            "unsupported registry package schema {}, expected {LEGACY_REGISTRY_SCHEMA_VERSION} or {REGISTRY_SCHEMA_VERSION}",
            package.schema_version
        ));
    }
    validate_registry_name(&package.name)?;
    validate_registry_publisher(&package.publisher)?;
    let files = decode_package_files(&package.files)?;
    let actual_sha256 = package_checksum(&files)?;
    if !constant_time_eq(actual_sha256.as_bytes(), package.sha256.as_bytes()) {
        return Err(format!(
            "extension `{}` checksum mismatch: expected {}, got {actual_sha256}",
            package.name, package.sha256
        ));
    }
    let manifest = manifest_from_files(&files)?;
    validate_publishable_manifest(&manifest, &files)?;
    if manifest.name != package.name || manifest.version != package.version {
        return Err(format!(
            "registry envelope identity {}@{} does not match manifest {}@{}",
            package.name, package.version, manifest.name, manifest.version
        ));
    }
    let manifest_capabilities = manifest_capabilities(&manifest)?;
    let advertised_capabilities = normalized_capabilities(&package.capabilities)?;
    if manifest_capabilities != advertised_capabilities {
        return Err(format!(
            "extension `{}` capability declaration mismatch: registry={advertised_capabilities:?}, manifest={manifest_capabilities:?}",
            package.name
        ));
    }
    let manifest_dependencies = normalized_dependencies(&manifest.depends_on, &manifest.name)?;
    let advertised_dependencies = normalized_dependencies(&package.dependencies, &manifest.name)?;
    if manifest_dependencies != advertised_dependencies {
        return Err(format!(
            "extension `{}` dependency declaration mismatch: registry={advertised_dependencies:?}, manifest={manifest_dependencies:?}",
            package.name
        ));
    }
    let manifest_requirements = normalized_dependency_requirements(
        &manifest.depends_on,
        &manifest.dependency_requirements,
        &manifest.name,
        false,
    )?;
    let advertised_requirements = package_dependency_requirements(package)?;
    if manifest_requirements != advertised_requirements {
        return Err(format!(
            "extension `{}` dependency requirement mismatch: registry={advertised_requirements:?}, manifest={manifest_requirements:?}",
            package.name
        ));
    }

    let verifying_key = parse_registry_public_key(&package.publisher_public_key)?;
    let signature = base64::engine::general_purpose::STANDARD
        .decode(&package.signature)
        .map_err(|error| format!("decode package signature: {error}"))?;
    let signature: [u8; 64] = signature
        .try_into()
        .map_err(|_| "package signature must contain 64 bytes".to_string())?;
    verifying_key
        .verify(
            &signature_payload(package)?,
            &Signature::from_bytes(&signature),
        )
        .map_err(|_| format!("invalid publisher signature for `{}`", package.name))?;

    Ok(ValidatedRegistryPackage {
        files,
        manifest,
        verifying_key,
    })
}

/// Return the canonical dependency requirements authenticated by a package.
///
/// Legacy schema-v1 packages use `*` for every name-only dependency.
pub fn package_dependency_requirements(
    package: &RegistryPackage,
) -> Result<BTreeMap<String, String>, String> {
    match package.schema_version {
        LEGACY_REGISTRY_SCHEMA_VERSION => {
            if !package.dependency_requirements.is_empty() {
                return Err(
                    "registry schema 1 cannot advertise dependency requirements".to_string()
                );
            }
            normalized_dependency_requirements(
                &package.dependencies,
                &BTreeMap::new(),
                &package.name,
                false,
            )
        }
        REGISTRY_SCHEMA_VERSION => normalized_dependency_requirements(
            &package.dependencies,
            &package.dependency_requirements,
            &package.name,
            true,
        ),
        version => Err(format!("unsupported registry package schema {version}")),
    }
}

/// Revalidate a relay resolution before installation.
///
/// This checks every package signature, one-version-per-name selection,
/// semver satisfaction, dependency-first order, root identity, and excludes
/// unreachable packages.
pub fn validate_resolved_registry_graph(graph: &ResolvedRegistryGraph) -> Result<(), String> {
    validate_registry_name(&graph.root)?;
    let root_version = Version::parse(&graph.root_version)
        .map_err(|error| format!("invalid resolved root version: {error}"))?;
    if graph.packages.is_empty() {
        return Err("registry resolution contains no packages".to_string());
    }

    let mut indexes = BTreeMap::new();
    for (index, package) in graph.packages.iter().enumerate() {
        validate_signed_package(package)?;
        Version::parse(&package.version).map_err(|error| {
            format!(
                "resolved extension {}@{} has invalid version: {error}",
                package.name, package.version
            )
        })?;
        if indexes.insert(package.name.as_str(), index).is_some() {
            return Err(format!(
                "registry resolution selected multiple versions of `{}`",
                package.name
            ));
        }
    }

    let root_index = *indexes.get(graph.root.as_str()).ok_or_else(|| {
        format!(
            "registry resolution does not contain requested root `{}`",
            graph.root
        )
    })?;
    let root = &graph.packages[root_index];
    let selected_root_version = Version::parse(&root.version)
        .map_err(|error| format!("invalid resolved root package version: {error}"))?;
    let root_version_text_mismatch = root.version != graph.root_version;
    let root_version_semver_mismatch = selected_root_version != root_version;
    if root_version_text_mismatch || root_version_semver_mismatch {
        return Err(format!(
            "registry resolution root identity mismatch: expected {}@{}, got {}@{}",
            graph.root, graph.root_version, root.name, root.version
        ));
    }
    if root_index + 1 != graph.packages.len() {
        return Err("registry resolution root must be the final package".to_string());
    }

    let mut reachable = BTreeSet::new();
    let mut pending = vec![root_index];
    while let Some(index) = pending.pop() {
        if !reachable.insert(index) {
            continue;
        }
        let package = &graph.packages[index];
        for (dependency, requirement) in package_dependency_requirements(package)? {
            let dependency_index = *indexes.get(dependency.as_str()).ok_or_else(|| {
                format!(
                    "resolved extension {}@{} is missing dependency `{dependency}` ({requirement})",
                    package.name, package.version
                )
            })?;
            if dependency_index >= index {
                return Err(format!(
                    "registry resolution is not dependency-first at {}@{} -> `{dependency}`",
                    package.name, package.version
                ));
            }
            let selected = &graph.packages[dependency_index];
            let requirement = VersionReq::parse(&requirement)
                .map_err(|error| format!("invalid dependency requirement: {error}"))?;
            let version = Version::parse(&selected.version)
                .map_err(|error| format!("invalid selected dependency version: {error}"))?;
            if !requirement.matches(&version) {
                return Err(format!(
                    "resolved dependency conflict: {}@{} requires `{dependency}` {requirement}, selected {}",
                    package.name, package.version, selected.version
                ));
            }
            pending.push(dependency_index);
        }
    }
    if reachable.len() != graph.packages.len() {
        return Err("registry resolution contains unreachable packages".to_string());
    }
    Ok(())
}

fn package_checksum(files: &BTreeMap<PathBuf, Vec<u8>>) -> Result<String, String> {
    let mut digest = Sha256::new();
    let mut portable_files = files
        .iter()
        .map(|(path, contents)| Ok((portable_package_path(path)?, contents)))
        .collect::<Result<Vec<_>, String>>()?;
    portable_files.sort_by(|left, right| left.0.cmp(&right.0));
    for (path, contents) in portable_files {
        digest.update((path.len() as u64).to_be_bytes());
        digest.update(path.as_bytes());
        digest.update((contents.len() as u64).to_be_bytes());
        digest.update(contents);
    }
    Ok(format!("{:x}", digest.finalize()))
}

/// Validate a registry extension or dependency name.
pub fn validate_registry_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(format!("invalid registry extension name `{name}`"));
    }
    Ok(())
}

/// Validate a registry publisher identity.
pub fn validate_registry_publisher(publisher: &str) -> Result<(), String> {
    if publisher.is_empty()
        || publisher.len() > 128
        || !publisher.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '@')
        })
    {
        return Err(format!("invalid registry publisher `{publisher}`"));
    }
    Ok(())
}

/// Decode and validate a canonical standard-base64 Ed25519 publisher key.
pub fn parse_registry_public_key(encoded: &str) -> Result<VerifyingKey, String> {
    let public_key = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|error| format!("decode publisher public key: {error}"))?;
    let public_key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| "publisher public key must contain 32 bytes".to_string())?;
    let verifying_key = VerifyingKey::from_bytes(&public_key)
        .map_err(|error| format!("invalid publisher public key: {error}"))?;
    if base64::engine::general_purpose::STANDARD.encode(verifying_key.as_bytes()) != encoded {
        return Err("publisher public key must use canonical standard base64".to_string());
    }
    Ok(verifying_key)
}

fn collect_package_files(root: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    if !root.is_dir() {
        return Err(format!(
            "extension source is not a directory: {}",
            root.display()
        ));
    }
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    let mut total = 0usize;
    while let Some(directory) = pending.pop() {
        let mut entries = std::fs::read_dir(&directory)
            .map_err(|error| format!("read {}: {error}", directory.display()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("read {}: {error}", directory.display()))?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path)
                .map_err(|error| format!("inspect {}: {error}", path.display()))?;
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "registry packages cannot contain symlinks: {}",
                    path.display()
                ));
            }
            if metadata.is_dir() {
                pending.push(path);
                continue;
            }
            if !metadata.is_file() {
                return Err(format!(
                    "unsupported registry package entry: {}",
                    path.display()
                ));
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| format!("package path escaped source root: {}", path.display()))?
                .to_path_buf();
            validate_package_path(&relative)?;
            let contents = std::fs::read(&path)
                .map_err(|error| format!("read {}: {error}", path.display()))?;
            total = total
                .checked_add(contents.len())
                .ok_or_else(|| "registry package size overflow".to_string())?;
            if total > MAX_PACKAGE_BYTES {
                return Err("registry package exceeds the maximum decoded size".to_string());
            }
            if files.len() >= MAX_PACKAGE_FILES {
                return Err(format!(
                    "registry package exceeds {MAX_PACKAGE_FILES} files"
                ));
            }
            files.insert(relative, contents);
        }
    }
    if files.is_empty() {
        return Err("registry package contains no files".to_string());
    }
    Ok(files)
}

fn encode_package_files(files: BTreeMap<PathBuf, Vec<u8>>) -> Result<Vec<RegistryFile>, String> {
    let mut encoded = files
        .into_iter()
        .map(|(path, contents)| {
            Ok(RegistryFile {
                path: portable_package_path(&path)?,
                content_base64: base64::engine::general_purpose::STANDARD.encode(contents),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    encoded.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(encoded)
}

fn decode_package_files(files: &[RegistryFile]) -> Result<BTreeMap<PathBuf, Vec<u8>>, String> {
    if files.is_empty() || files.len() > MAX_PACKAGE_FILES {
        return Err(format!(
            "registry package file count must be within 1..={MAX_PACKAGE_FILES}"
        ));
    }
    let mut decoded = BTreeMap::new();
    let mut total = 0usize;
    for file in files {
        validate_wire_package_path(&file.path)?;
        let path = PathBuf::from(&file.path);
        validate_package_path(&path)?;
        let contents = base64::engine::general_purpose::STANDARD
            .decode(&file.content_base64)
            .map_err(|error| format!("decode {}: {error}", file.path))?;
        total = total
            .checked_add(contents.len())
            .ok_or_else(|| "registry package size overflow".to_string())?;
        if total > MAX_PACKAGE_BYTES {
            return Err("registry package exceeds the maximum decoded size".to_string());
        }
        if decoded.insert(path, contents).is_some() {
            return Err(format!("duplicate path in registry package: {}", file.path));
        }
    }
    Ok(decoded)
}

fn validate_package_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "unsafe path in registry package: {}",
            path.display()
        ));
    }
    Ok(())
}

fn portable_package_path(path: &Path) -> Result<String, String> {
    validate_package_path(path)?;
    path.components()
        .map(|component| match component {
            Component::Normal(value) => value
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("package path is not UTF-8: {}", path.display())),
            _ => unreachable!("validated package path contains only normal components"),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|components| components.join("/"))
}

fn validate_wire_package_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(format!("unsafe portable path in registry package: {path}"));
    }
    Ok(())
}

fn manifest_from_files(files: &BTreeMap<PathBuf, Vec<u8>>) -> Result<ExtensionManifest, String> {
    let bytes = files
        .get(Path::new("extension.toml"))
        .or_else(|| files.get(Path::new("manifest.toml")))
        .ok_or_else(|| "registry package is missing extension.toml or manifest.toml".to_string())?;
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("extension manifest is not UTF-8: {error}"))?;
    #[derive(Deserialize)]
    struct WrappedManifest {
        extension: ExtensionManifest,
    }
    let manifest = toml::from_str::<WrappedManifest>(text)
        .map(|wrapped| wrapped.extension)
        .or_else(|_| toml::from_str::<ExtensionManifest>(text))
        .map_err(|error| format!("parse extension manifest: {error}"))?;
    manifest.validate().map_err(|error| error.to_string())?;
    Ok(manifest)
}

fn validate_publishable_manifest(
    manifest: &ExtensionManifest,
    files: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), String> {
    semver::Version::parse(&manifest.version).map_err(|error| {
        format!(
            "extension `{}` has invalid semantic version `{}`: {error}",
            manifest.name, manifest.version
        )
    })?;
    if manifest.tier != PackageTier::Wasm {
        return Err(format!(
            "registry publication currently supports only the executable WASM tier, got {:?}",
            manifest.tier
        ));
    }
    let module = manifest
        .config
        .get("module")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            manifest
                .config
                .pointer("/entrypoint/module")
                .and_then(serde_json::Value::as_str)
        })
        .ok_or_else(|| "WASM extension config requires a string `module` path".to_string())?;
    let module_path = PathBuf::from(module);
    validate_package_path(&module_path)?;
    let module = files.get(&module_path).ok_or_else(|| {
        format!(
            "WASM module `{}` is missing from the package",
            module_path.display()
        )
    })?;
    if module.len() > MAX_WASM_MODULE_BYTES {
        return Err(format!(
            "WASM module `{}` exceeds {MAX_WASM_MODULE_BYTES} bytes",
            module_path.display()
        ));
    }
    validate_wasm_abi(manifest, &module_path, module)?;
    Ok(())
}

fn validate_wasm_abi(
    manifest: &ExtensionManifest,
    module_path: &Path,
    bytes: &[u8],
) -> Result<(), String> {
    let module = Module::new(&Engine::default(), bytes)
        .map_err(|error| format!("parse WASM module `{}`: {error}", module_path.display()))?;
    if let Some(import) = module.imports().next() {
        return Err(format!(
            "WASM module `{}` imports {}::{}, but registry extensions have no host imports",
            module_path.display(),
            import.module(),
            import.name()
        ));
    }
    if !matches!(module.get_export("memory"), Some(ExternType::Memory(_))) {
        return Err("WASM JSON ABI requires exported `memory`".to_string());
    }
    validate_function_export(&module, "roko_alloc", &[ValType::I32], &[ValType::I32])?;

    let hooks = manifest
        .config
        .get("hooks")
        .or_else(|| manifest.config.pointer("/entrypoint/hooks"))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "WASM extension config requires a non-empty `hooks` array".to_string())?;
    if hooks.is_empty() {
        return Err("WASM extension config requires at least one declared hook".to_string());
    }
    let mut declared = BTreeSet::new();
    for hook in hooks {
        let hook = hook
            .as_str()
            .ok_or_else(|| "WASM hook declarations must be strings".to_string())?;
        if !WASM_HOOK_NAMES.contains(&hook) {
            return Err(format!("unsupported WASM extension hook `{hook}`"));
        }
        if !declared.insert(hook) {
            return Err(format!("duplicate WASM extension hook `{hook}`"));
        }
        validate_function_export(
            &module,
            hook,
            &[ValType::I32, ValType::I32],
            &[ValType::I64],
        )?;
    }

    if let Some(fuel) = manifest
        .config
        .get("fuel")
        .and_then(serde_json::Value::as_u64)
        .or_else(|| {
            manifest
                .config
                .pointer("/permissions/fuel")
                .and_then(serde_json::Value::as_u64)
        })
        && !(1..=MAX_WASM_FUEL).contains(&fuel)
    {
        return Err(format!(
            "WASM fuel must be within 1..={MAX_WASM_FUEL}, got {fuel}"
        ));
    }
    if let Some(memory_mb) = manifest
        .config
        .get("memory_mb")
        .and_then(serde_json::Value::as_u64)
        .or_else(|| {
            manifest
                .config
                .pointer("/permissions/memory_mb")
                .and_then(serde_json::Value::as_u64)
        })
        && !(1..=MAX_WASM_MEMORY_MB).contains(&memory_mb)
    {
        return Err(format!(
            "WASM memory_mb must be within 1..={MAX_WASM_MEMORY_MB}, got {memory_mb}"
        ));
    }
    Ok(())
}

fn validate_function_export(
    module: &Module,
    name: &str,
    params: &[ValType],
    results: &[ValType],
) -> Result<(), String> {
    let Some(ExternType::Func(function)) = module.get_export(name) else {
        return Err(format!("WASM JSON ABI requires function export `{name}`"));
    };
    if function.params() != params || function.results() != results {
        return Err(format!(
            "WASM function `{name}` has type {:?} -> {:?}, expected {params:?} -> {results:?}",
            function.params(),
            function.results()
        ));
    }
    Ok(())
}

fn manifest_capabilities(manifest: &ExtensionManifest) -> Result<BTreeSet<String>, String> {
    let Some(value) = manifest.config.get("capabilities") else {
        return Ok(BTreeSet::new());
    };
    let capabilities = value
        .as_array()
        .ok_or_else(|| "extension config.capabilities must be an array of strings".to_string())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "extension capability must be a string".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    normalized_capabilities(&capabilities)
}

fn normalized_capabilities(capabilities: &[String]) -> Result<BTreeSet<String>, String> {
    let mut normalized = BTreeSet::new();
    for capability in capabilities {
        let capability = capability.trim();
        if capability.is_empty()
            || !capability.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '.' | ':' | '-' | '_')
            })
        {
            return Err(format!("invalid extension capability `{capability}`"));
        }
        if !normalized.insert(capability.to_string()) {
            return Err(format!("duplicate extension capability `{capability}`"));
        }
    }
    Ok(normalized)
}

fn normalized_dependencies(
    dependencies: &[String],
    package: &str,
) -> Result<BTreeSet<String>, String> {
    let mut normalized = BTreeSet::new();
    for dependency in dependencies {
        validate_registry_name(dependency)?;
        if dependency == package {
            return Err(format!("extension `{package}` cannot depend on itself"));
        }
        if !normalized.insert(dependency.clone()) {
            return Err(format!("duplicate extension dependency `{dependency}`"));
        }
    }
    Ok(normalized)
}

fn normalized_dependency_requirements(
    dependencies: &[String],
    requirements: &BTreeMap<String, String>,
    package: &str,
    require_explicit: bool,
) -> Result<BTreeMap<String, String>, String> {
    let dependencies = normalized_dependencies(dependencies, package)?;
    for dependency in requirements.keys() {
        validate_registry_name(dependency)?;
        if !dependencies.contains(dependency) {
            return Err(format!(
                "extension `{package}` declares a requirement for undeclared dependency `{dependency}`"
            ));
        }
    }
    let mut normalized = BTreeMap::new();
    for dependency in dependencies {
        let requirement = match requirements.get(&dependency) {
            Some(requirement) if !requirement.trim().is_empty() => requirement.trim(),
            Some(_) => {
                return Err(format!(
                    "extension `{package}` has an empty requirement for `{dependency}`"
                ));
            }
            None if require_explicit => {
                return Err(format!(
                    "registry schema {REGISTRY_SCHEMA_VERSION} is missing a requirement for dependency `{dependency}`"
                ));
            }
            None => "*",
        };
        let parsed = VersionReq::parse(requirement).map_err(|error| {
            format!(
                "extension `{package}` has invalid requirement `{requirement}` for `{dependency}`: {error}"
            )
        })?;
        let canonical = parsed.to_string();
        if require_explicit && requirement != canonical {
            return Err(format!(
                "dependency requirement for `{dependency}` must be canonical `{canonical}`, got `{requirement}`"
            ));
        }
        normalized.insert(dependency, canonical);
    }
    Ok(normalized)
}

fn signature_payload(package: &RegistryPackage) -> Result<Vec<u8>, String> {
    let mut payload = match package.schema_version {
        LEGACY_REGISTRY_SCHEMA_VERSION => b"roko.extension.registry.v1\0".to_vec(),
        REGISTRY_SCHEMA_VERSION => b"roko.extension.registry.v2\0".to_vec(),
        version => return Err(format!("unsupported registry package schema {version}")),
    };
    append_field(&mut payload, package.schema_version.to_string().as_bytes());
    append_field(&mut payload, package.name.as_bytes());
    append_field(&mut payload, package.version.as_bytes());
    append_field(&mut payload, package.sha256.as_bytes());
    append_field(&mut payload, package.publisher.as_bytes());
    append_field(&mut payload, package.publisher_public_key.as_bytes());
    let capabilities = normalized_capabilities(&package.capabilities)?;
    append_field(
        &mut payload,
        capabilities
            .into_iter()
            .collect::<Vec<_>>()
            .join("\n")
            .as_bytes(),
    );
    let dependencies = normalized_dependencies(&package.dependencies, &package.name)?;
    append_field(
        &mut payload,
        dependencies
            .into_iter()
            .collect::<Vec<_>>()
            .join("\n")
            .as_bytes(),
    );
    if package.schema_version == REGISTRY_SCHEMA_VERSION {
        let requirements = package_dependency_requirements(package)?;
        append_field(&mut payload, &(requirements.len() as u64).to_be_bytes());
        for (dependency, requirement) in requirements {
            append_field(&mut payload, dependency.as_bytes());
            append_field(&mut payload, requirement.as_bytes());
        }
    }
    Ok(payload)
}

fn append_field(payload: &mut Vec<u8>, value: &[u8]) {
    payload.extend_from_slice(&(value.len() as u64).to_be_bytes());
    payload.extend_from_slice(value);
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (left, right)| difference | (left ^ right))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_test_package(manifest_dependencies: &str) -> Result<RegistryPackage, String> {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("extension.toml"),
            format!(
                r#"[extension]
name = "signed"
version = "1.0.0"
layer = "cognition"
tier = "wasm"
{manifest_dependencies}
[extension.config]
module = "hook.wasm"
capabilities = ["bus.publish"]
hooks = ["on_init"]
"#
            ),
        )
        .unwrap();
        let wasm = wat::parse_str(
            r#"(module
                (memory (export "memory") 1)
                (func (export "roko_alloc") (param i32) (result i32) i32.const 4096)
                (func (export "on_init") (param i32 i32) (result i64) i64.const 4))"#,
        )
        .unwrap();
        std::fs::write(root.path().join("hook.wasm"), wasm).unwrap();
        build_signed_package(root.path(), "publisher", &SigningKey::from_bytes(&[7; 32]))
    }

    #[test]
    fn signed_package_rejects_tampered_checksum() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("extension.toml"),
            r#"[extension]
name = "signed"
version = "1.0.0"
layer = "cognition"
tier = "wasm"
[extension.config]
module = "hook.wasm"
capabilities = ["bus.publish"]
hooks = ["on_init"]
"#,
        )
        .unwrap();
        let wasm = wat::parse_str(
            r#"(module
                (memory (export "memory") 1)
                (func (export "roko_alloc") (param i32) (result i32) i32.const 4096)
                (func (export "on_init") (param i32 i32) (result i64) i64.const 4))"#,
        )
        .unwrap();
        std::fs::write(root.path().join("hook.wasm"), wasm).unwrap();
        let mut package =
            build_signed_package(root.path(), "publisher", &SigningKey::from_bytes(&[7; 32]))
                .unwrap();
        package.sha256 = "0".repeat(64);
        assert!(
            validate_signed_package(&package)
                .unwrap_err()
                .contains("checksum mismatch")
        );
    }

    #[test]
    fn signed_package_authenticates_canonical_dependency_requirements() {
        let package = build_test_package(
            r#"depends_on = ["dependency"]
[extension.dependency_requirements]
dependency = "^1.2"
"#,
        )
        .unwrap();

        assert_eq!(package.schema_version, 2);
        assert_eq!(
            package.dependency_requirements.get("dependency"),
            Some(&"^1.2".to_string())
        );
        validate_signed_package(&package).unwrap();

        let mut tampered = package;
        tampered
            .dependency_requirements
            .insert("dependency".to_string(), "^2.0".to_string());
        assert!(validate_signed_package(&tampered).is_err());
    }

    #[test]
    fn invalid_or_undeclared_dependency_requirements_are_rejected() {
        let invalid = build_test_package(
            r#"depends_on = ["dependency"]
[extension.dependency_requirements]
dependency = "not a range"
"#,
        )
        .unwrap_err();
        assert!(invalid.contains("invalid requirement"));

        let undeclared = build_test_package(
            r#"depends_on = []
[extension.dependency_requirements]
dependency = "^1"
"#,
        )
        .unwrap_err();
        assert!(undeclared.contains("undeclared dependency"));
    }

    #[test]
    fn schema_one_name_only_packages_remain_verifiable() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let mut package = build_test_package("depends_on = [\"dependency\"]\n").unwrap();
        package.schema_version = LEGACY_REGISTRY_SCHEMA_VERSION;
        package.dependency_requirements.clear();
        package.signature = base64::engine::general_purpose::STANDARD.encode(
            signing_key
                .sign(&signature_payload(&package).unwrap())
                .to_bytes(),
        );

        validate_signed_package(&package).unwrap();
        assert_eq!(
            package_dependency_requirements(&package)
                .unwrap()
                .get("dependency"),
            Some(&"*".to_string())
        );
    }
}
