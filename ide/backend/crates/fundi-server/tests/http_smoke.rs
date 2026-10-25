use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

#[tokio::test]
async fn health_ok() {
    let _ = tracing_subscriber::fmt::try_init();
    let cfg = fundi_config::GatewayConfig::default();
    let state = fundi_server::HttpServerState::build(cfg).await.unwrap();
    let app = fundi_server::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn metrics_prometheus_has_counters() {
    let _ = tracing_subscriber::fmt::try_init();
    let cfg = fundi_config::GatewayConfig::default();
    let state = fundi_server::HttpServerState::build(cfg).await.unwrap();
    let app = fundi_server::router(state);
    let res = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let s = String::from_utf8(body.to_vec()).unwrap();
    assert!(s.contains("fundi_llm_requests_total"));
}

#[tokio::test]
async fn oauth_stub_routes_return_json_contract() {
    let _ = tracing_subscriber::fmt::try_init();
    let cfg = fundi_config::GatewayConfig::default();
    let state = fundi_server::HttpServerState::build(cfg).await.unwrap();
    let app = fundi_server::router(state);

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/providers/oauth")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(v.get("providers").is_some());
    assert!(v.get("connected").is_some());

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/providers/oauth/openai")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v.get("ok"), Some(&serde_json::json!(true)));
    assert_eq!(v.get("provider"), Some(&serde_json::json!("openai")));

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/providers/oauth/openai/start")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v.get("flow"), Some(&serde_json::json!("device_code")));
    assert!(v.get("session_id").and_then(|x| x.as_str()).is_some());

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/providers/oauth/openai/poll/sess-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v.get("status"), Some(&serde_json::json!("error")));

    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/providers/oauth/openai/submit")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({"session_id": "s1", "code": "abc"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v.get("ok"), Some(&serde_json::json!(false)));
    assert_eq!(v.get("status"), Some(&serde_json::json!("error")));

    let res = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/providers/oauth/sessions/sess-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v.get("ok"), Some(&serde_json::json!(true)));
}

#[tokio::test]
#[ignore = "requires live provider credentials to pass reliably"]
async fn command_help_runs_through_gateway() {
    let _ = tracing_subscriber::fmt::try_init();
    let cfg = fundi_config::GatewayConfig::default();
    let state = fundi_server::HttpServerState::build(cfg).await.unwrap();
    let app = fundi_server::router(state);
    let payload = serde_json::json!({ "command": "/help" });
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/commands")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // The command endpoint should accept the request
    // Note: Without a real LLM provider configured, it might fail with 500
    // But the endpoint itself should at least accept the request
    assert_eq!(res.status(), StatusCode::OK);

    let body = res.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // The response should have the expected structure
    assert!(v.get("accepted").is_some());
    assert!(v.get("output").is_some());
}
