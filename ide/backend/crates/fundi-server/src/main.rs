use std::net::SocketAddr;

use fundi_config::load_config;
use fundi_core::AgentError;
use fundi_telemetry::init_telemetry_from_env;

#[tokio::main]
async fn main() -> Result<(), AgentError> {
    init_telemetry_from_env("fundi-server", "info");

    let config = load_config(None).map_err(|e| AgentError::Config(e.to_string()))?;
    let addr: SocketAddr = std::env::var("FUNDI_SERVER_ADDR")
        .or_else(|_| std::env::var("FUNDI_DASHBOARD_ADDR"))
        .or_else(|_| std::env::var("FUNDI_HTTP_ADDR"))
        .unwrap_or_else(|_| "127.0.0.1:8787".to_string())
        .parse()
        .map_err(|e| AgentError::Config(format!("invalid server addr: {}", e)))?;

    fundi_server::run_server(addr, config).await
}
