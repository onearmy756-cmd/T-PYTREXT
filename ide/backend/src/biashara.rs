//! biashara.rs — MTECHOS GROUP: jukwaa la biashara la huduma za teknolojia.
//!
//! Mahitaji ya mmiliki yameunganishwa hapa kama sehemu MOJA ya ukweli:
//!   - **Sarafu mbili**: mteja hulipa kwa **USD (Stripe)** au **TZS (simu/benki)**.
//!     Ubadilishaji ni wa KWELI (`convert`), si maandishi.
//!   - **Katalogi ya huduma**: cyber security, software development, AI fine-tuning,
//!     chatbot, custom AI, OS, IoT, gaming, finance, elimu, kilimo, usafiri, graphics,
//!     CV na zingine. Huduma za Tanzania pekee (BRELA/TRA/…) zina `audience: tz`.
//!   - **Wafanyakazi**: jina, picha, taaluma, ngazi, idara, zana walizopewa.
//!   - **Maombi ya mteja**: huingia, admin HUIDHINISHA, kisha HUPEWA mfanyakazi
//!     husika. Mteja HUONA maendeleo (`progress`) hatua kwa hatua.
//!   - **Mikataba**: hutiwa sahihi ya ECDSA (pkicrypto) — muhuri wa kriptografia.
//!   - **Ufuatiliaji wa maeneo**: mikoa yote ya Tanzania + nchi za kimataifa.
//!   - **Zana kwa mfanyakazi**: `bz_tool_grants` — mfanyakazi HARUHUSIWI kutumia
//!     zana asizopewa (`tool_allowed`).
//!
//! Tables: bz_currency bz_services bz_departments bz_employees bz_tool_grants
//!         bz_requests bz_request_events bz_contracts bz_coverage bz_audit

use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

pub async fn init_tables(db: &SqlitePool) {
    let ddl = [
        r#"CREATE TABLE IF NOT EXISTS bz_currency (
            code TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            rate_tzs REAL NOT NULL DEFAULT 1.0,
            active INTEGER NOT NULL DEFAULT 1,
            updated_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS bz_services (
            id TEXT PRIMARY KEY,
            code TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL DEFAULT '',
            name_sw TEXT NOT NULL DEFAULT '',
            category TEXT NOT NULL DEFAULT 'software',
            audience TEXT NOT NULL DEFAULT 'global', -- global|tz
            currency TEXT NOT NULL DEFAULT 'TZS',
            base_price_cents INTEGER NOT NULL DEFAULT 0,
            icon TEXT NOT NULL DEFAULT '',
            active INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS bz_departments (
            id TEXT PRIMARY KEY,
            code TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL DEFAULT '',
            tier TEXT NOT NULL DEFAULT 'operations', -- executive|finance|people|growth|operations
            head_employee_id TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS bz_employees (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL DEFAULT '',
            full_name TEXT NOT NULL DEFAULT '',
            photo TEXT NOT NULL DEFAULT '',
            profession TEXT NOT NULL DEFAULT '',
            level TEXT NOT NULL DEFAULT 'junior',
            department_id TEXT NOT NULL DEFAULT '',
            phone TEXT NOT NULL DEFAULT '',
            email TEXT NOT NULL DEFAULT '',
            tools TEXT NOT NULL DEFAULT '[]',
            status TEXT NOT NULL DEFAULT 'active', -- active|suspended|left
            hired_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS bz_tool_grants (
            id TEXT PRIMARY KEY,
            employee_id TEXT NOT NULL,
            tool_code TEXT NOT NULL,
            granted_by TEXT NOT NULL DEFAULT '',
            granted_at TEXT NOT NULL,
            revoked_at TEXT,
            UNIQUE(employee_id, tool_code)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS bz_requests (
            id TEXT PRIMARY KEY,
            ref TEXT NOT NULL UNIQUE,
            client_name TEXT NOT NULL DEFAULT '',
            client_phone TEXT NOT NULL DEFAULT '',
            client_email TEXT NOT NULL DEFAULT '',
            service_id TEXT NOT NULL DEFAULT '',
            title TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL DEFAULT '',
            country TEXT NOT NULL DEFAULT 'TZ',
            region TEXT NOT NULL DEFAULT '',
            currency TEXT NOT NULL DEFAULT 'TZS',
            amount_cents INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'submitted',
            progress INTEGER NOT NULL DEFAULT 0,
            priority TEXT NOT NULL DEFAULT 'normal',
            assignee_id TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            approved_at TEXT,
            assigned_at TEXT,
            completed_at TEXT
        )"#,
        r#"CREATE TABLE IF NOT EXISTS bz_request_events (
            id TEXT PRIMARY KEY,
            request_id TEXT NOT NULL,
            actor TEXT NOT NULL DEFAULT '',
            kind TEXT NOT NULL DEFAULT 'note',
            note TEXT NOT NULL DEFAULT '',
            progress INTEGER NOT NULL DEFAULT 0,
            visible_to_client INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS bz_contracts (
            id TEXT PRIMARY KEY,
            request_id TEXT NOT NULL DEFAULT '',
            title TEXT NOT NULL DEFAULT '',
            scope TEXT NOT NULL DEFAULT '',
            amount_cents INTEGER NOT NULL DEFAULT 0,
            currency TEXT NOT NULL DEFAULT 'TZS',
            status TEXT NOT NULL DEFAULT 'draft',
            client_sig TEXT NOT NULL DEFAULT '',
            client_signed_at TEXT,
            admin_sig TEXT NOT NULL DEFAULT '',
            admin_signed_at TEXT,
            seal_hash TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS bz_coverage (
            id TEXT PRIMARY KEY,
            kind TEXT NOT NULL DEFAULT 'tz_region', -- tz_region|country
            code TEXT NOT NULL,
            name TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'planned', -- live|planned|paused
            requests_count INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL,
            UNIQUE(kind, code)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS bz_audit (
            id TEXT PRIMARY KEY,
            actor TEXT NOT NULL DEFAULT '',
            action TEXT NOT NULL DEFAULT '',
            target TEXT NOT NULL DEFAULT '',
            detail TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        )"#,
        r#"CREATE TABLE IF NOT EXISTS bz_branding (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL DEFAULT '',
            updated_by TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL
        )"#,
    ];
    for d in ddl {
        let _ = sqlx::query(d).execute(db).await;
    }
    seed(db).await;
}

fn now() -> String {
    chrono::Local::now().to_rfc3339()
}

fn uid() -> String {
    uuid::Uuid::new_v4().to_string()
}

// =====================================================================
// SARAFU
// =====================================================================

/// Viwango vya ubadilishaji (TZS kwa uniti moja). TZS = 1.0 kwa ufafanuzi.
pub const RATES: [(&str, &str, f64); 7] = [
    ("TZS", "Shilingi ya Tanzania", 1.0),
    ("USD", "Dola ya Marekani", 2600.0),
    ("EUR", "Yuro", 2850.0),
    ("GBP", "Pauni ya Uingereza", 3300.0),
    ("KES", "Shilingi ya Kenya", 20.0),
    ("ZAR", "Randi ya Afrika Kusini", 140.0),
    ("INR", "Rupia ya India", 31.0),
];

pub fn norm_currency(c: &str) -> String {
    let u = c.trim().to_ascii_uppercase();
    if RATES.iter().any(|(code, _, _)| *code == u) {
        u
    } else {
        "TZS".to_string()
    }
}

pub fn rate(code: &str) -> f64 {
    let c = norm_currency(code);
    RATES
        .iter()
        .find(|(k, _, _)| *k == c)
        .map(|(_, _, r)| *r)
        .unwrap_or(1.0)
}

/// Badilisha kiasi (kwa *cents* ya sarafu husika) kutoka sarafu moja hadi nyingine.
/// Zote zina decimal 2, hivyo kipimo ni kimoja — ubadilishaji ni uwiano wa viwango.
pub fn convert(amount_cents: i64, from: &str, to: &str) -> i64 {
    let (f, t) = (rate(from), rate(to));
    if t <= 0.0 {
        return amount_cents;
    }
    ((amount_cents as f64) * f / t).round() as i64
}

/// Kiasi cha chini kinachokubalika kwa sarafu (kuzuia 0/negative).
pub fn amount_ok(amount_cents: i64) -> bool {
    amount_cents > 0
}

// =====================================================================
// KATALOGI YA HUDUMA
// =====================================================================

/// Huduma zote za kampuni. `audience: tz` = Watanzania pekee (BRELA/TRA/…).
pub const SERVICES: [(&str, &str, &str, &str, &str); 24] = [
    // (code, name, name_sw, category, audience)
    ("cyber_sec", "Cyber Security", "Usalama wa Mtandao", "security", "global"),
    ("pentest", "Penetration Testing", "Upimaji wa Uvunjifu", "security", "global"),
    ("soc", "Security Operations Centre", "Kituo cha Ulinzi", "security", "global"),
    ("sw_dev", "Software Development", "Uundaji wa Programu", "software", "global"),
    ("web_app", "Web Applications", "Programu za Wavuti", "software", "global"),
    ("mobile_app", "Mobile Applications", "Programu za Simu", "software", "global"),
    ("desktop_app", "Desktop Applications", "Programu za Kompyuta", "software", "global"),
    ("os_eng", "Operating Systems", "Mifumo ya Uendeshaji", "software", "global"),
    ("ai_finetune", "AI Fine-Tuning", "Urekebishaji wa AI", "ai", "global"),
    ("ai_chatbot", "AI Chatbot", "Roboti ya Mazungumzo", "ai", "global"),
    ("custom_ai", "Custom AI", "AI Maalum", "ai", "global"),
    ("ai_agents", "AI Agent Workforce", "Wafanyakazi wa AI", "ai", "global"),
    ("iot", "IoT Solutions", "Vifaa Vinavyohusiana", "hardware", "global"),
    ("gaming", "Game Development", "Uundaji wa Michezo", "software", "global"),
    ("fintech", "Financial Systems", "Mifumo ya Fedha", "finance", "global"),
    ("edtech", "Education Technology", "Teknolojia ya Elimu", "education", "global"),
    ("agritech", "Agriculture Technology", "Teknolojia ya Kilimo", "agriculture", "global"),
    ("transport", "Transport & Logistics", "Usafiri na Usafirishaji", "transport", "global"),
    ("graphics", "Graphics Design", "Ubunifu wa Picha", "creative", "global"),
    ("cv_brand", "CV & Personal Branding", "CV na Utambulisho", "creative", "global"),
    ("cloud_devops", "Cloud & DevOps", "Wingu na Uendeshaji", "software", "global"),
    ("data_ai_ops", "Data Engineering", "Uhandisi wa Data", "ai", "global"),
    ("brela", "BRELA Registration", "Usajili wa BRELA", "local", "tz"),
    ("tra_tax", "TRA Tax Services", "Huduma za Kodi TRA", "local", "tz"),
];

pub fn norm_category(c: &str) -> &'static str {
    match c.trim().to_lowercase().as_str() {
        "security" | "usalama" => "security",
        "software" | "programu" => "software",
        "ai" | "artificial" => "ai",
        "hardware" | "iot" => "hardware",
        "finance" | "fedha" => "finance",
        "education" | "elimu" => "education",
        "agriculture" | "kilimo" => "agriculture",
        "transport" | "usafiri" => "transport",
        "creative" | "ubunifu" => "creative",
        "local" | "tz" => "local",
        _ => "software",
    }
}

// =====================================================================
// WAFANYAKAZI, IDARA NA ZANA
// =====================================================================

/// Ngazi za wafanyakazi (mpangilio wa juu hadi chini).
pub const LEVELS: [&str; 6] = ["intern", "junior", "mid", "senior", "lead", "executive"];

pub fn norm_level(l: &str) -> &'static str {
    match l.trim().to_lowercase().as_str() {
        "intern" | "mwanafunzi" => "intern",
        "junior" | "mdogo" => "junior",
        "mid" | "katikati" => "mid",
        "senior" | "mkubwa" => "senior",
        "lead" | "kiongozi" => "lead",
        "executive" | "mtendaji" | "ceo" | "mkuu" => "executive",
        _ => "junior",
    }
}

/// Idara za usimamizi — CEO ndiye mmiliki (Admin Mkuu).
pub const DEPARTMENTS: [(&str, &str, &str, &str); 5] = [
    ("ceo", "CEO Management", "Usimamizi wa Mkurugenzi", "executive"),
    ("employee", "Employee Management", "Usimamizi wa Wafanyakazi", "people"),
    ("finance", "Finance Management", "Usimamizi wa Fedha", "finance"),
    ("marketing", "Marketing", "Masoko", "growth"),
    ("hr", "HR Management", "Usimamizi wa Rasilimali Watu", "people"),
];

/// Zana zote za kazi zilizopo ndani ya mfumo. Mfanyakazi HURUHUSIWA tu zile
/// alizopewa (`bz_tool_grants`) — hakuna kutumia zana za nje.
pub const TOOLS: [(&str, &str, &str); 20] = [
    ("ide_code_run", "Kukimbiza Code", "software"),
    ("ide_build", "Kujenga Miradi", "software"),
    ("ide_testkit", "Vipimo vya Kiotomatiki", "software"),
    ("ide_e2e", "Vipimo vya Mwisho-hadi-Mwisho", "software"),
    ("forge", "Kizalisha cha Miradi", "software"),
    ("code_audit", "Ukaguzi wa Code", "security"),
    ("secpack", "Pakiti ya Ulinzi", "security"),
    ("threatintel", "Akili ya Vitisho", "security"),
    ("gateprobe", "Uchunguzi wa Lango", "security"),
    ("fastscan", "Uchanganuzi wa Haraka", "security"),
    ("aiengine", "Injini ya AI", "ai"),
    ("forge_generators", "Vizalisha vya AI", "ai"),
    ("llm_setup", "Usanidi wa Model", "ai"),
    ("vector", "Hifadhi ya Vector", "ai"),
    ("scribe", "Uandishi wa Maneno", "media"),
    ("diarize", "Kutenganisha Wazungumzaji", "media"),
    ("forge_rust3d", "Uundaji wa 3D", "creative"),
    ("report", "Ripoti zenye Muhuri", "operations"),
    ("maps", "Ramani", "operations"),
    ("analytics", "Uchanganuzi wa Data", "operations"),
];

pub fn tool_exists(code: &str) -> bool {
    TOOLS.iter().any(|(c, _, _)| *c == code)
}

/// Zana zinazohusiana na kategoria ya huduma. Ombi la `security` linahitaji
/// mfanyakazi mwenye zana za usalama, si yeyote.
pub fn category_tools(category: &str) -> &'static [&'static str] {
    match category {
        "security" => &["code_audit", "secpack", "threatintel", "gateprobe", "fastscan"],
        "software" => &["ide_code_run", "ide_build", "ide_testkit", "ide_e2e", "forge"],
        "ai" => &["aiengine", "forge_generators", "llm_setup", "vector"],
        "hardware" => &["forge_rust3d", "ide_code_run", "ide_build"],
        "finance" => &["report", "analytics", "ide_code_run"],
        "education" => &["scribe", "report", "ide_code_run"],
        "agriculture" => &["maps", "analytics", "ide_code_run"],
        "transport" => &["maps", "analytics", "ide_code_run"],
        "creative" => &["forge_rust3d", "scribe", "report"],
        "local" => &["report", "ide_code_run"],
        _ => &["ide_code_run"],
    }
}

/// Uwezo wa mfanyakazi: ngazi ya juu = uzoefu zaidi. Hutumika kama kigezo cha
/// mwisho wakati wafanyakazi wawili wana zana zinazolingana.
pub fn level_rank(level: &str) -> i64 {
    match norm_level(level) {
        "intern" => 1,
        "junior" => 2,
        "mid" => 3,
        "senior" => 4,
        "lead" => 5,
        "executive" => 6,
        _ => 2,
    }
}

/// Mgombea wa kupewa ombi. `tools` ni zile alizopewa (`bz_tool_grants`).
#[derive(Debug, Clone)]
pub struct Candidate {
    pub id: String,
    pub name: String,
    pub level: String,
    pub department_id: String,
    pub tools: Vec<String>,
    /// Maombi yanayoendelea aliyo nayo sasa — mgawanyiko wa kazi.
    pub load: i64,
}

/// Chagua mfanyakazi anayefaa zaidi kwa ombi.
///
/// Vigezo kwa mpangilio: (1) ana zana zote zinazohitajika na kategoria,
/// (2) mzigo mdogo wa kazi, (3) ngazi ya juu. Hii huzuia kumpa ombi la usalama
/// mfanyakazi wa uundaji, na kuzuia mtu mmoja kubeba kila kitu.
pub fn pick_assignee(candidates: &[Candidate], category: &str, department: &str) -> Option<String> {
    let needed = category_tools(category);
    let mut best: Option<(&Candidate, (usize, usize, i64, i64))> = None;
    for c in candidates {
        let have = needed.iter().filter(|t| tool_allowed(&c.tools, t)).count();
        if have == 0 {
            continue;
        }
        let dep_match = usize::from(!department.is_empty() && c.department_id == department);
        let key = (have, dep_match, -c.load, level_rank(&c.level));
        if best.is_none_or(|(_, bk)| key > bk) {
            best = Some((c, key));
        }
    }
    best.map(|(c, _)| c.id.clone())
}

/// Mfanyakazi anaweza kutumia zana hii? (kanuni ya upendeleo mdogo)
pub fn tool_allowed(granted: &[String], code: &str) -> bool {
    granted.iter().any(|g| g == code)
}

/// Zana zisizoruhusiwa kati ya zilizoombolwa.
pub fn denied_tools(granted: &[String], requested: &[String]) -> Vec<String> {
    requested
        .iter()
        .filter(|t| !tool_allowed(granted, t))
        .cloned()
        .collect()
}

// =====================================================================
// MTAZAMO WA MAOMBI (workflow)
// =====================================================================

/// Mabadiliko halali ya hali ya ombi. Admin HUIDHINISHA kabla kazi ianze.
pub fn request_transition(from: &str, to: &str) -> bool {
    matches!(
        (from, to),
        ("draft", "submitted")
            | ("submitted", "approved")
            | ("submitted", "rejected")
            | ("submitted", "cancelled")
            | ("approved", "assigned")
            | ("assigned", "in_progress")
            | ("in_progress", "review")
            | ("review", "delivered")
            | ("review", "in_progress")
            | ("delivered", "closed")
            | ("delivered", "review")
            | ("approved", "cancelled")
            | ("assigned", "cancelled")
    )
}

/// Maombi yanayohitaji uangalizi wa admin (kuidhinisha).
pub fn needs_admin(status: &str) -> bool {
    status == "submitted"
}

/// Maombi yanayoonekana kwa mteja kama "inaendelea".
pub fn in_flight(status: &str) -> bool {
    matches!(status, "approved" | "assigned" | "in_progress" | "review")
}

/// Kumbukumbu ya ombi (`REQ-2026-000123`).
pub fn request_ref(year: i32, seq: i64) -> String {
    format!("REQ-{year}-{seq:06}")
}

/// Aina za matukio ya maendeleo (yanayoonekana kwa mteja).
pub const EVENT_KINDS: [&str; 7] = [
    "created",
    "approved",
    "assigned",
    "progress",
    "review",
    "delivered",
    "note",
];

// =====================================================================
// SEED
// =====================================================================

async fn seed(db: &SqlitePool) {
    let ts = now();
    for (code, name, r) in RATES {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO bz_currency (code,name,rate_tzs,active,updated_at) VALUES (?,?,?,1,?)",
        )
        .bind(code)
        .bind(name)
        .bind(r)
        .bind(&ts)
        .execute(db)
        .await;
    }
    for (code, name, name_sw, cat, aud) in SERVICES {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO bz_services (id,code,name,name_sw,category,audience,currency,base_price_cents,icon,active,created_at)
             VALUES (?,?,?,?,?,?,'TZS',0,'',1,?)",
        )
        .bind(format!("SVC-{code}"))
        .bind(code)
        .bind(name)
        .bind(name_sw)
        .bind(cat)
        .bind(aud)
        .bind(&ts)
        .execute(db)
        .await;
    }
    for (code, name, _name_sw, tier) in DEPARTMENTS {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO bz_departments (id,code,name,tier,head_employee_id,created_at) VALUES (?,?,?,?,'',?)",
        )
        .bind(format!("DEP-{code}"))
        .bind(code)
        .bind(name)
        .bind(tier)
        .bind(&ts)
        .execute(db)
        .await;
    }
    // Mikoa yote 31 ya Tanzania + nchi za kimataifa zinazolengwa.
    for (code, name) in TZ_REGIONS {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO bz_coverage (id,kind,code,name,status,requests_count,updated_at)
             VALUES (?, 'tz_region', ?, ?, 'live', 0, ?)",
        )
        .bind(format!("COV-TZ-{code}"))
        .bind(code)
        .bind(name)
        .bind(&ts)
        .execute(db)
        .await;
    }
    for (code, name) in COUNTRIES {
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO bz_coverage (id,kind,code,name,status,requests_count,updated_at)
             VALUES (?, 'country', ?, ?, 'live', 0, ?)",
        )
        .bind(format!("COV-C-{code}"))
        .bind(code)
        .bind(name)
        .bind(&ts)
        .execute(db)
        .await;
    }
}

