//! Job pipeline v2 (P2):
//! plan (AI+rules) → HITL → backup (halisi) → WOL → PXE → [multicast] → install → report
//!
//! Mabadiliko kutoka v1:
//!   - backup_stub → backup halisi (files/image/stub kutoka backup.rs)
//!   - multicast (image moja kwa PCs nyingi) ikiwa `multicast: true` + `image`
//!   - OS selection: osselect.rs (profiles JSON + Ollama assist + reasons)

use crate::backup;
use crate::multicast::{self, SharedAcks};
use crate::osselect;
use crate::wol;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::OnceLock;

/// Ripoti ya install kutoka kwa client (PXE live/initramfs) baada ya kuandika disk.
/// Hii inafanya hatua ya INSTALL kuwa HALISI: server husubiri uthibitisho, si
/// kuhesabu maendeleo ya uongo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallReport {
    pub mac: String,
    pub os: String,
    pub ok: bool,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub installed_at: String,
}

fn install_reports() -> &'static std::sync::Mutex<std::collections::HashMap<String, InstallReport>> {
    static REPORTS: OnceLock<std::sync::Mutex<std::collections::HashMap<String, InstallReport>>> =
        OnceLock::new();
    REPORTS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

pub fn record_install(rep: InstallReport) {
    install_reports()
        .lock()
        .unwrap()
        .insert(rep.mac.to_lowercase(), rep);
}

pub fn install_status(mac: &str) -> Option<InstallReport> {
    install_reports().lock().unwrap().get(&mac.to_lowercase()).cloned()
}

/// Hii ndiyo config ya per-MAC ya PXE. Inaambatanisha `fundi.*` (api/job/os/of)
/// ili client initramfs (fundi/) iripoti install kwa JOB HII — bila `fundi.job`,
/// INSTALL haitakamilika (server inasubiri ripoti).
/// `api` = base ya API (install-report/ACK). `http` = base ya nginx (images/seeds).
fn pxe_config(os: &str, api: &str, http: &str, job_id: &str) -> String {
    let label = match os {
        "win11" | "win10" | "ubuntu" | "debian" | "fedora" | "kali" => os,
        _ => "auto",
    };
    let (kernel, append) = match os {
        "win11" => ("wimboot", "initrd=win11/boot.wim".to_string()),
        "win10" => ("wimboot", "initrd=win10/boot.wim".to_string()),
        "ubuntu" => (
            "ubuntu/vmlinuz",
            format!("initrd=ubuntu/initrd url={http}/images/ubuntu/ubuntu.seed auto=true"),
        ),
        "debian" => (
            "debian/linux",
            format!("initrd=debian/initrd.gz auto=true preseed/url={http}/images/debian/preseed.cfg"),
        ),
        "fedora" => ("fedora/vmlinuz", "initrd=fedora/initrd.img".to_string()),
        "kali" => ("kali/vmlinuz", "initrd=kali/initrd.img".to_string()),
        _ => ("fundi/vmlinuz", "initrd=fundi/initrd.img quiet".to_string()),
    };
    format!(
        "DEFAULT {label}\nPROMPT 0\nTIMEOUT 50\n\
         LABEL {label}\n  KERNEL {kernel}\n  APPEND {append} fundi.api={api} fundi.http={http} fundi.job={job_id} fundi.os={os} fundi.of=/dev/sda\n\
         LABEL local\n  LOCALBOOT 0\n"
    )
}

/// Rasilimali za PXE zinazohitajika kwa kila OS (kuhusiana na server/tftp/pxelinux.cfg/default).
fn pxe_assets(os: &str) -> Vec<&'static str> {
    match os {
        "win11" => vec!["wimboot", "win11/boot.wim"],
        "win10" => vec!["wimboot", "win10/boot.wim"],
        "ubuntu" => vec!["ubuntu/vmlinuz", "ubuntu/initrd"],
        "debian" => vec!["debian/linux", "debian/initrd.gz"],
        "fedora" => vec!["fedora/vmlinuz", "fedora/initrd.img"],
        "kali" => vec!["kali/vmlinuz", "kali/initrd.img"],
        _ => vec!["fundi/vmlinuz", "fundi/initrd.img"],
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub device_mac: String,
    pub device_name: String,
    pub os_type: String,
    pub status: String,
    pub stage: String,
    pub progress: u32,
    pub message: String,
    pub needs_approval: bool,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub multicast: bool,
}

