//! Relay-backed extension registry installation.
//!
//! Registry packages use a JSON envelope so the client can validate every
//! path and byte before exposing anything under `.roko/extensions/{name}`.
//! The checksum is over the deterministic sequence of sorted package files:
//! big-endian path length, UTF-8 path, big-endian content length, content.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use base64::Engine as _;
use ed25519_dalek::SigningKey;
use roko_plugin::registry::{
    RegistryPackage, ResolvedRegistryGraph, build_signed_package, validate_registry_name,
    validate_resolved_registry_graph, validate_signed_package,
};
use semver::{Version, VersionReq};

const MAX_REGISTRY_RESPONSE_BYTES: usize = 96 * 1024 * 1024;

/// Resolve the HTTP registry base URL from an explicit override or relay URL.
///
/// `ROKO_EXTENSION_REGISTRY_URL` takes precedence. Relay WebSocket schemes
/// are mapped to their HTTP equivalents.
#[must_use]
pub fn registry_base_url(relay_url: Option<&str>) -> Option<String> {
    let raw = std::env::var("ROKO_EXTENSION_REGISTRY_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| relay_url.map(str::to_owned))?;
    let mut url = reqwest::Url::parse(raw.trim()).ok()?;
    match url.scheme() {
        "wss" => url.set_scheme("https").ok()?,
        "ws" => url.set_scheme("http").ok()?,
        "http" | "https" => {}
        _ => return None,
    }
    Some(url.to_string().trim_end_matches('/').to_string())
}

/// Fetch config-referenced extensions that are not already installed locally.
///
/// Each package is fully downloaded and verified before an atomic rename into
/// the live extension directory. Already installed names are never replaced.
pub fn fetch_missing_registry_extensions(
    workdir: &Path,
    names: &[String],
    registry_base: Option<&str>,
) -> Vec<(String, String)> {
    let mut failures = Vec::new();
    let Some(registry_base) = registry_base else {
        for name in names {
            if !extension_is_installed(workdir, name) {
                failures.push((
                    name.clone(),
                    "no extension registry URL is configured".to_string(),
                ));
            }
        }
        return failures;
    };

    for name in names {
        if extension_is_installed(workdir, name) {
            continue;
        }
        if let Err(error) = install_registry_extension(workdir, name, registry_base) {
            failures.push((name.clone(), error.to_string()));
        }
    }
    failures
}

/// Download, authenticate, validate, and atomically install one registry package.
pub fn install_registry_extension(
    workdir: &Path,
    name: &str,
    registry_base: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    install_registry_extension_requirement(workdir, name, "*", registry_base)
}

/// Parse `name` or `name@semver-requirement` registry install syntax.
pub fn parse_registry_selector(selector: &str) -> Result<(String, String), String> {
    if validate_registry_name(selector).is_ok() {
        return Ok((selector.to_string(), "*".to_string()));
    }
    let (name, requirement) = selector.rsplit_once('@').ok_or_else(|| {
        format!("invalid registry extension selector `{selector}`; expected name or name@range")
    })?;
    validate_registry_name(name)?;
    if requirement.trim().is_empty() {
        return Err(format!(
            "invalid registry extension selector `{selector}`: version range is empty"
        ));
    }
    let requirement = VersionReq::parse(requirement.trim())
        .map_err(|error| format!("invalid registry extension selector `{selector}`: {error}"))?;
    Ok((name.to_string(), requirement.to_string()))
}

/// Download and install a complete graph whose root matches `requirement`.
pub fn install_registry_extension_requirement(
    workdir: &Path,
    name: &str,
    requirement: &str,
    registry_base: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    validate_registry_name(name)
        .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { error.into() })?;
    let requirement = VersionReq::parse(requirement).map_err(|error| {
        format!("invalid registry version requirement `{requirement}`: {error}")
    })?;
    if extension_is_installed(workdir, name) {
        return Err(format!("extension `{name}` is already installed").into());
    }

    let graph = fetch_registry_resolution(name, &requirement.to_string(), registry_base)?;
    validate_resolved_registry_graph(&graph)
        .map_err(|error| format!("verify registry resolution for `{name}`: {error}"))?;
    if graph.root != name {
        return Err(format!(
            "registry returned root `{}` for requested `{name}`",
            graph.root
        )
        .into());
    }
    let root_version = Version::parse(&graph.root_version)?;
    if !requirement.matches(&root_version) {
        return Err(format!(
            "registry returned {name}@{} outside requested range `{requirement}`",
            graph.root_version
        )
        .into());
    }

    let mut validated = Vec::with_capacity(graph.packages.len());
    for package in graph.packages {
        let contents = validate_signed_package(&package).map_err(|error| {
            format!(
                "verify resolved registry extension {}@{}: {error}",
                package.name, package.version
            )
        })?;
        validated.push((package, contents));
    }

    // Check every already-installed dependency before writing anything. A
    // name can have only one installed version, so mismatches are conflicts.
    for (package, _) in &validated {
        let target = workdir.join(".roko").join("extensions").join(&package.name);
        if !target.exists() {
            continue;
        }
        let manifest_path = [target.join("extension.toml"), target.join("manifest.toml")]
            .into_iter()
            .find(|path| path.is_file())
            .ok_or_else(|| {
                format!(
                    "installed dependency `{}` has no extension manifest",
                    package.name
                )
            })?;
        let installed = super::extension_loader::load_extension_manifest(&manifest_path)?;
        if installed.name != package.name || installed.version != package.version {
            return Err(format!(
                "registry dependency conflict: selected {}@{}, but installed {}@{}",
                package.name, package.version, installed.name, installed.version
            )
            .into());
        }
    }

    for (package, contents) in &validated {
        if !extension_is_installed(workdir, &package.name) {
            install_validated_files(workdir, &package.name, &contents.files)?;
        }
    }
    Ok(workdir.join(".roko").join("extensions").join(name))
}

