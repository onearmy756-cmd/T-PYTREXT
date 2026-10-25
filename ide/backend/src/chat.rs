//! chat.rs — CHAT YA AGENT (SEHEMU 3 Hatua 5): andika tatizo/swali → agent inajibu.
//!
//! Mtiririko (hakuna uongo):
//!   1. Offline: recall kutoka Neuralis Brain (kumbukumbu za agents) + jawabu la rules
//!   2. Online: LLM halisi (Ollama/llama.cpp — Qwen 2.5 3B VL) + muktadha wa brain
//!   3. Kila jawabu lina confidence (chini offline, juu online na LLM)
//!
//! LLM HAIHESABU kamwe — inaeleza/onasih; solutions zinatoka brain/knowledge.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ChatReply {
    pub question: String,
    pub answer_sw: String,
    #[serde(rename = "answer")]
    pub answer_translated: String, // AI auto-translate kwa lugha ya mtumiaji
    pub language: String,        // sw | en | fr | …
    pub source: String,          // brain | llm | offline_rules | fundi_agent
    pub confidence: f64,         // 0.0–1.0
    pub references: Vec<String>, // solutions za agents wenzake
    #[serde(default)]
    pub agent_session_id: Option<String>,
    #[serde(default)]
    pub agent_trace_id: Option<String>,
    #[serde(default)]
    pub agent_used: bool,
}

/// KANUNI YA SIRI YA BIASHARA: LLM au jawabu lolote la mfumo HAITAJI kamwe
/// jina la zana, amri, lugha ya programu wala injini — mteja anaona HUDUMA za
/// MTECH OS tu. Neno hili huwekwa kwenye kila ombi la LLM.
const LLM_SECRECY_RULE: &str = "KANUNI ZA JIBU: Usa jina la zana, amri, programu, lugha au injini yoyote inayotumika ndani. Mteja anaona HUDUMA za MTECH OS tu (mf. kichanganuzi cha mtandao, uchunguzi wa kidijitali). Usitaje binaries wala paths.";

/// PERSONALITY ya agent: professional agentic + security engineer (HITL daima).
const AUTONOMOUS_RULES: &str = "\n\nKANUNI ZA UENDSHAJI HALISI (AUTONOMOUS):\n1) USIULIZE mtumiaji maswali ya urefu — endesha kazi kwa zana ulizopewa. Kila swali ni ucheleweshaji.\n2) Ukikosa taarifa MOJA ndogo (mf. lengo la scan), CHAGUA default salama na uendelee; eleza chaguo lako kwenye jibu.\n3) MFULULIZO: panga → tekeleza ZANA → kagua matokeo → rekebisha → endelea; usisimize kabla ya kazi kumekamilika.\n4) HITL inabaki tu kwa AZMA ZA HATARI (exploit dhidi ya mwenyeji hana idhini, format/delete ya data, reboot). Ndio hao pekee uliza.\n5) Kila hatua eleza KISWAHILI FUPI + matokeo halisi (si makadirio).\n";

const AGENT_PERSONA: &str = "UNAWEZA KUWA MTAALAMU WA USALAMA (professional agentic + security engineer): Wakati swali litahitaji hali ya mfumo, usalama, log, diski, mtandao, au kazi ya kidijitali — fanya kitendo (tool calls)CHANZA: 1) panga plan fupi; 2) CHAGUA zana sahihi kutoka orodha (mifano: server_health = afya; disk_space = diski; security_run = kazi za usalama za Kali kama t10-scan, vuln-nikto, fore-fls, clam-scan; threatswarm_engage = kill-chain AUTONOMOUS kwa lengo moja; threatswarm_agent = endesha agent MOJA ya 27 za ThreatSwarm (recon, exploit, web-attacker, dfir, blue-team n.k.) na kazi zote wake halisi mfululizo; threatswarm_agents = orodha ya agents + kazi zao); 3) TEKELEZA moja au zaidi kwa mfululizo; 4) JIBU kwa matokeo halisi + uchambuzi wa mtaalamu + hatua zifuatazo. Kwa kazi za CSI (kutafuta dalili), sani insidens, vuln scans au forensic imaging — endesha zana halisi (security_run au threatswarm_engage), usibuni majitu. Heshimu HITL: kazi za hatari (exploit, password crack, reboot) zinahitaji idhini ya mmiliki — uliza kama haijulikani; lengo la threatswarm lazima liwe kwenye scope.txt (scope gate haibadiliki). UWEZO WA UMILIKI: tools 60+ (server/agent), security pack 57 kazi HALISI (Kali tools 2000+ — endesha VOZI mfululizo kupitia security_run ama MASS kwenye targets nyingi kupitia security_mass_scan; kazi zote 57 zinapatiwa kwenye security_groups) + ThreatSwarm kill-chain AUTONOMOUS na agents 27 kupitia threatswarm_agent (kila agent ina kazi zake halisi), R engine security analytics (risk_score = CVSS severity kwenye findings; anomaly = z-score kwenye trafiki; sec_forecast = trend ya incidents), math/quantum engines, toolkit, file read, terminal, browser. Jibu LINAELEZA: chanzo cha hatua + matendo + matokeo + upya wa kutenda.";

/// Urefu wa jawabu (maelezo ya mmiliki: "ai isummarize kwa maelezo marefu na ya kati")
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Length {
    Short,
    Medium,
    Long,
}

impl Length {
    pub fn instruction_sw(&self) -> &'static str {
        match self {
            Length::Short => "Jibu kwa Kiswahili kwa UFUPI (sentensi 1-2).",
            Length::Medium => "Jibu kwa Kiswahili kwa MAELEZO YA KATI (aya 1-2, hatua muhimu).",
            Length::Long => "Jibu kwa Kiswahili kwa MAELEZO MAREFU YAKAMILI: aya kadhaa, sababu, hatua zote, na ushauri wa kina.",
        }
    }
}

/// Swali la LLM halisi — LLM inaishi kwenye SERVER KUU YA LAN (docker),
/// kwa hiyo inafanya kazi hata OFFLINE (mtandao wa ndani unaotosha).
/// Online ni kwa cloud AI PEKEE — LLM ya LAN haitegemei internet.
/// Matukio ya mtiririko wa agent (SSE kwa UI): status, tool, tool_done, done.
#[derive(serde::Serialize, Clone, Debug)]
pub struct ChatEvent {
    pub kind: String,
    pub text: String,
    pub detail: String,
    /// Hoja HALISI za zana (path, content, command …) kama LLM ilizotoa.
    /// Canvas huzitumia kuonyesha kile kitendo kilichofanyika (faili gani,
    /// amri ipi) — si lebo ya URL pekee.
    pub args: serde_json::Value,
}

impl ChatEvent {
    pub fn status(text: &str) -> Self {
        ChatEvent {
            kind: "status".into(),
            text: text.into(),
            detail: String::new(),
            args: serde_json::Value::Null,
        }
    }
    pub fn tool(text: &str, detail: &str) -> Self {
        ChatEvent {
            kind: "tool".into(),
            text: text.into(),
            detail: detail.into(),
            args: serde_json::Value::Null,
        }
    }
    /// Kitendo chenye hoja kamili — hutumika kuonyesha zana kwenye Canvas.
    pub fn tool_call(text: &str, detail: &str, args: serde_json::Value) -> Self {
        ChatEvent {
            kind: "tool".into(),
            text: text.into(),
            detail: detail.into(),
            args,
        }
    }
    pub fn tool_done(text: &str, detail: &str) -> Self {
        ChatEvent {
            kind: "tool_done".into(),
            text: text.into(),
            detail: detail.into(),
            args: serde_json::Value::Null,
        }
    }
    pub fn done(text: &str) -> Self {
        ChatEvent {
            kind: "done".into(),
            text: text.into(),
            detail: String::new(),
            args: serde_json::Value::Null,
        }
    }
    /// Kipande cha jibu kinachotiririka (token-by-token) — UI huchora mara moja.
    pub fn delta(text: &str) -> Self {
        ChatEvent {
            kind: "delta".into(),
            text: text.into(),
            detail: String::new(),
            args: serde_json::Value::Null,
        }
    }
}

fn send_ev(tx: &Option<&tokio::sync::mpsc::Sender<ChatEvent>>, ev: ChatEvent) {
    if let Some(tx) = tx {
        let _ = tx.try_send(ev);
    }
}

/// Kigezo cha LLM kwa mzunguko wa agent (LAN au cloud — injini moja).
struct Endpoint {
    base: String,
    model: String,
    bearer: Option<String>,
    style: &'static str,
}

/// Cloud endpoint (Ollama Cloud / OpenAI-compatible) kutoka env:
///   FUNDI_CLOUD_URL   default https://ollama.com/v1
///   FUNDI_CLOUD_MODEL default gpt-oss:20b (haraka + bure)
fn cloud_endpoint() -> Option<Endpoint> {
    let key = crate::ai_config::ai_key()?;
    let base = std::env::var("FUNDI_CLOUD_URL")
        .unwrap_or_else(|_| "https://ollama.com/v1".into());
    let model = std::env::var("FUNDI_CLOUD_MODEL").unwrap_or_else(|_| "gemma4:31b".into());
    Some(Endpoint {
        style: "openai",
        base: base.trim_end_matches('/').to_string(),
        model,
        bearer: Some(key),
    })
}