/// Mikoa yote 31 ya Tanzania.
pub const TZ_REGIONS: [(&str, &str); 31] = [
    ("ARU", "Arusha"),
    ("DAR", "Dar es Salaam"),
    ("DOD", "Dodoma"),
    ("GEI", "Geita"),
    ("IRI", "Iringa"),
    ("KAG", "Kagera"),
    ("KAT", "Katavi"),
    ("KIG", "Kigoma"),
    ("KIL", "Kilimanjaro"),
    ("LIN", "Lindi"),
    ("MAN", "Manyara"),
    ("MAR", "Mara"),
    ("MBE", "Mbeya"),
    ("MOR", "Morogoro"),
    ("MTW", "Mtwara"),
    ("MWA", "Mwanza"),
    ("NJO", "Njombe"),
    ("PEM", "Pemba"),
    ("PWA", "Pwani"),
    ("RUK", "Rukwa"),
    ("RUV", "Ruvuma"),
    ("SHI", "Shinyanga"),
    ("SIM", "Simiyu"),
    ("SIN", "Singida"),
    ("SON", "Songwe"),
    ("TAB", "Tabora"),
    ("TAN", "Tanga"),
    ("UNZ", "Unguja"),
    ("KAS", "Kaskazini Unguja"),
    ("KUS", "Kusini Unguja"),
    ("KUP", "Kusini Pemba"),
];

/// Nchi zinazolengwa (huduma za kimataifa).
pub const COUNTRIES: [(&str, &str); 14] = [
    ("TZ", "Tanzania"),
    ("KE", "Kenya"),
    ("UG", "Uganda"),
    ("RW", "Rwanda"),
    ("BI", "Burundi"),
    ("CD", "DR Congo"),
    ("ZM", "Zambia"),
    ("MW", "Malawi"),
    ("MZ", "Mozambique"),
    ("ZA", "South Africa"),
    ("NG", "Nigeria"),
    ("US", "United States"),
    ("GB", "United Kingdom"),
    ("AE", "United Arab Emirates"),
];

pub fn is_tz_region(code: &str) -> bool {
    TZ_REGIONS.iter().any(|(c, _)| *c == code)
}

pub fn is_country(code: &str) -> bool {
    COUNTRIES.iter().any(|(c, _)| *c == code)
}

/// Huduma hii inapatikana kwa mteja wa nje? (`audience`).
pub fn service_available_for(audience: &str, country: &str) -> bool {
    audience != "tz" || country.eq_ignore_ascii_case("TZ")
}