fn fetch_registry_resolution(
    name: &str,
    requirement: &str,
    registry_base: &str,
) -> Result<ResolvedRegistryGraph, Box<dyn std::error::Error + Send + Sync>> {
    validate_registry_name(name)
        .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { error.into() })?;

    let mut url = reqwest::Url::parse(registry_base)?;
    match url.scheme() {
        "https" => {}
        "http" if url.host_str().is_some_and(is_loopback_host) => {}
        "http" => {
            return Err(
                "remote extension registries must use HTTPS; HTTP is allowed only for loopback"
                    .into(),
            );
        }
        _ => return Err("extension registry URL must use http or https".into()),
    }
    url.path_segments_mut()
        .map_err(|_| "extension registry URL cannot be a base URL")?
        .extend(["registry", "extensions", name, "resolve"]);
    url.query_pairs_mut()
        .append_pair("requirement", requirement);

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let response = client.get(url).send()?.error_for_status()?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_REGISTRY_RESPONSE_BYTES as u64)
    {
        return Err("extension registry response exceeds the maximum size".into());
    }
    let response_bytes = response.bytes()?;
    if response_bytes.len() > MAX_REGISTRY_RESPONSE_BYTES {
        return Err("extension registry response exceeds the maximum size".into());
    }
    Ok(serde_json::from_slice(&response_bytes)?)
}

fn install_validated_files(
    workdir: &Path,
    name: &str,
    files: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let extensions_dir = workdir.join(".roko").join("extensions");
    std::fs::create_dir_all(&extensions_dir)?;
    let staging = extensions_dir.join(format!(".registry-{name}-{}.tmp", uuid::Uuid::new_v4()));
    std::fs::create_dir(&staging)?;
    let result = stage_files(&staging, &files).and_then(|()| {
        let target = extensions_dir.join(name);
        if target.exists() {
            return Err(format!("extension `{name}` was installed concurrently").into());
        }
        std::fs::rename(&staging, &target)?;
        Ok(target)
    });
    if result.is_err() && staging.exists() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    result
}

/// Build, sign, and publish one local WASM extension to the relay registry.
pub fn publish_registry_extension(
    extension_dir: &Path,
    publisher: &str,
    bearer_token: &str,
    signing_key_secret: &str,
    registry_base: &str,
) -> Result<RegistryPackage, Box<dyn std::error::Error + Send + Sync>> {
    if bearer_token.is_empty() {
        return Err("registry publisher bearer token is empty".into());
    }
    let signing_key = parse_signing_key(signing_key_secret)?;
    let package = build_signed_package(extension_dir, publisher, &signing_key)
        .map_err(|error| format!("build signed registry package: {error}"))?;
    let mut url = reqwest::Url::parse(registry_base)?;
    match url.scheme() {
        "https" => {}
        "http" if url.host_str().is_some_and(is_loopback_host) => {}
        "http" => {
            return Err(
                "remote extension registries must use HTTPS; HTTP is allowed only for loopback"
                    .into(),
            );
        }
        _ => return Err("extension registry URL must use http or https".into()),
    }
    url.path_segments_mut()
        .map_err(|_| "extension registry URL cannot be a base URL")?
        .extend(["registry", "extensions"]);
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let response = client
        .post(url)
        .bearer_auth(bearer_token)
        .json(&package)
        .send()?;
    let status = response.status();
    if response
        .content_length()
        .is_some_and(|length| length > MAX_REGISTRY_RESPONSE_BYTES as u64)
    {
        return Err("extension registry response exceeds the maximum size".into());
    }
    let body = response.bytes()?;
    if body.len() > MAX_REGISTRY_RESPONSE_BYTES {
        return Err("extension registry response exceeds the maximum size".into());
    }
    if !status.is_success() {
        let message = serde_json::from_slice::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| String::from_utf8_lossy(&body).into_owned());
        return Err(format!("registry publish failed with {status}: {message}").into());
    }
    let published: RegistryPackage = serde_json::from_slice(&body)?;
    validate_signed_package(&published)
        .map_err(|error| format!("relay returned invalid published package: {error}"))?;
    if published.name != package.name
        || published.version != package.version
        || published.sha256 != package.sha256
        || published.signature != package.signature
    {
        return Err("relay publish response does not match submitted immutable package".into());
    }
    Ok(published)
}

