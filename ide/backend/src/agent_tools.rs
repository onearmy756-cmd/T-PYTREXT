//! agent_tools.rs — ZANA ZENYE MAJINA (typed tools) kwa AGENT wa MTECH OS.
//!
//! KANUNI: agent HAITUMI path yoyote ya API atakayoivumbua. Inatumia ZANA
//! ZILIZOSAJILIWA hapa — jina, maelezo, na parameters zenye aina. Hii ni njia
//! ile ile ambayo mawakala wa kweli (Claude, GPT, n.k.) hufanya kazi:
//!   ombi → chagua zana → toa hoja → tekeleza → angalia matokeo → rudia.
//!
//! Faida za kubadilisha `fundi_api` (path huru) kuwa zana zenye majina:
//!   1. SALAMA: ni zana zilizo kwenye orodha pekee zinazoweza kuitwa —
//!      hakuna path ya kubuni inayoweza kufikia sehemu isiyokusudiwa.
//!   2. WAZI: model inaona jina + maelezo + aina ya hoja (schema halisi).
//!   3. INAJIELEZA: `tool_menu_sw()` huzalisha orodha ya zana kwenye system
//!      prompt moja kwa moja — hakuna kuandika orodha kwa mkono (hakuna drift).
//!
//! Utekelezaji halisi bado ni wito wa ndani wa API ya MTECH OS
//! (`http://127.0.0.1:PORT...`) kutoka `chat.rs` — kwa hiyo kila zana
//! inarudisha data HALISI ya mfumo, si makadirio.

use serde_json::{json, Value};

/// Hoja moja ya zana (jina, aina, maelezo, kama ni lazima).
pub struct ToolParam {
    pub name: &'static str,
    /// "string" | "number"
    pub ty: &'static str,
    pub desc_sw: &'static str,
    pub required: bool,
}

/// Zana moja: jina → wito wa ndani wa API.
pub struct ApiTool {
    pub name: &'static str,
    pub desc_sw: &'static str,
    pub method: &'static str, // GET | POST
    pub path: &'static str,   // njia ya API ya ndani (bila query)
    pub params: &'static [ToolParam],
}

