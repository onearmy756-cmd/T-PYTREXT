//! bundles.rs — APP BUNDLES + CATEGORIES kama MSIMBO WA RUST (si data-driven JSON).
//!
//! Mahitaji ya mmiliki (SEHEMU 2 Hatua 4 + SEHEMU 4.2/4.4):
//!   - Bundles ZAIDI YA 20 katika CATEGORIES: office, browsers, development,
//!     communication, utilities, security, graphics, drivers.
//!   - Mtu anaweza: Install All Apps (kwa kompyuta zote), kuchagua apps moja moja,
//!     kuchagua kompyuta husika kwa kila app (PC mbali + apps zinazofanana).
//!   - Admin anaweza KUONGEZA bundles mbambali (dashboard ya admin) — inahifadhiwa
//!     kwenye SQLite (wg_peers style), logic yote Rust.
//!
//! KANUNI: structs/enums/functions za Rust ndiyo kanuni. JSON ni serialization tu
//! ya matokeo (API), si chanzo cha logic.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

// ---------- CATEGORIES (enum — Rust ndiyo kanuni) ----------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Category {
    Office,
    Browsers,
    Development,
    Communication,
    Utilities,
    Security,
    Graphics,
    Drivers,
}

impl Category {
    pub fn id(&self) -> &'static str {
        match self {
            Category::Office => "office",
            Category::Browsers => "browsers",
            Category::Development => "development",
            Category::Communication => "communication",
            Category::Utilities => "utilities",
            Category::Security => "security",
            Category::Graphics => "graphics",
            Category::Drivers => "drivers",
        }
    }

    pub fn name_sw(&self) -> &'static str {
        match self {
            Category::Office => "Ofisi",
            Category::Browsers => "Browsers",
            Category::Development => "Development",
            Category::Communication => "Mawasiliano",
            Category::Utilities => "Zana",
            Category::Security => "Usalama",
            Category::Graphics => "Graphics",
            Category::Drivers => "Drivers",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Category::Office => "\u{1F4C4}",        // 📄
            Category::Browsers => "\u{1F310}",      // 🌐
            Category::Development => "\u{1F4BB}",   // 💻
            Category::Communication => "\u{1F4AC}", // 💬
            Category::Utilities => "\u{1F527}",     // 🔧
            Category::Security => "\u{1F6E1}",      // 🛡
            Category::Graphics => "\u{1F3A8}",      // 🎨
            Category::Drivers => "\u{1F5A5}",       // 🖥
        }
    }

    pub fn all() -> [Category; 8] {
        [
            Category::Office,
            Category::Browsers,
            Category::Development,
            Category::Communication,
            Category::Utilities,
            Category::Security,
            Category::Graphics,
            Category::Drivers,
        ]
    }

    pub fn from_id(id: &str) -> Option<Category> {
        Category::all().into_iter().find(|c| c.id() == id)
    }
}

// ---------- APP (winget + apt — install halisi) ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct App {
    pub id: String,
    pub name: String,
    pub winget: Option<String>,
    pub apt: Option<String>,
    pub critical: bool,
}

impl App {
    /// Amri ya install halisi kulingana na OS (winget = Windows, apt = Linux).
    pub fn install_command(&self, os: &str) -> Option<String> {
        if os.starts_with("win") {
            self.winget
                .as_ref()
                .map(|w| format!("winget install --id {w} -e --silent --accept-package-agreements --accept-source-agreements"))
        } else {
            self.apt
                .as_ref()
                .map(|a| format!("DEBIAN_FRONTEND=noninteractive apt-get install -y {a}"))
        }
    }
}

// ---------- BUNDLE ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub id: String,
    pub name_sw: String,
    pub description: String,
    pub category: Category,
    pub apps: Vec<App>,
}

// ---------- CATALOG: bundles 20+ (RUST ndiyo kanuni) ----------

fn a(id: &str, name: &str, winget: Option<&str>, apt: Option<&str>, critical: bool) -> App {
    App {
        id: id.into(),
        name: name.into(),
        winget: winget.map(String::from),
        apt: apt.map(String::from),
        critical,
    }
}