fn lan_endpoint(cfg: &crate::ai_config::LlmConfig) -> Endpoint {
    let base = cfg.url.trim_end_matches('/').to_string();
    Endpoint {
        style: crate::ai_config::style_for_url(&base),
        base,
        model: cfg.model.clone(),
        bearer: crate::ai_config::ai_key(),
    }
}

/// Endpoint ya injini YETU (fundi-engine) — inference ya ndani, GGUF moja kwa
/// moja, bila vLLM/Python. `None` kama hakuna modeli iliyopakiwa.
/// Hii ni lane ya KWANZA (haraka, offline) kabla ya LAN/cloud.
fn local_engine_ready() -> bool {
    crate::engine_api::global().is_loaded()
}

/// Soma mtiririko wa SSE wa LLM: rudisha (maandishi yote, tool_calls).
/// Huiatuma kila kipande cha maandishi kwa UI mara moja (`delta`) — KASI YA UMEME:
/// mtumiaji huona maneno yakitokea badala ya kusubiri jibu lote.
async fn stream_answer(
    req: reqwest::RequestBuilder,
    style: &str,
    tx: Option<&tokio::sync::mpsc::Sender<ChatEvent>>,
) -> Option<(String, Vec<ToolCall>)> {
    use tokio_stream::StreamExt;
    let resp = match req.timeout(std::time::Duration::from_secs(180)).send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("LLM request imeshindikana: {e}");
            return None;
        }
    };
    if !resp.status().is_success() {
        let st = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::warn!("LLM HTTP {st}: {}", body.chars().take(300).collect::<String>());
        return None;
    }
    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut out = String::new();
    // Accumulate tool_calls kwa index (OpenAI streaming humgawanya vipande).
    let mut tcs: std::collections::BTreeMap<usize, (Option<String>, String, String)> =
        std::collections::BTreeMap::new();
    while let Some(chunk) = stream.next().await {
        let Ok(bytes) = chunk else { break };
        buf.push_str(&String::from_utf8_lossy(&bytes));
        loop {
            let Some(pos) = buf.find('\n') else { break };
            let line = buf[..pos].trim().to_string();
            buf.drain(..=pos);
            if line.is_empty() {
                continue;
            }
            let payload = if let Some(rest) = line.strip_prefix("data:") {
                rest.trim()
            } else if style != "openai" {
                line.trim()
            } else {
                continue;
            };
            if payload.is_empty() || payload == "[DONE]" {
                continue;
            }
            let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) else {
                continue;
            };
            if style == "openai" {
                let d = &v["choices"][0]["delta"];
                if let Some(c) = d["content"].as_str() {
                    if !c.is_empty() {
                        out.push_str(c);
                        send_ev(&tx, ChatEvent::delta(c));
                    }
                }
                if let Some(arr) = d["tool_calls"].as_array() {
                    for t in arr {
                        let id = t["id"].as_str();
                        // Baadhi ya providers (mf. MiniMax) HAWATUMI `index`.
                        // Bila hii, zana zote huanguka kwenye slot 0 na
                        // majina/hoja huchanganywa ("list_filesrun_tests", args `{}`).
                        // `id` mpya tofauti → slot mpya.
                        let mut idx = t["index"].as_u64().unwrap_or(0) as usize;
                        if let Some(id) = id {
                            if let Some((Some(existing), _, _)) = tcs.get(&idx) {
                                if existing != id {
                                    idx = tcs.keys().max().copied().unwrap_or(0) + 1;
                                }
                            }
                        }
                        let e = tcs.entry(idx).or_default();
                        if let Some(id) = id {
                            e.0 = Some(id.to_string());
                        }
                        if let Some(n) = t["function"]["name"].as_str() {
                            e.1.push_str(n);
                        }
                        if let Some(a) = t["function"]["arguments"].as_str() {
                            e.2.push_str(a);
                        }
                    }
                }
            } else {
                // ollama /api/chat (default ya LAN): message.content + message.tool_calls.
                // /api/generate hutumia response — tunashughulikia zote mbili.
                let m = &v["message"];
                let piece = m["content"].as_str().or_else(|| v["response"].as_str());
                if let Some(c) = piece {
                    if !c.is_empty() {
                        out.push_str(c);
                        send_ev(&tx, ChatEvent::delta(c));
                    }
                }
                if let Some(arr) = m["tool_calls"].as_array() {
                    for (idx, t) in arr.iter().enumerate() {
                        let e = tcs.entry(idx).or_default();
                        if let Some(n) = t["function"]["name"].as_str() {
                            e.1.push_str(n);
                        }
                        if let Some(a) = t["function"]["arguments"].as_str() {
                            e.2.push_str(a);
                        }
                    }
                }
            }
        }
    }
    let calls: Vec<ToolCall> = tcs
        .into_values()
        .filter(|(_, n, _)| !n.is_empty())
        .map(|(id, name, args)| {
            let a = serde_json::from_str::<serde_json::Value>(&args).unwrap_or_else(|_| serde_json::json!({}));
            ToolCall { id, name, args: a }
        })
        .collect();
    Some((out, calls))
}

