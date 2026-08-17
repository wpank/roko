//! TLS transport for webhook bindings that require mutual authentication.

use std::fmt::Write as _;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use hyper::Request;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use roko_core::trigger::{TriggerAuth, TriggerBinding};
use rustls::RootCertStore;
use rustls::server::WebPkiClientVerifier;
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::task::JoinSet;
use tokio_rustls::TlsAcceptor;
use tower::ServiceExt;
use tracing::{debug, warn};

use crate::state::AppState;
use crate::trigger_runtime::resolve_trigger_secret;
use roko_runtime::cancel::CancelToken;

/// Cryptographically verified TLS client identity attached to one HTTP connection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct VerifiedClientIdentity {
    /// SHA-256 fingerprint of the verified leaf certificate.
    pub certificate_sha256: String,
}

#[derive(Clone)]
pub(crate) struct TriggerTlsConfig {
    acceptor: TlsAcceptor,
}

struct MutualTlsMaterial {
    cert: PathBuf,
    key_pem: String,
    client_ca: PathBuf,
}

/// Build the HTTPS transport required by mTLS webhook bindings.
///
/// Every mTLS binding must share one server identity and client CA because
/// TLS authentication happens before HTTP route selection. Client
/// certificates are optional at the handshake so ordinary routes remain
/// reachable; the mTLS webhook route itself rejects requests without the
/// verified identity extension.
pub(crate) async fn load(state: &AppState) -> Result<Option<TriggerTlsConfig>> {
    let bindings = state.trigger_bindings.read().await;
    let materials = bindings
        .values()
        .filter_map(|binding| binding_mtls_material(state, binding).transpose())
        .collect::<Result<Vec<_>>>()?;
    drop(bindings);
    let Some(first) = materials.first() else {
        return Ok(None);
    };
    for material in materials.iter().skip(1) {
        anyhow::ensure!(
            material.cert == first.cert
                && material.key_pem == first.key_pem
                && material.client_ca == first.client_ca,
            "all mTLS webhook bindings must share the same cert, key, and client_ca"
        );
    }

    let certificate_pem = read_workspace_file(&state.workdir, &first.cert)?;
    let client_ca_pem = read_workspace_file(&state.workdir, &first.client_ca)?;
    build_config(&certificate_pem, first.key_pem.as_bytes(), &client_ca_pem).map(Some)
}

fn build_config(
    certificate_pem: &[u8],
    key_pem: &[u8],
    client_ca_pem: &[u8],
) -> Result<TriggerTlsConfig> {
    let mut certificate_reader = Cursor::new(certificate_pem);
    let certificates = rustls_pemfile::certs(&mut certificate_reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("parse mTLS server certificate chain")?;
    anyhow::ensure!(
        !certificates.is_empty(),
        "mTLS server cert contains no certificates"
    );

    let mut key_reader = Cursor::new(key_pem);
    let key = rustls_pemfile::private_key(&mut key_reader)
        .context("parse mTLS server private key")?
        .context("mTLS server key contains no private key")?;

    let mut client_ca_reader = Cursor::new(client_ca_pem);
    let client_cas = rustls_pemfile::certs(&mut client_ca_reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("parse mTLS client CA certificates")?;
    anyhow::ensure!(
        !client_cas.is_empty(),
        "mTLS client_ca contains no certificates"
    );
    let mut roots = RootCertStore::empty();
    for certificate in client_cas {
        roots
            .add(certificate)
            .context("add mTLS client CA trust anchor")?;
    }
    // The workspace dependency graph enables both rustls providers (directly
    // and through reqwest), so never rely on feature-based auto-selection.
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let verifier =
        WebPkiClientVerifier::builder_with_provider(Arc::new(roots), Arc::clone(&provider))
            .allow_unauthenticated()
            .build()
            .context("build mTLS client certificate verifier")?;
    let server = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .context("select mTLS protocol versions")?
        .with_client_cert_verifier(verifier)
        .with_single_cert(certificates, key)
        .context("build mTLS server configuration")?;
    Ok(TriggerTlsConfig {
        acceptor: TlsAcceptor::from(Arc::new(server)),
    })
}

fn binding_mtls_material(
    state: &AppState,
    binding: &TriggerBinding,
) -> Result<Option<MutualTlsMaterial>> {
    let Some(TriggerAuth::MutualTls {
        cert,
        key,
        client_ca,
    }) = binding.auth.as_ref()
    else {
        return Ok(None);
    };
    Ok(Some(MutualTlsMaterial {
        cert: cert.clone(),
        key_pem: resolve_trigger_secret(state, key)?,
        client_ca: client_ca.clone(),
    }))
}

fn read_workspace_file(workdir: &Path, path: &Path) -> Result<Vec<u8>> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workdir.join(path)
    };
    std::fs::read(&path).with_context(|| format!("read TLS material {}", path.display()))
}