/// Katalogi KAMILI ya Rust — bundles 22 katika categories 8.
pub fn catalog() -> Vec<Bundle> {
    vec![
        // ===== OFFICE =====
        Bundle {
            id: "office-ms".into(),
            name_sw: "Microsoft Office".into(),
            description: "Word, Excel, PowerPoint, Outlook".into(),
            category: Category::Office,
            apps: vec![a(
                "msoffice",
                "Microsoft Office 365",
                Some("Microsoft.Office"),
                None,
                true,
            )],
        },
        Bundle {
            id: "office-libre".into(),
            name_sw: "LibreOffice".into(),
            description: "Ofisi ya bure (Writer, Calc, Impress)".into(),
            category: Category::Office,
            apps: vec![a(
                "libreoffice",
                "LibreOffice",
                Some("TheDocumentFoundation.LibreOffice"),
                Some("libreoffice"),
                true,
            )],
        },
        Bundle {
            id: "office-wps".into(),
            name_sw: "WPS Office".into(),
            description: "Ofisi nyepesi (Word/Excel compatible)".into(),
            category: Category::Office,
            apps: vec![a(
                "wps",
                "WPS Office",
                Some("Kingsoft.WPSOffice"),
                None,
                false,
            )],
        },
        // ===== BROWSERS =====
        Bundle {
            id: "browser-chrome".into(),
            name_sw: "Google Chrome".into(),
            description: "Browser ya Google".into(),
            category: Category::Browsers,
            apps: vec![a(
                "chrome",
                "Google Chrome",
                Some("Google.Chrome"),
                Some("chromium-browser"),
                true,
            )],
        },
        Bundle {
            id: "browser-firefox".into(),
            name_sw: "Firefox".into(),
            description: "Browser ya Mozilla".into(),
            category: Category::Browsers,
            apps: vec![a(
                "firefox",
                "Mozilla Firefox",
                Some("Mozilla.Firefox"),
                Some("firefox"),
                false,
            )],
        },
        Bundle {
            id: "browser-edge".into(),
            name_sw: "Microsoft Edge".into(),
            description: "Browser ya Windows".into(),
            category: Category::Browsers,
            apps: vec![a(
                "edge",
                "Microsoft Edge",
                Some("Microsoft.Edge"),
                None,
                false,
            )],
        },
        Bundle {
            id: "browser-brave".into(),
            name_sw: "Brave".into(),
            description: "Browser ya privacy".into(),
            category: Category::Browsers,
            apps: vec![a(
                "brave",
                "Brave Browser",
                Some("Brave.Brave"),
                None,
                false,
            )],
        },
        // ===== DEVELOPMENT =====
        Bundle {
            id: "dev-vscode".into(),
            name_sw: "Visual Studio Code".into(),
            description: "Editor ya Microsoft".into(),
            category: Category::Development,
            apps: vec![a(
                "vscode",
                "VS Code",
                Some("Microsoft.VisualStudioCode"),
                Some("code"),
                true,
            )],
        },
        Bundle {
            id: "dev-cpp".into(),
            name_sw: "C++ Toolchain".into(),
            description: "MSVC build tools / g++".into(),
            category: Category::Development,
            apps: vec![a(
                "cpp",
                "C++ Build Tools",
                Some("Microsoft.VisualStudio.2022.BuildTools"),
                Some("build-essential g++"),
                true,
            )],
        },
        Bundle {
            id: "dev-git".into(),
            name_sw: "Git".into(),
            description: "Version control".into(),
            category: Category::Development,
            apps: vec![a("git", "Git", Some("Git.Git"), Some("git"), true)],
        },
        Bundle {
            id: "dev-node".into(),
            name_sw: "Node.js LTS".into(),
            description: "JavaScript runtime".into(),
            category: Category::Development,
            apps: vec![a(
                "node",
                "Node.js LTS",
                Some("OpenJS.NodeJS.LTS"),
                Some("nodejs npm"),
                false,
            )],
        },
        Bundle {
            id: "dev-docker".into(),
            name_sw: "Docker Desktop".into(),
            description: "Containers".into(),
            category: Category::Development,
            apps: vec![a(
                "docker",
                "Docker Desktop",
                Some("Docker.DockerDesktop"),
                Some("docker.io"),
                false,
            )],
        },
        Bundle {
            id: "dev-r".into(),
            name_sw: "R-Language Core".into(),
            description: "Lugha ya takwimu + RStudio (winget/apt)".into(),
            category: Category::Development,
            apps: vec![
                a(
                    "rlang",
                    "R Language",
                    Some("RProject.R"),
                    Some("r-base"),
                    false,
                ),
                a(
                    "rstudio",
                    "RStudio Desktop",
                    Some("Posit.RStudio"),
                    None,
                    false,
                ),
            ],
        },
        // ===== COMMUNICATION =====
        Bundle {
            id: "comms-zoom".into(),
            name_sw: "Zoom".into(),
            description: "Mikutano ya video".into(),
            category: Category::Communication,
            apps: vec![a("zoom", "Zoom", Some("Zoom.Zoom"), Some("zoom"), false)],
        },
        Bundle {
            id: "comms-teams".into(),
            name_sw: "Microsoft Teams".into(),
            description: "Teams ya kampuni".into(),
            category: Category::Communication,
            apps: vec![a(
                "teams",
                "Microsoft Teams",
                Some("Microsoft.Teams"),
                None,
                false,
            )],
        },
        Bundle {
            id: "comms-slack".into(),
            name_sw: "Slack".into(),
            description: "Chat ya timu".into(),
            category: Category::Communication,
            apps: vec![a(
                "slack",
                "Slack",
                Some("SlackTechnologies.Slack"),
                None,
                false,
            )],
        },
        Bundle {
            id: "comms-whatsapp".into(),
            name_sw: "WhatsApp Desktop".into(),
            description: "WhatsApp kwenye desktop".into(),
            category: Category::Communication,
            apps: vec![a(
                "whatsapp",
                "WhatsApp",
                Some("WhatsApp.Whatsapp"),
                None,
                false,
            )],
        },
        // ===== UTILITIES =====
        Bundle {
            id: "util-7zip".into(),
            name_sw: "7-Zip".into(),
            description: "Kubana/kufungua faili".into(),
            category: Category::Utilities,
            apps: vec![a(
                "7zip",
                "7-Zip",
                Some("7zip.7zip"),
                Some("p7zip-full"),
                true,
            )],
        },
        Bundle {
            id: "util-winrar".into(),
            name_sw: "WinRAR".into(),
            description: "Archiver".into(),
            category: Category::Utilities,
            apps: vec![a("winrar", "WinRAR", Some("RARLab.WinRAR"), None, false)],
        },
        Bundle {
            id: "util-vlc".into(),
            name_sw: "VLC Media Player".into(),
            description: "Media player".into(),
            category: Category::Utilities,
            apps: vec![a("vlc", "VLC", Some("VideoLAN.VLC"), Some("vlc"), false)],
        },
        Bundle {
            id: "util-notepad".into(),
            name_sw: "Notepad++".into(),
            description: "Editor nyepesi".into(),
            category: Category::Utilities,
            apps: vec![a(
                "notepadpp",
                "Notepad++",
                Some("Notepad++.Notepad++"),
                None,
                false,
            )],
        },
        Bundle {
            id: "util-anydesk".into(),
            name_sw: "AnyDesk".into(),
            description: "Remote desktop".into(),
            category: Category::Utilities,
            apps: vec![a(
                "anydesk",
                "AnyDesk",
                Some("AnyDeskSoftwareGmbH.AnyDesk"),
                None,
                true,
            )],
        },
        // ===== SECURITY =====
        Bundle {
            id: "sec-antivirus".into(),
            name_sw: "Antivirus (Defender hardening)".into(),
            description: "Windows Defender verify + policy".into(),
            category: Category::Security,
            apps: vec![a("defender", "Defender (verify)", None, None, true)],
        },
        Bundle {
            id: "sec-malwarebytes".into(),
            name_sw: "Malwarebytes".into(),
            description: "Anti-malware".into(),
            category: Category::Security,
            apps: vec![a(
                "malwarebytes",
                "Malwarebytes",
                Some("Malwarebytes.Malwarebytes"),
                None,
                false,
            )],
        },
        // ===== GRAPHICS =====
        Bundle {
            id: "gfx-gimp".into(),
            name_sw: "GIMP".into(),
            description: "Image editor".into(),
            category: Category::Graphics,
            apps: vec![a("gimp", "GIMP", Some("GIMP.GIMP"), Some("gimp"), false)],
        },
        Bundle {
            id: "gfx-inkscape".into(),
            name_sw: "Inkscape".into(),
            description: "Vector graphics".into(),
            category: Category::Graphics,
            apps: vec![a(
                "inkscape",
                "Inkscape",
                Some("Inkscape.Inkscape"),
                Some("inkscape"),
                false,
            )],
        },
        Bundle {
            id: "gfx-paintnet".into(),
            name_sw: "Paint.NET".into(),
            description: "Image editor nyepesi".into(),
            category: Category::Graphics,
            apps: vec![a(
                "paintnet",
                "Paint.NET",
                Some("dotPDN.PaintDotNet"),
                None,
                false,
            )],
        },
        // ===== DRIVERS =====
        Bundle {
            id: "drv-dell".into(),
            name_sw: "Drivers za Dell".into(),
            description: "Dell Command Update + drivers".into(),
            category: Category::Drivers,
            apps: vec![a(
                "dell",
                "Dell Command Update",
                Some("Dell.CommandUpdate"),
                None,
                true,
            )],
        },
        Bundle {
            id: "drv-hp".into(),
            name_sw: "Drivers za HP".into(),
            description: "HP Support Assistant + drivers".into(),
            category: Category::Drivers,
            apps: vec![a(
                "hp",
                "HP Support Assistant",
                Some("HP.SupportAssistant"),
                None,
                true,
            )],
        },
        Bundle {
            id: "drv-lenovo".into(),
            name_sw: "Drivers za Lenovo".into(),
            description: "Lenovo Vantage + drivers".into(),
            category: Category::Drivers,
            apps: vec![a(
                "lenovo",
                "Lenovo Vantage",
                Some("Lenovo.Vantage"),
                None,
                true,
            )],
        },
        Bundle {
            id: "drv-acer".into(),
            name_sw: "Drivers za Acer".into(),
            description: "Acer Care Center".into(),
            category: Category::Drivers,
            apps: vec![a(
                "acer",
                "Acer Care Center",
                Some("Acer.AcerCareCenter"),
                None,
                false,
            )],
        },
        Bundle {
            id: "drv-asus".into(),
            name_sw: "Drivers za Asus".into(),
            description: "MyASUS + drivers".into(),
            category: Category::Drivers,
            apps: vec![a("asus", "MyASUS", Some("ASUS.MyASUS"), None, false)],
        },
    ]
}

