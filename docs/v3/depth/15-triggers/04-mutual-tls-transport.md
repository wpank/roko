# Depth: Mutual TLS Transport

> Parent: [15-TRIGGERS](../../15-TRIGGERS.md) -- Section 9

---

## Overview

Webhook bindings that require mutual TLS (mTLS) authentication use a
dedicated HTTPS transport layer implemented in
`crates/roko-serve/src/trigger_tls.rs`. This transport provides
cryptographic verification of webhook callers using X.509 client
certificates, ensuring that only callers presenting a certificate signed
by a trusted CA can submit events.

## Design constraints

1. **Shared server identity**: TLS authentication happens before HTTP route
   selection (at the connection level, not the request level). All mTLS
   webhook bindings must share the same server certificate, private key,
   and client CA trust anchor. The loader validates this with an explicit
   `anyhow::ensure!`.

2. **Optional client authentication**: The `WebPkiClientVerifier` is
   configured with `allow_unauthenticated()` so that ordinary (non-mTLS)
   HTTP routes remain reachable on the same TLS listener. The mTLS webhook
   route itself rejects requests without a verified client identity.

3. **Explicit crypto provider**: The `aws_lc_rs` provider is selected
   explicitly via `rustls::crypto::aws_lc_rs::default_provider()`, never
   relying on feature-based auto-selection, because the workspace dependency
   graph enables both rustls providers through different transitive paths.

## Loading: `trigger_tls::load`

```rust
pub(crate) async fn load(state: &AppState) -> Result<Option<TriggerTlsConfig>>
```

1. Iterates all trigger bindings and extracts `MutualTlsMaterial` from those
   with `TriggerAuth::MutualTls` auth.
2. If no mTLS bindings exist, returns `None`.
3. Validates that all mTLS bindings share identical cert, key, and client_ca.
4. Reads the server certificate PEM and client CA PEM from workspace-relative
   paths (resolved against `state.workdir`).
5. Resolves the private key from the `SecretRef` (env, store, or file).
6. Calls `build_config` to construct the TLS stack.

## Config construction: `build_config`

```rust
fn build_config(
    certificate_pem: &[u8],
    key_pem: &[u8],
    client_ca_pem: &[u8],
) -> Result<TriggerTlsConfig>
```

1. Parse server certificate chain from PEM.
2. Parse private key from PEM.
3. Parse client CA certificates and build a `RootCertStore`.
4. Build a `WebPkiClientVerifier` with `allow_unauthenticated()`.
5. Build a `rustls::ServerConfig` with the server cert/key and client verifier.
6. Wrap in a `TokioTlsAcceptor`.

## Serving: `trigger_tls::serve`

```rust
pub(crate) async fn serve(
    listener: TcpListener,
    router: Router,
    cancel: CancelToken,
    tls: TriggerTlsConfig,
) -> Result<()>
```

The serve function runs a connection accept loop:

1. Accept a TCP connection.
2. Perform TLS handshake via the `TlsAcceptor`.
3. Extract the peer's verified client certificate chain from the rustls
   session.
4. Compute the SHA-256 fingerprint of the leaf certificate.
5. Attach a `VerifiedClientIdentity` extension to the request.
6. Route through the Axum router.

```rust
pub(crate) struct VerifiedClientIdentity {
    pub certificate_sha256: String,  // hex-encoded SHA-256 of leaf cert
}
```

Failed TLS handshakes are logged at debug level and the connection is
dropped. This is intentional: failed handshakes should not generate warnings
during normal operation (e.g. health checks from load balancers that do not
present client certificates).

## Webhook route enforcement

The mTLS webhook route handler checks for the `VerifiedClientIdentity`
extension on the request. Requests without this extension (i.e. connections
that did not present a client certificate) are rejected with a 403 status.
The extension is only present when the rustls session successfully verified
a client certificate against the configured CA trust anchor.

**Source:** `crates/roko-serve/src/trigger_tls.rs`
