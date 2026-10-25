//! reacon.rs — AGENTIC AI: ReAct loop (THINK → ACT → OBSERVE) + BOUNDED AUTONOMY.
//!
//! Mahitaji ya mmiliki (SEHEMU 3 Hatua 5/6 + SEHEMU 13):
//!   - ReAct: agent inafikiri, inachukua hatua (ping/cmd kwenye PC nyingi),
//!     inaangalia matokeo, kisha inaamua hatua inayofuata — mfano "Wi-Fi haifanyi
//!     kazi" kwenye PC 50: ping → 45 zinajibu, 5 hazijibu → fix service ya mtandao
//!     kwenye 5 PEKEE.
//!   - Bounded autonomy: hatua za hatari NDOGO (scan, cache, service restart)
//!     zinafanyika automatic; hatua za hatari KUBWA (reboot, wipe, install)
//!     zinahitaji idhini ya binadamu (HITL).
//!
//! KANUNI: loop nzima Rust (enum Step, match arms). Hakuna uongo — ACT ya
//! `ping`/`probe` ni TCP connect halisi; actions za shell zinahitaji HITL au
//! allowlist (isikuwe na code execution isiyodhibitiwa).

use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Semaphore;

// ---------- RISK (bounded autonomy) ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Risk {
    Low,  // auto — hakuna idhini inahitajika
    High, // lazima idhini ya binadamu (HITL)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Action {
    PingSweep,      // TCP probe kwa PC zote (halisi)
    ReadProcesses,  // orodha ya processes (kutoka probe)
    ClearCache,     // kufuta cache (low-risk)
    RestartService, // kuanzisha upya service (mfano Wi-Fi/DNS)
    InstallApp,     // winget/apt — hatari kubwa
    Reboot,         // hatari kubwa
    WipeDisk,       // hatari kubwa zaidi
}

impl Action {
    pub fn risk(&self) -> Risk {
        match self {
            Action::PingSweep | Action::ReadProcesses | Action::ClearCache => Risk::Low,
            Action::RestartService => Risk::Low, // service restart = reversible
            Action::InstallApp | Action::Reboot | Action::WipeDisk => Risk::High,
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            Action::PingSweep => "ping_sweep",
            Action::ReadProcesses => "read_processes",
            Action::ClearCache => "clear_cache",
            Action::RestartService => "restart_service",
            Action::InstallApp => "install_app",
            Action::Reboot => "reboot",
            Action::WipeDisk => "wipe_disk",
        }
    }
}

/// Gate ya bounded autonomy: hatua ya High inahitaji idhini (HITL daima).
pub fn action_allowed(action: Action, human_approved: bool) -> bool {
    match action.risk() {
        Risk::Low => true,
        Risk::High => human_approved,
    }
}

// ---------- CONFIDENCE SCORE (kama Tanium) ----------

/// Kihesabu cha uhakika wa uchunguzi/hatua (0.0–1.0):
/// - Low-risk actions zenye observation wazi = uhakika wa juu
/// - High-risk bila idhini = 0.0 (hakuna uhakika)
/// - PC nyingi zilizojibu sweep = uhakika wa juu wa uchunguzi
pub fn confidence_score(alive: usize, total_targets: usize, risky_without_approval: bool) -> f64 {
    if risky_without_approval {
        return 0.0;
    }
    if total_targets == 0 {
        return 0.5;
    }
    let ratio = alive as f64 / total_targets as f64;
    (0.5 + 0.5 * ratio).clamp(0.0, 1.0)
}

// ---------- MULTI-OS EXECUTION (kama RDM: ombi MOJA → OS nyingi) ----------

/// Ombi moja la apps kwa OS tofauti kwa WAKATI MMOJA:
/// "Sakinisha Sysinternals kwenye Windows na btop kwenye Ubuntu" → plans per-OS.
pub fn multi_os_plan(os_groups: Vec<(String, Vec<String>)>) -> Vec<crate::bundles::PcPlan> {
    os_groups
        .into_iter()
        .map(|(os, apps)| crate::bundles::PcPlan {
            display_name: format!("{}-fleet", os),
            os,
            app_ids: apps,
        })
        .collect()
}