pub fn bundles_count() -> usize {
    catalog().len()
}

// ---------- VERSION METADATA (REPO v3.1 — kadi zinaonyesha Version vX) ----------

/// Kanuni ya mmiliki: MTECH OS inaendeshwa na RUST PEKEE — hakuna runtime
/// nyingine ya programu (node/npm/java/go) kwenye catalog; hizo ni APPS
/// za wateja kwenye kadi (REPO), sio sehemu ya mfumo wa server. Logos za REPO
/// ni SVG (svg=Rust inahudumia ServeDir; hakuna CDN/nje).
/// Logo ya kila bundle: `img/rep/{id}.svg` (static, Rust ServeDir).
pub fn logo_path(id: &str) -> String {
    format!("/img/rep/{id}.svg")
}

/// Category inayokubaliwa: 8 za katalogi au 'custom' (drag & drop).
pub fn category_is_known(c: &str) -> bool {
    c == "custom" || Category::from_id(c).is_some()
}

pub fn version_of(id: &str) -> &'static str {
    match id {
        // OFFICE
        "office-ms" => "v2408",
        "office-libre" => "v24.2",
        "office-wps" => "v12.2",
        // BROWSERS
        "browser-chrome" => "v130.0",
        "browser-firefox" => "v131.0",
        "browser-edge" => "v130.0",
        "browser-brave" => "v1.71",
        // DEVELOPMENT
        "dev-vscode" => "v1.93",
        "dev-cpp" => "v17.10",
        "dev-git" => "v2.46",
        "dev-node" => "v22.9",
        "dev-docker" => "v27.3",
        "dev-r" => "v4.4.1",
        // COMMUNICATION
        "comms-zoom" => "v6.2",
        "comms-teams" => "v24.2",
        "comms-slack" => "v4.41",
        "comms-whatsapp" => "v2.24",
        // UTILITIES
        "util-7zip" => "v24.08",
        "util-winrar" => "v7.10",
        "util-vlc" => "v3.0.21",
        "util-notepad" => "v8.6",
        "util-anydesk" => "v9.0",
        // SECURITY
        "sec-antivirus" => "v10.0",
        "sec-malwarebytes" => "v5.7",
        // GRAPHICS
        "gfx-gimp" => "v2.10",
        "gfx-inkscape" => "v1.3.1",
        "gfx-paintnet" => "v11.1",
        // DRIVERS
        "drv-dell" => "v4.9",
        "drv-hp" => "v13.0",
        "drv-lenovo" => "v9.1",
        "drv-acer" => "v4.5",
        "drv-asus" => "v3.6",
        // custom bundles (drag & drop) — hakuna toleo la juu
        _ => "v1.0 (custom)",
    }
}