#[derive(Debug, Deserialize)]
pub struct DeployRequest {
    pub computers: Vec<ComputerTarget>,
    pub os_type: Option<String>,
    pub auto_approve: Option<bool>,
    pub user_need: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub multicast: Option<bool>,
    #[serde(default)]
    pub backup_mode: Option<String>,
    #[serde(default)]
    pub backup_source: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ComputerTarget {
    pub mac: String,
    pub name: String,
    pub specs: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PipelineOpts {
    pub os: String,
    pub specs: String,
    pub need: String,
    pub auto_approve: bool,
    pub backup_mode: String,
    pub backup_source: String,
    pub image: Option<String>,
    pub multicast: bool,
}

impl PipelineOpts {
    pub fn from_request(req: &DeployRequest, i: usize) -> Self {
        let default_backup = std::env::var("FUNDI_BACKUP_MODE").unwrap_or_else(|_| "auto".into());
        PipelineOpts {
            os: req.os_type.clone().unwrap_or_else(|| "auto".into()),
            specs: req
                .computers
                .get(i)
                .and_then(|c| c.specs.clone())
                .unwrap_or_default(),
            need: req.user_need.clone().unwrap_or_else(|| "office".into()),
            auto_approve: req.auto_approve.unwrap_or(false),
            backup_mode: req.backup_mode.clone().unwrap_or(default_backup),
            backup_source: req.backup_source.clone().unwrap_or_default(),
            image: req.image.clone(),
            multicast: req.multicast.unwrap_or(false),
        }
    }
}

pub async fn set_job(
    db: &SqlitePool,
    jobs: &tokio::sync::RwLock<Vec<Job>>,
    id: &str,
    status: &str,
    stage: &str,
    progress: u32,
    message: &str,
) {
    if let Some(j) = jobs.write().await.iter_mut().find(|j| j.id == id) {
        j.status = status.into();
        j.stage = stage.into();
        j.progress = progress;
        j.message = message.into();
        if status == "awaiting_approval" {
            j.needs_approval = true;
        }
        if status == "running" || status == "done" || status == "failed" {
            j.needs_approval = false;
        }
    }
    let _ = sqlx::query("UPDATE jobs SET status=?, stage=?, progress=?, message=? WHERE id=?")
        .bind(status)
        .bind(stage)
        .bind(progress as i64)
        .bind(message)
        .bind(id)
        .execute(db)
        .await;
}

async fn set_os(
    db: &SqlitePool,
    jobs: &tokio::sync::RwLock<Vec<Job>>,
    id: &str,
    os: &str,
    reason: &str,
) {
    if let Some(j) = jobs.write().await.iter_mut().find(|j| j.id == id) {
        j.os_type = os.into();
    }
    let _ = sqlx::query("UPDATE jobs SET os_type=?, os_reason=? WHERE id=?")
        .bind(os)
        .bind(reason)
        .bind(id)
        .execute(db)
        .await;
}

pub async fn run_pipeline(
    db: SqlitePool,
    jobs: std::sync::Arc<tokio::sync::RwLock<Vec<Job>>>,
    acks: SharedAcks,
    job_id: String,
    mac: String,
    mut opts: PipelineOpts,
) {
    let tftp = std::env::var("FUNDI_TFTP").unwrap_or_else(|_| "/var/lib/tftpboot".into());

    // ---------- 1. PLAN (AI + rules, data-driven) ----------
    set_job(
        &db,
        &jobs,
        &job_id,
        "running",
        "plan",
        5,
        "OS selection: profiles + AI",
    )
    .await;
    let mut decision_note = String::new();
    if opts.os.is_empty() || opts.os == "auto" {
        let d = osselect::decide_full(&opts.specs, &opts.need).await;
        decision_note = format!(
            "{} (confidence {:.0}%, njia: {})",
            d.reasons.join("; "),
            d.confidence * 100.0,
            d.method
        );
        opts.os = d.os.clone();
        set_os(&db, &jobs, &job_id, &d.os, &decision_note).await;
    }
    if opts.os == "skip" {
        set_job(
            &db,
            &jobs,
            &job_id,
            "failed",
            "plan",
            0,
            "Hardware shida — skip (SMART/RAM). Msimamizi arekebishe.",
        )
        .await;
        return;
    }

    // ---------- 2. HITL ----------
    if !opts.auto_approve {
        set_job(
            &db,
            &jobs,
            &job_id,
            "awaiting_approval",
            "hitl",
            10,
            &format!(
                "OS iliyochaguliwa: {}. Msimamizi: POST /jobs/{job_id}/approve",
                opts.os
            ),
        )
        .await;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let st = jobs
                .read()
                .await
                .iter()
                .find(|j| j.id == job_id)
                .map(|j| j.status.clone())
                .unwrap_or_default();
            if st == "approved" || st == "running" {
                break;
            }
            if st == "cancelled" || st == "failed" {
                return;
            }
        }
    }

    // ---------- 3. BACKUP (halisi) ----------
    set_job(
        &db,
        &jobs,
        &job_id,
        "running",
        "backup",
        20,
        &format!("Backup mode: {}", opts.backup_mode),
    )
    .await;
    let h = tokio::runtime::Handle::current();
    let (db2, jobs2, jid2) = (db.clone(), jobs.clone(), job_id.clone());
    let log = move |m: String| {
        let (db3, jobs3, j3) = (db2.clone(), jobs2.clone(), jid2.clone());
        h.spawn(async move {
            set_job(&db3, &jobs3, &j3, "running", "backup", 22, &m).await;
        });
    };
    let rep = backup::run_backup(&mac, &opts.backup_mode, &opts.backup_source, &log).await;
    let _ = sqlx::query("UPDATE jobs SET backup_info=? WHERE id=?")
        .bind(serde_json::to_string(&rep).unwrap_or_default())
        .bind(&job_id)
        .execute(&db)
        .await;
    if !rep.ok && opts.backup_mode != "stub" {
        set_job(
            &db,
            &jobs,
            &job_id,
            "failed",
            "backup",
            20,
            &format!("Backup imeshindikana: {}", rep.message),
        )
        .await;
        return;
    }
    set_job(&db, &jobs, &job_id, "running", "backup", 25, &rep.message).await;

    // ---------- 4. WOL ----------
    set_job(&db, &jobs, &job_id, "running", "wol", 30, "Wake-on-LAN").await;
    if let Err(e) = wol::wake(&mac).await {
        set_job(
            &db,
            &jobs,
            &job_id,
            "running",
            "wol",
            32,
            &format!("WOL warn: {e}"),
        )
        .await;
    }

    // ---------- 5. PXE ----------
    set_job(&db, &jobs, &job_id, "running", "pxe", 45, "PXE boot config").await;
    let mac_file = format!(
        "{}/pxelinux.cfg/01-{}",
        tftp,
        mac.replace(':', "-").to_lowercase()
    );
    if let Some(parent) = Path::new(&mac_file).parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    // Server URLs: `api` = API (install-report/ACK), `http` = nginx (images/seeds).
    // Weka FUNDI_PUBLIC_URL + FUNDI_HTTP_URL kwenye compose/agent.
    let api = std::env::var("FUNDI_PUBLIC_URL")
        .unwrap_or_else(|_| "http://192.168.1.10:8080".to_string());
    let http = std::env::var("FUNDI_HTTP_URL")
        .unwrap_or_else(|_| "http://192.168.1.10".to_string());
    let conf = pxe_config(&opts.os, &api, &http, &job_id);
    let _ = tokio::fs::write(&mac_file, conf).await;
    set_os(&db, &jobs, &job_id, &opts.os, &decision_note).await;

    // ---------- 6. MULTICAST (hiari) ----------
    if opts.multicast {
        match opts.image.as_deref().map(crate::images::find_image) {
            Some(Some(img)) => {
                set_job(
                    &db,
                    &jobs,
                    &job_id,
                    "running",
                    "multicast",
                    50,
                    &format!(
                        "Multicast {}: {} bytes (group {}:{})",
                        img.name,
                        img.bytes,
                        multicast::MC_GROUP,
                        multicast::MC_PORT
                    ),
                )
                .await;
                let path = std::path::PathBuf::from(&img.path);
                let session = job_id.clone();
                let sent =
                    tokio::task::spawn_blocking(move || multicast::send_file(&path, &session))
                        .await;
                match sent {
                    Ok(Ok(m)) => {
                        // subiri ACKs za clients (sekunde 30) — LAZIMA ziwe zilizothibitishwa
                        let mut verified = 0usize;
                        for _ in 0..30 {
                            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                            verified = acks.verified(&m.sha256);
                            if verified > 0 {
                                break;
                            }
                        }
                        if verified == 0 {
                            // Hakuna client iliyothibitisha sha256 → job inashindikana
                            // (sha mismatch / clients hazijajibu). Hakuna mafanikio ya uongo.
                            set_job(
                                &db, &jobs, &job_id, "failed", "multicast", 50,
                                &format!(
                                    "Multicast imeshindikana: hakuna ACK iliyothibitisha sha256 (ACKs {}) — clients hazijajibu au sha mismatch",
                                    acks.count()
                                ),
                            )
                            .await;
                            return;
                        }
                        set_job(
                            &db, &jobs, &job_id, "running", "multicast", 52,
                            &format!("Multicast OK: sha256 imethibitishwa na clients {verified} (ACKs {})", acks.count()),
                        )
                        .await;
                    }
                    Ok(Err(e)) => {
                        set_job(
                            &db,
                            &jobs,
                            &job_id,
                            "failed",
                            "multicast",
                            50,
                            &format!("Multicast imeshindikana: {e}"),
                        )
                        .await;
                        return;
                    }
                    Err(e) => {
                        set_job(
                            &db,
                            &jobs,
                            &job_id,
                            "failed",
                            "multicast",
                            50,
                            &format!("Multicast task: {e}"),
                        )
                        .await;
                        return;
                    }
                }
            }
            Some(None) => {
                set_job(
                    &db,
                    &jobs,
                    &job_id,
                    "failed",
                    "multicast",
                    50,
                    "Image haipatikani kwenye /images",
                )
                .await;
                return;
            }
            None => {
                set_job(
                    &db,
                    &jobs,
                    &job_id,
                    "running",
                    "multicast",
                    50,
                    "Multicast ombwa lakini hakuna image — inaendelea bila",
                )
                .await;
            }
        }
    }

    // ---------- 7. INSTALL (HALISI) ----------
    // Kagua rasilimali za PXE kwa OS hii. Kama hazipo, hatuwezi ku-install —
    // job inashindikana kwa uwazi (hakuna maendeleo ya uongo).
    let missing: Vec<&str> = pxe_assets(&opts.os)
        .into_iter()
        .filter(|rel| !Path::new(&tftp).join(rel).exists())
        .collect();
    if !missing.is_empty() {
        set_job(
            &db,
            &jobs,
            &job_id,
            "failed",
            "install",
            55,
            &format!(
                "PXE haiwezi: rasilimali hazipo kwenye {tftp}: {} — pakia boot assets (wimboot/vmlinuz/initrd) kisha ujaribu tena",
                missing.join(", ")
            ),
        )
        .await;
        return;
    }

    set_job(
        &db,
        &jobs,
        &job_id,
        "running",
        "install",
        55,
        &format!(
            "Install {} (PXE): inasubiri uthibitisho kutoka client baada ya kuandika disk (POST /jobs/{job_id}/install-report)",
            opts.os
        ),
    )
    .await;

    // Subiri ripoti HALISI ya install kutoka client (mac). Hakuna kuhesabu uongo:
    // maendeleo yanaonyesha muda unaopita huku tukisubiri uthibitisho.
    let wait_s: u64 = std::env::var("FUNDI_INSTALL_TIMEOUT_S")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1800);
    let mut installed: Option<InstallReport> = None;
    for elapsed in 1..=wait_s {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        if let Some(rep) = install_status(&mac) {
            installed = Some(rep);
            break;
        }
        // cancel/failed kutoka msimamizi
        let st = jobs
            .read()
            .await
            .iter()
            .find(|j| j.id == job_id)
            .map(|j| j.status.clone())
            .unwrap_or_default();
        if st == "cancelled" || st == "failed" {
            return;
        }
        if elapsed % 15 == 0 {
            let pct = 55 + ((elapsed * 40 / wait_s.max(1)).min(40) as u32);
            set_job(
                &db,
                &jobs,
                &job_id,
                "running",
                "install",
                pct,
                &format!("Inaendelea: {elapsed}s — inasubiri ripoti ya install kutoka {mac}"),
            )
            .await;
        }
    }