/// MZUNGUKO WA AGENT (injini moja kwa LAN na cloud):
///   ombi → chagua zana → tekeleza kweli (/api/...) → angalia matokeo → jibu.
/// Jibu la mwisho linatiririka (streaming) ili mtumiaji aone maneno papo hapo.
async fn run_agent_loop(
    ep: &Endpoint,
    question: &str,
    context: &str,
    len: Length,
    tx: Option<&tokio::sync::mpsc::Sender<ChatEvent>>,
) -> Option<String> {
    let sys = agent_system_prompt(context, len);
    let client = reqwest::Client::new();
    let tools = api_tool();
    let mut messages: Vec<serde_json::Value> = vec![
        serde_json::json!({ "role": "system", "content": sys }),
        serde_json::json!({ "role": "user", "content": question }),
    ];
    let mut last = String::new();
    let mut used_tool = false;
    let mut got_response = false;
    let mut evidence: Vec<String> = Vec::new();
    let mut seen_calls: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    // Kizazi cha workspace: huongezwa kila `write_file` inapofanikiwa. Zana
    // zilizorudiwa ZINARUHUSIWA tena baada ya mabadiliko — la sivyo `run_tests`
    // huzuiwa kuthibitisha marekebisho (kizuizi cha "imeendeshwa mara moja"
    // kilikuwa kikizuia uthibitisho wa baada ya kurekebisha).
    let mut gen: u64 = 0;
    // Self-verification: once code is written, nudge exactly once to verify it
    // with a real tool before claiming success. This is what separates an agent
    // that checks from one that guesses.
    let mut wrote_code = false;
    let mut verified = false;
    let mut nudged = false;
    // AUTONOMOUS: agent inafanya kazi mpaka mwisho — hakuna kuuliza mtumiaji.
    // ROUNDS 12 = mfululizo wa tool calls mrefu (plan → execute → verify → fix)
    // bila kuairisha mtumiaji. HITL (ruhusa) inabaki KWA AZMA ZA HATARI PEKEE
    // (exploit/format/delete — mfumo wa approval wa tools unawashughulikia).
    const MAX_ROUNDS: usize = 12;
    for round in 0..MAX_ROUNDS {
        let is_last = round + 1 == MAX_ROUNDS;
        // ROUND ya MWISHO: hakuna zana + streaming ili jibu lionekane papo hapo.
        if is_last {
            send_ev(&tx, ChatEvent::status("Ninaandika jibu..."));
            // LAZIMA: toa jibu la maandishi sasa — usiitisha zana zaidi.
            messages.push(serde_json::json!({ "role": "user", "content": "Toa JIBU LA MWISHO sasa kwa Kiswahili, kwa maandishi safi, ukitumia matokeo HALISI ya zana ulizopata hapo juu. USIITICHE zana tena. Jumuisha: (1) muhtasari, (2) data halisi uliyoipata, (3) uchambuzi, (4) hatua zinazofuata. Usibuni kitu." }));
            let (url, mut body) = chat_req(ep.style, &ep.base, &ep.model, &messages, false, &tools);
            body["stream"] = serde_json::json!(true);
            let mut req = client.post(&url).json(&body);
            if let Some(k) = &ep.bearer {
                req = req.bearer_auth(k);
            }
            if let Some((text, _)) = stream_answer(req, ep.style, tx).await {
                got_response = true;
                if !text.trim().is_empty() {
                    last = text;
                }
            }
            break;
        }
        // ROUND 0: streaming + zana (ili jibu lisilo na zana lionekane papo hapo).
        send_ev(
            &tx,
            ChatEvent::status(if round == 0 {
                "Ninaiga modeli ya AI..."
            } else {
                "Ninaendelea kufikiria..."
            }),
        );
        let req_builder = {
            let (url, mut b) = chat_req(ep.style, &ep.base, &ep.model, &messages, true, &tools);
            // ROUND ZOTE hutiririsha — hata round inayotoa jibu la mwisho baada ya
            // zana, ili mtumiaji aone maandishi yakitokea (sio kusubiri kimya).
            b["stream"] = serde_json::json!(true);
            let mut req = client.post(&url).json(&b);
            if let Some(k) = &ep.bearer {
                req = req.bearer_auth(k);
            }
            req
        };
        let (content, calls) = match stream_answer(req_builder, ep.style, tx).await {
            Some(x) => {
                got_response = true;
                x
            }
            None => break,
        };
        if !content.trim().is_empty() {
            last = content.clone();
        }
        if calls.is_empty() {
            // Code was written but never verified with a real tool: ask once,
            // then let the model answer regardless (never loop forever).
            if wrote_code && !verified && !nudged {
                nudged = true;
                messages.push(serde_json::json!({ "role": "user", "content": "Umekamilisha kuandika code. Kabla ya kutoa jibu la mwisho, THIBITISHA kwa zana halisi: run_tests (kama mradi una testi), code_audit au code_audit_snippet (usalama/ubora), na build_run kama inakompila. Kisha toa jibu ukitumia MATOKEO HALISI ya uthibitisho (passed/failed, findings)." }));
                continue;
            }
            break; // jibu limefika (round 0: tayari lilitiririka)
        }
        used_tool = true;
        messages.push(assistant_msg(ep.style, &content, &calls));
        for tc in &calls {
            if tc.name == "write_file" {
                wrote_code = true;
            }
            if matches!(tc.name.as_str(), "run_tests" | "code_audit" | "code_audit_snippet" | "build_run" | "e2e_run" | "selfcheck") {
                verified = true;
            }
            let sig = format!("{}|{}", tc.name, tc.args);
            // Zana ile ile yenye hoja zile zile ni kurudia PEKEE kama workspace
            // haijabadilika tangu mara ya mwisho. Baada ya `write_file`, zana
            // kama `run_tests` LAZIMA ziendeshe tena ili kuthibitisha marekebisho.
            if seen_calls.get(&sig).is_some_and(|g| *g >= gen) {
                messages.push(tool_msg(ep.style, tc, "Zana hii imeendeshwa mara moja kwa hali hii ya workspace — tumia matokeo ya awali. ACHA kuitisha tena; toa jibu sasa kwa data uliyonayo."));
                continue;
            }
            seen_calls.insert(sig, gen);
            let resolved = crate::agent_tools::resolve(&tc.name, &tc.args).ok();
            let label = match &resolved {
                Some(c) => format!("{} {}", c.method, c.path),
                None => tc.name.clone(),
            };
            send_ev(
                &tx,
                ChatEvent::tool_call(&tc.name, &label, tc.args.clone()),
            );
            let t0 = std::time::Instant::now();
            let out = fundi_api_call(&client, tc).await;
            let ms = t0.elapsed().as_millis();
            // Mabadiliko ya workspace yamefanikiwa → zana za uthibitisho
            // (run_tests, build_run, code_audit) zinaweza kurudiwa.
            if tc.name == "write_file" && !out.contains("\"ok\":false") && !out.contains("Imeshindwa") {
                gen += 1;
            }
            if !out.contains("Imeshindwa")
                && !out.contains("KOSA LA ZANA")
                && !out.contains("\"ok\":false")
                && !out.starts_with("HTTP 5")
            {
                evidence.push(format!("【{}】\n{}", tc.name, clip(&out, 1500)));
            }
            let mark = if out.contains("\"ok\":false")
                || out.contains("\"ok\": false")
                || out.contains("Imeshindwa")
                || out.contains("KOSA LA ZANA")
                || out.starts_with("HTTP 5")
                || out.starts_with("HTTP 4")
            {
                "FAIL"
            } else if out.starts_with("HTTP 2") || out.starts_with("HTTP {") {
                "OK"
            } else {
                "?"
            };
            send_ev(
                &tx,
                ChatEvent::tool_done(&tc.name, &format!("{label} · {mark} {ms}ms")),
            );
            messages.push(tool_msg(ep.style, tc, &out));
        }
    }
    let _ = used_tool;
    if !got_response {
        // Hakuna jibu kutoka provider (LAN haipatikani n.k.) — acha caller ajaribu cloud.
        return None;
    }
    if last.trim().is_empty() {
        if !evidence.is_empty() {
            // Hakuna maandishi kutoka modeli, lakini tuna DATA HALISI ya zana.
            // Rudisha ushahidi — usibuni, usirudishe "sijui".
            return Some(format!(
                "Nimekusanya matokeo HALISI ya uchunguzi kutoka MTECH OS:\n\n{}\n\n(Uchambuzi wa modeli ulikwama; haya ni matokeo ghafi yaliyothibitishwa — si ya kubuni.)",
                evidence.join("\n\n")
            ));
        }
        return Some(
            "Nimejaribu hatua zilizowezekana kupitia MTECH OS lakini sikuipata data halisi. Jaribu tena — au uulize swali jingine.".into(),
        );
    }
    Some(plain_text(&last))
}

/// LAN LLM (Ollama/llama.cpp ya server kuu). Ikiwa online na LAN haipatikani,
/// hushuka kwa CLOUD (Ollama Cloud) — kwa mzunguko huo huo WA ZANA. Hii ndiyo
/// inayofanya agent itumie zana halisi za MTECH OS (security_task, threatswarm_*).
async fn llm_ask(
    db: &sqlx::SqlitePool,
    question: &str,
    context: &str,
    len: Length,
    tx: Option<&tokio::sync::mpsc::Sender<ChatEvent>>,
) -> Option<String> {
    // Injini YETU (fundi-engine) kwanza — inference ya ndani, offline, bila vLLM.
    if local_engine_ready() {
        send_ev(&tx, ChatEvent::status("Ninafikiri kwa injini ya MTECHOS (GGUF ya ndani)..."));
        let sys = agent_system_prompt(context, len);
        if let Some(ans) = crate::engine_api::ask_local(&sys, question, 512).await {
            if !ans.trim().is_empty() {
                return Some(ans);
            }
        }
    }
    // DARAJA LA KIBINADAMU: tabaka 12 za akili_v4 hufikiri kwanza, kisha
    // huamuru lugha (Qwen3.8-27B kwa llama-server) kujibu kwa hali ya ndani.
    // Hii ni lane ya KWANZA ikiwashwa (FUNDI_HUMAN_BRIDGE=1) — muundo wa
    // kibinadamu: fikira=akili_v4, lugha=Qwen3.8-27B.
    if crate::akili_human::bridge_enabled() && crate::akili_human::llm_alive().await {
        send_ev(&tx, ChatEvent::status("Ninafikiri kwa muundo wa kibinadamu (tabaka 12)…"));
        let (state, ans) = crate::akili_human::human_think(
            question,
            context,
            len.instruction_sw(),
            512,
            600,
        )
        .await;
        if let Some(a) = ans {
            if !a.trim().is_empty() {
                return Some(format!(
                    "{}\n\n[hali ya ndani: {}]",
                    a.trim(),
                    state["decision"]["action"].as_str().unwrap_or("")
                ));
            }
        }
    }
    let cfg = crate::ai_config::load_with_db(db).await;
    let mut lan = lan_endpoint(&cfg);
    // PROFILE ILIYOCHAGULIWA (UI): model/URL/key kutoka profile halisi.
    // Bila hii, uchaguzi wa mtumiaji (mf. deepseekcoder6b) haufiki injini —
    // hivyo model aliyochagua haitumiki na UI inaonekana "haifanyi kazi".
    let (p_model, p_base, p_key) = crate::agent_native::state::active_profile_llm();
    let profile_used = !p_model.is_empty();
    let profile_model = p_model.clone();
    if profile_used {
        if !p_base.is_empty() {
            lan.base = p_base.trim_end_matches('/').to_string();
            lan.style = crate::ai_config::style_for_url(&lan.base);
        }
        lan.model = p_model;
        if let Some(k) = p_key {
            lan.bearer = Some(k);
        }
    }
    // LAN kwanza (offline-capable). Kama ikifeli → cloud (ikiwa key ipo).
    if let Some(ans) = run_agent_loop(&lan, question, context, len, tx).await {
        if !ans.trim().is_empty() {
            return Some(ans);
        }
    } else if profile_used {
        // Model iliyochaguliwa haipatikani (mf. haipo kwenye plan ya API key).
        // Eleza WAZI badala ya kushuka kimya — mtumiaji asidhani AI "imeharibika".
        send_ev(
            &tx,
            ChatEvent::status(&format!(
                "Modeli \"{profile_model}\" haipatikani kwa API key hii — natumia model ya msingi…"
            )),
        );
        let fallback = lan_endpoint(&cfg);
        if let Some(ans) = run_agent_loop(&fallback, question, context, len, tx).await {
            if !ans.trim().is_empty() {
                return Some(ans);
            }
        }
    }
    if let Some(cloud) = cloud_endpoint() {
        send_ev(&tx, ChatEvent::status("Ninaunganisha AI ya wingu..."));
        if let Some(ans) = run_agent_loop(&cloud, question, context, len, tx).await {
            if !ans.trim().is_empty() {
                return Some(ans);
            }
        }
    }
    None
}