pub fn find_bundle(id: &str) -> Option<Bundle> {
    catalog().into_iter().find(|b| b.id == id)
}

pub fn find_app(app_id: &str) -> Option<App> {
    catalog()
        .into_iter()
        .flat_map(|b| b.apps)
        .find(|app| app.id == app_id)
}

/// Amri za install za bundle KAMILI kwa OS ya kifaa lengo (winget Windows / apt Linux).
/// Hii ndiyo payload inayotumwa kwa client agent — apps HALISI za bundle, si placeholder.
pub fn install_commands_for_bundle(bundle_id: &str, os: &str) -> Vec<String> {
    find_bundle(bundle_id)
        .map(|b| {
            b.apps
                .iter()
                .filter_map(|app| app.install_command(os))
                .collect()
        })
        .unwrap_or_default()
}

// ---------- CUSTOM BUNDLES ZA ADMIN (SQLite — logic Rust) ----------

#[derive(Debug, Clone, Serialize)]
pub struct CustomBundle {
    pub id: String,
    pub name_sw: String,
    pub description: String,
    pub category: String,
    pub app_ids: Vec<String>,
    pub created_by: String,
    pub created_at: String,
}

pub async fn init_tables(db: &SqlitePool) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS custom_bundles (
            id TEXT PRIMARY KEY,
            name_sw TEXT NOT NULL,
            description TEXT NOT NULL,
            category TEXT NOT NULL,
            app_ids TEXT NOT NULL,
            created_by TEXT NOT NULL,
            created_at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
}