// =====================================================================
// ROUTER
// =====================================================================

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/biashara/status", get(status))
        .route("/api/biashara/currencies", get(currencies))
        .route("/api/biashara/convert", get(convert_h))
        .route("/api/biashara/services", get(services_list))
        .route("/api/biashara/departments", get(departments_list))
        .route("/api/biashara/tools", get(tools_list))
        .route("/api/biashara/employees", get(employees_list))
        .route("/api/biashara/employee/add", post(employee_add))
        .route("/api/biashara/employee/:id", get(employee_get))
        .route("/api/biashara/employee/:id/status", post(employee_status))
        .route("/api/biashara/employee/:id/tools", post(employee_tools))
        .route("/api/biashara/employee/:id/tool/use", post(employee_tool_use))
        .route("/api/biashara/employee/:id/tasks", get(employee_tasks))
        .route("/api/biashara/requests", get(requests_list))
        .route("/api/biashara/request/create", post(request_create))
        .route("/api/biashara/request/:id", get(request_get))
        .route("/api/biashara/request/:id/approve", post(request_approve))
        .route("/api/biashara/request/:id/reject", post(request_reject))
        .route("/api/biashara/request/:id/assign", post(request_assign))
        .route("/api/biashara/request/:id/auto-assign", post(request_auto_assign))
        .route("/api/biashara/request/:id/progress", post(request_progress))
        .route("/api/biashara/request/:id/review", post(request_review))
        .route("/api/biashara/request/:id/deliver", post(request_deliver))
        .route("/api/biashara/request/:id/close", post(request_close))
        .route("/api/biashara/request/:id/track", get(request_track))
        // Umma kwa makusudi: mteja (hana login) hufuatilia kwa ref + simu.
        .route("/api/biashara/client/track", post(client_track))
        .route("/api/biashara/contracts", get(contracts_list))
        .route("/api/biashara/contract/create", post(contract_create))
        .route("/api/biashara/contract/:id/sign", post(contract_sign))
        .route("/api/biashara/contract/:id/verify", get(contract_verify))
        .route("/api/biashara/coverage", get(coverage_list))
        .route("/api/biashara/audit", get(audit_list))
        .route("/api/biashara/branding", get(branding_get))
        .route("/api/biashara/branding/set", post(branding_set))
        .route("/api/biashara/branding/logo", post(branding_logo))
        .route("/api/biashara/branding/reset", post(branding_reset))
}

/// Rangi/maandishi ya chapa yanayodhibitiwa na Admin Mkuu. `logo` huhifadhiwa
/// kama data-URI (PNG/JPEG/SVG), kikomo 512 KB.
pub const BRANDING_KEYS: [(&str, &str); 7] = [
    ("company_name", "MTECHOS GROUP"),
    ("tagline", "Teknolojia ya Dunia, Kasi ya Kipekee"),
    ("primary_color", "#FF6B2C"),
    ("accent_color", "#22D3EE"),
    ("surface_color", "#130A07"),
    ("logo", ""),
    ("favicon", ""),
];

const LOGO_MAX_BYTES: usize = 512 * 1024;

async fn branding_get(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String)>("SELECT key,value FROM bz_branding")
        .fetch_all(&s.db)
        .await
        .unwrap_or_default();
    let map: std::collections::HashMap<String, String> = rows.into_iter().collect();
    let mut out = serde_json::Map::new();
    for (k, default) in BRANDING_KEYS {
        let v = map.get(k).cloned().unwrap_or_else(|| default.to_string());
        // logo/favicon ni kubwa — toa tu ikiwa imewekwa.
        if (k == "logo" || k == "favicon") && v.is_empty() {
            out.insert(k.to_string(), json!(""));
        } else {
            out.insert(k.to_string(), json!(v));
        }
    }
    Json(json!({"ok": true, "branding": Value::Object(out)}))
}

#[derive(Deserialize)]
struct BrandingSetIn {
    #[serde(default)]
    company_name: Option<String>,
    #[serde(default)]
    tagline: Option<String>,
    #[serde(default)]
    primary_color: Option<String>,
    #[serde(default)]
    accent_color: Option<String>,
    #[serde(default)]
    surface_color: Option<String>,
}

/// Rangi halali: `#RGB` au `#RRGGBB`. Hurudisha `None` kwa tupu.
fn norm_color(c: &str) -> Option<String> {
    let t = c.trim();
    if t.is_empty() {
        return None;
    }
    let hex = t.strip_prefix('#')?;
    if !(hex.len() == 3 || hex.len() == 6) || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", hex.to_ascii_uppercase()))
}

async fn put_branding(db: &SqlitePool, key: &str, value: &str, by: &str) {
    let _ = sqlx::query(
        "INSERT INTO bz_branding (key,value,updated_by,updated_at) VALUES (?1,?2,?3,?4)
         ON CONFLICT(key) DO UPDATE SET value=?2, updated_by=?3, updated_at=?4",
    )
    .bind(key)
    .bind(value)
    .bind(by)
    .bind(now())
    .execute(db)
    .await;
}

async fn branding_set(State(s): State<AppState>, Json(b): Json<BrandingSetIn>) -> Json<Value> {
    let mut changed: Vec<String> = vec![];
    if let Some(v) = b.company_name.as_deref() {
        let v = v.trim();
        if !v.is_empty() {
            put_branding(&s.db, "company_name", v, "MBILINYI").await;
            changed.push("company_name".into());
        }
    }
    if let Some(v) = b.tagline.as_deref() {
        let v = v.trim();
        if !v.is_empty() {
            put_branding(&s.db, "tagline", v, "MBILINYI").await;
            changed.push("tagline".into());
        }
    }
    for (key, val) in [
        ("primary_color", b.primary_color.as_deref()),
        ("accent_color", b.accent_color.as_deref()),
        ("surface_color", b.surface_color.as_deref()),
    ] {
        if let Some(raw) = val {
            let Some(c) = norm_color(raw) else {
                return Json(json!({"ok": false, "error": format!("rangi si sahihi: {raw} (tumia #RRGGBB)")}));
            };
            put_branding(&s.db, key, &c, "MBILINYI").await;
            changed.push(key.into());
        }
    }
    if changed.is_empty() {
        return Json(json!({"ok": false, "error": "hakuna kitu kilichobadilishwa"}));
    }
    audit(&s.db, "MBILINYI", "branding_set", &changed.join(","), "chapa imebadilishwa").await;
    Json(json!({"ok": true, "changed": changed}))
}

#[derive(Deserialize)]
struct LogoIn {
    /// Data-URI (`data:image/png;base64,…`) au SVG/URL ndogo.
    data: String,
    #[serde(default)]
    kind: Option<String>, // logo|favicon
}

async fn branding_logo(State(s): State<AppState>, Json(b): Json<LogoIn>) -> Json<Value> {
    let d = b.data.trim();
    if d.is_empty() {
        return Json(json!({"ok": false, "error": "picha haipo"}));
    }
    if d.len() > LOGO_MAX_BYTES {
        return Json(json!({"ok": false, "error": "picha ni kubwa mno (kikomo 512 KB)"}));
    }
    let ok_scheme = d.starts_with("data:image/")
        && ["png", "jpeg", "jpg", "svg+xml", "webp", "gif"]
            .iter()
            .any(|f| d.starts_with(&format!("data:image/{f}")));
    if !ok_scheme {
        return Json(json!({"ok": false, "error": "tumia data-URI ya picha (data:image/…)"}));
    }
    let key = match b.kind.as_deref().unwrap_or("logo") {
        "favicon" => "favicon",
        _ => "logo",
    };
    put_branding(&s.db, key, d, "MBILINYI").await;
    audit(&s.db, "MBILINYI", "branding_logo", key, "picha ya chapa imebadilishwa").await;
    Json(json!({"ok": true, "key": key, "bytes": d.len()}))
}

async fn branding_reset(State(s): State<AppState>) -> Json<Value> {
    let _ = sqlx::query("DELETE FROM bz_branding").execute(&s.db).await;
    audit(&s.db, "MBILINYI", "branding_reset", "all", "chapa imerudishwa kwenye default").await;
    Json(json!({"ok": true, "note": "chapa imerudishwa kwenye default"}))
}

async fn table_count(db: &SqlitePool, t: &str) -> i64 {
    sqlx::query_as::<_, (i64,)>(&format!("SELECT COUNT(*) FROM {t}"))
        .fetch_one(db)
        .await
        .map(|r| r.0)
        .unwrap_or(0)
}

async fn status(State(s): State<AppState>) -> Json<Value> {
    let (pending,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM bz_requests WHERE status='submitted'")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    let (active,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM bz_requests WHERE status IN ('approved','assigned','in_progress','review')",
    )
    .fetch_one(&s.db)
    .await
    .unwrap_or((0,));
    let (done,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM bz_requests WHERE status IN ('delivered','closed')")
            .fetch_one(&s.db)
            .await
            .unwrap_or((0,));
    Json(json!({
        "ok": true,
        "module": "biashara",
        "services": table_count(&s.db, "bz_services").await,
        "employees": table_count(&s.db, "bz_employees").await,
        "requests": table_count(&s.db, "bz_requests").await,
        "contracts": table_count(&s.db, "bz_contracts").await,
        "currencies": table_count(&s.db, "bz_currency").await,
        "tools": TOOLS.len(),
        "regions": TZ_REGIONS.len(),
        "countries": COUNTRIES.len(),
        "pending_approval": pending,
        "active": active,
        "completed": done,
    }))
}

async fn currencies(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, f64, i64)>(
        "SELECT code,name,rate_tzs,active FROM bz_currency ORDER BY rate_tzs DESC",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({"ok": true, "currencies": rows.into_iter().map(|r| json!({
        "code": r.0, "name": r.1, "rate_tzs": r.2, "active": r.3 == 1,
        "stripe": r.0 != "TZS"
    })).collect::<Vec<_>>()}))
}

#[derive(Deserialize)]
struct ConvertQuery {
    amount: String,
    from: String,
    to: String,
}

async fn convert_h(Query(q): Query<ConvertQuery>) -> Json<Value> {
    let Some(cents) = crate::fees::to_cents(&q.amount) else {
        return Json(json!({"ok": false, "error": "kiasi si sahihi"}));
    };
    let from = norm_currency(&q.from);
    let to = norm_currency(&q.to);
    let out = convert(cents, &from, &to);
    Json(json!({
        "ok": true,
        "amount": crate::fees::fmt_money(cents), "from": from,
        "converted": crate::fees::fmt_money(out), "to": to,
        "rate": rate(&from) / rate(&to),
        "via": "TZS"
    }))
}

#[derive(Deserialize)]
struct ServiceQuery {
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    audience: Option<String>,
    #[serde(default)]
    country: Option<String>,
}

async fn services_list(State(s): State<AppState>, Query(q): Query<ServiceQuery>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, i64, String)>(
        "SELECT id,code,name,name_sw,category,audience,base_price_cents,currency
         FROM bz_services WHERE active=1 ORDER BY category, code",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let country = q.country.clone().unwrap_or_else(|| "TZ".into());
    let items: Vec<Value> = rows
        .into_iter()
        .filter(|r| {
            q.category
                .as_deref()
                .filter(|c| !c.is_empty())
                .map(|c| r.4 == norm_category(c))
                .unwrap_or(true)
        })
        .filter(|r| {
            q.audience
                .as_deref()
                .filter(|a| !a.is_empty())
                .map(|a| r.5 == a)
                .unwrap_or(true)
        })
        // Huduma za TZ pekee HAZIONEKANI kwa wateja wa nje.
        .filter(|r| service_available_for(&r.5, &country))
        .map(|r| {
            json!({
                "id": r.0, "code": r.1, "name": r.2, "name_sw": r.3,
                "category": r.4, "audience": r.5,
                "base_price_cents": r.6,
                "base_price": crate::fees::fmt_money(r.6),
                "currency": r.7
            })
        })
        .collect();
    Json(json!({"ok": true, "total": items.len(), "services": items}))
}

async fn departments_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT id,code,name,tier FROM bz_departments ORDER BY tier, code",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({"ok": true, "departments": rows.into_iter().map(|r| json!({
        "id": r.0, "code": r.1, "name": r.2, "tier": r.3
    })).collect::<Vec<_>>()}))
}

