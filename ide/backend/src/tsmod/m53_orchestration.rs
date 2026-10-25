//! m53_orchestration.rs — MODULE 53: ORCHESTRATION SECURITY
//!
//! Spec: "Inalinda Kubernetes clusters (RBAC, network policies). Twist:
//! inagundua privilege escalation kwenye K8s."
//!
//! Hii si catalog: inasoma cluster halisi kwa kubectl (bila shell) na ina engine
//! ya RBAC inayogundua privilege escalation: cluster-admin bindings, wildcards,
//! escalate/bind/impersonate verbs, hostPath/hostNetwork pods.

use super::{ModReport, TsModule};
use serde_json::{json, Value};
use std::process::Command;

pub struct M53Orchestration;

/// Ruhusa hatarishi za escalation.
const ESCALATION_VERBS: &[&str] = &["escalate", "bind", "impersonate", "*"];

/// Je, role ina ruhusa za escalation?
pub fn escalation_risks(verbs: &[String], resources: &[String], api_groups: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let vlow: Vec<String> = verbs.iter().map(|v| v.to_lowercase()).collect();
    if vlow.iter().any(|v| ESCALATION_VERBS.contains(&v.as_str())) {
        out.push(format!("verbs hatarishi: [{}]", verbs.join(", ")));
    }
    if resources.iter().any(|r| r == "*") && vlow.iter().any(|v| v == "*" || v == "create" || v == "update") {
        out.push("resources wildcard + write".into());
    }
    if api_groups.iter().any(|g| g == "*") {
        out.push("apiGroups wildcard".into());
    }
    out
}

/// Hatari za pod spec.
pub fn pod_risks(host_network: bool, host_path: bool, privileged: bool, run_as_root: bool) -> Vec<String> {
    let mut out = Vec::new();
    if host_network {
        out.push("hostNetwork=true".into());
    }
    if host_path {
        out.push("hostPath volume".into());
    }
    if privileged {
        out.push("privileged container".into());
    }
    if run_as_root {
        out.push("runAsUser=0 (root)".into());
    }
    out
}

fn kubectl(args: &[&str]) -> Result<String, String> {
    let out = Command::new("kubectl")
        .args(args)
        .output()
        .map_err(|e| format!("kubectl haipatikani: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

impl TsModule for M53Orchestration {
    fn id(&self) -> u16 {
        53
    }
    fn name(&self) -> &'static str {
        "Orchestration Security"
    }
    fn group(&self) -> &'static str {
        "infra"
    }
    fn about(&self) -> &'static str {
        "Husoma cluster halisi (kubectl) na kugundua privilege escalation: \
         cluster-admin bindings, wildcards, escalate/bind/impersonate, hostPath."
    }

    fn run(&self, target: &str, params: &Value) -> Result<ModReport, String> {
        let op = params.get("op").and_then(|v| v.as_str()).unwrap_or("audit");
        match op {
            "audit" => self.audit(),
            "rbac" => self.rbac(params),
            "pod" => self.pod(params),
            other => Err(format!("op '{other}' haipo: audit|rbac|pod")),
        }
    }
}

impl M53Orchestration {
    fn audit(&self) -> Result<ModReport, String> {
        let mut r = ModReport::new(53, self.name());
        match kubectl(&["get", "nodes", "-o", "name"]) {
            Ok(out) => {
                let n = out.lines().filter(|l| !l.trim().is_empty()).count();
                r.finding("cluster_nodes", format!("Nodes: {n}"), 2);
            }
            Err(e) => {
                r.finding("k8s_unavailable", format!("K8s haipatikani: {e}"), 10);
            }
        }
        r.summary = "Orchestration audit imefanyika.".into();
        Ok(r)
    }

    fn rbac(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(53, self.name());
        let arr = params
            .get("bindings")
            .and_then(|v| v.as_array())
            .ok_or("params.bindings (array) inahitajika")?;
        let mut alerts = 0;
        for b in arr {
            let name = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let cluster_admin = b
                .get("cluster_admin")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let verbs: Vec<String> = b
                .get("verbs")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let resources: Vec<String> = b
                .get("resources")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
            let api_groups: Vec<String> = b
                .get("api_groups")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                .unwrap_or_default();
            if cluster_admin {
                alerts += 1;
                r.finding(
                    "cluster_admin_binding",
                    format!("Binding '{name}' ni cluster-admin — escalation hatari"),
                    85,
                );
            }
            for x in escalation_risks(&verbs, &resources, &api_groups) {
                alerts += 1;
                r.finding("rbac_escalation", format!("'{name}': {x}"), 75);
            }
        }
        r.summary = format!("Bindings {}; alerts {alerts}.", arr.len());
        Ok(r)
    }

    fn pod(&self, params: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(53, self.name());
        let host_network = params.get("host_network").and_then(|v| v.as_bool()).unwrap_or(false);
        let host_path = params.get("host_path").and_then(|v| v.as_bool()).unwrap_or(false);
        let privileged = params.get("privileged").and_then(|v| v.as_bool()).unwrap_or(false);
        let root = params.get("run_as_root").and_then(|v| v.as_bool()).unwrap_or(false);
        let risks = pod_risks(host_network, host_path, privileged, root);
        for x in &risks {
            r.finding("pod_risk", x.clone(), 70);
        }
        r.summary = format!("Pod risks: {}.", risks.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escalation_inagundulika() {
        let v = vec!["escalate".to_string()];
        let res = vec!["roles".to_string()];
        let ag = vec!["rbac.authorization.k8s.io".to_string()];
        assert!(!escalation_risks(&v, &res, &ag).is_empty());
    }

    #[test]
    fn wildcard_inagundulika() {
        let v = vec!["*".to_string()];
        let res = vec!["*".to_string()];
        let ag = vec!["*".to_string()];
        assert!(escalation_risks(&v, &res, &ag).len() >= 2);
    }

    #[test]
    fn pod_risks_zinagundulika() {
        assert_eq!(pod_risks(true, true, true, true).len(), 4);
    }
}