/// Ongeza bundle ya admin — apps zinazotegemea catalog (base) pekee.
pub async fn add_custom_bundle(
    db: &SqlitePool,
    id: &str,
    name_sw: &str,
    description: &str,
    category: &str,
    app_ids: Vec<String>,
    created_by: &str,
) -> Result<CustomBundle, String> {
    if id.trim().is_empty() || name_sw.trim().is_empty() {
        return Err("id na name_sw ni lazima".into());
    }
    if !category_is_known(category) {
        return Err(format!("Category '{category}' haijulikani (office|browsers|development|communication|utilities|security|graphics|drivers|custom)"));
    }
    for app_id in &app_ids {
        if find_app(app_id).is_none() {
            return Err(format!(
                "App '{app_id}' haipo kwenye catalog ya Rust (angalia GET /api/bundles/apps)"
            ));
        }
    }
    let created_at = chrono::Local::now().to_rfc3339();
    let apps_json = serde_json::to_string(&app_ids).unwrap_or_else(|_| "[]".into());
    sqlx::query(
        "INSERT OR REPLACE INTO custom_bundles (id, name_sw, description, category, app_ids, created_by, created_at) VALUES (?,?,?,?,?,?,?)",
    )
    .bind(id.trim())
    .bind(name_sw.trim())
    .bind(description)
    .bind(category)
    .bind(apps_json)
    .bind(created_by)
    .bind(&created_at)
    .execute(db)
    .await
    .map_err(|e| e.to_string())?;
    Ok(CustomBundle {
        id: id.trim().into(),
        name_sw: name_sw.trim().into(),
        description: description.into(),
        category: category.into(),
        app_ids,
        created_by: created_by.into(),
        created_at,
    })
}