async fn tools_list() -> Json<Value> {
    Json(json!({"ok": true, "total": TOOLS.len(), "tools": TOOLS.iter().map(|(c, n, cat)| json!({
        "code": c, "name": n, "category": cat
    })).collect::<Vec<_>>()}))
}

// ---------- wafanyakazi ----------

async fn employees_list(State(s): State<AppState>) -> Json<Value> {
    let rows = sqlx::query_as::<_, (String, String, String, String, String, String, String, String, String, String, String)>(
        "SELECT id,user_id,full_name,photo,profession,level,department_id,phone,email,tools,status
         FROM bz_employees ORDER BY level DESC, full_name",
    )
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({"ok": true, "employees": rows.into_iter().map(|r| json!({
        "id": r.0, "user_id": r.1, "full_name": r.2, "photo": r.3,
        "profession": r.4, "level": r.5, "department_id": r.6,
        "phone": r.7, "email": r.8,
        "tools": serde_json::from_str::<Value>(&r.9).unwrap_or(json!([])),
        "status": r.10
    })).collect::<Vec<_>>()}))
}

#[derive(Deserialize)]
pub struct EmployeeIn {
    pub full_name: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub photo: String,
    #[serde(default)]
    pub profession: String,
    #[serde(default)]
    pub level: String,
    #[serde(default)]
    pub department: String,
    #[serde(default)]
    pub phone: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub tools: Vec<String>,
}

async fn employee_add(State(s): State<AppState>, Json(b): Json<EmployeeIn>) -> Json<Value> {
    if b.full_name.trim().is_empty() {
        return Json(json!({"ok": false, "error": "jina la mfanyakazi linahitajika"}));
    }
    let level = norm_level(&b.level);
    // Idara: inakubali code au id.
    let dep = b.department.trim();
    let dep_id = if dep.is_empty() {
        String::new()
    } else {
        let found = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM bz_departments WHERE code=?1 OR id=?1 LIMIT 1",
        )
        .bind(dep)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten();
        match found {
            Some((id,)) => id,
            None => return Json(json!({"ok": false, "error": format!("idara `{dep}` haipo")})),
        }
    };
    // Zana: HAZIRUHUSIWI kuwa za nje ya katalogi.
    let bad: Vec<&String> = b.tools.iter().filter(|t| !tool_exists(t)).collect();
    if !bad.is_empty() {
        return Json(json!({
            "ok": false,
            "error": format!("zana zisizojulikana: {}", bad.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "))
        }));
    }
    let id = uid();
    let tools_json = serde_json::to_string(&b.tools).unwrap_or_else(|_| "[]".into());
    let _ = sqlx::query(
        "INSERT INTO bz_employees (id,user_id,full_name,photo,profession,level,department_id,phone,email,tools,status,hired_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'active',?11)",
    )
    .bind(&id)
    .bind(&b.user_id)
    .bind(b.full_name.trim())
    .bind(&b.photo)
    .bind(&b.profession)
    .bind(level)
    .bind(&dep_id)
    .bind(&b.phone)
    .bind(&b.email)
    .bind(&tools_json)
    .bind(now())
    .execute(&s.db)
    .await;
    for t in &b.tools {
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO bz_tool_grants (id,employee_id,tool_code,granted_by,granted_at,revoked_at)
             VALUES (?1,?2,?3,'admin',?4,NULL)",
        )
        .bind(uid())
        .bind(&id)
        .bind(t)
        .bind(now())
        .execute(&s.db)
        .await;
    }
    audit(&s.db, "admin", "employee.add", &id, &b.full_name).await;
    Json(json!({"ok": true, "id": id, "level": level, "tools": b.tools}))
}

async fn employee_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, String, String, String, String, String)>(
        "SELECT id,user_id,full_name,photo,profession,level,department_id,phone,email,tools,status
         FROM bz_employees WHERE id=?1 OR user_id=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some(r) = row else {
        return Json(json!({"ok": false, "error": "mfanyakazi haipo"}));
    };
    let grants = sqlx::query_as::<_, (String,)>(
        "SELECT tool_code FROM bz_tool_grants WHERE employee_id=?1 AND revoked_at IS NULL",
    )
    .bind(&r.0)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({
        "ok": true,
        "employee": {
            "id": r.0, "user_id": r.1, "full_name": r.2, "photo": r.3,
            "profession": r.4, "level": r.5, "department_id": r.6,
            "phone": r.7, "email": r.8, "status": r.10
        },
        "granted_tools": grants.into_iter().map(|g| g.0).collect::<Vec<_>>()
    }))
}

#[derive(Deserialize)]
pub struct StatusIn {
    pub status: String,
}

async fn employee_status(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<StatusIn>,
) -> Json<Value> {
    let st = match b.status.trim().to_lowercase().as_str() {
        "active" | "hai" => "active",
        "suspended" | "amesimamishwa" => "suspended",
        "left" | "ameondoka" => "left",
        _ => return Json(json!({"ok": false, "error": "hali si sahihi (active|suspended|left)"})),
    };
    let res = sqlx::query("UPDATE bz_employees SET status=?2 WHERE id=?1 OR user_id=?1")
        .bind(&id)
        .bind(st)
        .execute(&s.db)
        .await;
    let changed = res.map(|r| r.rows_affected()).unwrap_or(0);
    if changed == 0 {
        return Json(json!({"ok": false, "error": "mfanyakazi haipo"}));
    }
    audit(&s.db, "admin", "employee.status", &id, st).await;
    Json(json!({"ok": true, "id": id, "status": st}))
}

#[derive(Deserialize)]
pub struct ToolsIn {
    pub tools: Vec<String>,
}

/// Weka zana za mfanyakazi (admin pekee). Hufuta zilizotolewa na kuweka mpya.
async fn employee_tools(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<ToolsIn>,
) -> Json<Value> {
    let bad: Vec<&String> = b.tools.iter().filter(|t| !tool_exists(t)).collect();
    if !bad.is_empty() {
        return Json(json!({"ok": false, "error": format!("zana zisizojulikana: {}", bad.len())}));
    }
    let Some((emp_id,)) =
        sqlx::query_as::<_, (String,)>("SELECT id FROM bz_employees WHERE id=?1 OR user_id=?1 LIMIT 1")
            .bind(&id)
            .fetch_optional(&s.db)
            .await
            .ok()
            .flatten()
    else {
        return Json(json!({"ok": false, "error": "mfanyakazi haipo"}));
    };
    let _ = sqlx::query("UPDATE bz_tool_grants SET revoked_at=?2 WHERE employee_id=?1 AND revoked_at IS NULL")
        .bind(&emp_id)
        .bind(now())
        .execute(&s.db)
        .await;
    for t in &b.tools {
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO bz_tool_grants (id,employee_id,tool_code,granted_by,granted_at,revoked_at)
             VALUES (?1,?2,?3,'admin',?4,NULL)",
        )
        .bind(uid())
        .bind(&emp_id)
        .bind(t)
        .bind(now())
        .execute(&s.db)
        .await;
    }
    let _ = sqlx::query("UPDATE bz_employees SET tools=?2 WHERE id=?1")
        .bind(&emp_id)
        .bind(serde_json::to_string(&b.tools).unwrap_or_else(|_| "[]".into()))
        .execute(&s.db)
        .await;
    audit(&s.db, "admin", "employee.tools", &emp_id, &b.tools.join(",")).await;
    Json(json!({"ok": true, "id": emp_id, "tools": b.tools}))
}

#[derive(Deserialize)]
pub struct ToolUseIn {
    pub tool: String,
}

/// Mfanyakazi anataka kutumia zana — hukaguliwa kwa `bz_tool_grants`.
async fn employee_tool_use(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<ToolUseIn>,
) -> Json<Value> {
    let Some((emp_id,)) =
        sqlx::query_as::<_, (String,)>("SELECT id FROM bz_employees WHERE id=?1 OR user_id=?1 LIMIT 1")
            .bind(&id)
            .fetch_optional(&s.db)
            .await
            .ok()
            .flatten()
    else {
        return Json(json!({"ok": false, "error": "mfanyakazi haipo"}));
    };
    let grants = sqlx::query_as::<_, (String,)>(
        "SELECT tool_code FROM bz_tool_grants WHERE employee_id=?1 AND revoked_at IS NULL",
    )
    .bind(&emp_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|r| r.0)
    .collect::<Vec<String>>();
    let allowed = tool_allowed(&grants, &b.tool);
    audit(
        &s.db,
        &emp_id,
        if allowed { "tool.allow" } else { "tool.deny" },
        &b.tool,
        "",
    )
    .await;
    Json(json!({
        "ok": allowed,
        "tool": b.tool,
        "allowed": allowed,
        "error": if allowed { Value::Null } else { json!("mfanyakazi hajapewa zana hii — admin aidhinishe") }
    }))
}

// ---------- maombi ----------

async fn requests_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(200).clamp(1, 1000);
    let rows = if let Some(st) = q.status.as_deref().filter(|v| !v.is_empty()) {
        sqlx::query_as::<_, RequestRow>(
            "SELECT id,ref,client_name,service_id,title,country,region,currency,amount_cents,status,progress,priority,assignee_id,created_at FROM bz_requests WHERE status=?1 ORDER BY created_at DESC LIMIT ?2",
        )
        .bind(st)
        .bind(lim)
        .fetch_all(&s.db)
        .await
    } else {
        sqlx::query_as::<_, RequestRow>(
            "SELECT id,ref,client_name,service_id,title,country,region,currency,amount_cents,status,progress,priority,assignee_id,created_at FROM bz_requests ORDER BY created_at DESC LIMIT ?1",
        )
        .bind(lim)
        .fetch_all(&s.db)
        .await
    }
    .unwrap_or_default();
    Json(json!({"ok": true, "requests": rows.into_iter().map(row_json).collect::<Vec<_>>()}))
}

#[derive(sqlx::FromRow)]
struct RequestRow {
    id: String,
    r#ref: String,
    client_name: String,
    service_id: String,
    title: String,
    country: String,
    region: String,
    currency: String,
    amount_cents: i64,
    status: String,
    progress: i64,
    priority: String,
    assignee_id: String,
    created_at: String,
}