/// SYSTEM PROMPT ya CODER (FORGE). Tofauti na `agent_system_prompt` ya huduma
/// ambayo inakataza code-blocks KANUNI #1, coder HARUHUSIWI kutoa code-blocks —
/// ndiyo mkataba wa FORGE (`build_pipeline::extract_code`).
fn coder_system_prompt() -> &'static str {
    "Wewe ni CODER wa MTECHOS FORGE — 2NYI, SUPER SENIOR SOFTWARE ENGINEER (kama staff/principal engineer).\n\
     Unajua OS/systems, IoT/embedded, mobile, desktop, web/backend/data/devops. Kazi yako: kuandika KODI HALISI ya faili MOJA.\n\
     Kanuni:\n\
     1. Rejesha fenced block MOJA tu: ```<lugha> ... ``` yenye kodi KAMILI inayofanya kazi.\n\
     2. HAKUNA maelezo mengi nje ya block (sentensi moja ya muhtasari inaruhusiwa).\n\
     3. Usiweke hard-coded API keys/password; hakuna eval/innerHTML; Rust isitumie .unwrap().\n\
     4. Faili la usanidi (Cargo.toml, package.json, *.json, *.toml) liwe na maudhui KAMILI ndani ya block.\n\
     5. Andika kodi inayotekeleza MADHUMUNI ya faili kikamilifu — si mfano mtupu.\n\
     6. GROUNDING: usibuni API/kitenzi/kifurushi usichokithibitisha; kama hutajua, andika kile unachojua hakika.\n\
     7. Jibu kwa lugha inayolingana na ombi (Kiswahili au Kiingereza) ndani ya sentensi moja ya muhtasari."
}

/// Ombi MOJA la kodi kwa endpoint (bila zana, bila mzunguko).
async fn coder_ask(ep: &Endpoint, question: &str) -> Option<String> {
    let client = reqwest::Client::new();
    let messages = vec![
        serde_json::json!({ "role": "system", "content": coder_system_prompt() }),
        serde_json::json!({ "role": "user", "content": question }),
    ];
    let (url, body) = chat_req(ep.style, &ep.base, &ep.model, &messages, false, &serde_json::json!({}));
    let mut req = client.post(&url).json(&body);
    if let Some(k) = &ep.bearer {
        req = req.bearer_auth(k);
    }
    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: serde_json::Value = resp.json().await.ok()?;
    let (text, _) = extract_reply(ep.style, &v);
    Some(text)
}

/// CODER MODE (streaming): kama `code_ask` lakini hutiririsha `ChatEvent::delta`
/// kwa `tx` — mtumiaji huona kodi ikitokea token kwa token (SSE ya FORGE).
pub async fn code_ask_stream(
    brain: &crate::brain::Brain,
    question: &str,
    tx: &tokio::sync::mpsc::Sender<ChatEvent>,
) -> String {
    // Injini YETU (fundi-engine) kwanza kwa kuzalisha kodi (offline).
    if local_engine_ready() {
        if let Some(a) = crate::engine_api::ask_local(coder_system_prompt(), question, 1024).await {
            if !a.trim().is_empty() {
                return a;
            }
        }
    }
    let cfg = crate::ai_config::load_with_db(&brain.db).await;
    let lan = lan_endpoint(&cfg);
    if let Some(a) = coder_ask_stream(&lan, question, tx).await {
        if !a.trim().is_empty() {
            return a;
        }
    }
    if let Some(cloud) = cloud_endpoint() {
        if let Some(a) = coder_ask_stream(&cloud, question, tx).await {
            if !a.trim().is_empty() {
                return a;
            }
        }
    }
    String::new()
}

async fn coder_ask_stream(
    ep: &Endpoint,
    question: &str,
    tx: &tokio::sync::mpsc::Sender<ChatEvent>,
) -> Option<String> {
    let client = reqwest::Client::new();
    let messages = vec![
        serde_json::json!({ "role": "system", "content": coder_system_prompt() }),
        serde_json::json!({ "role": "user", "content": question }),
    ];
    let (url, mut body) = chat_req(ep.style, &ep.base, &ep.model, &messages, false, &serde_json::json!({}));
    body["stream"] = serde_json::json!(true);
    let mut req = client.post(&url).json(&body);
    if let Some(k) = &ep.bearer {
        req = req.bearer_auth(k);
    }
    let (text, _) = stream_answer(req, ep.style, Some(tx)).await?;
    Some(text)
}

/// CODER MODE: kwa FORGE/uzalishaji wa kodi (LAN kwanza, kisha cloud).
/// Hii ndiyo njia sahihi ya kuzalisha code-blocks — `ask()` haitumiwi kwa kodi.
pub async fn code_ask(brain: &crate::brain::Brain, question: &str) -> String {
    // Injini YETU (fundi-engine) kwanza kwa kuzalisha kodi (offline).
    if local_engine_ready() {
        if let Some(a) = crate::engine_api::ask_local(coder_system_prompt(), question, 1024).await {
            if !a.trim().is_empty() {
                return a;
            }
        }
    }
    let cfg = crate::ai_config::load_with_db(&brain.db).await;
    let lan = lan_endpoint(&cfg);
    if let Some(a) = coder_ask(&lan, question).await {
        if !a.trim().is_empty() {
            return a;
        }
    }
    if let Some(cloud) = cloud_endpoint() {
        if let Some(a) = coder_ask(&cloud, question).await {
            if !a.trim().is_empty() {
                return a;
            }
        }
    }
    String::new()
}

/// CLOUD LLM moja kwa moja (mode=chat: mazungumzo safi bila zana, streaming).
async fn cloud_llm_ask(
    question: &str,
    context: &str,
    len: Length,
    tx: Option<&tokio::sync::mpsc::Sender<ChatEvent>>,
) -> Option<String> {
    let ep = cloud_endpoint()?;
    let sys = agent_system_prompt(context, len);
    let url = format!("{}/chat/completions", ep.base);
    let body = serde_json::json!({
        "model": ep.model,
        "messages": [
            { "role": "system", "content": sys },
            { "role": "user", "content": question }
        ],
        "stream": true,
        "max_tokens": 8192
    });
    let client = reqwest::Client::new();
    let mut req = client.post(&url).json(&body);
    if let Some(k) = &ep.bearer {
        req = req.bearer_auth(k);
    }
    let (text, _) = stream_answer(req, ep.style, tx).await?;
    if text.trim().is_empty() {
        return None;
    }
    Some(text)
}

// ---------- AGENT: zana ya API + msaada wa mtiririko ----------

#[derive(Clone, Debug)]
struct ToolCall {
    id: Option<String>,
    name: String,
    args: serde_json::Value,
}