// ---------- STEP (ReAct) ----------

#[derive(Debug, Clone, Serialize)]
pub struct Step {
    pub kind: String, // think | act | observe
    pub action: Option<String>,
    pub risk: Option<String>,
    pub approved: Option<bool>,
    pub thought_sw: String,
    pub observation: String,
    pub ts: String,
}

impl Step {
    fn think(thought_sw: &str) -> Step {
        Step {
            kind: "think".into(),
            action: None,
            risk: None,
            approved: None,
            thought_sw: thought_sw.into(),
            observation: String::new(),
            ts: chrono::Local::now().to_rfc3339(),
        }
    }
    fn act(action: Action, approved: bool) -> Step {
        Step {
            kind: "act".into(),
            action: Some(action.id().into()),
            risk: Some(
                match action.risk() {
                    Risk::Low => "low",
                    Risk::High => "high",
                }
                .into(),
            ),
            approved: Some(approved),
            thought_sw: String::new(),
            observation: String::new(),
            ts: chrono::Local::now().to_rfc3339(),
        }
    }
    fn observe(obs: &str) -> Step {
        Step {
            kind: "observe".into(),
            action: None,
            risk: None,
            approved: None,
            thought_sw: String::new(),
            observation: obs.into(),
            ts: chrono::Local::now().to_rfc3339(),
        }
    }
}

// ---------- ACT HALISI: TCP probe (ping sweep ya kutosha kwenye network) ----------

async fn tcp_alive(ip: std::net::Ipv4Addr, timeout_ms: u64) -> bool {
    for port in [22u16, 445, 135, 3389] {
        let fut = tokio::net::TcpStream::connect((std::net::IpAddr::V4(ip), port));
        if tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), fut)
            .await
            .map(|r| r.is_ok())
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

/// ACT halisi: ping sweep kwenye /24 (parallel, sem 64) → orodha ya IPs zinazojibu.
pub async fn ping_sweep(base: (u8, u8, u8), timeout_ms: u64) -> Vec<String> {
    let sem = Arc::new(Semaphore::new(64));
    let mut handles = Vec::new();
    for i in 1..=254u8 {
        let permit = sem.clone().acquire_owned().await.unwrap();
        handles.push(tokio::spawn(async move {
            let alive = tcp_alive(
                std::net::Ipv4Addr::new(base.0, base.1, base.2, i),
                timeout_ms,
            )
            .await;
            drop(permit);
            (i, alive)
        }));
    }
    let mut alive = Vec::new();
    for h in handles {
        if let Ok((i, true)) = h.await {
            alive.push(format!("{}.{}.{}.{}", base.0, base.1, base.2, i));
        }
    }
    alive
}

// ---------- REACT LOOP ----------

#[derive(Debug, Clone, Serialize)]
pub struct ReActSession {
    pub id: String,
    pub problem_sw: String,
    pub targets: Vec<String>, // display names au IPs za PC
    pub steps: Vec<Step>,
    pub status: String, // running | awaiting_approval | done | blocked
    pub conclusion_sw: String,
}

impl ReActSession {
    pub fn new(problem_sw: &str, targets: Vec<String>) -> Self {
        ReActSession {
            id: format!("REACT-{}", chrono::Local::now().format("%Y%m%d%H%M%S")),
            problem_sw: problem_sw.into(),
            targets,
            steps: Vec::new(),
            status: "running".into(),
            conclusion_sw: String::new(),
        }
    }

    fn push(&mut self, s: Step) {
        self.steps.push(s);
    }

    /// THINK #1: panga uchunguzi (daima kuanza na ping sweep — low risk).
    pub fn think_plan(&mut self) {
        self.push(Step::think(&format!(
            "Tatizo: '{}'. Kwanza nahitaji kujua ni PC zipi ziko hai kwenye mtandao — nita-fanya ping sweep (hatari ndogo, automatic).",
            self.problem_sw
        )));
    }