fn row_json(r: RequestRow) -> Value {
    json!({
        "id": r.id, "ref": r.r#ref, "client_name": r.client_name,
        "service_id": r.service_id, "title": r.title,
        "country": r.country, "region": r.region,
        "currency": r.currency, "amount_cents": r.amount_cents,
        "amount": crate::fees::fmt_money(r.amount_cents),
        "status": r.status, "progress": r.progress, "priority": r.priority,
        "assignee_id": r.assignee_id, "created_at": r.created_at,
        "needs_admin": needs_admin(&r.status), "in_flight": in_flight(&r.status)
    })
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct RequestIn {
    pub client_name: String,
    pub client_phone: String,
    #[serde(default)]
    pub client_email: String,
    pub service: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub country: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub currency: String,
    pub amount: String,
    #[serde(default)]
    pub priority: String,
}

async fn request_create(State(s): State<AppState>, Json(b): Json<RequestIn>) -> Json<Value> {
    if b.client_name.trim().is_empty() || b.title.trim().is_empty() {
        return Json(json!({"ok": false, "error": "jina la mteja na kichwa cha ombi vinahitajika"}));
    }
    if !crate::sms::phone_ok(&b.client_phone) {
        return Json(json!({"ok": false, "error": "namba ya simu ya mteja si sahihi"}));
    }
    // Huduma: code au id.
    let svc = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id,audience,currency FROM bz_services WHERE code=?1 OR id=?1 LIMIT 1",
    )
    .bind(b.service.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((service_id, audience, svc_currency)) = svc else {
        return Json(json!({"ok": false, "error": format!("huduma `{}` haipo", b.service)}));
    };
    let country = if b.country.trim().is_empty() {
        "TZ".to_string()
    } else {
        b.country.trim().to_ascii_uppercase()
    };
    if !is_country(&country) {
        return Json(json!({"ok": false, "error": format!("nchi `{country}` haijasajiliwa")}));
    }
    // Huduma za TZ pekee: wateja wa nje hawazipati.
    if !service_available_for(&audience, &country) {
        return Json(json!({
            "ok": false,
            "error": "huduma hii ni ya Tanzania pekee (BRELA/TRA) — wateja wa nje hawapati"
        }));
    }
    if country == "TZ" {
        let region = if b.region.trim().is_empty() {
            "DAR".to_string()
        } else {
            b.region.trim().to_ascii_uppercase()
        };
        if !is_tz_region(&region) {
            return Json(json!({"ok": false, "error": format!("mkoa `{region}` haupo")}));
        }
    }
    let Some(amount_cents) = crate::fees::to_cents(&b.amount) else {
        return Json(json!({"ok": false, "error": "kiasi si sahihi"}));
    };
    if !amount_ok(amount_cents) {
        return Json(json!({"ok": false, "error": "kiasi lazima kiwe zaidi ya sifuri"}));
    }
    let currency = if b.currency.trim().is_empty() {
        svc_currency
    } else {
        norm_currency(&b.currency)
    };
    let priority = match b.priority.trim().to_lowercase().as_str() {
        "low" | "chini" => "low",
        "high" | "juu" => "high",
        "urgent" | "haraka" => "urgent",
        _ => "normal",
    };
    let (seq,): (i64,) = sqlx::query_as("SELECT COUNT(*)+1 FROM bz_requests")
        .fetch_one(&s.db)
        .await
        .unwrap_or((1,));
    let year = chrono::Local::now()
        .format("%Y")
        .to_string()
        .parse::<i32>()
        .unwrap_or(2026);
    let reference = request_ref(year, seq);
    let id = uid();
    let _ = sqlx::query(
        "INSERT INTO bz_requests (id,ref,client_name,client_phone,client_email,service_id,title,description,country,region,currency,amount_cents,status,progress,priority,assignee_id,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,'submitted',0,?13,'',?14)",
    )
    .bind(&id)
    .bind(&reference)
    .bind(b.client_name.trim())
    .bind(b.client_phone.trim())
    .bind(&b.client_email)
    .bind(&service_id)
    .bind(b.title.trim())
    .bind(&b.description)
    .bind(&country)
    .bind(b.region.trim().to_ascii_uppercase())
    .bind(&currency)
    .bind(amount_cents)
    .bind(priority)
    .bind(now())
    .execute(&s.db)
    .await;
    event(&s.db, &id, "mteja", "created", "Ombi limewasilishwa — linasubiri idhini ya admin", 0).await;
    // Ufuatiliaji wa eneo.
    let cov_code = if country == "TZ" {
        b.region.trim().to_ascii_uppercase()
    } else {
        country.clone()
    };
    if !cov_code.is_empty() {
        let _ = sqlx::query("UPDATE bz_coverage SET requests_count=requests_count+1, updated_at=?2 WHERE code=?1")
            .bind(&cov_code)
            .bind(now())
            .execute(&s.db)
            .await;
    }
    // Taarifa kwa admin kwa SMS (ikiwa namba imewekwa).
    let _ = crate::sms::notify_admin_new_request(
        &s.db,
        &reference,
        b.title.trim(),
        &crate::fees::fmt_money(amount_cents),
        &currency,
    )
    .await;
    audit(&s.db, "mteja", "request.create", &id, &reference).await;
    Json(json!({
        "ok": true, "id": id, "ref": reference, "status": "submitted",
        "currency": currency, "amount_cents": amount_cents,
        "amount": crate::fees::fmt_money(amount_cents),
        "usd_equivalent": crate::fees::fmt_money(convert(amount_cents, &currency, "USD")),
        "note": "ombi linasubiri idhini ya admin kabla kazi kuanza"
    }))
}

async fn request_get(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, RequestRow>(
        "SELECT id,ref,client_name,service_id,title,country,region,currency,amount_cents,status,progress,priority,assignee_id,created_at FROM bz_requests WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some(r) = row else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    let events = events_for(&s.db, &r.id, false).await;
    Json(json!({"ok": true, "request": row_json(r), "events": events}))
}

/// Mteja hufuatilia maendeleo — matukio yanayoonekana pekee.
async fn request_track(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, i64, String)>(
        "SELECT id,ref,status,progress,title FROM bz_requests WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((rid, reference, status, progress, title)) = row else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    let events = events_for(&s.db, &rid, true).await;
    let pct = progress.clamp(0, 100);
    Json(json!({
        "ok": true, "ref": reference, "title": title, "status": status,
        "progress": pct,
        "stage": stage_label(&status, pct),
        "timeline": events
    }))
}

/// Sakafu ya maendeleo kwa kila hali. Ombi huwekwa kwenye kiwango hiki cha
/// chini kabisa; `MAX(progress, floor)` huhakikisha maendeleo HAYAPUNGUKI.
/// Tukio (event) na ombi hutumia thamani ILE ILE, la sivyo mteja huona asilimia
/// tofauti na maelezo ya tukio.
pub fn progress_floor(status: &str) -> i64 {
    match status {
        "approved" => 5,
        "assigned" => 10,
        "review" => 90,
        "delivered" | "closed" => 100,
        _ => 0,
    }
}

/// Hatua inayoonekana kwa mteja.
pub fn stage_label(status: &str, pct: i64) -> String {
    match status {
        "submitted" => "Inasubiri idhini".into(),
        "approved" => "Imeidhinishwa".into(),
        "assigned" => "Imepewa mfanyakazi".into(),
        "in_progress" => format!("Inaendelea ({pct}%)"),
        "review" => "Ukaguzi wa mwisho".into(),
        "delivered" => "Imekamilika — inasubiri kufungwa".into(),
        "closed" => "Imekamilika".into(),
        "rejected" => "Imekataliwa".into(),
        "cancelled" => "Imesitishwa".into(),
        _ => status.to_string(),
    }
}

async fn request_approve(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let out = transition(&s, &id, "approved", "admin", "Idhini ya admin — kazi inaweza kuanza", progress_floor("approved")).await;
    if out.get("ok").and_then(Value::as_bool) != Some(true) {
        return out;
    }
    // Kazi HAIANZI bila mtu: mara baada ya idhini, chagua mfanyakazi anayefaa.
    let rid = out.get("id").and_then(Value::as_str).unwrap_or("").to_string();
    match auto_assign(&s, &rid).await {
        Ok(Some((emp_id, emp_name))) => Json(json!({
            "ok": true, "id": rid, "status": "assigned",
            "assignee": emp_id, "assignee_name": emp_name,
            "note": format!("Imeidhinishwa na kupewa {emp_name} moja kwa moja")
        })),
        Ok(None) => Json(json!({
            "ok": true, "id": rid, "status": "approved",
            "note": "Imeidhinishwa — hakuna mfanyakazi mwenye zana zinazofaa; mpe kwa mkono"
        })),
        Err(e) => Json(json!({"ok": true, "id": rid, "status": "approved", "note": e})),
    }
}

/// Chagua na weka mfanyakazi anayefaa kwa ombi. `Ok(None)` ikiwa hakuna
/// mfanyakazi mwenye zana zinazohitajika (admin hulazimika kumpa kwa mkono).
async fn auto_assign(
    s: &AppState,
    id: &str,
) -> Result<Option<(String, String)>, String> {
    let Some((rid, cur, service_id, assignee)) = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT id,status,service_id,assignee_id FROM bz_requests WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Err("ombi haipo".into());
    };
    if !assignee.trim().is_empty() {
        return Err("ombi tayari lina mfanyakazi".into());
    }
    if !request_transition(&cur, "assigned") {
        return Err(format!("hali `{cur}` hairuhusu kupewa mfanyakazi"));
    }
    let category = sqlx::query_as::<_, (String,)>("SELECT category FROM bz_services WHERE id=?1")
        .bind(&service_id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        .map(|c| norm_category(&c.0))
        .unwrap_or("software");
    let department = department_for_category(category);
    let candidates = candidates_for(&s.db, category).await;
    let Some(emp_id) = pick_assignee(&candidates, category, department) else {
        return Ok(None);
    };
    let emp_name = candidates
        .iter()
        .find(|c| c.id == emp_id)
        .map(|c| c.name.clone())
        .unwrap_or_default();
    let _ = sqlx::query("UPDATE bz_requests SET status='assigned', assignee_id=?2, assigned_at=?3, progress=MAX(progress,?4) WHERE id=?1")
        .bind(&rid)
        .bind(&emp_id)
        .bind(now())
        .bind(progress_floor("assigned"))
        .execute(&s.db)
        .await;
    event(
        &s.db,
        &rid,
        "admin",
        "assigned",
        &format!("Imepewa {emp_name} (kwa kategoria: {category})"),
        progress_floor("assigned"),
    )
    .await;
    audit(&s.db, "admin", "request.auto_assign", &rid, &emp_name).await;
    Ok(Some((emp_id, emp_name)))
}

/// Idara inayosimamia kategoria fulani (kwa upendeleo wa kupeana kazi).
pub fn department_for_category(category: &str) -> &'static str {
    match category {
        "finance" | "local" => "finance",
        "creative" | "education" => "marketing",
        "security" | "software" | "ai" | "hardware" | "agriculture" | "transport" => "employee",
        _ => "employee",
    }
}

/// Wafanyakazi wote hai + zana zao + mzigo wa kazi wa sasa.
async fn candidates_for(db: &SqlitePool, _category: &str) -> Vec<Candidate> {
    let rows = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT id,full_name,level,department_id FROM bz_employees WHERE status='active'",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let mut out = Vec::with_capacity(rows.len());
    for (id, name, level, department_id) in rows {
        let tools = sqlx::query_as::<_, (String,)>(
            "SELECT tool_code FROM bz_tool_grants WHERE employee_id=?1 AND revoked_at IS NULL",
        )
        .bind(&id)
        .fetch_all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|t| t.0)
        .collect();
        let (load,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM bz_requests WHERE assignee_id=?1
             AND status IN ('assigned','in_progress','review')",
        )
        .bind(&id)
        .fetch_one(db)
        .await
        .unwrap_or((0,));
        out.push(Candidate {
            id,
            name,
            level,
            department_id,
            tools,
            load,
        });
    }
    out
}

async fn request_auto_assign(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    match auto_assign(&s, &id).await {
        Ok(Some((emp_id, emp_name))) => Json(json!({
            "ok": true, "id": id, "assignee": emp_id, "assignee_name": emp_name,
            "status": "assigned"
        })),
        Ok(None) => Json(json!({
            "ok": false,
            "error": "hakuna mfanyakazi mwenye zana zinazofaa kwa huduma hii — ongeza zana kwa mfanyakazi kwanza"
        })),
        Err(e) => Json(json!({"ok": false, "error": e})),
    }
}

/// Foleni ya kazi ya mfanyakazi: maombi aliyopewa + maendeleo + zana anazoruhusiwa.
async fn employee_tasks(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let Some((emp_id, name, level, department_id, status)) =
        sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id,full_name,level,department_id,status FROM bz_employees
             WHERE id=?1 OR user_id=?1 LIMIT 1",
        )
        .bind(&id)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
    else {
        return Json(json!({"ok": false, "error": "mfanyakazi haipo"}));
    };
    let rows = sqlx::query_as::<_, RequestRow>(
        "SELECT id,ref,client_name,service_id,title,country,region,currency,amount_cents,status,progress,priority,assignee_id,created_at
         FROM bz_requests WHERE assignee_id=?1 ORDER BY created_at DESC LIMIT 200",
    )
    .bind(&emp_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let open = rows
        .iter()
        .filter(|r| matches!(r.status.as_str(), "assigned" | "in_progress" | "review"))
        .count();
    let done = rows
        .iter()
        .filter(|r| matches!(r.status.as_str(), "delivered" | "closed"))
        .count();
    let grants = sqlx::query_as::<_, (String,)>(
        "SELECT tool_code FROM bz_tool_grants WHERE employee_id=?1 AND revoked_at IS NULL",
    )
    .bind(&emp_id)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|g| g.0)
    .collect::<Vec<_>>();
    Json(json!({
        "ok": true,
        "employee": {
            "id": emp_id, "full_name": name, "level": level,
            "department_id": department_id, "status": status, "tools": grants
        },
        "open": open, "completed": done,
        "tasks": rows.into_iter().map(row_json).collect::<Vec<_>>()
    }))
}