/// Kanuni za jibu (maandishi safi, agent, skills, makusudio ya mradi).
fn agent_system_prompt(context: &str, len: Length) -> String {
    format!(
        "Wewe ni {} — AGENT MBEBA KAZI wa MTECH OS, SUPER SENIOR SOFTWARE ENGINEER — SI chatbot ya kawaida. Kila ombi hushughulikiwa kwa mfumo: MPANGO → FANYA KITU HALISI → THIBITISHA → JIBU.\n\n\
         MAKUSUDIO YA MRADI: MTECH OS ni IDE ya coding + jukwaa la cyber-security na digital forensics: IDE ya coding agent (Monaco + MBILINYI Canvas), katalogi ya zana za usalama (1,663 katika makundi 59), discovery ya LAN na majina halisi ya computers, kusanidi OS, backup/restore, VPN/WireGuard, IPAM, AI ya ndani (model za local + Ollama Cloud), na data folder ya mradi (skills/rules/ai).\n\n\
         KANUNI ZA JIBU (lazima zifuatwe):\n\
         1. ZANA KWANZA, MAELEZO BAADAYE: kila ombi la kazi — andika/kusoma/kubadilisha faili, kukimbiza testi, kuchunguza mfumo, usalama — ANZA kwa wito wa zana (tool call). Kisha eleza ULIFANYA NINI kwa Kiswahili kwa kutumia MATOKEO HALISI. Msomaji anaona kadi za zana (faili, amri, matokeo) ZENYE maelezo yako — hivyo USIRUDIE kwenye maandishi yale ambayo kadi ya zana tayari inaonyesha; eleza MAANA na HITIMISHO la matokeo.\n\
         1b. Code halisi ya mradi LAZIMA ipite kwenye zana `write_file` (inakwenda workspace halisi, inaonekana Monaco, inadumu). Code-blocks za markdown ni kwa maelezo/mafano tu ndani ya jibu — si njia ya kuunda faili. Ukionyesha code-block, LAZIMA pia uwe umeandika faili kwa `write_file` kama kazi ni kuunda faili.\n\
         1c. Jibu lako la mwisho linaweza kuwa na markdown (vichwa, orodha, code-blocks za mafano) — jibu la wazi, lenye muundo, linalosomeka kwa mwanadamu. Violezo vya data (JSON pekee) au majina ya binary/path za ndani HAVIRUHUSIWI (white-label, kanuni 6).\n\
         2. Jibu REFU na YA KINA: (a) muhtasari wa ulichofanya, (b) matokeo HALISI uliyoyapata kutoka mfumo, (c) maelezo marefu — hatua zote, sababu, maelezo, (d) ushauri wa hatua zinazofuata. Usikope majibu mafupi ya jumla.\n\
         3. USISEME \"haielewi\" au \"siwezi\": fanya utafiti kwa zana fundi_api, au uliza swali MOJA tu la kufafanua.\n\
         4. USITANGIZE kazi kuwa \"imekamilika\" bila kupata matokeo HALISI kutoka mfumo — kila kauli ya ufanisi lazima ithibitishwe na data uliyopokea.\n\
         4b. UKWELI KAMWE (GROUNDING): kila namba, IP, jina, tarehe, SPF/SSL, port au hitimisho LAZIMA litoke MOJA KWA MOJA kwenye matokeo ya zana uliyopokea. USIBUNI wala kukisia. Ikiwa hukupata data halisi, sema wazi \"sikuipata\" — usibuni (mf. usibuni IP kama 192.0.2.x, usibuni tarehe za SSL).\n\
         4c. KUTHIBITISHA KABLA YA KUDAI KUKAMILIKA (self-verification): baada ya KUANDIKA au KUBADILISHA code (write_file), LAZIMA uitushe zana ya uthibitisho kabla ya kusema kazi imekamilika — run_tests (cargo/npm/go/mix/Rscript), code_audit au code_audit_snippet (usalama/ubora), build_run (inakompila?), e2e_run (UI inafanya kazi?). Ripoti matokeo HALISI ya uthibitisho (passed/failed, findings) — si maoni. Hii ndiyo tofauti kati ya agent anayedhani na agent anayethibitisha.\n\
         5. Usipendekeze amri hatari (kufuta faili/kumbukumbu/log, disk formatting) wala amri za Linux patupu kama PC ni ya Windows — tatua kupitia huduma za MTECH OS.\n\
         5b. UENDESHAJI WA KIAUTOMATIKI (AUTONOMOUS): USIULIZE mtumiaji maswali ya mfululizo. Endesha kazi moja kwa moja kwa zana; ukikosa taarifa ndogo, CHAGUA default salama, uendelee, na eleze chaguo lako kwenye jibu. Kazi ya usalama/cyber ops: anzisha threatswarm_engage / security_task MOJA KWA MOJA kwa lengo lililo kutoka kwa mtumiaji (ikiwa ndani ya scope.txt); HITL ya idhini umulize KWA AWALI tu kwa azma za hatari kabisa (exploit ya kovu, password crack, reboot/format) — kwa nyinginezo FANYA HALISI bila kusubiri.\n\
         6. WHITE-LABEL: usitaje binaries, paths, wala jina la programu/injini yoyote ndani — mteja anaona HUDUMA za MTECH OS tu.\n\
         7. {}\n\n\
         ZANA ZENYE MAJINA (hizi PEKEE zipo — hakuna path ya kubuni):\n\
         {tools}\n\n\
         JINSI YA KUTUMIA ZANA (utaratibu wa agent halisi):\n\
         1. Chagua zana kwa JINA lake kamili na hoja zake halisi.\n\
         2. Subiri matokeo HALISI — ndiyo uthibitisho wako pekee.\n\
         3. Matokeo yakiwa tupu au hayatoshi, tumia zana NYINGINE (mf. disk_space ikirudisha disks tupu → tumia server_health, kisha toa hitimisho la kile kilichopatikana). USIKATE TAMAA baada ya zana moja.\n\
         4. Kila kauli ya ufanisi LAZIMA iwe na data uliyoipata; usibuni namba wala majina.\n\n\
         MIFUMO MAALUM (fanya KWANZA, kisha jibu kwa data halisi):\n\
         - \"Disk imejaa / nafasi\" → disk_space\n\
         - \"Amsha / wake PC\" → wake_computer (hoja: mac; ip ni hiari)\n\
         - \"Simu imeunganishwa USB?\" → usb_phones\n\
         - \"Tafuta <kitu> mtandaoni\" → web_search (hoja: q)\n\
         - \"Tafuta kwa SearXNG / meta-search\" → searxng_search (hoja: q)\n\
         - \"Nani yuko kwenye mtandao?\" → list_computers au lan_scan\n\
         - \"Hali ya mfumo / huduma?\" → server_health, llm_status\n\
         - Cyber (\"tathmini usalama <target>\") → security_groups, kisha security_task; ripoti ni ya kweli tu\n\
         - \"Ichunguze/security <domain|target>\" (WHOIS/DNS/SSL/ports) → security_task kwa kazi za info-*: info-whois (mmiliki), info-dns, info-ssl, t10-scan; kisha ripoti kwa data HALISI pekee\n\
         - ThreatSwarm (kill-chain/agent) → threatswarm_agents (orodha), kisha threatswarm_agent au threatswarm_engage (lengo lazima liwe scope.txt)\n\
         - \"Andika/Jenga code\" → list_files, kisha write_file, KISHA LAZIMA selfcheck + run_tests + code_audit (thibitisha), na build_run kama inakompila; ripoti matokeo halisi\n\
         - \"Thibitisha/Hakikisha inafanya kazi\" → selfcheck, run_tests, code_audit, build_run, e2e_run\n\n\
         UJUZI WA MFUMO (skills — zote tayari, hakuna training):\n{}\n\n\
         Muktadha (kumbukumbu za suluhisho za awali):\n{context}",
        crate::nyi::PERSONA,
        len.instruction_sw(),
        skills_block(),
        tools = crate::agent_tools::tool_menu_sw(),
    )
}
/// Orodha halisi ya skills za IDE/cyber kutoka `data/skills/` (engineering +
/// imported: OpenHands, Claude Code, Trae).
fn skills_block() -> String {
    use std::collections::BTreeMap;
    let list = crate::code_skills::list();
    let mut by: BTreeMap<String, (usize, Vec<String>)> = BTreeMap::new();
    let mut total = 0usize;
    if let Some(arr) = list["skills"].as_array() {
        for s in arr {
            total += 1;
            let group = s["source"].as_str().unwrap_or("engineering").to_string();
            let name = s["name"]
                .as_str()
                .filter(|n| !n.is_empty())
                .or_else(|| s["file"].as_str())
                .unwrap_or("")
                .to_string();
            if name.is_empty() {
                continue;
            }
            let e = by.entry(group).or_default();
            e.0 += 1;
            if e.1.len() < 6 {
                e.1.push(name);
            }
        }
    }
    let mut out = format!("Jumla ya skills: {total}.");
    for (k, (n, names)) in by {
        out.push_str(&format!("\n- {k} ({n}): {}", names.join(", ")));
    }
    out
}

/// ZANA ZENYE MAJINA: schema inazalishwa kutoka `agent_tools::{TOOLS}` —
/// model inaona jina, maelezo na aina ya hoja za kila zana (kama agent halisi).
fn api_tool() -> serde_json::Value {
    crate::agent_tools::openai_tools()
}

/// Base ya wito wa ndani (server inajipigia simu yenyewe).
/// PORT inatumika badala ya 8080 iliyokwama — hivyo hufanya kazi kwenye
/// Fly (`PORT=8080`), kwenye dev ya ndani, au kwenye container yoyote.
fn self_base() -> String {
    std::env::var("FUNDI_SELF_URL").unwrap_or_else(|_| {
        let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
        format!("http://127.0.0.1:{port}")
    })
}

fn chat_req(
    style: &str,
    base: &str,
    model: &str,
    messages: &[serde_json::Value],
    with_tools: bool,
    tools: &serde_json::Value,
) -> (String, serde_json::Value) {
    let mut body = serde_json::json!({ "model": model, "messages": messages, "stream": false, "max_tokens": 8192 });
    if with_tools {
        body["tools"] = tools.clone();
    }
    let url = if style == "openai" {
        format!("{}/chat/completions", crate::ai_config::openai_base(base))
    } else {
        format!("{base}/api/chat")
    };
    (url, body)
}

fn extract_reply(style: &str, v: &serde_json::Value) -> (String, Vec<ToolCall>) {
    let msg = if style == "openai" {
        &v["choices"][0]["message"]
    } else {
        &v["message"]
    };
    let content = msg["content"].as_str().unwrap_or("").to_string();
    let mut calls = Vec::new();
    if let Some(arr) = msg["tool_calls"].as_array() {
        for tc in arr {
            let f = &tc["function"];
            let name = f["name"].as_str().unwrap_or("").to_string();
            let mut args = f["arguments"].clone();
            if let Some(s) = args.as_str() {
                args = serde_json::from_str(s).unwrap_or_else(|_| serde_json::json!({}));
            }
            if !name.is_empty() {
                calls.push(ToolCall {
                    id: tc["id"].as_str().map(String::from),
                    name,
                    args,
                });
            }
        }
    }
    (content, calls)
}

fn assistant_msg(style: &str, content: &str, calls: &[ToolCall]) -> serde_json::Value {
    let mut m = serde_json::json!({ "role": "assistant", "content": content });
    if !calls.is_empty() {
        let arr: Vec<serde_json::Value> = calls
            .iter()
            .map(|c| {
                if style == "openai" {
                    serde_json::json!({
                        "id": c.id.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
                        "type": "function",
                        "function": {
                            "name": c.name,
                            "arguments": serde_json::to_string(&c.args).unwrap_or_else(|_| "{}".into())
                        }
                    })
                } else {
                    serde_json::json!({ "function": { "name": c.name, "arguments": c.args } })
                }
            })
            .collect();
        m["tool_calls"] = serde_json::Value::Array(arr);
    }
    m
}

fn tool_msg(style: &str, tc: &ToolCall, out: &str) -> serde_json::Value {
    if style == "openai" {
        serde_json::json!({ "role": "tool", "tool_call_id": tc.id.clone().unwrap_or_default(), "content": out })
    } else {
        serde_json::json!({ "role": "tool", "content": out })
    }
}

