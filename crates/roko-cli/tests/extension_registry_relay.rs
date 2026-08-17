//! Authenticated relay publisher and recursive verified installer integration tests.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_relay::registry::{RegistryPublisherConfig, RegistryStore};
use agent_relay::{app, state::RelayState};
use base64::Engine as _;
use ed25519_dalek::SigningKey;
use roko_cli::runner::extension_registry::{
    install_registry_extension, install_registry_extension_requirement, publish_registry_extension,
};
use roko_plugin::registry::{build_signed_package, validate_signed_package};
use sha2::{Digest, Sha256};

struct TestRegistry {
    base_url: String,
    token: String,
    signing_key_secret: String,
    _storage: tempfile::TempDir,
    task: tokio::task::JoinHandle<()>,
}

impl TestRegistry {
    async fn spawn() -> Self {
        let storage = tempfile::tempdir().unwrap();
        let token = "publisher-test-token".to_string();
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let signing_key_secret = base64::engine::general_purpose::STANDARD.encode([7; 32]);
        let publisher = RegistryPublisherConfig {
            id: "test-publisher".to_string(),
            token_sha256: format!("{:x}", Sha256::digest(token.as_bytes())),
            public_key: base64::engine::general_purpose::STANDARD
                .encode(signing_key.verifying_key().as_bytes()),
        };
        let registry = RegistryStore::open(storage.path(), vec![publisher]).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let state = Arc::new(RelayState::with_registry(registry));
        let task = tokio::spawn(async move {
            axum::serve(listener, app(state)).await.unwrap();
        });
        Self {
            base_url: format!("http://{address}"),
            token,
            signing_key_secret,
            _storage: storage,
            task,
        }
    }
}

impl Drop for TestRegistry {
    fn drop(&mut self) {
        self.task.abort();
    }
}

fn write_extension(root: &Path, name: &str, version: &str, dependencies: &[&str]) -> PathBuf {
    let requirements = dependencies
        .iter()
        .map(|dependency| (*dependency, "*"))
        .collect::<Vec<_>>();
    write_extension_with_requirements(root, name, version, &requirements)
}