/// Mteja hufuatilia ombi kwa `ref` + namba ya simu iliyotumika kuwasilisha.
/// Hii ni njia ya umma (mteja hana login) — hivyo HAIruhusiwi kurudisha data ya
/// mfanyakazi (simu/barua pepe) wala maombi ya wateja wengine.
async fn client_track(State(s): State<AppState>, Json(b): Json<ClientTrackIn>) -> Json<Value> {
    let reference = b.reference.trim().to_ascii_uppercase();
    if reference.is_empty() || !crate::sms::phone_ok(&b.phone) {
        return Json(json!({"ok": false, "error": "kumbukumbu ya ombi na namba ya simu vinahitajika"}));
    }
    let Some((rid, ref_no, title, status, progress, amount_cents, currency, service_id, assignee)) =
        sqlx::query_as::<_, (String, String, String, String, i64, i64, String, String, String)>(
            "SELECT id,ref,title,status,progress,amount_cents,currency,service_id,assignee_id
             FROM bz_requests WHERE ref=?1 LIMIT 1",
        )
        .bind(&reference)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
    else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    // Uthibitisho: namba ya simu LAZIMA ilingane na ile ya ombi.
    let ok: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM bz_requests WHERE ref=?1 AND replace(replace(client_phone,' ',''),'-','')=?2 LIMIT 1",
    )
    .bind(&reference)
    .bind(b.phone.trim().replace([' ', '-'], ""))
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    if ok.is_none() {
        // Sera ya umma: usifichue kama ref ipo — kosa lile lile kwa vyote viwili.
        return Json(json!({"ok": false, "error": "kumbukumbu au namba ya simu si sahihi"}));
    }
    let svc = sqlx::query_as::<_, (String, String)>(
        "SELECT name_sw,name FROM bz_services WHERE id=?1",
    )
    .bind(&service_id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    .unwrap_or(("".into(), "".into()));
    let worker = if assignee.trim().is_empty() {
        Value::Null
    } else {
        sqlx::query_as::<_, (String, String)>(
            "SELECT full_name,profession FROM bz_employees WHERE id=?1",
        )
        .bind(&assignee)
        .fetch_optional(&s.db)
        .await
        .ok()
        .flatten()
        .map(|(n, p)| json!({"full_name": n, "profession": p}))
        .unwrap_or(Value::Null)
    };
    let (pay,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM bz_payments WHERE request_id=?1 AND status='paid'",
    )
    .bind(&rid)
    .fetch_one(&s.db)
    .await
    .unwrap_or((0,));
    let contracts = sqlx::query_as::<_, (String, String, String, i64)>(
        "SELECT title,status,currency,amount_cents FROM bz_contracts WHERE request_id=?1",
    )
    .bind(&rid)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    let events = events_for(&s.db, &rid, true).await;
    let pct = progress.clamp(0, 100);
    Json(json!({
        "ok": true,
        "ref": ref_no,
        "title": title,
        "service": svc.0,
        "service_name": svc.1,
        "status": status,
        "stage": stage_label(&status, pct),
        "progress": pct,
        "amount": crate::fees::fmt_money(amount_cents),
        "currency": currency,
        "assignee": worker,
        "paid": pay > 0,
        "contracts": contracts.into_iter().map(|(t, st, cur, amt)| json!({
            "title": t, "status": st, "currency": cur, "amount_cents": amt
        })).collect::<Vec<_>>(),
        "timeline": events
    }))
}

#[derive(Deserialize)]
pub struct ClientTrackIn {
    pub reference: String,
    pub phone: String,
}

async fn request_reject(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    transition(&s, &id, "rejected", "admin", "Ombi limekataliwa", 0).await
}

#[derive(Deserialize)]
pub struct AssignIn {
    pub employee: String,
}

async fn request_assign(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<AssignIn>,
) -> Json<Value> {
    let Some((emp_id, emp_name)) = sqlx::query_as::<_, (String, String)>(
        "SELECT id,full_name FROM bz_employees WHERE (id=?1 OR user_id=?1) AND status='active' LIMIT 1",
    )
    .bind(b.employee.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "mfanyakazi haipo au hayupo hai"}));
    };
    let Some((rid, cur)) = sqlx::query_as::<_, (String, String)>(
        "SELECT id,status FROM bz_requests WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    if !request_transition(&cur, "assigned") {
        return Json(json!({"ok": false, "error": format!("ombi ({cur}) haliwezi kupewa mfanyakazi")}));
    }
    let _ = sqlx::query("UPDATE bz_requests SET status='assigned', assignee_id=?2, assigned_at=?3, progress=MAX(progress,?4) WHERE id=?1")
        .bind(&rid)
        .bind(&emp_id)
        .bind(now())
        .bind(progress_floor("assigned"))
        .execute(&s.db)
        .await;
    event(&s.db, &rid, "admin", "assigned", &format!("Imepewa {emp_name}"), progress_floor("assigned")).await;
    audit(&s.db, "admin", "request.assign", &rid, &emp_name).await;
    Json(json!({"ok": true, "id": rid, "assignee": emp_id, "assignee_name": emp_name, "status": "assigned"}))
}

#[derive(Deserialize)]
pub struct ProgressIn {
    #[serde(default)]
    pub progress: Option<i64>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub actor: String,
    #[serde(default)]
    pub visible: Option<bool>,
}

async fn request_progress(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<ProgressIn>,
) -> Json<Value> {
    let Some((rid, cur, assignee)) = sqlx::query_as::<_, (String, String, String)>(
        "SELECT id,status,assignee_id FROM bz_requests WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    if assignee.is_empty() {
        return Json(json!({"ok": false, "error": "ombi halijapewa mfanyakazi bado"}));
    }
    // Kuweka maendeleo huanza kazi ikiwa bado `assigned`.
    if cur == "assigned" {
        if !request_transition(&cur, "in_progress") {
            return Json(json!({"ok": false, "error": format!("hali `{cur}` hairuhusu maendeleo")}));
        }
        let _ = sqlx::query("UPDATE bz_requests SET status='in_progress' WHERE id=?1")
            .bind(&rid)
            .execute(&s.db)
            .await;
    } else if cur != "in_progress" && cur != "review" {
        return Json(json!({"ok": false, "error": format!("hali `{cur}` hairuhusu maendeleo")}));
    }
    let pct = b.progress.unwrap_or(50).clamp(0, 100);
    let _ = sqlx::query("UPDATE bz_requests SET progress=?2 WHERE id=?1")
        .bind(&rid)
        .bind(pct)
        .execute(&s.db)
        .await;
    let actor = if b.actor.trim().is_empty() {
        assignee.clone()
    } else {
        b.actor.clone()
    };
    event(&s.db, &rid, &actor, "progress", &b.note, pct).await;
    Json(json!({"ok": true, "id": rid, "progress": pct, "status": "in_progress"}))
}

async fn request_review(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    transition(&s, &id, "review", "mfanyakazi", "Kazi imekamilika — inasubiri ukaguzi", progress_floor("review")).await
}

async fn request_deliver(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let Some((rid, cur)) = sqlx::query_as::<_, (String, String)>(
        "SELECT id,status FROM bz_requests WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    if !request_transition(&cur, "delivered") {
        return Json(json!({"ok": false, "error": format!("hali `{cur}` hairuhusu kuwasilisha")}));
    }
    let _ = sqlx::query("UPDATE bz_requests SET status='delivered', progress=?2, completed_at=?3 WHERE id=?1")
        .bind(&rid)
        .bind(progress_floor("delivered"))
        .bind(now())
        .execute(&s.db)
        .await;
    event(&s.db, &rid, "admin", "delivered", "Kazi imewasilishwa kwa mteja", progress_floor("delivered")).await;
    audit(&s.db, "admin", "request.deliver", &rid, "").await;
    Json(json!({"ok": true, "id": rid, "status": "delivered", "progress": progress_floor("delivered")}))
}

async fn request_close(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    transition(&s, &id, "closed", "mteja", "Mteja amekubali kazi — ombi limefungwa", progress_floor("closed")).await
}

/// Mabadiliko ya jumla ya hali (yenye ukaguzi wa mpangilio).
async fn transition(
    s: &AppState,
    id: &str,
    to: &str,
    actor: &str,
    note: &str,
    progress: i64,
) -> Json<Value> {
    let Some((rid, cur)) = sqlx::query_as::<_, (String, String)>(
        "SELECT id,status FROM bz_requests WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    if !request_transition(&cur, to) {
        return Json(json!({"ok": false, "error": format!("mabadiliko `{cur}` → `{to}` hayaruhusiwi")}));
    }
    let _ = sqlx::query("UPDATE bz_requests SET status=?2, progress=MAX(progress,?3) WHERE id=?1")
        .bind(&rid)
        .bind(to)
        .bind(progress)
        .execute(&s.db)
        .await;
    let kind = match to {
        "approved" => "approved",
        "rejected" => "note",
        "review" => "review",
        "closed" => "delivered",
        _ => "note",
    };
    event(&s.db, &rid, actor, kind, note, progress).await;
    audit(&s.db, actor, &format!("request.{to}"), &rid, note).await;
    Json(json!({"ok": true, "id": rid, "status": to, "progress": progress}))
}

async fn event(
    db: &SqlitePool,
    request_id: &str,
    actor: &str,
    kind: &str,
    note: &str,
    progress: i64,
) {
    let kind = if EVENT_KINDS.contains(&kind) { kind } else { "note" };
    let _ = sqlx::query(
        "INSERT INTO bz_request_events (id,request_id,actor,kind,note,progress,visible_to_client,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,1,?7)",
    )
    .bind(uid())
    .bind(request_id)
    .bind(actor)
    .bind(kind)
    .bind(note)
    .bind(progress.clamp(0, 100))
    .bind(now())
    .execute(db)
    .await;
}

async fn events_for(db: &SqlitePool, request_id: &str, client_only: bool) -> Vec<Value> {
    let sql = if client_only {
        "SELECT actor,kind,note,progress,created_at FROM bz_request_events
         WHERE request_id=?1 AND visible_to_client=1 ORDER BY created_at ASC"
    } else {
        "SELECT actor,kind,note,progress,created_at FROM bz_request_events
         WHERE request_id=?1 ORDER BY created_at ASC"
    };
    sqlx::query_as::<_, (String, String, String, i64, String)>(sql)
        .bind(request_id)
        .fetch_all(db)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            json!({"actor": r.0, "kind": r.1, "note": r.2, "progress": r.3, "at": r.4})
        })
        .collect()
}

// ---------- mikataba ----------

async fn contracts_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(200).clamp(1, 1000);
    let rows = sqlx::query_as::<_, (String, String, String, i64, String, String, String)>(
        "SELECT id,request_id,title,amount_cents,currency,status,created_at FROM bz_contracts
         ORDER BY created_at DESC LIMIT ?1",
    )
    .bind(lim)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({"ok": true, "contracts": rows.into_iter().map(|r| json!({
        "id": r.0, "request_id": r.1, "title": r.2,
        "amount_cents": r.3, "amount": crate::fees::fmt_money(r.3),
        "currency": r.4, "status": r.5, "created_at": r.6
    })).collect::<Vec<_>>()}))
}

#[derive(Deserialize)]
pub struct ContractIn {
    pub request: String,
    pub title: String,
    #[serde(default)]
    pub scope: String,
    pub amount: String,
    #[serde(default)]
    pub currency: String,
}