fn extension_is_installed(workdir: &Path, name: &str) -> bool {
    if validate_registry_name(name).is_err() {
        return false;
    }
    let directory = workdir.join(".roko").join("extensions").join(name);
    directory.join("extension.toml").is_file() || directory.join("manifest.toml").is_file()
}

fn parse_signing_key(
    encoded: &str,
) -> Result<SigningKey, Box<dyn std::error::Error + Send + Sync>> {
    let encoded = encoded.trim();
    let bytes = if encoded.len() == 64 && encoded.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        (0..32)
            .map(|index| u8::from_str_radix(&encoded[index * 2..index * 2 + 2], 16))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        base64::engine::general_purpose::STANDARD.decode(encoded)?
    };
    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "registry signing key must contain exactly 32 bytes")?;
    Ok(SigningKey::from_bytes(&bytes))
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn stage_files(
    staging: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for (relative, contents) in files {
        let destination = staging.join(relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        file.write_all(contents)?;
        file.sync_all()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn signed_package(name: &str) -> RegistryPackage {
        let source = tempfile::tempdir().unwrap();
        std::fs::write(
            source.path().join("extension.toml"),
            format!(
                r#"[extension]
name = "{name}"
version = "1.2.3"
layer = "cognition"
tier = "wasm"

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
        std::fs::write(source.path().join("hook.wasm"), wasm).unwrap();
        build_signed_package(
            source.path(),
            "test-publisher",
            &SigningKey::from_bytes(&[7; 32]),
        )
        .unwrap()
    }
    fn serve_once(package: RegistryPackage) -> (String, std::thread::JoinHandle<()>) {
        let graph = ResolvedRegistryGraph {
            root: package.name.clone(),
            root_version: package.version.clone(),
            packages: vec![package],
        };
        let body = serde_json::to_string(&graph).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0u8; 2048];
            let read = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with("GET /registry/extensions/registry-test/resolve?"));
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        (format!("http://{address}"), handle)
    }

    #[test]
    fn registry_install_verifies_and_atomically_exposes_package() {
        let tmp = tempfile::tempdir().unwrap();
        let (base, server) = serve_once(signed_package("registry-test"));
        let installed = install_registry_extension(tmp.path(), "registry-test", &base).unwrap();
        server.join().unwrap();

        assert_eq!(installed.file_name().unwrap(), "registry-test");
        assert!(installed.join("extension.toml").is_file());
        assert!(installed.join("hook.wasm").is_file());
        assert!(scan_staging_dirs(tmp.path()).is_empty());
    }

    #[test]
    fn registry_install_rejects_bad_checksum_without_partial_install() {
        let tmp = tempfile::tempdir().unwrap();
        let mut package = signed_package("registry-test");
        package.sha256 = "0".repeat(64);
        let (base, server) = serve_once(package);
        let error = install_registry_extension(tmp.path(), "registry-test", &base).unwrap_err();
        server.join().unwrap();

        assert!(error.to_string().contains("checksum mismatch"));
        assert!(!extension_is_installed(tmp.path(), "registry-test"));
        assert!(scan_staging_dirs(tmp.path()).is_empty());
    }

    #[test]
    fn registry_package_rejects_parent_paths() {
        let mut package = signed_package("registry-test");
        package.files[0].path = "../escape".to_string();
        assert!(validate_signed_package(&package).is_err());
    }

    #[test]
    fn registry_selector_supports_semver_ranges_and_legacy_names() {
        assert_eq!(
            parse_registry_selector("registry-test").unwrap(),
            ("registry-test".to_string(), "*".to_string())
        );
        assert_eq!(
            parse_registry_selector("registry-test@^1.2").unwrap(),
            ("registry-test".to_string(), "^1.2".to_string())
        );
        assert!(parse_registry_selector("registry-test@not-a-range").is_err());
    }

    fn scan_staging_dirs(workdir: &Path) -> Vec<PathBuf> {
        let directory = workdir.join(".roko").join("extensions");
        std::fs::read_dir(directory)
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(".registry-"))
            })
            .collect()
    }
}
