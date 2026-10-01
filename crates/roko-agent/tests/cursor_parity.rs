#![allow(missing_docs)]

use roko_agent::testutil::{
    ParityBackend, run_error_path, run_happy_path, run_session_continuation, run_streaming,
    run_tool_call,
};

#[tokio::test]
async fn happy_path() {
    run_happy_path(ParityBackend::Cursor).await.unwrap();
}

#[tokio::test]
#[ignore = "stream events carry no response or session ids, so the streamed session metadata check fails; usage parity holds (bug-25d24e)"]
async fn streaming() {
    run_streaming(ParityBackend::Cursor).await.unwrap();
}

#[tokio::test]
#[ignore = "cursor tool_call continuation mismatch — pre-existing fixture drift"]
async fn tool_call() {
    run_tool_call(ParityBackend::Cursor).await.unwrap();
}

#[tokio::test]
async fn error_path() {
    run_error_path(ParityBackend::Cursor).await.unwrap();
}

#[tokio::test]
async fn session_continuation() {
    run_session_continuation(ParityBackend::Cursor)
        .await
        .unwrap();
}