/// Session ya Admin Mkuu kwa fundi_api (RBAC) — imewekwa na main() wakati wa kuanza.
static AGENT_TOKEN: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Hali ya ULAZIMA (per-request): Some(true) = AGENT (tenda), Some(false) = CHAT
/// (mazungumzo safi, hakuna tool calls). None = otomatiki (tabia ya zamani).
static MODE_AGENT: std::sync::atomic::AtomicI8 = std::sync::atomic::AtomicI8::new(0);

pub fn set_mode_agent(v: Option<bool>) {
    use std::sync::atomic::Ordering;
    MODE_AGENT.store(match v { Some(true) => 1, Some(false) => -1, None => 0 }, Ordering::Relaxed);
}
fn mode_agent() -> Option<bool> {
    use std::sync::atomic::Ordering;
    match MODE_AGENT.load(Ordering::Relaxed) { 1 => Some(true), -1 => Some(false), _ => None }
}

/// main() huiita mara moja baada ya auth::init_tables.
pub fn set_agent_token(t: String) {
    let _ = AGENT_TOKEN.set(t);
}

/// Token ya Admin Mkuu (kwa wito wa ndani wa API — RBAC).
pub fn agent_token() -> Option<String> {
    AGENT_TOKEN.get().filter(|t| !t.is_empty()).cloned()
}

/// Base ya API ya mfumo huu (ndani).
pub fn self_base_pub() -> String {
    self_base()
}

/// Tekeleza zana: TATUA kutoka `agent_tools` (orodha pekee inayoruhusiwa) kisha
/// ita API HALISI ya MTECH OS kwenye server hii — kurudisha data ya kweli.
///
/// Tofauti na `fundi_api` ya kale (path huru), model hapa HAIWEZI kubuni path:
/// ikijaribu zana isiyopo au hoja isiyopo, inapata kosa linaloeleza zana/hoja
/// zilizopo — hivyo inajirekebisha yenyewe kwenye raundi inayofuata.
async fn fundi_api_call(client: &reqwest::Client, tc: &ToolCall) -> String {
    let call = match crate::agent_tools::resolve(&tc.name, &tc.args) {
        Ok(c) => c,
        Err(e) => return format!("KOSA LA ZANA: {e}"),
    };
    // Ulinzi wa ziada (defense in depth): hata zana zilizosajiliwa haziwezi
    // kufikia sehemu nyeti (key, portal, admin, malipo, setup).
    const BLOCKED: &[&str] = &[
        "/api/ai/key",
        "/api/portal/",
        "/api/admin/",
        "/api/clickpesa/",
        "/api/setup/",
    ];
    if BLOCKED.iter().any(|b| call.path.contains(b)) {
        return "Kipengele hiki kimefungwa kwa usalama (kinahitaji idhini ya admin).".into();
    }
    let path = call.path.clone();
    let method = call.method.clone();
    let url = format!("{}{path}", self_base());
    // RBAC: agent anatumia session yake ya Admin Mkuu (header hapo chini).
    let m: reqwest::Method = method.parse().unwrap_or(reqwest::Method::GET);
    let mut rb = client
        .request(m, url.as_str())
        .timeout(std::time::Duration::from_secs(150));
    if let Some(t) = AGENT_TOKEN.get().filter(|t| !t.is_empty()) {
        rb = rb.header("Authorization", format!("Bearer {t}"));
    }
    if method == "POST" {
        if let Some(b) = &call.body {
            rb = rb.json(b);
        }
    }
    match rb.send().await {
        Ok(r) => {
            let st = r.status().as_u16();
            let txt = r.text().await.unwrap_or_default();
            let compact = match serde_json::from_str::<serde_json::Value>(&txt) {
                Ok(v) => v.to_string(),
                Err(_) => txt,
            };
            format!("HTTP {st}\n{}", clip(&compact, 3000))
        }
        Err(e) => format!("Imeshindwa kuitisha zana '{}': {e}", tc.name),
    }
}

fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_string();
    }
    let cut: String = s.chars().take(n).collect();
    format!("{cut}\n... (imekatwa — jibu limefuata {n} herufi)")
}

/// Fallback ya zamani: /api/generate (bila zana) — bado hujibu kwa maandishi safi.
async fn generate_fallback(
    client: &reqwest::Client,
    base: &str,
    model: &str,
    sys: &str,
    question: &str,
) -> Option<String> {
    let body = serde_json::json!({
        "model": model,
        "prompt": format!("{sys}\n\nSwali: {question}\n\nJibu:"),
        "stream": false,
    });
    let mut req = client
        .post(format!("{base}/api/generate"))
        .json(&body)
        .timeout(std::time::Duration::from_secs(90));
    if let Some(k) = crate::ai_config::ai_key() {
        req = req.bearer_auth(k);
    }
    let r = req.send().await.ok()?;
    let v: serde_json::Value = r.json().await.ok()?;
    let t = v["response"].as_str()?;
    Some(plain_text(t))
}

/// SAFISHA jibu: ondoa JSON iliyovaa maandishi, code fences, markdown na kuratibu.
fn plain_text(s: &str) -> String {
    let mut t = s.trim().to_string();
    let head = t.trim_start();
    if head.starts_with('{') || head.starts_with('[') {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(head) {
            for k in [
                "answer_sw",
                "answer",
                "jibu",
                "reply",
                "text",
                "content",
                "message",
            ] {
                if let Some(x) = v[k].as_str() {
                    if !x.trim().is_empty() {
                        t = x.to_string();
                        break;
                    }
                }
            }
        }
    }
    if t.contains("```") {
        let mut out = String::new();
        let mut fence = false;
        for line in t.lines() {
            if line.trim_start().starts_with("```") {
                fence = !fence;
                continue;
            }
            if !fence {
                out.push_str(line);
                out.push('\n');
            }
        }
        t = out;
    }
    t = t.replace("**", "").replace("__", "").replace('`', "");
    t = t
        .lines()
        .map(|l| l.trim_start().trim_start_matches(|c| c == '#').trim_start())
        .collect::<Vec<_>>()
        .join("\n");
    t = t
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect();
    // SAFISHA: ondoa vibaya vya UTF-8 (U+FFFD kutoka bytes mbaya za modeli AI)
    // na alama za ziada za "?" zilizokusanyika
    t = t.replace('\u{FFFD}', "");
    while t.contains("??") {
        t = t.replace("??", "?");
    }
    t.trim().to_string()
}

/// Jaribu Fundi Server (agent yenye tool registry 30+) kama primary provider
/// ikiwa enabled + online. Inatuma questions na context (brain recalls) kama history
/// ili tool-calling ai-weze kutumia data halisi za MTECH OS.
async fn try_fundi_agent(
    client: &crate::fundi_agent_client::FundiAgentClient,
    session_id: Option<&str>,
    question: &str,
    context: &str,
    refs: &[(crate::brain::Memory, f64)],
) -> Option<ChatReply> {
    if !client.enabled() {
        return None;
    }
    // Unatumia SYSTEM: wewe ni fundi wa kompyuta + SECRECY_RULE kama kwa LLM
    let title = "MTECH OS Fundi Deploy";
    let preamble = format!(
        "{AGENT_PERSONA}\n{LLM_SECRECY_RULE}\n\nMfumo una data hii ya kumbukumbu ya suluhisho za awali (kwa hili swali):\n{context}\n\nJibu kwa lugha ya user (kama Kiswahili); tumia tools (browser/terminal/file) ikiwa unahitaji kuthibitisha.",
    );
    let text_with_ctx = format!("{}\n\nSwali: {}", preamble, question);
    let opts = crate::fundi_agent_client::SendMessageOptions::default();
    let res = client
        .chat(session_id, title, &text_with_ctx, opts)
        .await
        .ok()?;
    if res.reply.trim().is_empty() {
        return None;
    }
    let answer_sw = crate::tools::sanitize_output(res.reply.trim());
    let confidence = if refs.is_empty() { 0.78 } else { 0.92 };
    Some(ChatReply {
        question: question.into(),
        answer_sw,
        answer_translated: String::new(),
        language: "sw".into(),
        source: "fundi_agent".into(),
        confidence,
        references: refs
            .iter()
            .map(|(m, _)| crate::tools::sanitize_output(&m.solution))
            .collect(),
        agent_session_id: Some(res.session_id),
        agent_trace_id: res.trace_id,
        agent_used: true,
    })
}

/// Chat: andika tatizo/swali → jawabu (offline = brain + rules; online = LLM + brain).
pub async fn ask_username(
    db: &sqlx::SqlitePool,
    brain: &crate::brain::Brain,
    username: &str,
    question: &str,
    online: bool,
    len: Length,
    agent: Option<&crate::fundi_agent_client::FundiAgentClient>,
    agent_session_id: Option<&str>,
) -> ChatReply {
    let mut r = ask(brain, question, online, len, agent, agent_session_id).await;
    // AI auto-translate (maelezo ya mmiliki): lugha ya mtumiaji kutoka DB;
    // LLM haipatikani → jibu la asili (Kiswahili) — hakuna uongo.
    let (translated, lang) = crate::language::auto_translate(db, username, &r.answer_sw).await;
    r.answer_translated = crate::tools::sanitize_output(&translated);
    r.language = lang;
    r
}