/// Tengeneza mkataba kutoka ombi. Huhesabu hash ya SHA-256 (muhuri baadaye).
async fn contract_create(State(s): State<AppState>, Json(b): Json<ContractIn>) -> Json<Value> {
    let Some((rid, reference, amount_cents, currency)) = sqlx::query_as::<_, (String, String, i64, String)>(
        "SELECT id,ref,amount_cents,currency FROM bz_requests WHERE id=?1 OR ref=?1 LIMIT 1",
    )
    .bind(b.request.trim())
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten()
    else {
        return Json(json!({"ok": false, "error": "ombi haipo"}));
    };
    if b.title.trim().is_empty() {
        return Json(json!({"ok": false, "error": "kichwa cha mkataba kinahitajika"}));
    }
    // Kiasi: cha mkataba ikiwa kimetolewa, la sivyo cha ombi.
    let (amt, cur) = match crate::fees::to_cents(&b.amount) {
        Some(c) if c > 0 => (
            c,
            if b.currency.trim().is_empty() {
                currency
            } else {
                norm_currency(&b.currency)
            },
        ),
        _ => (amount_cents, currency),
    };
    let id = uid();
    let payload = format!("{reference}|{}|{amt}|{cur}", b.title.trim());
    let seal = crate::pkicrypto::sha256_hex(payload.as_bytes());
    let _ = sqlx::query(
        "INSERT INTO bz_contracts (id,request_id,title,scope,amount_cents,currency,status,client_sig,client_signed_at,admin_sig,admin_signed_at,seal_hash,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,'draft','',NULL,'',NULL,?7,?8)",
    )
    .bind(&id)
    .bind(&rid)
    .bind(b.title.trim())
    .bind(&b.scope)
    .bind(amt)
    .bind(&cur)
    .bind(&seal)
    .bind(now())
    .execute(&s.db)
    .await;
    audit(&s.db, "admin", "contract.create", &id, &reference).await;
    Json(json!({
        "ok": true, "id": id, "request_ref": reference, "seal_hash": seal,
        "amount_cents": amt, "amount": crate::fees::fmt_money(amt), "currency": cur,
        "status": "draft"
    }))
}

#[derive(Deserialize)]
pub struct SignIn {
    pub party: String,
    #[serde(default)]
    pub name: String,
}

/// Sahihi ya mkataba kwa ECDSA P-256 (pkicrypto). `party` = admin|client.
async fn contract_sign(
    State(s): State<AppState>,
    Path(id): Path<String>,
    Json(b): Json<SignIn>,
) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, i64, String)>(
        "SELECT id,request_id,title,scope,status,amount_cents,currency FROM bz_contracts WHERE id=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((cid, request_id, title, scope, status, amt, cur)) = row else {
        return Json(json!({"ok": false, "error": "mkataba haupo"}));
    };
    let party = b.party.trim().to_lowercase();
    if party != "admin" && party != "client" && party != "mteja" {
        return Json(json!({"ok": false, "error": "party lazima iwe admin au client"}));
    }
    let is_admin = party == "admin";
    if is_admin && status != "draft" && status != "client_signed" {
        return Json(json!({"ok": false, "error": format!("mkataba ({status}) hauwezi kutiwa sahihi na admin")}));
    }
    if !is_admin && status != "draft" && status != "admin_signed" {
        return Json(json!({"ok": false, "error": format!("mkataba ({status}) hauwezi kutiwa sahihi na mteja")}));
    }
    // Maudhui ya muhuri — yale yale yanayothibitishwa baadaye.
    let payload = format!("{cid}|{request_id}|{title}|{scope}|{amt}|{cur}");
    let pem = match crate::pkicrypto::active_private_pem(&s.db).await {
        Ok(p) => p,
        Err(e) => return Json(json!({"ok": false, "error": format!("funguo ya kampuni haipatikani: {e}")})),
    };
    let sig = match crate::pkicrypto::sign(&pem, payload.as_bytes()) {
        Ok(s) => s,
        Err(e) => return Json(json!({"ok": false, "error": e})),
    };
    let (client_sig, client_at, admin_sig, admin_at) = if is_admin {
        (String::new(), None, sig.clone(), Some(now()))
    } else {
        (sig.clone(), Some(now()), String::new(), None)
    };
    let new_status = if is_admin && status == "client_signed" {
        "signed"
    } else if !is_admin && status == "admin_signed" {
        "signed"
    } else if is_admin {
        "admin_signed"
    } else {
        "client_signed"
    };
    if is_admin {
        let _ = sqlx::query(
            "UPDATE bz_contracts SET admin_sig=?2, admin_signed_at=?3, status=?4 WHERE id=?1",
        )
        .bind(&cid)
        .bind(&admin_sig)
        .bind(&admin_at)
        .bind(new_status)
        .execute(&s.db)
        .await;
    } else {
        let _ = sqlx::query(
            "UPDATE bz_contracts SET client_sig=?2, client_signed_at=?3, status=?4 WHERE id=?1",
        )
        .bind(&cid)
        .bind(&client_sig)
        .bind(&client_at)
        .bind(new_status)
        .execute(&s.db)
        .await;
    }
    let _ = (client_sig, admin_sig);
    audit(&s.db, if is_admin { "admin" } else { "mteja" }, "contract.sign", &cid, new_status).await;
    Json(json!({
        "ok": true, "id": cid, "party": if is_admin { "admin" } else { "client" },
        "status": new_status, "alg": crate::pkicrypto::ALG_ECDSA
    }))
}

/// Thibitisha sahihi zote za mkataba kwa funguo za kampuni.
async fn contract_verify(State(s): State<AppState>, Path(id): Path<String>) -> Json<Value> {
    let row = sqlx::query_as::<_, (String, String, String, String, i64, String, String, String)>(
        "SELECT id,request_id,title,scope,amount_cents,currency,client_sig,admin_sig FROM bz_contracts WHERE id=?1 LIMIT 1",
    )
    .bind(&id)
    .fetch_optional(&s.db)
    .await
    .ok()
    .flatten();
    let Some((cid, request_id, title, scope, amt, cur, client_sig, admin_sig)) = row else {
        return Json(json!({"ok": false, "error": "mkataba haupo"}));
    };
    let payload = format!("{cid}|{request_id}|{title}|{scope}|{amt}|{cur}");
    let key = match crate::pkicrypto::active_key(&s.db).await {
        Ok(k) => k,
        Err(e) => return Json(json!({"ok": false, "error": e})),
    };
    let verify_one = |sig: &str| -> bool {
        if sig.is_empty() {
            return false;
        }
        crate::pkicrypto::verify(&key.public_pem, payload.as_bytes(), sig).unwrap_or(false)
    };
    let c_ok = verify_one(&client_sig);
    let a_ok = verify_one(&admin_sig);
    Json(json!({
        "ok": true, "id": cid,
        "client_signature_valid": c_ok,
        "admin_signature_valid": a_ok,
        "fully_signed": c_ok && a_ok,
        "alg": crate::pkicrypto::ALG_ECDSA,
        "key_fingerprint": key.fingerprint
    }))
}

// ---------- ufuatiliaji wa maeneo ----------

async fn coverage_list(State(s): State<AppState>, Query(q): Query<CoverageQuery>) -> Json<Value> {
    let rows = if let Some(k) = q.kind.as_deref().filter(|v| !v.is_empty()) {
        sqlx::query_as::<_, (String, String, String, String, i64)>(
            "SELECT kind,code,name,status,requests_count FROM bz_coverage WHERE kind=?1 ORDER BY name",
        )
        .bind(k)
        .fetch_all(&s.db)
        .await
    } else {
        sqlx::query_as::<_, (String, String, String, String, i64)>(
            "SELECT kind,code,name,status,requests_count FROM bz_coverage ORDER BY kind, name",
        )
        .fetch_all(&s.db)
        .await
    }
    .unwrap_or_default();
    let regions: Vec<Value> = rows
        .iter()
        .filter(|r| r.0 == "tz_region")
        .map(|r| json!({"code": r.1, "name": r.2, "status": r.3, "requests": r.4}))
        .collect();
    let countries: Vec<Value> = rows
        .iter()
        .filter(|r| r.0 == "country")
        .map(|r| json!({"code": r.1, "name": r.2, "status": r.3, "requests": r.4}))
        .collect();
    Json(json!({
        "ok": true,
        "regions_total": regions.len(), "countries_total": countries.len(),
        "regions": regions, "countries": countries
    }))
}

#[derive(Deserialize)]
struct CoverageQuery {
    #[serde(default)]
    kind: Option<String>,
}

// ---------- audit ----------

async fn audit_list(State(s): State<AppState>, Query(q): Query<ListQuery>) -> Json<Value> {
    let lim = q.limit.unwrap_or(200).clamp(1, 1000);
    let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
        "SELECT actor,action,target,detail,created_at FROM bz_audit ORDER BY created_at DESC LIMIT ?1",
    )
    .bind(lim)
    .fetch_all(&s.db)
    .await
    .unwrap_or_default();
    Json(json!({"ok": true, "audit": rows.into_iter().map(|r| json!({
        "actor": r.0, "action": r.1, "target": r.2, "detail": r.3, "at": r.4
    })).collect::<Vec<_>>()}))
}

pub async fn audit(db: &SqlitePool, actor: &str, action: &str, target: &str, detail: &str) {
    let _ = sqlx::query(
        "INSERT INTO bz_audit (id,actor,action,target,detail,created_at) VALUES (?1,?2,?3,?4,?5,?6)",
    )
    .bind(uid())
    .bind(actor)
    .bind(action)
    .bind(target)
    .bind(detail)
    .bind(now())
    .execute(db)
    .await;
}

// =====================================================================
// AMRI ZA SMS → VITENDO (hutumika na sms.rs)
// =====================================================================

