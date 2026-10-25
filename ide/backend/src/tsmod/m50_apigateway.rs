//! m50_apigateway.rs — MODULE 50: API GATEWAY SECURITY
//!
//! Spec: "Inalinda API calls, API keys, rate limiting. Twist: inagundua API abuse
//! na credential stuffing."
//!
//! Hii si catalog: ina rate limiter halisi (token bucket kwa IP/key) na engine ya
//! credential stuffing (IP nyingi, username moja; au IP moja, username nyingi)
//! kwenye rekodi halisi za login.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct M50ApiGateway;

/// Token bucket kwa key mmoja.
pub struct Bucket {
    capacity: f64,
    tokens: f64,
    refill_per_s: f64,
    last: Instant,
}

impl Bucket {
    pub fn new(capacity: u32, refill_per_s: f64) -> Self {
        Bucket {
            capacity: capacity as f64,
            tokens: capacity as f64,
            refill_per_s,
            last: Instant::now(),
        }
    }
    pub fn allow(&mut self) -> bool {
        let now = Instant::now();
        let dt = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        self.tokens = (self.tokens + dt * self.refill_per_s).min(self.capacity);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Rate limiter kwa keys nyingi.
pub struct RateLimiter {
    buckets: HashMap<String, Bucket>,
    capacity: u32,
    refill: f64,
}

impl RateLimiter {
    pub fn new(capacity: u32, refill_per_s: f64) -> Self {
        RateLimiter {
            buckets: HashMap::new(),
            capacity,
            refill: refill_per_s,
        }
    }
    pub fn check(&mut self, key: &str) -> bool {
        let b = self
            .buckets
            .entry(key.to_string())
            .or_insert_with(|| Bucket::new(self.capacity, self.refill));
        b.allow()
    }
}

/// Rekodi ya login moja.
#[derive(Debug, Clone)]
pub struct LoginRec {
    pub ip: String,
    pub user: String,
    pub ok: bool,
}

/// Gundua credential stuffing: IP nyingi kwa user mmoja au user nyingi kwa IP moja.
pub fn credential_stuffing(recs: &[LoginRec]) -> Vec<(String, String)> {
    let mut user_ips: HashMap<&str, std::collections::HashSet<&str>> = HashMap::new();
    let mut ip_users: HashMap<&str, std::collections::HashSet<&str>> = HashMap::new();
    for r in recs {
        if r.ok {
            continue;
        }
        user_ips.entry(r.user.as_str()).or_default().insert(r.ip.as_str());
        ip_users.entry(r.ip.as_str()).or_default().insert(r.user.as_str());
    }
    let mut out = Vec::new();
    for (u, ips) in &user_ips {
        if ips.len() >= 5 {
            out.push(("user_many_ips".to_string(), format!("{u} kutoka IP {}", ips.len())));
        }
    }
    for (ip, us) in &ip_users {
        if us.len() >= 5 {
            out.push(("ip_many_users".to_string(), format!("{ip} ilijaribu users {}", us.len())));
        }
    }
    out
}

impl TsModule for M50ApiGateway {
    fn id(&self) -> u16 {
        50
    }
    fn name(&self) -> &'static str {
        "API Gateway Security"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Hutekeleza rate limiting halisi (token bucket) na kugundua API abuse na \
         credential stuffing kwenye rekodi za login."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "ratelimit" => self.ratelimit(params),
            "stuffing" => self.stuffing(params),
            other => Err(format!("op '{other}' haipo: audit|ratelimit|stuffing")),
        }
    }
}

impl M50ApiGateway {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(50, self.name());
        r.finding(
            "gateway_policy",
            "Rate limiting: token bucket; stuffing detection: IP/user fan-in",
            2,
        );
        r.summary = "API gateway audit imefanyika.".into();
        Ok(r)
    }

    fn ratelimit(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(50, self.name());
        let cap = params.get("capacity").and_then(|v| v.as_u64()).unwrap_or(10) as u32;
        let refill = params.get("refill_per_s").and_then(|v| v.as_f64()).unwrap_or(5.0);
        let key = params.get("key").and_then(|v| v.as_str()).unwrap_or("default");
        let requests = params.get("requests").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let mut rl = RateLimiter::new(cap, refill);
        let mut allowed = 0;
        for _ in 0..requests {
            if rl.check(key) {
                allowed += 1;
            }
        }
        let blocked = requests - allowed;
        r.findings.push(json!({
            "kind": "rate_limit", "severity": 2,
            "detail": { "key": key, "capacity": cap, "requests": requests,
                        "allowed": allowed, "blocked": blocked }
        }));
        if blocked > 0 {
            r.finding("rate_limited", format!("Requests {blocked} zimezuiwa kwa {key}"), 10);
        }
        r.summary = format!("Rate limit: allowed {allowed}, blocked {blocked}.");
        Ok(r)
    }

    fn stuffing(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(50, self.name());
        let arr = params
            .get("logins")
            .and_then(|v| v.as_array())
            .ok_or("params.logins (array) inahitajika")?;
        let recs: Vec<LoginRec> = arr
            .iter()
            .map(|l| LoginRec {
                ip: l.get("ip").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                user: l.get("user").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                ok: l.get("ok").and_then(|v| v.as_bool()).unwrap_or(false),
            })
            .collect();
        let hits = credential_stuffing(&recs);
        for (kind, detail) in &hits {
            r.finding(
                "credential_stuffing",
                format!("{kind}: {detail}"),
                80,
            );
        }
        r.summary = format!("Logins {}; dalili {} za stuffing.", recs.len(), hits.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_inazuia_baada_ya_capacity() {
        let mut b = Bucket::new(3, 0.0);
        assert!(b.allow());
        assert!(b.allow());
        assert!(b.allow());
        assert!(!b.allow());
    }

    #[test]
    fn stuffing_inagundulika() {
        let recs: Vec<LoginRec> = (0..6)
            .map(|i| LoginRec {
                ip: format!("10.0.0.{i}"),
                user: "admin".into(),
                ok: false,
            })
            .collect();
        let h = credential_stuffing(&recs);
        assert!(h.iter().any(|(k, _)| k == "user_many_ips"));
    }

    #[test]
    fn logins_salama_hayagundui() {
        let recs = vec![LoginRec { ip: "1.1.1.1".into(), user: "a".into(), ok: true }];
        assert!(credential_stuffing(&recs).is_empty());
    }
}