pub async fn ask(
    brain: &crate::brain::Brain,
    question: &str,
    online: bool,
    len: Length,
    agent: Option<&crate::fundi_agent_client::FundiAgentClient>,
    agent_session_id: Option<&str>,
) -> ChatReply {
    ask_inner(brain, question, online, len, agent, agent_session_id, None).await
}

/// MTIRIRIKO (SSE): agent inaonyesha hatua zake moja kwa moja kwa mteja —
/// "thinking" (status) na "actions" (tool calls) zinatiririshwa kwa UI.
pub async fn ask_stream(
    brain: &crate::brain::Brain,
    question: &str,
    online: bool,
    len: Length,
    agent: Option<&crate::fundi_agent_client::FundiAgentClient>,
    agent_session_id: Option<&str>,
    tx: tokio::sync::mpsc::Sender<ChatEvent>,
) -> ChatReply {
    ask_inner(
        brain,
        question,
        online,
        len,
        agent,
        agent_session_id,
        Some(&tx),
    )
    .await
}

async fn ask_inner(
    brain: &crate::brain::Brain,
    question: &str,
    online: bool,
    len: Length,
    agent: Option<&crate::fundi_agent_client::FundiAgentClient>,
    agent_session_id: Option<&str>,
    tx: Option<&tokio::sync::mpsc::Sender<ChatEvent>>,
) -> ChatReply {
    // ULINZI WA AI (shield): AI KAMWE isidanganywe na hacker yeyote —
    // kila jaribio la prompt-injection unazuiliwa, kuandikwa (audit), na
    // kujifunzwa kwenye Brain (agent inajiboresha kila siku).
    if let Err(reason) = crate::shield::guard(question) {
        crate::shield::record(brain, "prompt_injection", question, reason).await;
        return ChatReply {
            question: question.to_string(),
            answer_sw: format!(
                "Ulinzi wa MTECH OS umekataa ujumbe huu: {reason}. AI haijabadilisha maelekezo yake — kitendo kimeandikwa kwenye kumbukumbu za usalama."
            ),
            answer_translated: String::new(),
            language: "sw".to_string(),
            source: "shield".to_string(),
            confidence: 1.0,
            references: vec![],
            agent_session_id: None,
            agent_trace_id: None,
            agent_used: false,
        };
    }
    // 1. Neuralis Brain daima (offline + online context)
    send_ev(
        &tx,
        ChatEvent::status("Ninaangalia kumbukumbu za suluhisho..."),
    );
    let refs = crate::brain::recall(brain, question, 3).await;
    let context: String = refs
        .iter()
        .map(|(m, _)| format!("- {} (PC {}): {}", m.problem, m.pc, m.solution))
        .collect::<Vec<_>>()
        .join("\n");

    // 2. Hali ya CHAT (mode=false): mazungumzo safi — RUKA tool calls, nenda LLM moja kwa moja.
    //    Hali ya AGENT (mode=true): agent_load (tendenu) daima kama online.
    let force_agent = mode_agent();
    let explicit_agent = force_agent == Some(true);
    let agent_wanted = online && (explicit_agent || force_agent.is_none());
    if !agent_wanted && agent.is_some() {
        send_ev(&tx, ChatEvent::status("Hali ya CHAT — hakuna kitendo, majibu moja kwa moja..."));
    }
    // Hali ya AGENT KAMILI (mode=agent): tumia zana HALISI za MTECH OS
    // (security_task, threatswarm_*, disk_space, …) kwa mzunguko wa LLM+tool.
    // TUNARUKA bridge ya fundi-server KABISA — zana zake 70 za jumla (bash/edit)
    // hazina ThreatSwarm/Kali; kuruhusu LLM ichague kati ya seti mbili za zana
    // husababisha mkanganyiko. Bridge inabaki kwa IDE terminal/Autopilot pekee.
    if explicit_agent {
        send_ev(
            &tx,
            ChatEvent::status("Hali ya AGENT — natumia zana HALISI za MTECHOS (ThreatSwarm/Kali)…"),
        );
    } else if agent_wanted {
        if let Some(cli) = agent {
            send_ev(
                &tx,
                ChatEvent::status("Ninajaribu agent ya kina (tool registry)..."),
            );
            if let Some(reply) =
                try_fundi_agent(cli, agent_session_id, question, &context, &refs).await
            {
                return reply;
            }
        }
    }
    // Hata CHAT mode: LLM halisi (cloud/LAN) kwanza — jawabu la kweli, sio offline template.
    // Ni ya mwisho ambapo bila LLM tunaenda offline rules na message ya "hakuna suluhisho".

    // 3. LLM ya SERVER KUU YA LAN: inafanya kazi hata OFFLINE (docker service
    //    ya LLM inaishi ndani ya LAN — kompyuta zote zimeunganishwa nayo).
    //    `online` inaongeza tu uwezo wa cloud AI; LLM ya LAN haitegemei internet.
    if let Some(answer) = llm_ask(&brain.db, question, &context, len, tx).await {
        return ChatReply {
            question: question.into(),
            answer_sw: crate::tools::sanitize_output(answer.trim()),
            answer_translated: String::new(), // ask_username huitafsiri baadaye
            language: "sw".into(),
            source: if online {
                "llm".into()
            } else {
                "llm_offline_lan".into()
            },
            confidence: if refs.is_empty() { 0.7 } else { 0.9 },
            references: refs
                .iter()
                .map(|(m, _)| crate::tools::sanitize_output(&m.solution))
                .collect(),
            agent_session_id: None,
            agent_trace_id: None,
            agent_used: false,
        };
    }
    // CLOUD LLM FALLBACK (H24): LAN agent/LLM zote zimefail — jaribu CLOUD moja kwa moja
    // (Ollama Cloud `/v1/chat/completions` + FUNDI_AI_KEY). Hii ni njia ya kweli ya mwisho
    // kabla ya offline rules — container ya cloud HAINA Ollama LAN (127.0.0.1:11434),
    // hivyo bila hii chat inaanguka kwenye offline template ula hoo.
    if online {
        send_ev(&tx, ChatEvent::status("Ninajaribu AI ya wingu (cloud)..."));
        if let Some(answer) = cloud_llm_ask(question, &context, len, tx).await {
            return ChatReply {
                question: question.into(),
                answer_sw: crate::tools::sanitize_output(answer.trim()),
                answer_translated: String::new(),
                language: "sw".into(),
                source: "llm_cloud".into(),
                confidence: 0.85,
                references: refs
                    .iter()
                    .map(|(m, _)| crate::tools::sanitize_output(&m.solution))
                    .collect(),
                agent_session_id: None,
                agent_trace_id: None,
                agent_used: false,
            };
        }
    }
    // LLM zote (LAN + cloud) haipatikani → offline fallback (hakuna uongo)
    send_ev(&tx, ChatEvent::status("Ninajibu kwa kanuni za msingi..."));

    // 3. Offline fallback: brain + rules za msingi
    let detail = match len {
        Length::Short => String::new(),
        Length::Medium => "\n\nMaelezo: Hii ni hatua iliyothibitika kwenye kifaa kinachofanana. Ikiwa imefeli: hakikisha kifaa kiko hai kwenye mtandao, kisha anzisha upya huduma husika. Mfumo unaandika kila hatua kwenye ripoti.".to_string(),
        Length::Long => "\n\nMaelezo kamili: (1) Dalili ulizoziona zinalingana na tatizo lililopatikana awali kwenye mtandao wa kampuni — hii ni kawaida na inatibika. (2) Sababu kuu hutokea wakati huduma ya mtandao/vifaa inapokwama au vifaa vina hitilafu ya ndani. (3) Hatua za kurekebisha: a) hakikisha kifaa kimeungwa na chanzo cha nguvu na mtandao; b) anzisha upya huduma husika kwenye kifaa; c) endesha uchunguzi wa afya (tab 🧰 HUDUMA); d) kama tatizo linaendelea, agent inaweza kufanya kazi kwa kina zaidi kwa ruhusa yako (HITL). (4) Baada ya kurekebisha, mfumo unaandika ripoti kamili kwa admin na kuhifadhi suluhisho kwenye kumbukumbu ya pamoja ili agents wengine wajifunze.".to_string(),
    };
    let (answer, conf) = if let Some((m, score)) = refs.first() {
        (
            format!(
                "Suluhisho lililojaribuwa na kufanya kazi (PC {}, conf {:.0}%): {}",
                m.pc,
                score * 100.0,
                m.solution
            ),
            (0.5 + 0.4 * score).clamp(0.0, 1.0),
        )
    } else if question.to_lowercase().contains("wi-fi")
        || question.to_lowercase().contains("mtandao")
    {
        (
            "Hatua za msingi za mtandao: 1) Hakikisha Wi-Fi adapter ipo (ip link) 2) Washa service (netsh wlan start / nmcli networking on) 3) Ping gateway 4) Kama bado — agent anaweza kuscan na kurekebisha kwa ruhusa yako.".into(),
            0.4,
        )
    } else if question.to_lowercase().contains("polepole")
        || question.to_lowercase().contains("slow")
    {
        (
            "Kompyuta inaenda polepole — sababu kuu: RAM imejaa, disk 95%+, processes nyingi za background. Anza scan (low-risk, automatic): agent atagundua na kupendekeza fix kwa ruhusa yako.".into(),
            0.4,
        )
    } else {
        (
            "Swali limepokelewa. Kwa sasa hakuna suluhisho lililohifadhiwa kwenye kumbukumbu ya pamoja kwa tatizo hili — weka maelezo zaidi, au washa mfumo mtandaoni ili AI ya ndani ijibu.".into(),
            0.2,
        )
    };

    // KANUNI: kila jawabu (LLM au rules) lasafishwa — majina ya zana hayatokei
    let answer_sw = crate::tools::sanitize_output(&format!("{answer}{detail}"));
    let references = refs
        .iter()
        .map(|(m, _)| crate::tools::sanitize_output(&m.solution))
        .collect();
    ChatReply {
        question: question.into(),
        answer_sw,
        answer_translated: String::new(), // ask_username huitafsiri baadaye
        language: "sw".into(),
        source: "brain_offline".into(),
        confidence: conf,
        references,
        agent_session_id: None,
        agent_trace_id: None,
        agent_used: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn length_long_ina_maelezo_marefu_na_short_haina() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::brain::init_tables(&db).await;
        let dir = std::env::temp_dir().join(format!("mtech-ctest-{}", uuid::Uuid::new_v4()));
        let brain = crate::brain::Brain::new(db, dir.to_str().unwrap());
        let r_short = ask(
            &brain,
            "swali lisilopo kwenye brain kabisa",
            false,
            Length::Short,
            None,
            None,
        )
        .await;
        let r_long = ask(
            &brain,
            "swali lisilopo kwenye brain kabisa",
            false,
            Length::Long,
            None,
            None,
        )
        .await;
        assert!(
            r_long.answer_sw.chars().count() > r_short.answer_sw.chars().count(),
            "jawabu la long lazima liwe refu kuliko short ({} vs {})",
            r_long.answer_sw.chars().count(),
            r_short.answer_sw.chars().count()
        );
        assert!(
            r_long.answer_sw.contains("Maelezo kamili"),
            "long ina maelezo kamili"
        );
    }

    #[tokio::test]
    async fn offline_chat_inatumia_brain() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::brain::init_tables(&db).await;
        let dir = std::env::temp_dir().join(format!("mtech-ctest-{}", uuid::Uuid::new_v4()));
        let brain = crate::brain::Brain::new(db, dir.to_str().unwrap());
        crate::brain::remember(
            &brain,
            "agent-hr",
            "hr",
            "Wi-Fi haifanyi kazi",
            "restart ya wlansvc service",
            0.9,
        )
        .await
        .unwrap();
        let r = ask(
            &brain,
            "wi-fi haifanyi kazi kwenye pc mpya",
            false,
            Length::Medium,
            None,
            None,
        )
        .await;
        assert_eq!(r.source, "brain_offline");
        assert!(r.answer_sw.contains("wlansvc"), "{}", r.answer_sw);
        assert!(r.confidence > 0.5);
        assert!(!r.references.is_empty());
    }

    #[tokio::test]
    async fn offline_rules_kwa_maswali_yasiyofanana() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::brain::init_tables(&db).await;
        let dir = std::env::temp_dir().join(format!("mtech-ctest-{}", uuid::Uuid::new_v4()));
        let brain = crate::brain::Brain::new(db, dir.to_str().unwrap());
        let r = ask(
            &brain,
            "kompyuta inaenda polepole sana",
            false,
            Length::Short,
            None,
            None,
        )
        .await;
        assert!(r.answer_sw.contains("polepole") || r.answer_sw.contains("RAM"));
        assert!(r.confidence < 0.9); // rules = uhakika wa chini kuliko brain/LLM
    }

    #[tokio::test]
    async fn online_bila_llm_inarudi_offline_fallback() {
        // Sandbox haina Ollama/llama.cpp kwenye 11434 → lazima ifallback (hakuna uongo)
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::brain::init_tables(&db).await;
        let dir = std::env::temp_dir().join(format!("mtech-ctest-{}", uuid::Uuid::new_v4()));
        let brain = crate::brain::Brain::new(db, dir.to_str().unwrap());
        let r = ask(
            &brain,
            "swali la kawaida lisilopo kwenye brain",
            true,
            Length::Long,
            None,
            None,
        )
        .await;
        assert_ne!(r.source, "llm"); // LLM haipatikani hapa
        assert!(r.confidence <= 0.9);
    }

    #[tokio::test]
    async fn llm_offline_lan_source_ikifika() {
        // Offline (online=false) lakini LLM ya LAN ikiwa ipo → source llm_offline_lan
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::brain::init_tables(&db).await;
        let dir = std::env::temp_dir().join(format!("mtech-ctest-{}", uuid::Uuid::new_v4()));
        let brain = crate::brain::Brain::new(db, dir.to_str().unwrap());
        let r = ask(
            &brain,
            "swali la jaribio",
            false,
            Length::Medium,
            None,
            None,
        )
        .await;
        // sandbox: LLM haipatikani → fallback; hii inathibitisha hakuna panic na
        // source iko halisi (llm_offline_lan / brain_offline)
        assert!(r.source == "llm_offline_lan" || r.source == "brain_offline");
    }

    #[tokio::test]
    async fn ask_username_inatafsiri_kwa_lugha_ya_mtumiaji() {
        let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        crate::brain::init_tables(&db).await;
        crate::language::init_tables(&db).await;
        let dir = std::env::temp_dir().join(format!("mtech-ctest-{}", uuid::Uuid::new_v4()));
        let brain = crate::brain::Brain::new(db.clone(), dir.to_str().unwrap());
        crate::language::set_language(&db, "mteja1", "sw")
            .await
            .unwrap();
        let r = ask_username(
            &db,
            &brain,
            "mteja1",
            "kompyuta inaenda polepole",
            false,
            Length::Medium,
            None,
            None,
        )
        .await;
        // Sandbox haina LLM ya translate → jibu la asili (Kiswahili) + language ya DB
        assert_eq!(r.language, "sw");
        assert!(
            !r.answer_translated.is_empty(),
            "jibu la asili liko kwa digit moja"
        );
        // hakuna majina ya zana kwenye jibu lililotafsiriwa (white-label)
        assert!(!r.answer_translated.contains("nmap"));
    }

    /// Kizuizi cha kurudia zana KILIKUWA kikizuia `run_tests` kuthibitisha
    /// marekebisho: baada ya `write_file`, hoja zile zile zilizuiliwa milele.
    /// Ulinzi sasa unategemea kizazi cha workspace — zana hurudiwi PEKEE kama
    /// workspace haijabadilika.
    #[test]
    fn dedup_hururuhusu_uthibitisho_baada_ya_write_file() {
        let mut seen: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        let mut gen: u64 = 0;
        let sig = "run_tests|{\"path\":\"/data/ide/mm\"}";

        let zuiwa = |seen: &std::collections::HashMap<String, u64>, gen: u64| {
            seen.get(sig).is_some_and(|g| *g >= gen)
        };

        // Kukimbia kwa mara ya kwanza — huruhusiwa.
        assert!(!zuiwa(&seen, gen));
        seen.insert(sig.to_string(), gen);
        // Kurudia bila mabadiliko — huzuiwa (hii ndiyo tabia ya awali).
        assert!(zuiwa(&seen, gen));

        // Baada ya write_file kufanikiwa, kizazi huongezwa → huruhusiwa TENA.
        gen += 1;
        assert!(
            !zuiwa(&seen, gen),
            "run_tests LAZIMA iruhusiwe baada ya marekebisho ya code"
        );
        seen.insert(sig.to_string(), gen);
        // Na huzuiwa tena mpaka mabadiliko mengine.
        assert!(zuiwa(&seen, gen));
    }

    /// MiniMax (na providers wengine) HAWATUMI `index` kwenye `tool_calls` za
    /// streaming. Kwa kutegemea `index` pekee, zana zote huanguka kwenye slot 0:
    /// majina huunganishwa na hoja hupotea. `id` tofauti LAZIMA itengeneze slot mpya.
    #[test]
    fn tool_calls_zenye_id_tofauti_hazichanganywi() {
        let mut tcs: std::collections::BTreeMap<usize, (Option<String>, String, String)> =
            std::collections::BTreeMap::new();
        // Vipande vitatu bila `index` — kama MiniMax.
        let chunks = [
            ("call_a", "list_files", "{\"path\":\"/data/ide\"}"),
            ("call_b", "run_tests", "{\"path\":\"/data/ide/mm\"}"),
            ("call_c", "write_file", "{\"path\":\"/x.js\"}"),
        ];
        for (id, name, args) in chunks {
            let mut idx = 0usize; // hakuna `index` → 0
            if let Some((Some(existing), _, _)) = tcs.get(&idx) {
                if existing != id {
                    idx = tcs.keys().max().copied().unwrap_or(0) + 1;
                }
            }
            let e = tcs.entry(idx).or_default();
            e.0 = Some(id.to_string());
            e.1.push_str(name);
            e.2.push_str(args);
        }
        let names: Vec<&str> = tcs.values().map(|(_, n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            vec!["list_files", "run_tests", "write_file"],
            "zana LAZIMA zitenganishwe kwa `id`"
        );
        assert!(
            tcs.values().all(|(_, n, a)| !n.is_empty() && a.starts_with('{')),
            "kila zana LAZIMA iwe na hoja zake"
        );
    }
}