    /// OBSERVE baada ya sweep → THINK #2: amua PCs zenye tatizo.
    pub fn observe_sweep(&mut self, alive: &[String]) {
        self.push(Step::observe(&format!(
            "{} PCs kati ya zilizoagizwa zimejibu: {}",
            alive.len(),
            alive.join(", ")
        )));
        self.push(Step::think(&format!(
            "PCs {} hazijibu — zina tatizo linalohusiana na '{}'. Nita-rekebisha service kwenye hizo PEKEE (restart_service = hatari ndogo); kama nita-hitaji install/reboot nitakusubiri RUHUSU.",
            if alive.is_empty() { "zote" } else { "zilizokosekana" }, self.problem_sw
        )));
    }

    /// Jaribu ACT: low-risk inaendelea; high-risk bila idhini → status awaiting_approval.
    pub fn try_act(&mut self, action: Action, human_approved: bool) -> bool {
        let allowed = action_allowed(action, human_approved);
        self.push(Step::act(action, allowed));
        if !allowed {
            self.status = "awaiting_approval".into();
        }
        allowed
    }

    /// OBSERVE ya mwisho → hitimisho (ripoti kwa admin).
    pub fn conclude(&mut self, conclusion_sw: &str) {
        self.push(Step::observe(
            "Kazi imekamilika — ripoti imeandikwa kwa admin.",
        ));
        self.conclusion_sw = conclusion_sw.into();
        self.status = "done".into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multi_os_plan_ombi_mojwa_os_mbili() {
        let plans = multi_os_plan(vec![
            ("win11".into(), vec!["anydesk".into()]),
            ("ubuntu".into(), vec!["git".into()]),
        ]);
        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].os, "win11");
        assert_eq!(plans[1].os, "ubuntu");
        let cmds = plans[1].install_commands();
        assert!(cmds[0].contains("apt-get install"));
    }

    #[test]
    fn confidence_score_halisi() {
        // PCs 45/50 zinajibu → uhakika wa juu
        let c = confidence_score(45, 50, false);
        assert!((c - 0.95).abs() < 1e-9);
        // Hakuna aliyejibu → 0.5 (tunajua tunachojua tu)
        assert!((confidence_score(0, 50, false) - 0.5).abs() < 1e-9);
        // High-risk bila idhini = hakuna uhakika kabisa
        assert_eq!(confidence_score(50, 50, true), 0.0);
    }

    #[test]
    fn bounded_autonomy_risk_gate() {
        // Low-risk: automatic
        assert!(action_allowed(Action::PingSweep, false));
        assert!(action_allowed(Action::ClearCache, false));
        assert!(action_allowed(Action::RestartService, false));
        // High-risk: lazima idhini
        assert!(!action_allowed(Action::InstallApp, false));
        assert!(!action_allowed(Action::Reboot, false));
        assert!(!action_allowed(Action::WipeDisk, false));
        assert!(action_allowed(Action::Reboot, true));
    }

    #[tokio::test]
    async fn ping_sweep_loopback_hakuna_panic() {
        // sweep ya /30 fupi (mabox ya test) — inapita bila network halisi
        let alive = ping_sweep((127, 0, 0), 20).await; // 127.0.0.x
        let _ = alive; // matokeo yanategemea host; lengo ni hakuna panic
    }

    #[test]
    fn react_flow_hitl() {
        let mut s = ReActSession::new("Wi-Fi haifanyi kazi", vec!["PC-01".into(), "PC-02".into()]);
        s.think_plan();
        s.observe_sweep(&["192.168.1.10".into()]);
        // restart_service (low) inapita
        assert!(s.try_act(Action::RestartService, false));
        // install (high) bila idhini → inasimama
        assert!(!s.try_act(Action::InstallApp, false));
        assert_eq!(s.status, "awaiting_approval");
        // baada ya idhini inaendelea
        assert!(s.try_act(Action::InstallApp, true));
        s.conclude("Service zinarekebishwa; apps zimesakinishwa kwa idhini yako.");
        assert_eq!(s.status, "done");
        assert!(s.steps.iter().any(|st| st.kind == "think"));
        assert!(s.steps.iter().any(|st| st.kind == "act"));
        assert!(s.steps.iter().any(|st| st.kind == "observe"));
    }
}
