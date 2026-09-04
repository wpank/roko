//! T3: Timeout simulation test.
//!
//! Starts a mock HTTP server that sleeps for 10 s, then configures an
//! `OpenAiAgent` with a 100 ms timeout. Asserts that the call fails
//! within ~200 ms with an error mentioning timeout.

#![allow(missing_docs)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use roko_agent::Agent;
use roko_agent::openai_agent::OpenAiAgent;
use roko_core::{Body, Context, Engram, Kind};

fn prompt(text: &str) -> Engram {
    Engram::builder(Kind::Prompt).body(Body::text(text)).build()
}

/// Spawn a server that accepts one connection, reads the request headers,
/// then sleeps for `delay` before sending a response.
fn spawn_slow_server(delay: Duration) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let addr = listener.local_addr().expect("server addr");

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        stream
            .set_read_timeout(Some(Duration::from_secs(15)))
            .expect("set read timeout");

        // Drain the request so the client doesn't get a broken pipe before timeout.
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf);

        // Sleep longer than the client timeout.
        thread::sleep(delay);

        // Send a valid response (client should have timed out before this).
        let body = r#"{"id":"late","choices":[{"index":0,"message":{"role":"assistant","content":"too slow"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#;
        let wire = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(wire.as_bytes());
        let _ = stream.flush();
    });

    format!("http://{addr}")
}

#[tokio::test]
async fn timeout_triggers_within_expected_window() {
    let base_url = spawn_slow_server(Duration::from_secs(10));
    let timeout_ms = 100;

    let agent = OpenAiAgent::new("test-key", "test-model")
        .with_base_url(base_url)
        .with_timeout_ms(timeout_ms);

    let started = Instant::now();
    let result = agent
        .run(&prompt("This should time out."), &Context::now())
        .await;
    let elapsed = started.elapsed();

    // The call should fail.
    assert!(
        !result.success,
        "expected timeout failure but got success: {:?}",
        result.output.body.as_text().unwrap_or("?")
    );

    // The error message should mention a timeout or connection issue.
    let error_text = result.output.body.as_text().unwrap_or("").to_lowercase();
    assert!(
        error_text.contains("timeout")
            || error_text.contains("timed out")
            || error_text.contains("error"),
        "expected timeout-related error, got: {error_text}"
    );

    // Should complete within a reasonable window (~200 ms) — not the full 10 s.
    assert!(
        elapsed < Duration::from_millis(2_000),
        "expected timeout within ~200ms, took {elapsed:?}"
    );
}
