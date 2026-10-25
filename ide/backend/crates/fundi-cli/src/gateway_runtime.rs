//! gateway_runtime.rs — RUNTIME HALISI ya `fundi gateway start`.
//!
//! Hii inaendesha: (1) cron scheduler inayoendesha jobs zilizopangwa, (2) HTTP
//! webhook delivery ya `CronCompletionEvent` → `webhooks.json`, na (3) platform
//! adapters zilizowekwa (Telegram/Discord/Slack/…) kama zimewashwa. Awali
//! `gateway start` ilikuwa stub: iliandika PID, ikafuta, ikarudi bila kufanya
//! kazi. Sasa inaendesha hadi SIGINT/SIGTERM.
//!
//! HITL: kama `$FUNDI_HOME/gateway.consent` haipo, tunarun kwa mode salama —
//! cron jobs zinaendesha (zilizoidhinishwa kwa `fundi cron create`) lakini
//! adapters za platform hazianzishwi. Weka faili hiyo (au `--allow-platforms`)
//! kuwasha kutuma/kupokea ujumbe wa nje.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use fundi_core::AgentError;
use fundi_cron::{CronRunner, CronScheduler, FileJobPersistence};

use crate::live_messaging;

/// RUNTIME HALISI ya gateway: cron scheduler + webhook delivery + adapters.
pub struct GatewayRuntime {
    scheduler: Arc<CronScheduler>,
    shutdown: Arc<tokio::sync::Notify>,
    pid_path: PathBuf,
    /// Retains the gateway (and thus its platform adapters) for the runtime's
    /// lifetime. Dropping it would stop every adapter immediately.
    _gateway: Option<Arc<fundi_gateway::gateway::Gateway>>,
}

impl GatewayRuntime {
    /// Jenga runtime kwa `state_dir` (kawaida `$FUNDI_HOME`).
    ///
    /// - `allow_platforms`: washa adapters za nje (Telegram n.k.) badala ya mode salama.
    pub async fn build(
        state_dir: PathBuf,
        config: &fundi_config::GatewayConfig,
        allow_platforms: bool,
    ) -> Result<Self, AgentError> {
        let cron_dir = state_dir.join("cron");
        std::fs::create_dir_all(&cron_dir)
            .map_err(|e| AgentError::Io(format!("cron dir {}: {}", cron_dir.display(), e)))?;

        // Provider halisi kutoka config (StubProvider bila API key → job inafeli
        // kama inavyotarajiwa; webhook bado inatumwa).
        let model = config.model.clone().unwrap_or_else(|| "gpt-4o".to_string());
        let provider = fundi_agent::agent_builder::build_provider(config, &model);
        let tool_registry = Arc::new(fundi_agent::agent_loop::ToolRegistry::new());
        let runner = Arc::new(CronRunner::new(provider, tool_registry));
        let persistence = Arc::new(FileJobPersistence::with_dir(cron_dir));
        let mut scheduler = CronScheduler::new(persistence, runner);
        scheduler
            .load_persisted_jobs()
            .await
            .map_err(|e| AgentError::Config(format!("load cron jobs: {e}")))?;

        // Cron completion → webhooks.json (best-effort HTTP delivery).
        let (tx, mut rx) = tokio::sync::broadcast::channel(256);
        scheduler.set_completion_broadcast(tx);
        let webhooks_json = state_dir.join("webhooks.json");
        tokio::spawn(async move {
            let client = match crate::webhook_delivery::webhook_http_client() {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("webhook client: {e}");
                    return;
                }
            };
            while let Ok(ev) = rx.recv().await {
                if let Err(e) = crate::webhook_delivery::deliver_cron_completion_to_webhooks(
                    &webhooks_json,
                    &ev,
                    &client,
                )
                .await
                {
                    tracing::warn!("webhook delivery: {e}");
                }
            }
        });

        // Platform adapters (outbound). Mode salama: hazianzishwi bila consent.
        let consented = allow_platforms || state_dir.join("gateway.consent").exists();
        let mut gateway_handle: Option<Arc<fundi_gateway::gateway::Gateway>> = None;
        if consented {
            match live_messaging::build_outbound_gateway(config).await {
                Ok((gw, names)) => {
                    if let Err(e) = gw.start_all().await {
                        tracing::warn!("gateway adapters start: {e}");
                    } else {
                        tracing::info!("gateway platform adapters started: {}", names.join(", "));
                        gateway_handle = Some(gw);
                    }
                }
                Err(e) => tracing::warn!("gateway adapters: {e}"),
            }
        } else {
            tracing::info!(
                "gateway: mode salama (adapters hazijawashwa). Weka {} au tumia --allow-platforms.",
                state_dir.join("gateway.consent").display()
            );
        }

        Ok(Self {
            scheduler: Arc::new(scheduler),
            shutdown: Arc::new(tokio::sync::Notify::new()),
            pid_path: state_dir.join("gateway.pid"),
            _gateway: gateway_handle,
        })
    }

    /// Anza scheduler na subiri SIGINT/SIGTERM, kisha toa PID file.
    pub async fn run(self) -> Result<(), AgentError> {
        write_pid(&self.pid_path)?;
        self.scheduler.start().await;
        tracing::info!("gateway runtime started (cron scheduler active)");

        wait_for_shutdown().await;
        tracing::info!("gateway runtime: shutdown signal received");

        self.scheduler.stop().await;
        if let Some(gw) = &self._gateway {
            let _ = gw.stop_all().await;
        }
        let _ = std::fs::remove_file(&self.pid_path);
        Ok(())
    }

    /// Notify shutdown (kwa tests/CLI).
    pub fn shutdown(&self) {
        self.shutdown.notify_waiters();
    }
}

fn write_pid(pid_path: &Path) -> Result<(), AgentError> {
    if let Some(parent) = pid_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(pid_path, format!("{}\n", std::process::id()))
        .map_err(|e| AgentError::Io(format!("write PID {}: {}", pid_path.display(), e)))
}

#[cfg(unix)]
async fn wait_for_shutdown() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigint = match signal(SignalKind::interrupt()) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("sigint handler: {e}");
            return;
        }
    };
    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("sigterm handler: {e}");
            return;
        }
    };
    tokio::select! {
        _ = sigint.recv() => tracing::info!("SIGINT"),
        _ = sigterm.recv() => tracing::info!("SIGTERM"),
    }
}

#[cfg(not(unix))]
async fn wait_for_shutdown() {
    let _ = tokio::signal::ctrl_c().await;
}