    match installed {
        Some(rep) if rep.ok => {
            set_job(
                &db,
                &jobs,
                &job_id,
                "running",
                "install",
                95,
                &format!("Client {mac} imeripoti install kamili: {}", rep.message),
            )
            .await;
        }
        Some(rep) => {
            set_job(
                &db,
                &jobs,
                &job_id,
                "failed",
                "install",
                90,
                &format!("Client {mac} imeripoti install imeshindikana: {}", rep.message),
            )
            .await;
            return;
        }
        None => {
            set_job(
                &db,
                &jobs,
                &job_id,
                "failed",
                "install",
                90,
                &format!(
                    "Timeout: hakuna ripoti ya install kutoka {mac} ndani ya {wait_s}s (client haijajibu)"
                ),
            )
            .await;
            return;
        }
    }

    // ---------- 8. REPORT ----------
    let final_msg = if decision_note.is_empty() {
        format!("Kamili: {} kwenye {mac}", opts.os)
    } else {
        format!("Kamili: {} kwenye {mac} — {decision_note}", opts.os)
    };
    set_job(&db, &jobs, &job_id, "done", "report", 100, &final_msg).await;
}

#[cfg(test)]
mod tests {
    use super::{pxe_assets, pxe_config};

    #[test]
    fn pxe_config_ina_fundi_job_na_api() {
        // Client (fundi/initrd.img) LAZIMA apate job_id + api ili aripoti install.
        let c = pxe_config("ubuntu", "http://10.0.0.5:8080", "http://10.0.0.5", "job-xyz");
        assert!(c.contains("DEFAULT ubuntu"), "{c}");
        assert!(c.contains("KERNEL ubuntu/vmlinuz"), "{c}");
        assert!(c.contains("fundi.api=http://10.0.0.5:8080"), "{c}");
        assert!(c.contains("fundi.http=http://10.0.0.5"), "{c}");
        assert!(c.contains("fundi.job=job-xyz"), "{c}");
        assert!(c.contains("fundi.os=ubuntu"), "{c}");
        // seeds hutoka nginx (http), si API
        assert!(c.contains("url=http://10.0.0.5/images/ubuntu/ubuntu.seed"), "{c}");
    }

