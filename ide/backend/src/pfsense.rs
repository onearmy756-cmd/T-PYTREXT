//! pfsense.rs — PFSENSE FIREWALL API (H5b): udhibiti wa firewall ya mtandao mmoja.
//!
//! MTECH OS inasimamia mtandao mmoja (LAN + WireGuard wg0). pfSense ndiyo
//! firewall ya perimeter — kupitia **REST API halisi ya pfSense** (pfSense REST
//! API package: /api/v1/*) mfumo unaweza:
//!   - kupata/kuongeza **filter rules** (ruhusa za OS install, TFTP, HTTP, SMB…)
//!   - kupata/kuongeza **aliases** (vikundi vya hosts/ports za kompyuta)
//!   - **restart services** (dnsmasq, dhcpd…) wakati wa deployment kubwa
//!   - kuona **status** ya firewall
//!
//! KANUNI: client hii ni HALISI — inatumia reqwest (rustls) dhidi ya API ya
//! kweli. Bila PFSENSE_API_URL + PFSENSE_API_KEY (env), kila simu inarudisha
//! **error ya configuration** — hakuna majibu ya uongo.

use serde::Deserialize;
use std::time::Duration;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone)]
pub struct PfClient {
    base_url: Option<String>,
    api_key: Option<String>,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // inatumika kwenye tests + reference ya muundo wa majibu ya API
struct PfEnvelope<T> {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    data: Option<T>,
    #[serde(default)]
    message: Option<String>,
}

impl PfClient {
    /// Kutoka env: PFSENSE_API_URL (mf. https://10.66.66.1) + PFSENSE_API_KEY.
    /// Kwenye docker-compose: PFSENSE_API_URL=${PFSENSE_API_URL:-}
    pub fn from_env() -> PfClient {
        let base_url = std::env::var("PFSENSE_API_URL")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let api_key = std::env::var("PFSENSE_API_KEY")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let http = reqwest::Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .danger_accept_invalid_certs(true) // pfSense kwa kawaida inatumia self-signed cert ya LAN
            .build()
            .unwrap_or_default();
        PfClient {
            base_url,
            api_key,
            http,
        }
    }

    pub fn is_configured(&self) -> bool {
        self.base_url.is_some() && self.api_key.is_some()
    }

    fn require(&self) -> Result<(&str, &str), String> {
        match (&self.base_url, &self.api_key) {
            (Some(url), Some(key)) => Ok((url.as_str(), key.as_str())),
            _ => Err("pfSense API haijawekwa: weka PFSENSE_API_URL na PFSENSE_API_KEY (env) — mf. https://10.66.66.1 + client-auth key ya pfSense REST API".into()),
        }
    }

    /// GET /api/v1/<path> — inarudisha JSON ghafi ya API.
    async fn get(&self, path: &str) -> Result<serde_json::Value, String> {
        let (base, key) = self.require()?;
        let url = format!(
            "{}/api/v1/{}",
            base.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let resp = self
            .http
            .get(&url)
            .header("X-API-Key", key)
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("pfSense GET {path} imefeli: {e}"))?;
        let status = resp.status();
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("pfSense majibu si JSON: {e}"))?;
        if !status.is_success() {
            return Err(format!("pfSense GET {path} → HTTP {status}: {body}"));
        }
        Ok(body)
    }

    /// POST /api/v1/<path> na JSON body — inarudisha JSON ghafi ya API.
    async fn post(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value, String> {
        let (base, key) = self.require()?;
        let url = format!(
            "{}/api/v1/{}",
            base.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        let resp = self
            .http
            .post(&url)
            .header("X-API-Key", key)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("pfSense POST {path} imefeli: {e}"))?;
        let status = resp.status();
        let txt: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("pfSense majibu si JSON: {e}"))?;
        if !status.is_success() {
            return Err(format!("pfSense POST {path} → HTTP {status}: {txt}"));
        }
        Ok(txt)
    }

    // ---------- FIREWALL RULES ----------

    /// Rules zote za firewall (GET /api/v1/firewall/rule).
    pub async fn list_rules(&self) -> Result<serde_json::Value, String> {
        self.get("firewall/rule").await
    }

    /// Ongeza rule (POST /api/v1/firewall/rule) — mf. ruhusa ya TFTP/HTTP kwenye LAN.
    pub async fn add_rule(&self, rule: serde_json::Value) -> Result<serde_json::Value, String> {
        self.post("firewall/rule", rule).await
    }

    // ---------- ALIASES ----------

    /// Aliases zote (GET /api/v1/firewall/alias).
    pub async fn list_aliases(&self) -> Result<serde_json::Value, String> {
        self.get("firewall/alias").await
    }

    /// Ongeza alias (POST /api/v1/firewall/alias) — mf. kundi la kompyuta za HR.
    pub async fn add_alias(&self, alias: serde_json::Value) -> Result<serde_json::Value, String> {
        self.post("firewall/alias", alias).await
    }

    // ---------- SERVICES ----------

    /// Restart ya service (POST /api/v1/services/restart) — mf. {"name": "dnsmasq"}.
    pub async fn restart_service(&self, name: &str) -> Result<serde_json::Value, String> {
        if name.trim().is_empty() {
            return Err("jina la service ni lazima (mf. dnsmasq, dhcpd)".into());
        }
        self.post(
            "services/restart",
            serde_json::json!({ "name": name.trim() }),
        )
        .await
    }

    // ---------- STATUS ----------

    /// Status ya system (GET /api/v1/status/system) — host, version, uptime.
    pub async fn system_status(&self) -> Result<serde_json::Value, String> {
        self.get("status/system").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bila_env_kila_simu_inarudisha_error_ya_configuration() {
        // HAKUNA env kwenye test runner — client isiyowekwa
        let c = PfClient {
            base_url: None,
            api_key: None,
            http: reqwest::Client::new(),
        };
        assert!(!c.is_configured());
        // require() inazuiwa kabla ya network — error ni ya configuration, si uongo
        let err = c.require().unwrap_err();
        assert!(
            err.contains("PFSENSE_API_URL"),
            "error ieleze env zinazohitajika: {err}"
        );
    }

    #[test]
    fn is_configured_inatambua_env_kamili() {
        let c = PfClient {
            base_url: Some("https://10.66.66.1".into()),
            api_key: Some("test-key".into()),
            http: reqwest::Client::new(),
        };
        assert!(c.is_configured());
        assert!(c.require().is_ok());
    }

    #[test]
    fn url_join_ya_base_na_path_ni_sahihi() {
        // uthibitisho wa muundo wa URL unaotumika kwenye get/post
        let base = "https://10.66.66.1/";
        let path = "firewall/rule";
        let url = format!(
            "{}/api/v1/{}",
            base.trim_end_matches('/'),
            path.trim_start_matches('/')
        );
        assert_eq!(url, "https://10.66.66.1/api/v1/firewall/rule");
    }

    #[test]
    fn envelope_inasoma_majibu_ya_api() {
        let body = r#"{"status":"ok","data":[{"protocol":"tcp","action":"pass"}],"message":null}"#;
        let e: PfEnvelope<Vec<serde_json::Value>> = serde_json::from_str(body).unwrap();
        assert_eq!(e.status.as_deref(), Some("ok"));
        assert_eq!(e.data.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn restart_service_inakataa_mtupu() {
        let c = PfClient {
            base_url: Some("https://10.66.66.1".into()),
            api_key: Some("test-key".into()),
            http: reqwest::Client::new(),
        };
        assert!(c.restart_service("").await.is_err());
    }
}