const P_PATH: ToolParam = ToolParam {
    name: "path",
    ty: "string",
    desc_sw: "Njia ya faili ndani ya data/ (mf. knowledge/README.md)",
    required: true,
};
/// Njia ya faili ndani ya WORKSPACE ya IDE (mbilinyi-ide) — kwa zana za
/// `write_file`/`read_file`. Hii ndiyo workspace inayoonekana kwenye Monaco.
const P_WS_PATH: ToolParam = ToolParam {
    name: "path",
    ty: "string",
    desc_sw: "Njia ya faili ndani ya workspace (mf. src/app.js, index.html)",
    required: true,
};
/// Maudhui ya faili kamili ya kuandika (write_file).
const P_WS_CONTENT: ToolParam = ToolParam {
    name: "content",
    ty: "string",
    desc_sw: "Maudhui KAMILI ya faili (code halisi ya kuandika)",
    required: true,
};
const P_QUERY: ToolParam = ToolParam {
    name: "q",
    ty: "string",
    desc_sw: "Neno au swali la kutafuta mtandaoni",
    required: true,
};
const P_MAC: ToolParam = ToolParam {
    name: "mac",
    ty: "string",
    desc_sw: "MAC address ya kompyuta (mf. aa:bb:cc:dd:ee:ff)",
    required: true,
};
const P_IP: ToolParam = ToolParam {
    name: "ip",
    ty: "string",
    desc_sw: "IP ya kompyuta (hiari, kwa usahihi zaidi)",
    required: false,
};
const P_TASK: ToolParam = ToolParam {
    name: "task",
    ty: "string",
    desc_sw: "Kitambulisho cha kazi HALISI (mf. info-whois, info-dns, info-ssl, t10-scan, vuln-nikto, fore-binwalk, clam-scan). Kwa uchunguzi wa domain/mtandao tumia info-whois, info-dns, info-ssl. Orodha kamili: security_groups.",
    required: true,
};
const P_TARGET: ToolParam = ToolParam {
    name: "target",
    ty: "string",
    desc_sw: "Lengo la kazi (IP au hostname)",
    required: true,
};
const P_OP: ToolParam = ToolParam {
    name: "op",
    ty: "string",
    desc_sw: "Operesheni ya matriksi: det, inv, rank, trace, norm, eigen, transpose, add, sub, mul, solve, identity",
    required: true,
};
const P_CSV: ToolParam = ToolParam {
    name: "csv",
    ty: "string",
    desc_sw: "Matriksi A: kila safu kwenye mstari, vitufe vinatenganishwa kwa koma (mf. '4,7\\n2,6')",
    required: true,
};
const P_CSV2: ToolParam = ToolParam {
    name: "csv2",
    ty: "string",
    desc_sw: "Matriksi B (kwa add/sub/mul tu) — muundo ule ule wa csv",
    required: false,
};
const P_EXPR: ToolParam = ToolParam {
    name: "expr",
    ty: "string",
    desc_sw: "Fungu la hesabu: x, + - * / ^, sin cos tan exp ln log sqrt abs, pi, e (mf. 'x^2 + 3*x')",
    required: true,
};
const P_MODE: ToolParam = ToolParam {
    name: "mode",
    ty: "string",
    desc_sw: "value (f(x)), derivative (f′), second (f″), au integral (∫ a..b)",
    required: true,
};
const P_X: ToolParam = ToolParam {
    name: "x",
    ty: "number",
    desc_sw: "Thamani ya x (kwa value/derivative/second)",
    required: false,
};
const P_A: ToolParam = ToolParam {
    name: "a",
    ty: "number",
    desc_sw: "Mwanzo wa integral a",
    required: false,
};
const P_B: ToolParam = ToolParam {
    name: "b",
    ty: "number",
    desc_sw: "Mwisho wa integral b",
    required: false,
};
const P_QUBITS: ToolParam = ToolParam {
    name: "qubits",
    ty: "number",
    desc_sw: "Idadi ya qubits (1..14)",
    required: true,
};
const P_SHOTS: ToolParam = ToolParam {
    name: "shots",
    ty: "number",
    desc_sw: "Idadi ya measurements (1..4096)",
    required: false,
};
const P_OPS: ToolParam = ToolParam {
    name: "ops",
    ty: "string",
    desc_sw: "Gates kama JSON array, mf. [{\"gate\":\"h\",\"target\":0},{\"gate\":\"cnot\",\"control\":0,\"target\":1}]",
    required: true,
};
const P_BITS: ToolParam = ToolParam {
    name: "bits",
    ty: "number",
    desc_sw: "Idadi ya bits za nasibu za quantum (1..4096)",
    required: false,
};
const P_IMG_PROMPT: ToolParam = ToolParam {
    name: "prompt",
    ty: "string",
    desc_sw: "Maelezo kamili ya picha (mf. 'mji wa Dar es Salaam mchana, mwanga wa kijani')",
    required: true,
};
const P_IMG_URL: ToolParam = ToolParam {
    name: "image_url",
    ty: "string",
    desc_sw: "URL ya picha ya kuchunguza (vision)",
    required: true,
};
const P_IMG_QUESTION: ToolParam = ToolParam {
    name: "question",
    ty: "string",
    desc_sw: "Swali kuhusu picha (mf. ' Unaonyesha nini? Una hitilafu gani?')",
    required: false,
};
const P_CODE_LANG: ToolParam = ToolParam {
    name: "language",
    ty: "string",
    desc_sw: "Lugha ya kutekelezwa: rust, node, bash, go, elixir (default: rust)",
    required: false,
};
const P_CODE: ToolParam = ToolParam {
    name: "code",
    ty: "string",
    desc_sw: "Code ya kutekelezwa (mf. 'print(1+1)')",
    required: true,
};
const P_CODE_CWD: ToolParam = ToolParam {
    name: "cwd",
    ty: "string",
    desc_sw: "Saraka ya kufanyia kazi (si lazima). Weka ili faili za workspace zipatikane (mf. /data/ide)",
    required: false,
};
const P_R_TASK: ToolParam = ToolParam {
    name: "task",
    ty: "string",
    desc_sw: "summary | chart | risk_score | anomaly | sec_forecast | image | audio | forecast (custom HITL tu)",
    required: true,
};
const P_R_DATA: ToolParam = ToolParam {
    name: "data",
    ty: "array",
    desc_sw: "Namba za kuchambua (mf. [1,2,3,4])",
    required: false,
};
const P_R_FILE: ToolParam = ToolParam {
    name: "file",
    ty: "string",
    desc_sw: "File ya image au wav (path ndani ya sandbox)",
    required: false,
};
const P_TOOL: ToolParam = ToolParam {
    name: "tool",
    ty: "string",
    desc_sw: "Zana ya toolkit (mf. system/health, recon/dns, forensic/carve)",
    required: true,
};
const P_TASK_ID: ToolParam = ToolParam {
    name: "task_id",
    ty: "string",
    desc_sw: "ID ya kazi ya security pack (mf. t10-scan, vuln-nikto, fore-fls, clam-scan). Orodha: security_groups.",
    required: true,
};
const P_ACCOUNT: ToolParam = ToolParam {
    name: "account",
    ty: "string",
    desc_sw: "Akaunti ya kubillishwa (mf. mteja1). Ikiwa hujaweka, mfumo unatumia akaunti ya agent.",
    required: false,
};
const P_TARGET_TS: ToolParam = ToolParam {
    name: "target",
    ty: "string",
    desc_sw: "Lengo la kill-chain (IP, CIDR au domain). Lazima liwe kwenye scope.txt halo — vinginevyo command imezuiwa.",
    required: true,
};
const P_AGENT_TS: ToolParam = ToolParam {
    name: "agent",
    ty: "string",
    desc_sw: "Agent ya ThreatSwarm (mf. recon, exploit, web-attacker, dfir, blue-team — market 27). Orodha: threatswarm_agents.",
    required: true,
};
const P_APPROVED_TS: ToolParam = ToolParam {
    name: "human_approved",
    ty: "string",
    desc_sw: "Weka 'true' kama mmiliki ameidhinisha. Ukiondoa 'true' — high-risk stages zitasimama.",
    required: false,
};
const P_ENTRY_TS: ToolParam = ToolParam {
    name: "entry",
    ty: "string",
    desc_sw: "Lengo la kuongeza/kutoa kwenye scope.txt: IP, CIDR (10.0.0.0/24) au domain (mf. testlab.local).",
    required: true,
};
const P_TARGETS: ToolParam = ToolParam {
    name: "targets",
    ty: "array",
    desc_sw: "Orodha ya malengo (mf. [\"192.168.1.10\",\"scanme.sh\"]). Mass fleet inayawindia kwa WAKATI MMOJA.",
    required: true,
};
const P_SEARCH_Q: ToolParam = ToolParam {
    name: "q",
    ty: "string",
    desc_sw: "Neno la kutafuta.",
    required: true,
};
const P_V3_AGENTS: ToolParam = ToolParam {
    name: "agents",
    ty: "number",
    desc_sw: "Idadi ya mawakala kwa hesabu ya VRAM ya Warp-Cortex (mf. 1000).",
    required: false,
};
const P_V3_TASK: ToolParam = ToolParam {
    name: "task",
    ty: "string",
    desc_sw: "Kazi/tatizo la kuboresha (AgentDescent) au kuchunguza (Orthogonal).",
    required: true,
};
const P_V3_TOPIC: ToolParam = ToolParam {
    name: "topic",
    ty: "string",
    desc_sw: "Mada ya mzunguko wa ubunifu wa ReMIND (Wake→Dream→Judge→Re-wake).",
    required: true,
};
const P_V3_TIER: ToolParam = ToolParam {
    name: "tier",
    ty: "string",
    desc_sw: "Tabaka la kumbukumbu: working | episodic | semantic | procedural | emotional | meta.",
    required: false,
};
const P_V3_TASKS: ToolParam = ToolParam {
    name: "tasks",
    ty: "number",
    desc_sw: "Idadi ya kazi za kupanga kwa MegaFlow (mf. 10000).",
    required: false,
};
// --- AKILI (ubongo wa mfumo: kumbukumbu, maarifa, mipango, ToT, ethics) ---
const P_CONTENT: ToolParam = ToolParam {
    name: "content",
    ty: "string",
    desc_sw: "Maudhui ya kuhifadhi (kumbukumbu au maarifa).",
    required: true,
};
const P_KIND: ToolParam = ToolParam {
    name: "kind",
    ty: "string",
    desc_sw: "Aina ya kumbukumbu: fact | episode | preference | skill.",
    required: false,
};
const P_TITLE: ToolParam = ToolParam {
    name: "title",
    ty: "string",
    desc_sw: "Kichwa cha maarifa (knowledge base).",
    required: true,
};
const P_GOAL: ToolParam = ToolParam {
    name: "goal",
    ty: "string",
    desc_sw: "Lengo la kazi kwa mpango/pipeline ya Akili.",
    required: true,
};
const P_INPUT: ToolParam = ToolParam {
    name: "input",
    ty: "string",
    desc_sw: "Swali/hoja ya fikira (Akili V6 — tabaka 200).",
    required: true,
};
const P_ACTION: ToolParam = ToolParam {
    name: "action",
    ty: "string",
    desc_sw: "Tendo la kupimwa kwa maadili/HITL (mf. deploy, format, wipe).",
    required: true,
};
const P_OUTCOME: ToolParam = ToolParam {
    name: "outcome",
    ty: "string",
    desc_sw: "Matokeo ya kazi iliyopita (kwa Reflexion).",
    required: false,
};
const P_ERRORS: ToolParam = ToolParam {
    name: "errors",
    ty: "string",
    desc_sw: "Makosa yaliyotokea (kwa Reflexion).",
    required: false,
};
const P_PATTERN: ToolParam = ToolParam {
    name: "name",
    ty: "string",
    desc_sw: "Jina la pattern ya code (mf. rust-http-handler).",
    required: true,
};
const P_PATTERN_VARS: ToolParam = ToolParam {
    name: "vars",
    ty: "object",
    desc_sw: "Vigezo vya pattern (object: {\"Name\":\"value\"}).",
    required: false,
};
const P_IDEA: ToolParam = ToolParam {
    name: "idea",
    ty: "string",
    desc_sw: "Wazo la project (kwa autopilot/swarm wa IDE).",
    required: false,
};
const P_FAMILY: ToolParam = ToolParam {
    name: "family",
    ty: "string",
    desc_sw: "Familia ya agents: auto | swarm | ts | social | calls | messaging | device | humanitarian | blockchain.",
    required: false,
};
const P_AGENTS: ToolParam = ToolParam {
    name: "agents",
    ty: "array",
    desc_sw: "Orodha ya IDs za agents (array ya string); tupu = wote wa familia.",
    required: false,
};
const P_APPROVED: ToolParam = ToolParam {
    name: "human_approved",
    ty: "boolean",
    desc_sw: "Idhini ya binadamu (true/false) kwa kazi hatari za ThreatSwarm.",
    required: false,
};
const P_PATH_REQ: ToolParam = ToolParam {
    name: "path",
    ty: "string",
    desc_sw: "Njia ya faili ndani ya workspace ya IDE (kwa uthibitisho wa code).",
    required: true,
};
const P_CHART_KIND: ToolParam = ToolParam {
    name: "kind",
    ty: "string",
    desc_sw: "Aina ya chati: line | bar | pie | scatter | sparkline.",
    required: false,
};
const P_CHART_TITLE: ToolParam = ToolParam {
    name: "title",
    ty: "string",
    desc_sw: "Kichwa cha chati.",
    required: false,
};
const P_CHART_DATA: ToolParam = ToolParam {
    name: "data",
    ty: "array",
    desc_sw: "Data halisi: [{label,value}, ...] au [namba, ...].",
    required: true,
};
const P_VIDEO_FRAMES: ToolParam = ToolParam {
    name: "frames",
    ty: "array",
    desc_sw: "Njia za picha (png/jpg) kwa mfuatano wa fremu.",
    required: true,
};
const P_VIDEO_FPS: ToolParam = ToolParam {
    name: "fps",
    ty: "number",
    desc_sw: "Fremu kwa sekunde (default 8).",
    required: false,
};
const P_VIDEO_GIF: ToolParam = ToolParam {
    name: "gif",
    ty: "boolean",
    desc_sw: "true = GIF, false/blank = mp4.",
    required: false,
};
const P_STACK: ToolParam = ToolParam {
    name: "only",
    ty: "string",
    desc_sw: "Stack moja tu: rust | node | python | go | elixir | r | cpp-cmake ... (tupu = zote zilizogundulika).",
    required: false,
};
/// Njia ya hiari ndani ya workspace (tupu = mzizi wa workspace).
const P_WS_PATH_OPT: ToolParam = ToolParam {
    name: "path",
    ty: "string",
    desc_sw: "Njia ya folda ndani ya workspace (tupu = mzizi wa mradi).",
    required: false,
};
const P_SNIPPET: ToolParam = ToolParam {
    name: "code",
    ty: "string",
    desc_sw: "Maudhui ya code ya kuchanganua (kwa uthibitisho BILA kuandika faili).",
    required: false,
};
/// Maudhui ya code kwa selfcheck (jina tofauti na `code` ya code_execute).
const P_SC_CONTENT: ToolParam = ToolParam {
    name: "content",
    ty: "string",
    desc_sw: "Maudhui KAMILI ya code ya kukaguliwa (syntax + usalama).",
    required: true,
};
const P_LANG: ToolParam = ToolParam {
    name: "lang",
    ty: "string",
    desc_sw: "Lugha ya code: rust | js | python | sql | r | elixir | any.",
    required: false,
};
const P_PLATFORM: ToolParam = ToolParam {
    name: "platform",
    ty: "string",
    desc_sw: "Jukwaa la build. Inakubali jina fupi la OS (android | ios | linux | windows | macos | wasm) AU ID kamili kutoka /api/ide/build/targets (mf. linux-x64-gnu, windows-x64).",
    required: true,
};
const P_E2E_NAME: ToolParam = ToolParam {
    name: "name",
    ty: "string",
    desc_sw: "Jina la E2E spec (mf. smoke).",
    required: false,
};
const P_E2E_URL: ToolParam = ToolParam {
    name: "url",
    ty: "string",
    desc_sw: "URL ya ukurasa unaopimwa (mf. http://127.0.0.1:8080/).",
    required: false,
};