/// Serve an Axum router over TLS and attach verified peer identities directly
/// from the rustls session to each request.
pub(crate) async fn serve(
    listener: TcpListener,
    router: Router,
    cancel: CancelToken,
    tls: TriggerTlsConfig,
) -> Result<()> {
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            accepted = listener.accept() => {
                let (stream, remote_addr) = accepted.context("accept HTTPS connection")?;
                let acceptor = tls.acceptor.clone();
                let router = router.clone();
                connections.spawn(async move {
                    let stream = match acceptor.accept(stream).await {
                        Ok(stream) => stream,
                        Err(error) => {
                            debug!(%remote_addr, %error, "TLS client handshake rejected");
                            return;
                        }
                    };
                    let identity = stream
                        .get_ref()
                        .1
                        .peer_certificates()
                        .and_then(|chain| chain.first())
                        .map(|certificate| VerifiedClientIdentity {
                            certificate_sha256: bytes_hex(&Sha256::digest(certificate.as_ref())),
                        });
                    let service = service_fn(move |mut request: Request<Incoming>| {
                        let router = router.clone();
                        let identity = identity.clone();
                        async move {
                            request.extensions_mut().insert(ConnectInfo(remote_addr));
                            if let Some(identity) = identity {
                                request.extensions_mut().insert(identity);
                            }
                            router.oneshot(request.map(Body::new)).await
                        }
                    });
                    let io = TokioIo::new(stream);
                    if let Err(error) = Builder::new(TokioExecutor::new())
                        .serve_connection_with_upgrades(io, service)
                        .await
                    {
                        debug!(%remote_addr, %error, "HTTPS connection closed with error");
                    }
                });
            }
        }
    }
    connections.abort_all();
    while let Some(result) = connections.join_next().await {
        if let Err(error) = result
            && !error.is_cancelled()
        {
            warn!(%error, "HTTPS connection task failed");
        }
    }
    Ok(())
}

fn bytes_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Extension;
    use axum::http::StatusCode;
    use axum::routing::get;
    use rcgen::{BasicConstraints, CertificateParams, ExtendedKeyUsagePurpose, IsCa, KeyPair};

    struct Certificates {
        ca: String,
        server_cert: String,
        server_key: String,
        client_identity: String,
    }

    fn certificates() -> Certificates {
        let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
        ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        let ca_key = KeyPair::generate().unwrap();
        let ca = ca_params.self_signed(&ca_key).unwrap();

        let mut server_params = CertificateParams::new(vec!["localhost".to_string()]).unwrap();
        server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        let server_key = KeyPair::generate().unwrap();
        let server = server_params.signed_by(&server_key, &ca, &ca_key).unwrap();

        let mut client_params = CertificateParams::new(vec!["trigger-client".to_string()]).unwrap();
        client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
        let client_key = KeyPair::generate().unwrap();
        let client = client_params.signed_by(&client_key, &ca, &ca_key).unwrap();
        Certificates {
            ca: ca.pem(),
            server_cert: server.pem(),
            server_key: server_key.serialize_pem(),
            client_identity: format!("{}{}", client.pem(), client_key.serialize_pem()),
        }
    }

    #[tokio::test]
    async fn tls_transport_only_injects_identity_for_ca_verified_client() {
        let certificates = certificates();
        let tls = build_config(
            certificates.server_cert.as_bytes(),
            certificates.server_key.as_bytes(),
            certificates.ca.as_bytes(),
        )
        .expect("TLS config");
        let router = Router::new().route(
            "/identity",
            get(
                |identity: Option<Extension<VerifiedClientIdentity>>| async move {
                    if identity.is_some() {
                        StatusCode::OK
                    } else {
                        StatusCode::UNAUTHORIZED
                    }
                },
            ),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let cancel = CancelToken::new();
        let server_cancel = cancel.clone();
        let server = tokio::spawn(async move { serve(listener, router, server_cancel, tls).await });

        let root = reqwest::Certificate::from_pem(certificates.ca.as_bytes()).unwrap();
        let anonymous = reqwest::Client::builder()
            .use_rustls_tls()
            .add_root_certificate(root.clone())
            .build()
            .unwrap();
        let url = format!("https://localhost:{}/identity", address.port());
        assert_eq!(
            anonymous.get(&url).send().await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );

        let identity =
            reqwest::Identity::from_pem(certificates.client_identity.as_bytes()).unwrap();
        let authenticated = reqwest::Client::builder()
            .use_rustls_tls()
            .add_root_certificate(root)
            .identity(identity)
            .build()
            .unwrap();
        assert_eq!(
            authenticated.get(&url).send().await.unwrap().status(),
            StatusCode::OK
        );

        cancel.cancel();
        server.await.unwrap().unwrap();
    }
}