/// Tekeleza amri iliyoingia kwa SMS kwenye ombi/mkataba.
/// `APPROVE <ref>` → idhini; `REJECT <ref>` → kukataa; `SIGN <ref>` → sahihi.
pub async fn apply_sms_command(db: &SqlitePool, action: &str, reference: &str) -> Value {
    if reference.is_empty() {
        return json!({"ok": false, "error": "kumbukumbu (ref) haipo kwenye amri"});
    }
    // Ombi kwa ref?
    let req = sqlx::query_as::<_, (String, String)>(
        "SELECT id,status FROM bz_requests WHERE ref=?1 OR id=?1 LIMIT 1",
    )
    .bind(reference)
    .fetch_optional(db)
    .await
    .ok()
    .flatten();
    match action {
        "approve" | "reject" => {
            let Some((rid, cur)) = req else {
                return json!({"ok": false, "error": format!("ombi `{reference}` halipo")});
            };
            let to = if action == "approve" { "approved" } else { "rejected" };
            if !request_transition(&cur, to) {
                return json!({"ok": false, "error": format!("ombi ({cur}) haliwezi `{to}`")});
            }
            let note = if action == "approve" {
                "Imeidhinishwa kwa SMS na admin"
            } else {
                "Imekataliwa kwa SMS na admin"
            };
            let _ = sqlx::query("UPDATE bz_requests SET status=?2, approved_at=?3 WHERE id=?1")
                .bind(&rid)
                .bind(to)
                .bind(now())
                .execute(db)
                .await;
            event(db, &rid, "admin", if action == "approve" { "approved" } else { "note" }, note, 5)
                .await;
            audit(db, "admin-sms", &format!("request.{to}"), &rid, reference).await;
            json!({"ok": true, "kind": "request", "id": rid, "status": to})
        }
        "sign" => {
            // Mkataba kwa request ref, au kwa id ya mkataba.
            let cid = sqlx::query_as::<_, (String,)>(
                "SELECT c.id FROM bz_contracts c
                 JOIN bz_requests r ON r.id=c.request_id
                 WHERE r.ref=?1 OR c.id=?1 ORDER BY c.created_at DESC LIMIT 1",
            )
            .bind(reference)
            .fetch_optional(db)
            .await
            .ok()
            .flatten();
            let Some((cid,)) = cid else {
                return json!({"ok": false, "error": format!("mkataba wa `{reference}` haupo")});
            };
            let row = sqlx::query_as::<_, (String, String, String, String, i64, String, String)>(
                "SELECT id,request_id,title,scope,amount_cents,currency,status FROM bz_contracts WHERE id=?1",
            )
            .bind(&cid)
            .fetch_one(db)
            .await
            .ok();
            let Some((id, request_id, title, scope, amt, cur, status)) = row else {
                return json!({"ok": false, "error": "mkataba haupo"});
            };
            if status != "draft" && status != "admin_signed" && status != "client_signed" {
                return json!({"ok": false, "error": format!("mkataba ({status}) hauwezi kutiwa sahihi")});
            }
            let payload = format!("{id}|{request_id}|{title}|{scope}|{amt}|{cur}");
            let Ok(pem) = crate::pkicrypto::active_private_pem(db).await else {
                return json!({"ok": false, "error": "funguo ya kampuni haipatikani"});
            };
            let Ok(sig) = crate::pkicrypto::sign(&pem, payload.as_bytes()) else {
                return json!({"ok": false, "error": "sahihi imeshindikana"});
            };
            let new_status = if status == "admin_signed" || status == "client_signed" {
                "signed"
            } else {
                "client_signed"
            };
            let _ = sqlx::query(
                "UPDATE bz_contracts SET client_sig=?2, client_signed_at=?3, status=?4 WHERE id=?1",
            )
            .bind(&id)
            .bind(&sig)
            .bind(now())
            .bind(new_status)
            .execute(db)
            .await;
            audit(db, "mteja-sms", "contract.sign", &id, reference).await;
            json!({"ok": true, "kind": "contract", "id": id, "status": new_status})
        }
        other => json!({"ok": false, "error": format!("amri `{other}` haijulikani")}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ubadilishaji_wa_sarafu_ni_sahihi() {
        // TZS 2,600.00 = USD 1.00 (rate 2600 TZS kwa dola).
        assert_eq!(convert(260_000, "TZS", "USD"), 100);
        assert_eq!(convert(100, "USD", "TZS"), 260_000);
        // Sarafu ile ile haibadiliki.
        assert_eq!(convert(12_345, "TZS", "TZS"), 12_345);
        assert_eq!(convert(12_345, "USD", "USD"), 12_345);
        // EUR 1.00 = TZS 2,850.00
        assert_eq!(convert(100, "EUR", "TZS"), 285_000);
        assert_eq!(convert(285_000, "TZS", "EUR"), 100);
    }

    #[test]
    fn sarafu_isiyojulikana_hurudi_tzs() {
        assert_eq!(norm_currency("usd"), "USD");
        assert_eq!(norm_currency("XXX"), "TZS");
        assert_eq!(rate("USD"), 2600.0);
        assert_eq!(rate("TZS"), 1.0);
    }

    #[test]
    fn ngazi_na_kategoria_hupangwa() {
        assert_eq!(norm_level("CEO"), "executive");
        assert_eq!(norm_level("Kiongozi"), "lead");
        assert_eq!(norm_level(""), "junior");
        assert_eq!(norm_category("Usalama"), "security");
        assert_eq!(norm_category("AI"), "ai");
        assert_eq!(norm_category(""), "software");
    }

    #[test]
    fn zana_hukaguliwa_kwa_ruhusa() {
        let granted = vec!["ide_code_run".to_string(), "report".to_string()];
        assert!(tool_allowed(&granted, "ide_code_run"));
        assert!(!tool_allowed(&granted, "secpack"));
        assert!(tool_exists("secpack"));
        assert!(!tool_exists("zana_ya_nje"));
        let denied = denied_tools(&granted, &["ide_code_run".into(), "secpack".into()]);
        assert_eq!(denied, vec!["secpack".to_string()]);
    }

    #[test]
    fn mpangilio_wa_maombi_hulindwa() {
        // Njia halali.
        assert!(request_transition("submitted", "approved"));
        assert!(request_transition("approved", "assigned"));
        assert!(request_transition("assigned", "in_progress"));
        assert!(request_transition("in_progress", "review"));
        assert!(request_transition("review", "delivered"));
        assert!(request_transition("delivered", "closed"));
        // Haramu: kazi HAIANZI bila idhini ya admin.
        assert!(!request_transition("submitted", "in_progress"));
        assert!(!request_transition("submitted", "assigned"));
        assert!(!request_transition("closed", "in_progress"));
        assert!(!request_transition("rejected", "approved"));
    }

    #[test]
    fn huduma_za_tz_hazipatikani_kwa_wateja_wa_nje() {
        assert!(service_available_for("global", "US"));
        assert!(service_available_for("tz", "TZ"));
        assert!(!service_available_for("tz", "US"));
        assert!(service_available_for("global", "TZ"));
    }

    #[test]
    fn kumbukumbu_ya_ombi_ina_umbo() {
        assert_eq!(request_ref(2026, 1), "REQ-2026-000001");
        assert_eq!(request_ref(2026, 123456), "REQ-2026-123456");
    }

    #[test]
    fn maeneo_hukaguliwa() {
        assert!(is_tz_region("DAR"));
        assert!(is_tz_region("MWA"));
        assert!(!is_tz_region("NY"));
        assert!(is_country("US"));
        assert!(is_country("TZ"));
        assert!(!is_country("XX"));
        assert_eq!(TZ_REGIONS.len(), 31);
    }

    #[test]
    fn hatua_ya_mteja_ina_maana() {
        assert_eq!(stage_label("in_progress", 40), "Inaendelea (40%)");
        assert_eq!(stage_label("closed", 100), "Imekamilika");
        assert!(needs_admin("submitted"));
        assert!(!needs_admin("approved"));
        assert!(in_flight("in_progress"));
        assert!(!in_flight("closed"));
    }

    #[test]
    fn sakafu_ya_maendeleo_inalingana_na_hatua() {
        // Kila hali ina sakafu moja; tukio na ombi hutumia thamani ILE ILE.
        assert_eq!(progress_floor("submitted"), 0);
        assert_eq!(progress_floor("approved"), 5);
        assert_eq!(progress_floor("assigned"), 10);
        assert_eq!(progress_floor("review"), 90);
        assert_eq!(progress_floor("delivered"), 100);
        assert_eq!(progress_floor("closed"), 100);
        assert_eq!(progress_floor("kitu_kingine"), 0);
        // Maendeleo HAYAPUNGUKI: sakafu ni ya chini kabisa, si ya juu.
        for st in ["approved", "assigned", "review", "delivered", "closed"] {
            let f = progress_floor(st);
            assert!(f >= 0 && f <= 100, "sakafu ya {st} nje ya 0..=100");
        }
        // Mpangilio wa kawaida wa safari ya ombi haupungui.
        let ladder = ["submitted", "approved", "assigned", "review", "delivered", "closed"];
        let floors: Vec<i64> = ladder.iter().map(|s| progress_floor(s)).collect();
        assert!(
            floors.windows(2).all(|w| w[0] <= w[1]),
            "sakafu haipungui: {floors:?}"
        );
    }

    #[test]
    fn rangi_za_chapa_hukaguliwa() {
        assert_eq!(norm_color("#ff6b2c").as_deref(), Some("#FF6B2C"));
        assert_eq!(norm_color("  #abc ").as_deref(), Some("#ABC"));
        assert_eq!(norm_color("red"), None);
        assert_eq!(norm_color("#12345"), None);
        assert_eq!(norm_color("#zzzzzz"), None);
        assert_eq!(norm_color(""), None);
    }

    #[test]
    fn chapa_ina_default_kwa_kila_ufunguo() {
        assert_eq!(BRANDING_KEYS.len(), 7);
        let keys: Vec<&str> = BRANDING_KEYS.iter().map(|(k, _)| *k).collect();
        for k in ["company_name", "tagline", "primary_color", "logo", "favicon"] {
            assert!(keys.contains(&k), "ufunguo {k} haupo");
        }
        assert_eq!(norm_color(BRANDING_KEYS[2].1).as_deref(), Some("#FF6B2C"));
    }

    fn cand(id: &str, level: &str, dep: &str, tools: &[&str], load: i64) -> Candidate {
        Candidate {
            id: id.into(),
            name: id.into(),
            level: level.into(),
            department_id: dep.into(),
            tools: tools.iter().map(|s| s.to_string()).collect(),
            load,
        }
    }

    #[test]
    fn zana_za_kategoria_ni_za_kila_kategoria() {
        assert!(category_tools("security").contains(&"secpack"));
        assert!(category_tools("software").contains(&"ide_build"));
        assert!(category_tools("ai").contains(&"aiengine"));
        assert!(category_tools("creative").contains(&"forge_rust3d"));
        // Kategoria isiyojulikana inapata zana ya msingi.
        assert_eq!(category_tools("haipo"), &["ide_code_run"]);
        // Kila zana iliyoorodheshwa LAZIMA iwepo kwenye katalogi.
        for cat in [
            "security", "software", "ai", "hardware", "finance", "education", "agriculture",
            "transport", "creative", "local",
        ] {
            for t in category_tools(cat) {
                assert!(tool_exists(t), "zana `{t}` ya kategoria `{cat}` haipo kwenye TOOLS");
            }
        }
    }

    #[test]
    fn ngazi_hupangwa_kwa_uzoefu() {
        assert!(level_rank("executive") > level_rank("lead"));
        assert!(level_rank("lead") > level_rank("senior"));
        assert!(level_rank("senior") > level_rank("mid"));
        assert!(level_rank("mid") > level_rank("junior"));
        assert!(level_rank("junior") > level_rank("intern"));
    }

    #[test]
    fn mfanyakazi_asiye_na_zana_haipati_kazi() {
        // Mfanyakazi wa uundaji HAPATI ombi la usalama.
        let emps = vec![cand("E1", "senior", "DEP-employee", &["ide_build", "forge"], 0)];
        assert_eq!(pick_assignee(&emps, "security", "employee"), None);
        // Naye hupata ombi la uundaji.
        assert_eq!(
            pick_assignee(&emps, "software", "employee").as_deref(),
            Some("E1")
        );
    }

    #[test]
    fn mzigo_mdogo_hupendelewa() {
        // Wote wana zana sawa; yule mwenye kazi chache huchaguliwa.
        let emps = vec![
            cand("E1", "senior", "DEP-employee", &["ide_build", "forge", "ide_testkit", "ide_e2e", "ide_code_run"], 5),
            cand("E2", "junior", "DEP-employee", &["ide_build", "forge", "ide_testkit", "ide_e2e", "ide_code_run"], 0),
        ];
        assert_eq!(pick_assignee(&emps, "software", "employee").as_deref(), Some("E2"));
    }

    #[test]
    fn idara_inayoendana_hupendelewa_kwa_zana_sawa() {
        let emps = vec![
            cand("E1", "mid", "DEP-marketing", &["forge_rust3d", "scribe", "report"], 0),
            cand("E2", "mid", "DEP-marketing", &["forge_rust3d", "scribe", "report"], 0),
        ];
        // Mzigo sawa, idara sawa, ngazi sawa → chaguo la kwanza linalingana.
        let picked = pick_assignee(&emps, "creative", "marketing");
        assert!(picked.as_deref() == Some("E1") || picked.as_deref() == Some("E2"));
        // Mfanyakazi wa idara nyingine (lakini zana zinafaa) hupata ikiwa peke yake.
        let solo = vec![cand("E9", "mid", "DEP-finance", &["forge_rust3d", "scribe", "report"], 0)];
        assert_eq!(pick_assignee(&solo, "creative", "marketing").as_deref(), Some("E9"));
    }

    #[test]
    fn idara_husimamia_kategoria() {
        assert_eq!(department_for_category("finance"), "finance");
        assert_eq!(department_for_category("local"), "finance");
        assert_eq!(department_for_category("creative"), "marketing");
        assert_eq!(department_for_category("security"), "employee");
        assert_eq!(department_for_category("software"), "employee");
        // Kila idara iliyorejeshwa LAZIMA iwepo kwenye DEPARTMENTS.
        let codes: Vec<&str> = DEPARTMENTS.iter().map(|(c, _, _, _)| *c).collect();
        for cat in ["security", "software", "ai", "creative", "finance", "local", "education"] {
            let d = department_for_category(cat);
            assert!(codes.contains(&d), "idara `{d}` ya `{cat}` haipo");
        }
    }

    #[test]
    fn hakuna_wafanyakazi_ni_chaguo_batili() {
        assert_eq!(pick_assignee(&[], "software", "employee"), None);
        // Wote hawana zana zinazofaa.
        let emps = vec![cand("E1", "lead", "", &[], 0)];
        assert_eq!(pick_assignee(&emps, "ai", "employee"), None);
    }
}