/// ZANA ZOTE zilizosajiliwa (orodha pekee inayoruhusiwa).
pub const TOOLS: &[ApiTool] = &[
    ApiTool {
        name: "write_file",
        desc_sw: "ANDIKA faili KWENYE WORKSPACE ya IDE (mbilinyi-ide) — code halisi inaonekana kwenye Monaco na huendelea kudumu. Hoja: path (mf. src/app.js), content (maudhui KAMILI ya faili). Tumia hii kwa kazi ZOTE za kukodi; USITUMIE code_execute kwa faili za workspace (hiyo ni sandbox ya muda).",
        method: "POST",
        path: "/api/ide/file",
        params: &[P_WS_PATH, P_WS_CONTENT],
    },
    ApiTool {
        name: "read_file",
        desc_sw: "SOMA faili KWENYE WORKSPACE ya IDE. Hoja: path (mf. src/app.js). Hurudisha maudhui halisi.",
        method: "GET",
        path: "/api/ide/file",
        params: &[P_WS_PATH],
    },
    ApiTool {
        name: "list_files",
        desc_sw: "ORODHESHA faili ZOTE kwenye workspace ya IDE (mtiririko wa mradi) — ili kujua muundo wa project.",
        method: "GET",
        path: "/api/ide/tree",
        params: &[],
    },
    ApiTool {
        name: "server_health",
        desc_sw: "Afya ya MTECH OS: huduma zinazoendesha na hali ya jumla.",
        method: "GET",
        path: "/health",
        params: &[],
    },
    ApiTool {
        name: "disk_space",
        desc_sw: "Nafasi HALISI ya diski ya server: ukubwa, iliyotumika, iliyobaki.",
        method: "GET",
        path: "/api/system/disk",
        params: &[],
    },
    ApiTool {
        name: "list_computers",
        desc_sw: "Orodha ya kompyuta kwenye mtandao wa LAN — jina, IP, MAC, mmiliki.",
        method: "GET",
        path: "/computers/discover",
        params: &[],
    },
    ApiTool {
        name: "wake_computer",
        desc_sw: "Kuamsha kompyuta iliyozima kwa Wake-on-LAN (WOL).",
        method: "POST",
        path: "/api/wol",
        params: &[P_MAC, P_IP],
    },
    ApiTool {
        name: "usb_phones",
        desc_sw: "Simu zilizounganishwa kwa USB kwenye server (daktari wa simu).",
        method: "GET",
        path: "/api/mobile/usb",
        params: &[],
    },
    ApiTool {
        name: "data_tree",
        desc_sw: "Muundo wa folda za data/ za mradi (maarifa, skills, devices...).",
        method: "GET",
        path: "/api/data/tree",
        params: &[],
    },
    ApiTool {
        name: "read_data_file",
        desc_sw: "Kusoma faili moja ndani ya data/ (maarifa ya mradi).",
        method: "GET",
        path: "/api/data/file",
        params: &[P_PATH],
    },
    ApiTool {
        name: "lan_scan",
        desc_sw: "Uchunguzi wa mtandao wa LAN na mwongozo wa kufikia kompyuta kwa mbali.",
        method: "GET",
        path: "/api/lan/scan",
        params: &[],
    },
    ApiTool {
        name: "security_groups",
        desc_sw: "Vikundi vya kazi za usalama na uchunguzi wa kidijitali vinavyopatikana.",
        method: "GET",
        path: "/api/security/groups",
        params: &[],
    },
    ApiTool {
        name: "security_task",
        desc_sw: "Kanzisha kazi ya usalama kwa lengo (ripoti ni data HALISI). Hoja task ni ID kamili: info-whois (mmiliki wa domain), info-dns (A/AAAA/PTR), info-ssl (SSL/TLS), t10-scan (ports 22,80,443,445,3389), t10-sqlmap, info-harvest (OSINT), info-netdiscover. Tumia info-whois kwanza, kisha info-dns, kisha info-ssl.",
        method: "POST",
        path: "/api/security/task",
        params: &[P_TASK, P_TARGET],
    },
    ApiTool {
        name: "toolkit_catalog",
        desc_sw: "Katalogi ya huduma za usalama zinazopatikana kwa makundi.",
        method: "GET",
        path: "/api/toolkit/catalog",
        params: &[],
    },
    ApiTool {
        name: "web_search",
        desc_sw: "Kutafuta taarifa HALISI mtandaoni na kurudisha matokeo + vyanzo.",
        method: "GET",
        path: "/api/web/search",
        params: &[P_QUERY],
    },
    ApiTool {
        name: "searxng_search",
        desc_sw: "Kutafuta kwa SearXNG meta-search (matokeo kutoka engines nyingi) — self-hosted au public.",
        method: "GET",
        path: "/api/web/searxng",
        params: &[P_QUERY],
    },
    ApiTool {
        name: "llm_status",
        desc_sw: "Hali ya model ya AI inayotumika sasa na uwezo wa mfumo.",
        method: "GET",
        path: "/api/llm/status",
        params: &[],
    },
    ApiTool {
        name: "brain_stats",
        desc_sw: "Kumbukumbu za agents (suluhisho zilizoshirikiwa) zilizohifadhiwa.",
        method: "GET",
        path: "/api/brain/stats",
        params: &[],
    },
    ApiTool {
        name: "list_backups",
        desc_sw: "Backup zilizopo za mfumo na tarehe zake.",
        method: "GET",
        path: "/backups",
        params: &[],
    },
    ApiTool {
        name: "fm_problems",
        desc_sw: "Data HALISI ya Fundi Mobile: matatizo ya simu + suluhisho (brands, error codes).",
        method: "GET",
        path: "/api/problems",
        params: &[],
    },
    ApiTool {
        name: "math_matrix",
        desc_sw: "Injini ya MATRIX (Rust): det, inv, rank, trace, norm, eigen, transpose, add, sub, mul, solve, identity.",
        method: "POST",
        path: "/api/math/matrix",
        params: &[P_OP, P_CSV, P_CSV2],
    },
    ApiTool {
        name: "math_calc",
        desc_sw: "Injini ya CALCULUS (Rust, hakuna eval): thamani, derivative, second, integral.",
        method: "POST",
        path: "/api/math/calc",
        params: &[P_MODE, P_EXPR, P_X, P_A, P_B],
    },
    ApiTool {
        name: "quantum_run",
        desc_sw: "Injini ya QUANTUM (statevector halisi): endesha gates na pima shots.",
        method: "POST",
        path: "/api/quantum/run",
        params: &[P_QUBITS, P_OPS, P_SHOTS],
    },
    ApiTool {
        name: "quantum_qrng",
        desc_sw: "QRNG: namba za nasibu za quantum (Hadamard + measurement).",
        method: "POST",
        path: "/api/quantum/random",
        params: &[P_BITS],
    },
    ApiTool {
        name: "toolkit_run",
        desc_sw: "Endesha zana ya usalama kwenye lengo (RBAC).",
        method: "POST",
        path: "/api/toolkit/run",
        params: &[P_TOOL, P_TARGET, P_ACCOUNT],
    },
    ApiTool {
        name: "security_run",
        desc_sw: "ENDESHA KAZI HALISI ya security pack (Kali tools 1600+ kati ya 57 kazi zilizoandaliwa): uchunguzi (nmap/masscan), udhaifu (nikto), forensiki (fls, vol), antivirus (clamav). Iende info-*(dns/ssl/whois), vuln-* (nikto/lynis), fore-* (binwalk/fls/chkrootkit), clam-* na t10-*. Inahitaji kazi (task_id) + optionally lengo.",
        method: "POST",
        path: "/api/security/task",
        params: &[P_TASK_ID, P_TARGET, P_ACCOUNT],
    },
    ApiTool {
        name: "image_gen",
        desc_sw: "Tengeneza PICHA HALISI kutoka maelezo (fal.ai / gpt-image): art, diagrams, screenshots-style. Inarudi URL ya picha.",
        method: "POST",
        path: "/api/agent/image",
        params: &[P_IMG_PROMPT],
    },
    ApiTool {
        name: "vision",
        desc_sw: "ANAONA PICHA: chunguza picha kutoka URL — eleza unachoona, bug, UI, error screen.",
        method: "POST",
        path: "/api/agent/vision",
        params: &[P_IMG_URL, P_IMG_QUESTION],
    },
    ApiTool {
        name: "code_execute",
        desc_sw: "ENDESHA CODE HALISI kwa ndani (Rust/Node/Shell/Go/Elixir Hitch): Safe sandbox, timeout 30s. Hakuna shell injection — arg-array.",
        method: "POST",
        path: "/api/code/run",
        params: &[P_CODE_LANG, P_CODE, P_CODE_CWD],
    },
    ApiTool {
        name: "security_mass_scan",
        desc_sw: "MASS SCAN FLEET: endesha kazi moja ya usalama kwa malengo MASAA mengi KWA WAKATI MMOJA (wainotzi 64, mpaka 700M). Hoja: task_id (mf. t10-scan), targets (orodha), limit (cha kutosha). Malengo mengi: BIR tofauti hoja hii badala ya security_run.",
        method: "POST",
        path: "/api/security/mass",
        params: &[P_TASK_ID, P_TARGETS, P_ACCOUNT],
    },
    ApiTool {
        name: "security_report",
        desc_sw: "RIPOTI KAMILI YA USALAMA (HTML yenye rangi, tiles + tables): kazi zote za mwisho, mafanikio, muda, audit halisi.",
        method: "GET",
        path: "/api/security/report",
        params: &[],
    },
    ApiTool {
        name: "threatswarm_engage",
        desc_sw: "KILL-CHAIN KAMILI ya ThreatSwarm (agents 27) kwa lengo moja: recon → recon-details → web-vuln → tls → os-audit → antivirus. Scope gate (scope.txt) HALISI — lengo lisipo scope.txt honi kubaliki.",
        method: "POST",
        path: "/api/ts/engage",
        params: &[P_TARGET_TS, P_APPROVED_TS],
    },
    ApiTool {
        name: "threatswarm_scope",
        desc_sw: "CHECK RUHUSA kwanza: kama lengo (IP/CIDR/domain) limepo kwenye scope.txt — deterministik, scope gate.",
        method: "POST",
        path: "/api/ts/scope",
        params: &[P_TARGET_TS],
    },
    ApiTool {
        name: "threatswarm_agents",
        desc_sw: "ORODHA ya agents 27 za ThreatSwarm na kazi zao halisi (recon = t10-scan + info-masscan n.k.). Tumia kabla ya threatswarm_agent.",
        method: "GET",
        path: "/api/ts/agents",
        params: &[],
    },
    ApiTool {
        name: "threatswarm_agent",
        desc_sw: "ENDESHA AGENT MOJA ya ThreatSwarm AUTONOMOUS kwa lengo: inatekeleza KAZI ZOTE za agent husika (security pack halisi) mfululizo, scope-gate la MWANZO. Kama lengo hailike kwenye scope.txt, imezuiwa.",
        method: "POST",
        path: "/api/ts/agent",
        params: &[P_AGENT_TS, P_TARGET_TS, P_APPROVED_TS],
    },
    ApiTool {
        name: "threatswarm_reports",
        desc_sw: "ORODHA ya ripoti zote za kill-chain za ThreatSwarm zilizokamilika (id, tumia kufungua ripoti kamili).",
        method: "GET",
        path: "/api/ts/reports",
        params: &[],
    },
    ApiTool {
        name: "threatswarm_scope_list",
        desc_sw: "ORODHA ya malengo yaliyoruhusiwa (scope.txt): IP, CIDR, domain. Tumia kabla ya kuanza kazi za nje.",
        method: "GET",
        path: "/api/ts/scope/list",
        params: &[],
    },
    ApiTool {
        name: "threatswarm_scope_add",
        desc_sw: "ONGEZA lengo kwenye scope.txt (Admin Mkuu pekee): IP, CIDR (10.0.0.0/24) au domain. Hii inaruhusu kazi za nje kwa mujibu wa idhini.",
        method: "POST",
        path: "/api/ts/scope/add",
        params: &[P_ENTRY_TS],
    },
    ApiTool {
        name: "threatswarm_scope_remove",
        desc_sw: "TOA lengo kwenye scope.txt (Admin Mkuu pekee).",
        method: "POST",
        path: "/api/ts/scope/remove",
        params: &[P_ENTRY_TS],
    },
    ApiTool {
        name: "r_engine",
        desc_sw: "INJINI YA R: uchambuzi + SECURITY ANALYTICS halisi — summary (stats), chart (PNG), chart3d (3D PNG + plotly HTML), video (MP4/H.264 halisi), risk_score (CVSS severity kwa findings), anomaly (robust MAD), sec_forecast (trend ya incidents), image, audio, forecast (linear). Inaitwa /api/r/engine (Rscript, sandbox, timeout 60s). Hoja: task + data (namba, au [[x,y,z],...] kwa chart3d) au file.",
        method: "POST",
        path: "/api/r/engine",
        params: &[P_R_TASK, P_R_DATA, P_R_FILE],
    },
    // --- Safu ya pili ya maono (swarm_v3) — halisi ---
    ApiTool {
        name: "swarm_v3_status",
        desc_sw: "Hali ya safu ya pili ya maono (Warp-Cortex, Orthogonal, ReMIND, AgentDescent, kumbukumbu 6, MegaFlow).",
        method: "GET",
        path: "/api/swarm/v3/status",
        params: &[],
    },
    ApiTool {
        name: "warp_cortex_plan",
        desc_sw: "Warp-Cortex: hesabu HALISI ya VRAM kwa mawakala N (Singleton Weight Sharing + Topological Synapse). Hoja: agents.",
        method: "POST",
        path: "/api/swarm/v3/cortex",
        params: &[P_V3_AGENTS],
    },
    ApiTool {
        name: "orthogonal_insight",
        desc_sw: "Orthogonal Insight: WorldBuilder + Solver + kura — huchunguza dunia tofauti na kuchagua suluhisho la kipekee. Hoja: task.",
        method: "POST",
        path: "/api/swarm/v3/orthogonal",
        params: &[P_V3_TASK],
    },
    ApiTool {
        name: "remind_cycle",
        desc_sw: "ReMIND: mzunguko wa ubunifu Wake→Dream→Judge→Re-wake. Hoja: topic.",
        method: "POST",
        path: "/api/swarm/v3/remind",
        params: &[P_V3_TOPIC],
    },
    ApiTool {
        name: "agent_descent",
        desc_sw: "AgentDescent: diffs kama gradients — mawakala hupendekeza mabadiliko, bora huunganishwa. Hoja: task.",
        method: "POST",
        path: "/api/swarm/v3/descent",
        params: &[P_V3_TASK],
    },
    ApiTool {
        name: "memory_recall",
        desc_sw: "Kumbukumbu ya tabaka 6: tafuta kumbukumbu kwa neno. Hoja: q, tier (hiari).",
        method: "GET",
        path: "/api/swarm/v3/memory/recall",
        params: &[P_SEARCH_Q, P_V3_TIER],
    },
    ApiTool {
        name: "megaflow_plan",
        desc_sw: "MegaFlow: gawanya kazi katika huduma tatu huru (Model/Agent/Environment) bila kupoteza kazi. Hoja: tasks.",
        method: "POST",
        path: "/api/swarm/v3/flow",
        params: &[P_V3_TASKS],
    },
    // ================= AKILI (ubongo wa mfumo) =================
    ApiTool {
        name: "akili_remember",
        desc_sw: "AKILI: hifadhi kumbukumbu ya kudumu (SQLite) — fact/episode/preference/skill.",
        method: "POST",
        path: "/api/akili/remember",
        params: &[P_CONTENT, P_KIND],
    },
    ApiTool {
        name: "akili_recall",
        desc_sw: "AKILI: kumbuka kutoka kumbukumbu ya kudumu + working memory.",
        method: "GET",
        path: "/api/akili/recall",
        params: &[P_QUERY],
    },
    ApiTool {
        name: "akili_knowledge_add",
        desc_sw: "AKILI: ongeza maarifa kwenye knowledge base (title + content).",
        method: "POST",
        path: "/api/akili/knowledge",
        params: &[P_TITLE, P_CONTENT],
    },
    ApiTool {
        name: "akili_knowledge_search",
        desc_sw: "AKILI: tafuta maarifa kwenye knowledge base.",
        method: "GET",
        path: "/api/akili/knowledge",
        params: &[P_QUERY],
    },
    ApiTool {
        name: "akili_verify",
        desc_sw: "AKILI: THIBITISHA code HALISI (cargo check / node --check / go build) kwenye workspace.",
        method: "POST",
        path: "/api/akili/verify",
        params: &[P_PATH_REQ],
    },
    ApiTool {
        name: "akili_plan",
        desc_sw: "AKILI: panga kazi kwa heuristics halisi (goal → hatua).",
        method: "POST",
        path: "/api/akili/plan",
        params: &[P_GOAL],
    },
    ApiTool {
        name: "akili_tot",
        desc_sw: "AKILI: Tree of Thoughts — suluhisha tatizo kwa mti wa mawazo.",
        method: "POST",
        path: "/api/akili/tot",
        params: &[P_TASK],
    },
    ApiTool {
        name: "akili_pipeline",
        desc_sw: "AKILI: pipeline KAMILI (perception→memory→plan→ToT→verify→report) kwa lengo.",
        method: "POST",
        path: "/api/akili/run",
        params: &[P_GOAL],
    },
    ApiTool {
        name: "akili_reflect",
        desc_sw: "AKILI: Reflexion — jifunza kutoka kosa (task + outcome + errors).",
        method: "POST",
        path: "/api/akili/reflect",
        params: &[P_TASK, P_OUTCOME, P_ERRORS],
    },
    ApiTool {
        name: "akili_pattern",
        desc_sw: "AKILI: tumia pattern ya code + vigezo kuzalisha code HALISI.",
        method: "POST",
        path: "/api/akili/patterns/apply",
        params: &[P_PATTERN, P_PATTERN_VARS],
    },
    ApiTool {
        name: "akili_v6_think",
        desc_sw: "AKILI V6: fikira kamili — tabaka 200 + ethics + HITL decision.",
        method: "POST",
        path: "/api/akili/v6/live",
        params: &[P_INPUT],
    },
    ApiTool {
        name: "akili_v6_gate",
        desc_sw: "AKILI V6: lango la maadili/HITL kwa tendo hatari (deploy/format/wipe).",
        method: "POST",
        path: "/api/akili/v6/gate",
        params: &[P_ACTION],
    },
    ApiTool {
        name: "akili_v3_solve",
        desc_sw: "AKILI V3: suluhisha kwa tabaka 7 (episodic+semantic+procedural+MCTS).",
        method: "POST",
        path: "/api/akili/v3/solve",
        params: &[P_TASK],
    },
    // ================= AGENTS UNIFIED (familia zote) =================
    ApiTool {
        name: "agents_unified_run",
        desc_sw: "AGENTS: endesha familia ya agents kwa PARALLEL (swarm/ts/social/calls/messaging/device/humanitarian/blockchain).",
        method: "POST",
        path: "/api/agents/unified/run",
        params: &[P_FAMILY, P_TARGET, P_AGENTS, P_APPROVED],
    },
    ApiTool {
        name: "agents_unified_catalog",
        desc_sw: "AGENTS: orodha ya familia zote za agents (swarm, ts, social, calls, messaging, device, humanitarian, blockchain).",
        method: "GET",
        path: "/api/agents/unified/catalog",
        params: &[],
    },
    ApiTool {
        name: "chart",
        desc_sw: "TENGENEZA CHATI HALISI (line/bar/pie/scatter/sparkline) kutoka data uliyoipata — SVG offline, bila mtandao. Hoja data = JSON ya [{label,value}] au [namba]. Tumia hii kuonyesha matokeo ya uchunguzi/benchmark kwa picha.",
        method: "POST",
        path: "/api/nyi/chart",
        params: &[P_CHART_KIND, P_CHART_TITLE, P_CHART_DATA],
    },
    ApiTool {
        name: "media_status",
        desc_sw: "HALI YA MEDIA: zana zipi za picha/video/chati zipo HALISI kwenye mashine hii (chart, image, video/ffmpeg). Tumia kabla ya kuahidi video.",
        method: "GET",
        path: "/api/nyi/media/status",
        params: &[],
    },
    ApiTool {
        name: "video_make",
        desc_sw: "TENGENEZA VIDEO/GIF HALISI kutoka fremu (picha) kwa ffmpeg. Angalia media_status kwanza — kama ffmpeg haipo, hurudisha kosa la wazi. Hoja frames = njia za png/jpg.",
        method: "POST",
        path: "/api/nyi/video/make",
        params: &[P_VIDEO_FRAMES, P_VIDEO_FPS, P_VIDEO_GIF],
    },
    // --- uthibitisho (self-verification): zana hizi hurudisha MATOKEO HALISI
    // ya mtihani/uchanganuzi, si maoni. Agent HULAZIMIKA kuzitumia kabla ya
    // kudai kazi imekamilika — hivi ndivyo inavyozidi mawakala wanaokisia.
    ApiTool {
        name: "run_tests",
        desc_sw: "ENDESHA testi HALISI za workspace (cargo test / npm test / go test / mix test / Rscript) na urudishe matokeo. LAZIMA kuitumia baada ya kuandika code, kabla ya kudai imekamilika. Hoja: path (folda ya mradi), only (stack moja, hiari).",
        method: "POST",
        path: "/api/ide/test/run",
        params: &[P_WS_PATH_OPT, P_STACK],
    },
    ApiTool {
        name: "test_suites",
        desc_sw: "Orodha ya stacks zilizogundulika kwenye mradi + zana za testi zinazopatikana. Tumia kabla ya run_tests.",
        method: "GET",
        path: "/api/ide/test/suites",
        params: &[],
    },
    ApiTool {
        name: "code_audit",
        desc_sw: "CHANGANUA code kwa kanuni za usalama/ubora (secrets, XSS, SQLi, panic, TODO) — hurudisha findings HALISI na alama. Tumia baada ya kuandika code. Hoja: path (faili/folda ndani ya workspace).",
        method: "POST",
        path: "/api/ide/code/audit",
        params: &[P_PATH_REQ],
    },
    ApiTool {
        name: "code_audit_snippet",
        desc_sw: "CHANGANUA kipande cha code BILA kuandika faili — kwa uthibitisho wa haraka. Hoja: code, lang (hiari; hutambuliwa kutoka path).",
        method: "POST",
        path: "/api/ide/code/audit/file",
        params: &[P_SNIPPET, P_LANG],
    },
    ApiTool {
        name: "build_targets",
        desc_sw: "Orodha ya majukwaa ya build yanayopatikana (android, ios, linux, windows, macos, wasm).",
        method: "GET",
        path: "/api/ide/build/targets",
        params: &[],
    },
    ApiTool {
        name: "build_run",
        desc_sw: "JENGA mradi kwa jukwaa HALISI (cargo build --release --target ...). Huthibitisha code inakompila. Hoja: platform (lazima), path (folda ya mradi, hiari).",
        method: "POST",
        path: "/api/ide/build/run",
        params: &[P_PLATFORM, P_WS_PATH_OPT],
    },
    ApiTool {
        name: "e2e_run",
        desc_sw: "ENDESHA jaribio la E2E (Playwright) dhidi ya URL HALISI — huthibitisha UI inafanya kazi. Hoja: name (jina la spec), url (ukurasa).",
        method: "POST",
        path: "/api/ide/e2e/run",
        params: &[P_E2E_NAME, P_E2E_URL],
    },
    ApiTool {
        name: "e2e_status",
        desc_sw: "Hali ya mazingira ya E2E (Node/npx/Playwright zipo?) kabla ya e2e_run.",
        method: "GET",
        path: "/api/ide/e2e/status",
        params: &[],
    },
    ApiTool {
        name: "selfcheck",
        desc_sw: "MZUNGUZO wa KUJISAHIHISHA wa mradi: kagua kipande cha code kwa syntax + usalama (lugha 19) BILA kuandika faili. Hoja: content (code), path (kidokezo cha jina la faili). Hurudisha ok/errors/findings halisi.",
        method: "POST",
        path: "/api/ide/selfcheck",
        params: &[P_SC_CONTENT, P_WS_PATH_OPT],
    },
];