    #[test]
    fn pxe_config_win11_inatumia_wimboot() {
        let c = pxe_config("win11", "http://s:8080", "http://s", "j1");
        assert!(c.contains("KERNEL wimboot"), "{c}");
        assert!(c.contains("initrd=win11/boot.wim"), "{c}");
        assert!(c.contains("fundi.job=j1"), "{c}");
    }

    #[test]
    fn pxe_config_default_ni_auto_client() {
        let c = pxe_config("auto", "http://s:8080", "http://s", "j2");
        assert!(c.contains("DEFAULT auto"), "{c}");
        assert!(c.contains("KERNEL fundi/vmlinuz"), "{c}");
        assert!(c.contains("initrd=fundi/initrd.img"), "{c}");
    }

    #[test]
    fn pxe_assets_zinalingana_na_menu() {
        // Majina yanayotumika kwenye pxelinux.cfg/default LAZIMA yalingane.
        assert_eq!(pxe_assets("ubuntu"), vec!["ubuntu/vmlinuz", "ubuntu/initrd"]);
        assert_eq!(pxe_assets("debian"), vec!["debian/linux", "debian/initrd.gz"]);
        assert_eq!(pxe_assets("win11"), vec!["wimboot", "win11/boot.wim"]);
        assert_eq!(pxe_assets("auto"), vec!["fundi/vmlinuz", "fundi/initrd.img"]);
    }
}

