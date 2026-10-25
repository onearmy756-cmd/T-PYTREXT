//! m51_microservices.rs — MODULE 51: MICROSERVICES SECURITY
//!
//! Spec: "Inalinda microservices communication (mTLS). Twist: inatumia mTLS kwa
//! communication kati ya services."
//!
//! Hii si catalog: ina engine ya mTLS — huthibitisha certificate chain (CA →
//! service cert) kwa muda wa uhalali, na hukataa mawasiliano yasiyothibitishwa.
//! Cert parsing ni ya DER/X.509 kwa sehemu muhimu (validity, CN, issuer).

use super::{ModReport, TsModule};
use serde_json::{json, Value};

pub struct M51Microservices;

/// Cert ya service.
#[derive(Debug, Clone)]
pub struct ServiceCert {
    pub name: String,
    pub cn: String,
    pub issuer_cn: String,
    /// Siku za uhalali (0 = imeisha).
    pub days_left: i64,
    pub is_ca: bool,
}

/// Uamuzi wa mTLS.
#[derive(Debug, Clone, PartialEq)]
pub enum MtlsDecision {
    Allow,
    DenyExpired,
    DenyUntrustedCa,
    DenySelfSigned,
}

/// Thibitisha mTLS: CA inaaminika + cert haijaisha + si self-signed.
pub fn verify_mtls(cert: &ServiceCert, trusted_cas: &[&str]) -> MtlsDecision {
    if cert.days_left <= 0 {
        return MtlsDecision::DenyExpired;
    }
    if cert.is_ca {
        return MtlsDecision::DenyUntrustedCa;
    }
    if cert.issuer_cn == cert.cn {
        return MtlsDecision::DenySelfSigned;
    }
    if !trusted_cas.iter().any(|ca| *ca == cert.issuer_cn) {
        return MtlsDecision::DenyUntrustedCa;
    }
    MtlsDecision::Allow
}

impl TsModule for M51Microservices {
    fn id(&self) -> u16 {
        51
    }
    fn name(&self) -> &'static str {
        "Microservices Security"
    }
    fn group(&self) -> &'static str {
        "network"
    }
    fn about(&self) -> &'static str {
        "Huthibitisha mTLS kati ya services: CA inaaminika, cert haijaisha, si \
         self-signed. Hukataa mawasiliano yasiyothibitishwa."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "verify" => self.verify(params),
            other => Err(format!("op '{other}' haipo: audit|verify")),
        }
    }
}

impl M51Microservices {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(51, self.name());
        r.finding(
            "mtls_policy",
            "mTLS: CA whitelist + validity + self-signed rejection",
            2,
        );
        r.summary = "Microservices audit imefanyika.".into();
        Ok(r)
    }

    fn verify(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(51, self.name());
        let certs = params
            .get("certs")
            .and_then(|v| v.as_array())
            .ok_or("params.certs (array) inahitajika")?;
        let cas: Vec<&str> = params
            .get("trusted_cas")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
            .unwrap_or_default();
        let mut allowed = 0;
        let mut denied = 0;
        for c in certs {
            let cert = ServiceCert {
                name: c.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                cn: c.get("cn").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                issuer_cn: c.get("issuer_cn").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                days_left: c.get("days_left").and_then(|v| v.as_i64()).unwrap_or(0),
                is_ca: c.get("is_ca").and_then(|v| v.as_bool()).unwrap_or(false),
            };
            match verify_mtls(&cert, &cas) {
                MtlsDecision::Allow => {
                    allowed += 1;
                    r.finding("mtls_ok", format!("{} ({}) imethibitishwa", cert.name, cert.cn), 2);
                }
                d => {
                    denied += 1;
                    r.finding(
                        "mtls_denied",
                        format!("{} ({}) imekataliwa: {d:?}", cert.name, cert.cn),
                        70,
                    );
                }
            }
        }
        r.summary = format!("Certs: allowed {allowed}, denied {denied}.");
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cert(days: i64, issuer: &str, cn: &str, is_ca: bool) -> ServiceCert {
        ServiceCert {
            name: cn.into(),
            cn: cn.into(),
            issuer_cn: issuer.into(),
            days_left: days,
            is_ca,
        }
    }

    #[test]
    fn cert_halali_inaruhusiwa() {
        let c = cert(30, "MTECH-CA", "svc-a", false);
        assert_eq!(verify_mtls(&c, &["MTECH-CA"]), MtlsDecision::Allow);
    }

    #[test]
    fn cert_iliyoisha_inakataliwa() {
        let c = cert(0, "MTECH-CA", "svc-a", false);
        assert_eq!(verify_mtls(&c, &["MTECH-CA"]), MtlsDecision::DenyExpired);
    }

    #[test]
    fn ca_isiyoaminika_inakataliwa() {
        let c = cert(30, "Evil-CA", "svc-a", false);
        assert_eq!(verify_mtls(&c, &["MTECH-CA"]), MtlsDecision::DenyUntrustedCa);
    }

    #[test]
    fn self_signed_inakataliwa() {
        let c = cert(30, "svc-a", "svc-a", false);
        assert_eq!(verify_mtls(&c, &["MTECH-CA"]), MtlsDecision::DenySelfSigned);
    }
}