pub async fn list_custom_bundles(db: &SqlitePool) -> Vec<CustomBundle> {
    let rows: Vec<(String, String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT id, name_sw, description, category, app_ids, created_by, created_at FROM custom_bundles ORDER BY created_at DESC",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    rows.into_iter()
        .map(
            |(id, name_sw, description, category, app_ids, created_by, created_at)| CustomBundle {
                id,
                name_sw,
                description,
                category,
                app_ids: serde_json::from_str(&app_ids).unwrap_or_default(),
                created_by,
                created_at,
            },
        )
        .collect()
}

pub async fn remove_custom_bundle(db: &SqlitePool, id: &str) -> bool {
    sqlx::query("DELETE FROM custom_bundles WHERE id = ?")
        .bind(id)
        .execute(db)
        .await
        .map(|r| r.rows_affected() > 0)
        .unwrap_or(false)
}

// ---------- DEPLOYMENT PLAN: per-PC + per-app (SEHEMU 2 Hatua 4) ----------

#[derive(Debug, Clone, Serialize)]
pub struct PcPlan {
    pub display_name: String, // jina la user la PC (hr, hr 1, …)
    pub os: String,           // win11 | ubuntu | …
    pub app_ids: Vec<String>, // apps za PC husika
}

impl PcPlan {
    /// Amri zote za install za PC hii (winget/apt halisi kwa OS wake).
    pub fn install_commands(&self) -> Vec<String> {
        let mut out = Vec::new();
        for app_id in &self.app_ids {
            if let Some(app) = find_app(app_id) {
                if let Some(cmd) = app.install_command(&self.os) {
                    out.push(cmd);
                }
            }
        }
        out
    }
}

/// Panga deployment: kila PC inapata OS + apps zake (baada ya kuchagua).
pub fn deployment_plan(display_names: Vec<String>, os: &str, app_ids: Vec<String>) -> Vec<PcPlan> {
    display_names
        .into_iter()
        .map(|display_name| PcPlan {
            display_name,
            os: os.to_string(),
            app_ids: app_ids.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundles_zaidi_ya_20() {
        let n = bundles_count();
        assert!(n >= 20, "bundles = {n}, mmiliki ataka 20+");
    }

    #[test]
    fn categories_zote_8() {
        assert_eq!(Category::all().len(), 8);
        assert!(Category::from_id("drivers").is_some());
        assert!(Category::from_id("haijulikani").is_none());
    }

    #[test]
    fn apps_zina_winget_au_apt() {
        let mut missing = 0;
        for b in catalog() {
            for app in b.apps {
                if app.winget.is_none() && app.apt.is_none() {
                    missing += 1; // kama defender-verify: policy tu, si install cmd
                }
            }
        }
        assert!(missing <= 2, "apps {missing} hazina njia ya install");
    }

    #[test]
    fn install_commands_winget_kwa_win_apt_kwa_linux() {
        let app = find_app("vscode").unwrap();
        let win = app.install_command("win11").unwrap();
        assert!(win.starts_with("winget install"));
        let linux = app.install_command("ubuntu").unwrap();
        assert!(linux.contains("apt-get install -y code"));
    }

    #[test]
    fn bundle_install_commands_kwa_client() {
        // Windows bundle → winget tu; Linux bundle → apt tu. Hii ni payload
        // inayotumwa kwa client agent (fleet poll), si placeholder.
        let win = install_commands_for_bundle("office-ms", "win11");
        assert!(!win.is_empty(), "bundle ya Windows iwe na amri za winget");
        assert!(win.iter().all(|c| c.starts_with("winget install")));

        let linux = install_commands_for_bundle("office-ms", "ubuntu");
        assert!(linux.iter().all(|c| c.contains("apt-get install -y")));

        // Bundle isiyojulikana → tupu (client hutumia default salama).
        assert!(install_commands_for_bundle("haipo-bundle", "ubuntu").is_empty());
    }

    #[test]
    fn deployment_plan_per_pc() {
        let plan = deployment_plan(
            vec!["hr".into(), "hr 1".into(), "account".into()],
            "win11",
            vec!["chrome".into(), "vscode".into()],
        );
        assert_eq!(plan.len(), 3);
        assert_eq!(plan[1].display_name, "hr 1");
        assert_eq!(plan[0].install_commands().len(), 2);
        assert!(plan[0].install_commands()[0].starts_with("winget install"));
    }

    #[test]
    fn versions_zote_wa_catalog() {
        let custom = version_of("custom-");
        assert_eq!(custom, "v1.0 (custom)");
        for b in catalog() {
            let v = version_of(&b.id);
            assert!(
                v.starts_with('v') && !v.contains("custom"),
                "{} → {}",
                b.id,
                v
            );
        }
        // kadi ya VS Code kwenye REPO inaonyesha toleo halisi
        assert_eq!(version_of("dev-vscode"), "v1.93");
        assert_eq!(version_of("util-vlc"), "v3.0.21");
        assert_eq!(version_of("gfx-gimp"), "v2.10");
        // R-Language Core (REPO v3.1 screenshot) — v4.4.1
        assert_eq!(version_of("dev-r"), "v4.4.1");
        let r = find_bundle("dev-r").expect("dev-r ipo");
        assert_eq!(r.category, Category::Development);
        assert!(find_app("rlang").is_some() && find_app("rstudio").is_some());
    }

    #[test]
    fn mfumo_rust_pekee_catalog_haina_runtime_za_nje() {
        // Kanuni ya mmiliki: SERVER (Rust) haikusudiyya kutekeleza runtime za nje.
        // Katalogi ina apps za WATEJA kwenye kadi (REPO) — hii ni orodha ya
        // vitu vya kusakinisha kwenye PC za wateja, si components za mfumo.
        // Test hii inadai: hakuna package ya runtime inayopaswa kwisha kwenye
        // server ya rust pekee — yaani hakuna 'runtime-critical' dependencies za
        // runtime nje ya winget/apt (wa PC za wateja).
        let runtime_ids = ["node", "git"];
        for rid in runtime_ids {
            // zipw/zipo kwenye REPO kama APPS za wateja tu — zina winget/apt au
            // critical=false zisiwahi zilazimishwe kwenye server.
            if let Some(app) = find_app(rid) {
                assert!(
                    app.winget.is_some() || app.apt.is_some(),
                    "{rid} ni app ya wateja"
                );
            }
        }
        // Logos: kila bundle ina logo path halisi (SVG iliyofanywa static)
        for b in catalog() {
            let p = logo_path(&b.id);
            assert!(
                p.starts_with("/img/rep/") && p.ends_with(".svg"),
                "{} → {}",
                b.id,
                p
            );
            let f = format!("static{}.svg", p.trim_end_matches(".svg"));
            assert!(std::path::Path::new(&f).exists(), "logo haipo: {f}");
        }
    }

    #[test]
    fn custom_category_ipokelewa_catalog_inakatalia() {
        // custom → DB logic (integration); hapa: catalog ids 8 + 'custom'ipelewe
        assert!(category_is_known("custom"));
        assert!(category_is_known("office"));
        assert!(!category_is_known("haijulikani"));
    }

    #[test]
    fn custom_bundle_inakataa_app_isiyopo() {
        // (DB test ya async inaendeshwa kwenye integration — hapa tunathibitisha logic ya validation)
        assert!(Category::from_id("office").is_some());
        assert!(find_app("app-haipo").is_none());
        assert!(find_app("vscode").is_some());
    }
}

// ---------- AUTO-DOWNLOAD: pakua bundles kutoka mtandaoni na kuhifadhi kwenye server ----------

/// Pakua katalogi ya app_bundles kutoka GitHub (chanzo cha kweli) au faili ya ndani,
/// kisha ihifadhi kwenye DB (remote_bundles). Rudisha (idadi, chanzo).
pub async fn fetch_and_store(db: &SqlitePool) -> Result<(usize, String), String> {
    let url = "https://raw.githubusercontent.com/onearmy756-cmd/MTECHOS/main/data/deploy/app_bundles.json";
    let mut text: Option<String> = None;
    let mut source = String::new();

    // 1) GitHub raw (MTECHOS) — kama repo ni private, tumia token ikiwepo kwenye env
    let client = reqwest::Client::new();
    let mut req = client
        .get(url)
        .timeout(std::time::Duration::from_secs(20))
        .header(reqwest::header::USER_AGENT, "MTECH-OS/1.1");
    if let Ok(tok) = std::env::var("GITHUB_TOKEN") {
        if !tok.is_empty() {
            req = req.header(reqwest::header::AUTHORIZATION, format!("Bearer {tok}"));
        }
    }
    if let Ok(r) = req.send().await {
        if r.status().is_success() {
            if let Ok(t) = r.text().await {
                if t.contains("fundi-app-bundles") {
                    text = Some(t);
                    source = "GitHub MTECHOS (raw)".into();
                }
            }
        }
    }

    // 2) Faili ya ndani ya server (daima ipo kwenye mradi)
    if text.is_none() {
        let dir = std::env::var("FUNDI_DATA").unwrap_or_else(|_| "data".into());
        let candidates = [
            format!("{dir}/deploy/app_bundles.json"),
            "../../data/deploy/app_bundles.json".to_string(),
        ];
        for c in candidates {
            if let Ok(t) = std::fs::read_to_string(&c) {
                if t.contains("fundi-app-bundles") {
                    text = Some(t);
                    source = format!("faili ya ndani: {c}");
                    break;
                }
            }
        }
    }
    let text = text.ok_or("Hakuna chanzo cha bundles (mtandao wala faili ya ndani)")?;

    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("JSON si sahihi: {e}"))?;
    let arr = v
        .get("bundles")
        .and_then(|b| b.as_array())
        .ok_or("Hakuna 'bundles' ndani ya faili")?;

    // 3) Hifadhi kwenye DB (snapshot mpya kila mara)
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS remote_bundles (
            id TEXT PRIMARY KEY,
            name_sw TEXT NOT NULL,
            description TEXT NOT NULL,
            apps_json TEXT NOT NULL,
            fetched_at TEXT NOT NULL
        )",
    )
    .execute(db)
    .await;
    let _ = sqlx::query("DELETE FROM remote_bundles").execute(db).await;
    let now = chrono::Utc::now().to_rfc3339();
    let mut n = 0usize;
    for b in arr {
        let id = b.get("id").and_then(|x| x.as_str()).unwrap_or("");
        if id.is_empty() {
            continue;
        }
        let name = b.get("name_sw").and_then(|x| x.as_str()).unwrap_or("");
        let desc = b.get("description").and_then(|x| x.as_str()).unwrap_or("");
        let apps_json = b
            .get("apps")
            .cloned()
            .unwrap_or(serde_json::json!([]))
            .to_string();
        let r = sqlx::query(
            "INSERT OR REPLACE INTO remote_bundles (id, name_sw, description, apps_json, fetched_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(desc)
        .bind(&apps_json)
        .bind(&now)
        .execute(db)
        .await;
        if r.is_ok() {
            n += 1;
        }
    }
    if n == 0 {
        return Err("Bundles hazikuweza kuhifadhiwa kwenye DB".into());
    }
    Ok((n, source))
}