fn write_extension_with_requirements(
    root: &Path,
    name: &str,
    version: &str,
    dependencies: &[(&str, &str)],
) -> PathBuf {
    let directory = root.join(format!("{name}-{version}"));
    std::fs::create_dir_all(&directory).unwrap();
    let dependency_names = dependencies
        .iter()
        .map(|(dependency, _)| format!("\"{dependency}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let dependency_requirements = dependencies
        .iter()
        .map(|(dependency, requirement)| format!("{dependency} = \"{requirement}\""))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(
        directory.join("extension.toml"),
        format!(
            r#"[extension]
name = "{name}"
version = "{version}"
layer = "cognition"
tier = "wasm"
depends_on = [{dependency_names}]

[extension.dependency_requirements]
{dependency_requirements}

[extension.config]
module = "hook.wasm"
capabilities = []
hooks = ["on_init"]
"#,
        ),
    )
    .unwrap();
    let wasm = wat::parse_str(
        r#"(module
            (memory (export "memory") 1)
            (func (export "roko_alloc") (param i32) (result i32) i32.const 4096)
            (data (i32.const 0) "\6e\75\6c\6c")
            (func (export "on_init") (param i32 i32) (result i64) i64.const 4))"#,
    )
    .unwrap();
    std::fs::write(directory.join("hook.wasm"), wasm).unwrap();
    directory
}

async fn publish(
    registry: &TestRegistry,
    extension: PathBuf,
) -> Result<roko_plugin::registry::RegistryPackage, String> {
    let base = registry.base_url.clone();
    let token = registry.token.clone();
    let secret = registry.signing_key_secret.clone();
    tokio::task::spawn_blocking(move || {
        publish_registry_extension(&extension, "test-publisher", &token, &secret, &base)
            .map_err(|error| error.to_string())
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relay_publish_and_recursive_verified_install_round_trip() {
    let registry = TestRegistry::spawn().await;
    let source = tempfile::tempdir().unwrap();
    let dependency = write_extension(source.path(), "dependency", "1.0.0", &[]);
    let root = write_extension(source.path(), "root", "2.0.0", &["dependency"]);

    let root_before_dependency = {
        let root = root.clone();
        let base = registry.base_url.clone();
        let token = registry.token.clone();
        let secret = registry.signing_key_secret.clone();
        tokio::task::spawn_blocking(move || {
            publish_registry_extension(&root, "test-publisher", &token, &secret, &base)
                .map_err(|error| error.to_string())
        })
        .await
        .unwrap()
    };
    assert!(
        root_before_dependency
            .unwrap_err()
            .contains("unpublished dependency")
    );

    for extension in [dependency, root.clone()] {
        let base = registry.base_url.clone();
        let token = registry.token.clone();
        let secret = registry.signing_key_secret.clone();
        let package = tokio::task::spawn_blocking(move || {
            publish_registry_extension(&extension, "test-publisher", &token, &secret, &base)
                .map_err(|error| error.to_string())
        })
        .await
        .unwrap()
        .unwrap();
        validate_signed_package(&package).unwrap();
    }

    let root_path = root.clone();
    let base = registry.base_url.clone();
    let token = registry.token.clone();
    let secret = registry.signing_key_secret.clone();
    tokio::task::spawn_blocking(move || {
        publish_registry_extension(&root_path, "test-publisher", &token, &secret, &base)
            .map_err(|error| error.to_string())
    })
    .await
    .unwrap()
    .expect("identical publish should be idempotent");

    std::fs::OpenOptions::new()
        .append(true)
        .open(root.join("extension.toml"))
        .unwrap()
        .write_all(b"\ndescription = \"changed immutable package\"\n")
        .unwrap();
    let base = registry.base_url.clone();
    let token = registry.token.clone();
    let secret = registry.signing_key_secret.clone();
    let conflict = tokio::task::spawn_blocking(move || {
        publish_registry_extension(&root, "test-publisher", &token, &secret, &base)
            .map_err(|error| error.to_string())
    })
    .await
    .unwrap()
    .unwrap_err();
    assert!(conflict.contains("409 Conflict"));

    let install_root = tempfile::tempdir().unwrap();
    let workdir = install_root.path().to_path_buf();
    let base = registry.base_url.clone();
    let installed = tokio::task::spawn_blocking(move || {
        install_registry_extension(&workdir, "root", &base).map_err(|error| error.to_string())
    })
    .await
    .unwrap()
    .unwrap();
    assert!(installed.join("extension.toml").is_file());
    assert!(
        install_root
            .path()
            .join(".roko/extensions/dependency/extension.toml")
            .is_file()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relay_publish_rejects_wrong_bearer_and_signing_key() {
    let registry = TestRegistry::spawn().await;
    let source = tempfile::tempdir().unwrap();
    let extension = write_extension(source.path(), "rejected", "1.0.0", &[]);
    let base = registry.base_url.clone();
    let secret = registry.signing_key_secret.clone();
    let wrong_token = tokio::task::spawn_blocking(move || {
        publish_registry_extension(&extension, "test-publisher", "wrong-token", &secret, &base)
            .map_err(|error| error.to_string())
    })
    .await
    .unwrap()
    .unwrap_err();
    assert!(wrong_token.contains("401 Unauthorized"));

    let extension = write_extension(source.path(), "wrong-key", "1.0.0", &[]);
    let package = build_signed_package(
        &extension,
        "test-publisher",
        &SigningKey::from_bytes(&[9; 32]),
    )
    .unwrap();
    let response = reqwest::Client::new()
        .post(format!("{}/registry/extensions", registry.base_url))
        .bearer_auth(&registry.token)
        .json(&package)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn relay_enforces_ranges_selects_recursively_and_rejects_conflicts_and_cycles() {
    let registry = TestRegistry::spawn().await;
    let source = tempfile::tempdir().unwrap();

    let shared_v1 = write_extension(source.path(), "shared", "1.5.0", &[]);
    let shared_v2 = write_extension(source.path(), "shared", "2.1.0", &[]);
    publish(&registry, shared_v1).await.unwrap();
    publish(&registry, shared_v2).await.unwrap();
    let selected: roko_plugin::registry::RegistryPackage = reqwest::Client::new()
        .get(format!("{}/registry/extensions/shared", registry.base_url))
        .query(&[("requirement", "^1.0")])
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(selected.version, "1.5.0");

    let missing = write_extension_with_requirements(
        source.path(),
        "missing-range",
        "1.0.0",
        &[("shared", "^3.0")],
    );
    let error = publish(&registry, missing).await.unwrap_err();
    assert!(error.contains("unpublished dependency `shared` matching `^3.0`"));

    let left =
        write_extension_with_requirements(source.path(), "left", "1.0.0", &[("shared", "^1.0")]);
    let right =
        write_extension_with_requirements(source.path(), "right", "1.0.0", &[("shared", "^2.0")]);
    publish(&registry, left).await.unwrap();
    publish(&registry, right).await.unwrap();

    let conflicting = write_extension(
        source.path(),
        "conflicting-root",
        "1.0.0",
        &["left", "right"],
    );
    let error = publish(&registry, conflicting).await.unwrap_err();
    assert!(error.contains("409 Conflict"));
    assert!(error.contains("requires `shared`"));

    let compatible = write_extension_with_requirements(
        source.path(),
        "compatible-root",
        "1.0.0",
        &[("shared", "^1.0")],
    );
    publish(&registry, compatible).await.unwrap();
    let install_root = tempfile::tempdir().unwrap();
    let workdir = install_root.path().to_path_buf();
    let base = registry.base_url.clone();
    tokio::task::spawn_blocking(move || {
        install_registry_extension_requirement(&workdir, "compatible-root", "^1", &base)
            .map_err(|error| error.to_string())
    })
    .await
    .unwrap()
    .unwrap();
    let installed_shared = roko_cli::runner::extension_loader::load_extension_manifest(
        &install_root
            .path()
            .join(".roko/extensions/shared/extension.toml"),
    )
    .unwrap();
    assert_eq!(installed_shared.version, "1.5.0");

    let workdir = install_root.path().to_path_buf();
    let base = registry.base_url.clone();
    let installed_conflict = tokio::task::spawn_blocking(move || {
        install_registry_extension(&workdir, "right", &base).map_err(|error| error.to_string())
    })
    .await
    .unwrap()
    .unwrap_err();
    assert!(installed_conflict.contains("selected shared@2.1.0, but installed shared@1.5.0"));
    assert!(!install_root.path().join(".roko/extensions/right").exists());

    let cycle_b_v1 = write_extension(source.path(), "cycle-b", "1.0.0", &[]);
    publish(&registry, cycle_b_v1).await.unwrap();
    let cycle_a = write_extension(source.path(), "cycle-a", "1.0.0", &["cycle-b"]);
    publish(&registry, cycle_a).await.unwrap();
    let cycle_b_v2 = write_extension(source.path(), "cycle-b", "2.0.0", &["cycle-a"]);
    let error = publish(&registry, cycle_b_v2).await.unwrap_err();
    assert!(error.contains("409 Conflict"));
    assert!(error.contains("dependency cycle detected"));
}