/// Wito uliotatuliwa: method + path (na query) + body (kwa POST).
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedCall {
    pub method: String,
    pub path: String,
    pub body: Option<Value>,
}

/// Encoder ndogo ya query (hakuna dependency mpya): RFC 3986 unreserved pekee.
pub fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        let c = *b as char;
        if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Tafuta zana kwa jina.
pub fn find(name: &str) -> Option<&'static ApiTool> {
    TOOLS.iter().find(|t| t.name == name)
}

/// TATUA wito: chagua zana, hakikisha hoja za lazima, tengeneza path/body.
pub fn resolve(name: &str, args: &Value) -> Result<ResolvedCall, String> {
    let tool = find(name).ok_or_else(|| {
        format!(
            "Zana '{name}' haipo. Zana zilizopo: {}",
            TOOLS.iter().map(|t| t.name).collect::<Vec<_>>().join(", ")
        )
    })?;

    // Hoja za lazima zipo?
    for p in tool.params {
        if p.required {
            let v = args.get(p.name).and_then(|v| match v {
                Value::String(s) if !s.trim().is_empty() => Some(s.as_str()),
                Value::Number(_) => Some("n"),
                _ => None,
            });
            if v.is_none() {
                return Err(format!(
                    "Zana '{}' inahitaji hoja '{}' ({}).",
                    name, p.name, p.desc_sw
                ));
            }
        }
    }

    // Hoja zisizojulikana? (inasaidia model kujirekebisha)
    if let Some(obj) = args.as_object() {
        for k in obj.keys() {
            if !tool.params.iter().any(|p| p.name == k) {
                return Err(format!(
                    "Zana '{}' haina hoja '{}'. Hoja zinazokubalika: {}",
                    name,
                    k,
                    if tool.params.is_empty() {
                        "(hakuna)".to_string()
                    } else {
                        tool.params
                            .iter()
                            .map(|p| p.name)
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                ));
            }
        }
    }

    if tool.method == "POST" {
        let mut body = serde_json::Map::new();
        for p in tool.params {
            if let Some(v) = args.get(p.name) {
                if v.is_null() {
                    continue;
                }
                // Map aliyejulikana: task_id → task (SecTaskReq/MassReq zinasubiri "task").
                let key = if p.name == "task_id" { "task" } else { p.name };
                // 'ops' na 'targets' zinaweza kutumwa kama string ya JSON —
                // parse hadi Value halisi (array/object).
                let vv = key == "ops" || key == "targets";
                let vv = if vv {
                    match v {
                        Value::String(s) => serde_json::from_str::<Value>(s)
                            .unwrap_or_else(|_| v.clone()),
                        other => other.clone(),
                    }
                } else {
                    v.clone()
                };
                body.insert(key.to_string(), vv);
            }
        }
        return Ok(ResolvedCall {
            method: "POST".into(),
            path: tool.path.into(),
            body: Some(Value::Object(body)),
        });
    }

    // GET: hoja → query string
    let mut qs: Vec<String> = Vec::new();
    for p in tool.params {
        if let Some(v) = args.get(p.name) {
            let s = match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            if !s.is_empty() {
                qs.push(format!("{}={}", p.name, urlencode(&s)));
            }
        }
    }
    let path = if qs.is_empty() {
        tool.path.to_string()
    } else {
        format!("{}?{}", tool.path, qs.join("&"))
    };
    Ok(ResolvedCall {
        method: "GET".into(),
        path,
        body: None,
    })
}

/// Schema ya OpenAI-compatible (`tools`) inayotumwa kwa model.
pub fn openai_tools() -> Value {
    let arr: Vec<Value> = TOOLS
        .iter()
        .map(|t| {
            let mut props = serde_json::Map::new();
            let mut required: Vec<Value> = Vec::new();
            for p in t.params {
                props.insert(
                    p.name.to_string(),
                    json!({ "type": p.ty, "description": p.desc_sw }),
                );
                if p.required {
                    required.push(json!(p.name));
                }
            }
            let mut func = serde_json::Map::new();
            func.insert("name".into(), json!(t.name));
            func.insert("description".into(), json!(t.desc_sw));
            func.insert(
                "parameters".into(),
                json!({
                    "type": "object",
                    "properties": Value::Object(props),
                    "required": Value::Array(required),
                }),
            );
            json!({ "type": "function", "function": Value::Object(func) })
        })
        .collect();
    Value::Array(arr)
}

/// Orodha inayowekwa kwenye system prompt — inazalishwa kutoka registry,
/// kwa hiyo prompt na zana hazitengani kamwe.
pub fn tool_menu_sw() -> String {
    let mut out = String::new();
    for t in TOOLS {
        out.push_str(&format!("- {} — {}", t.name, t.desc_sw));
        if !t.params.is_empty() {
            let ps: Vec<String> = t
                .params
                .iter()
                .map(|p| {
                    if p.required {
                        format!("{}:{} (lazima)", p.name, p.ty)
                    } else {
                        format!("{}:{} (hiari)", p.name, p.ty)
                    }
                })
                .collect();
            out.push_str(&format!("  hoja: {}", ps.join(", ")));
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn majina_ya_zana_hayajirudii_na_yote_yanaelezwa() {
        let mut seen = std::collections::BTreeSet::new();
        for t in TOOLS {
            assert!(seen.insert(t.name), "zana '{}' imerudiwa", t.name);
            assert!(!t.desc_sw.trim().is_empty(), "{} haina maelezo", t.name);
            assert!(t.path.starts_with('/'), "{} ina path mbaya", t.name);
            assert!(t.method == "GET" || t.method == "POST");
            // params zote za lazima zina maelezo
            for p in t.params {
                assert!(!p.desc_sw.trim().is_empty());
                assert!(
                    p.ty == "string"
                        || p.ty == "number"
                        || p.ty == "boolean"
                        || p.ty == "array"
                        || p.ty == "object",
                    "{}: param '{}' ina aina si sahihi: {}",
                    t.name,
                    p.name,
                    p.ty
                );
            }
        }
        assert!(TOOLS.len() >= 15, "zana zinapaswa kuwa 15+");
    }

    #[test]
    fn resolve_inakataa_zana_isiyojulikana() {
        let e = resolve("hack_the_planet", &json!({})).unwrap_err();
        assert!(e.contains("haipo"));
        // inaonyesha zana zilizopo ili model ijirekebishe
        assert!(e.contains("disk_space"));
    }

    #[test]
    fn zana_za_uthibitisho_zipo_na_zinafanya_kazi() {
        // The verification toolset an agent needs to check its own work.
        for name in ["run_tests", "test_suites", "code_audit", "code_audit_snippet", "build_targets", "build_run", "e2e_run", "e2e_status", "selfcheck"] {
            assert!(find(name).is_some(), "zana ya uthibitisho '{name}' haipo");
        }
        // run_tests haina hoja ya lazima — unaweza kuitisha bila args.
        assert!(resolve("run_tests", &json!({})).is_ok());
        // code_audit inahitaji path.
        assert!(resolve("code_audit", &json!({})).is_err());
        assert!(resolve("code_audit", &json!({ "path": "src/main.rs" })).is_ok());
        // code_audit_snippet ni hiari kabisa (code na lang si za lazima).
        assert!(resolve("code_audit_snippet", &json!({})).is_ok());
        // build_run inahitaji platform.
        assert!(resolve("build_run", &json!({})).is_err());
        let c = resolve("build_run", &json!({ "platform": "linux" })).expect("build_run");
        assert_eq!(c.method, "POST");
        assert_eq!(c.path, "/api/ide/build/run");
    }

    #[test]
    fn resolve_inahitaji_hoja_za_lazima() {
        // read_data_file haina 'path'
        let e = resolve("read_data_file", &json!({})).unwrap_err();
        assert!(e.contains("path"), "error ilikuwa: {e}");
        // path tupu haikubaliki
        assert!(resolve("read_data_file", &json!({"path": "   "})).is_err());
        // web_search haina 'q'
        assert!(resolve("web_search", &json!({})).is_err());
        // hoja isiyojulikana
        let e = resolve("disk_space", &json!({"foo": 1})).unwrap_err();
        assert!(e.contains("haina hoja 'foo'"));
    }

    #[test]
    fn resolve_inatengeneza_query_salama() {
        let c = resolve("web_search", &json!({"q": "fundi mobile & tzs=bei"})).unwrap();
        assert_eq!(c.method, "GET");
        assert!(c.path.starts_with("/api/web/search?q="), "path: {}", c.path);
        // '&' na nafasi zimekodwa — hazivunji query
        assert!(c.path.contains("%26"), "& haikukodwa: {}", c.path);
        assert!(c.path.contains("%20") || c.path.contains('+'));
        assert!(!c.path.contains("tzs=bei"), "& ilivunja query: {}", c.path);
        assert!(c.body.is_none());
    }

    #[test]
    fn resolve_searxng_search_ina_path_sahihi() {
        let c = resolve("searxng_search", &json!({"q": "umeme breaker"})).unwrap();
        assert_eq!(c.method, "GET");
        assert!(c.path.starts_with("/api/web/searxng?q="), "path: {}", c.path);
        assert!(TOOLS.iter().any(|t| t.name == "searxng_search"), "zana ipo kwenye katalogi");
    }

    #[test]
    fn resolve_ya_post_inarudisha_body() {
        let c = resolve(
            "wake_computer",
            &json!({"mac": "aa:bb:cc:dd:ee:ff", "ip": "192.168.1.10"}),
        )
        .unwrap();
        assert_eq!(c.method, "POST");
        assert_eq!(c.path, "/api/wol");
        let b = c.body.unwrap();
        assert_eq!(b["mac"], "aa:bb:cc:dd:ee:ff");
        assert_eq!(b["ip"], "192.168.1.10");
    }

    #[test]
    fn swarm_v3_zana_zinatatuliwa() {
        // Safu ya pili ya maono inafikika kwa zana zenye majina (agent anaweza kuitumia).
        let cortex = resolve("warp_cortex_plan", &json!({"agents": 1000})).unwrap();
        assert_eq!(cortex.path, "/api/swarm/v3/cortex");
        assert_eq!(cortex.body.unwrap()["agents"], 1000);

        let orth = resolve("orthogonal_insight", &json!({"task": "punguza nishati"})).unwrap();
        assert_eq!(orth.path, "/api/swarm/v3/orthogonal");

        let flow = resolve("megaflow_plan", &json!({"tasks": 10000})).unwrap();
        assert_eq!(flow.path, "/api/swarm/v3/flow");

        let st = resolve("swarm_v3_status", &json!({})).unwrap();
        assert_eq!(st.method, "GET");
        assert_eq!(st.path, "/api/swarm/v3/status");

        // memory_recall ni GET yenye tier ya hiari
        let mem = resolve("memory_recall", &json!({"q": "umeme", "tier": "semantic"})).unwrap();
        assert!(mem.path.starts_with("/api/swarm/v3/memory/recall?q=umeme"));
        assert!(mem.path.contains("tier=semantic"));
    }

    #[test]
    fn openai_tools_ina_schema_sahihi() {
        let tools = openai_tools();
        let arr = tools.as_array().unwrap();
        assert_eq!(arr.len(), TOOLS.len());
        for t in arr {
            assert_eq!(t["type"], "function");
            assert!(t["function"]["name"].is_string());
            assert_eq!(t["function"]["parameters"]["type"], "object");
        }
        // read_data_file ina 'path' kama required
        let rd = arr
            .iter()
            .find(|t| t["function"]["name"] == "read_data_file")
            .unwrap();
        let req = rd["function"]["parameters"]["required"].as_array().unwrap();
        assert!(req.iter().any(|v| v == "path"));
    }

    #[test]
    fn menu_unataja_kila_zana() {
        let menu = tool_menu_sw();
        for t in TOOLS {
            assert!(menu.contains(t.name), "menu haina {}", t.name);
        }
        // lazima zimewekwa alama
        assert!(menu.contains("(lazima)"));
    }
}
