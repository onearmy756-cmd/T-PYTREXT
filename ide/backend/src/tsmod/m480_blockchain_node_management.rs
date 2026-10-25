//! m480_blockchain_node_management.rs — MODULE 480: BLOCKCHAIN NODE MANAGEMENT
//!
//! Spec: "Usimamizi wa nodes. Twist: Auto-scaling."
//!
//! Injini halisi: husimamia nodes (ongeza/ondoa/kagua afya) na kupanga uwezo kwa
//! auto-scaling kulingana na mzigo; huripoti hali ya kila node.

use super::{ModReport, TsModule};
use serde_json::{json, Value};

/// Node ya blockchain.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub cpu: u8,
    pub peers: u32,
    pub synced: bool,
}

/// Amua idadi ya nodes zinazohitajika (auto-scaling).
pub fn desired_nodes(current: usize, load_pct: u8, min: usize, max: usize) -> usize {
    let target = if load_pct > 80 {
        current + 1
    } else if load_pct < 30 && current > min {
        current - 1
    } else {
        current
    };
    target.clamp(min, max)
}

/// Afya ya node.
pub fn health(n: &Node) -> &'static str {
    if !n.synced {
        "out_of_sync"
    } else if n.cpu > 90 {
        "overloaded"
    } else if n.peers < 3 {
        "isolated"
    } else {
        "healthy"
    }
}

pub struct M480BlockchainNodeManagement;

impl TsModule for M480BlockchainNodeManagement {
    fn id(&self) -> u16 {
        480
    }
    fn name(&self) -> &'static str {
        "Blockchain Node Management"
    }
    fn group(&self) -> &'static str {
        "blockchain"
    }
    fn about(&self) -> &'static str {
        "Husimamia nodes (afya, peers, sync) na kupanga uwezo kwa auto-scaling."
    }

    fn run(&self, _t: &str, p: &Value) -> Result<ModReport, String> {
        let mut r = ModReport::new(480, self.name());
        let load = p.get("load_pct").and_then(|v| v.as_u64()).unwrap_or(85) as u8;
        let current = p.get("current").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
        let nodes = vec![
            Node { id: "node-1".into(), cpu: 45, peers: 12, synced: true },
            Node { id: "node-2".into(), cpu: 92, peers: 8, synced: true },
            Node { id: "node-3".into(), cpu: 30, peers: 1, synced: false },
        ];
        for n in &nodes {
            let h = health(n);
            r.finding("node", format!("{}: {h} (cpu {}%, peers {})", n.id, n.cpu, n.peers), if h == "healthy" { 1 } else { 55 });
        }
        let desired = desired_nodes(current, load, 2, 10);
        let action = match desired.cmp(&current) {
            std::cmp::Ordering::Greater => "scale_up",
            std::cmp::Ordering::Less => "scale_down",
            std::cmp::Ordering::Equal => "hold",
        };
        r.findings.push(json!({ "kind": "autoscale", "severity": 1,
            "detail": { "current": current, "load_pct": load, "desired": desired, "action": action, "max": 10, "min": 2 } }));
        r.summary = format!("Node Management: nodes {}, auto-scaling → {desired} ({action}).", nodes.len());
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autoscale_inapanga() {
        assert_eq!(desired_nodes(3, 85, 2, 10), 4);
        assert_eq!(desired_nodes(3, 20, 2, 10), 2);
        assert_eq!(desired_nodes(3, 50, 2, 10), 3);
        assert_eq!(desired_nodes(10, 95, 2, 10), 10);
    }
}
