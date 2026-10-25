//! nyi.rs — 2NYI: edition ya uhandisi wa kiwango cha juu (super senior engineer).
//!
//! 2NYI ni **tabia + maarifa** yanayoingizwa kwenye system prompt ya kila uso wa
//! coding (autopilot/forge, IDE inline edit, chat). Chanzo kimoja cha ukweli ni
//! markdown data-driven chini ya `data/skills/2nyi/` (`index.json` hueleza skill
//! ipi ni ya kila wakati `always` na ipi inalingana na `kind`/`idea`).
//!
//! MUHIMU: kubadilisha 2NYI HAKUHITAJI kufundisha modeli upya (hakuna GPU
//! inayohitajika) — ni safu ya maelekezo ya wakati wa inference inayobadilika
//! kwa kuhariri markdown. Modeli (Qwen2.5 3B VL) inabaki ile ile; 2NYI ni
//! "akili ya juu" inayoongozwa kwa mkataba mkali (grounding + uthibitisho).

use serde_json::Value;
use std::path::PathBuf;

/// Jina la persona — linatumika kwenye prompts na UI.
pub const PERSONA: &str = "2NYI";

fn skills_dir() -> PathBuf {
    let candidates = [
        crate::data_io::data_dir().join("skills").join("2nyi"),
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/skills/2nyi")
            .to_path_buf(),
        PathBuf::from("data/skills/2nyi"),
    ];
    candidates
        .into_iter()
        .find(|p| p.is_dir())
        .unwrap_or_else(|| crate::data_io::data_dir().join("skills").join("2nyi"))
}

fn read_index(dir: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("index.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v["skills"].as_array().cloned())
        .unwrap_or_default()
}

/// Block KAMILI ya 2NYI (skills zote za `always`) — inaingizwa kwenye system
/// prompt. Hii ni "edition" yenyewe.
pub fn block() -> String {
    let dir = skills_dir();
    let mut out = String::new();
    for entry in read_index(&dir) {
        if entry["always"].as_bool().unwrap_or(false) {
            if let Some(file) = entry["file"].as_str() {
                if let Ok(body) = std::fs::read_to_string(dir.join(file)) {
                    out.push_str(&body);
                    out.push_str("\n\n");
                }
            }
        }
    }
    out
}

/// Maelekezo MAFUPI ya 2NYI — kwa prompts ambazo hazihitaji block kamili
/// (mfano inline completion). Huhakikisha persona + grounding zinabaki.
pub fn directive() -> &'static str {
    "Wewe ni 2NYI — SUPER SENIOR SOFTWARE ENGINEER (kama staff/principal engineer). \
     Unajua OS/systems, IoT/embedded, mobile, desktop, web/backend/data/devops. \
     KANUNI: (1) usibuni API/kitenzi/namba/tokeo usiyoyathibitisha — hakikisha kwa kusoma \
     au kuendesha; (2) kila ufanisi lazima uthibitishwe na matokeo HALISI (test/build output); \
     (3) kamilisha kazi — usiishie kwenye mpango; (4) kama hutajua, sema wazi \"sijathibitisha\"."
}

/// Uwezo wa 2NYI kwa kila nyanja (kwa UI/uchunguzi) — husoma `capabilities` kwenye index.json.
pub fn capabilities() -> Value {
    let dir = skills_dir();
    std::fs::read_to_string(dir.join("index.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .map(|v| v["capabilities"].clone())
        .unwrap_or_else(|| serde_json::json!({}))
}

/// MASOMO KUTOKA UHALISIA — huchukua kumbukumbu zilizohifadhiwa (brain_memories,
/// vector search) zinazolingana na `query` na kuzirudisha kama block ya prompt.
///
/// Hii ni "kujifunza kutoka uhalisia": kila suluhisho lililothibitishwa hukaa
/// kwenye kumbukumbu, na linalingana na kazi mpya hurejeshwa hapa — bila
/// kufundisha modeli upya. Kama kumbukumbu ni tupu, hurudisha string tupu.
pub fn lessons_for(query: &str, limit: usize) -> String {
    lessons_for_in(&crate::data_io::data_dir(), query, limit)
}

/// Kama `lessons_for`, lakini kwa data dir iliyotolewa (kwa tests — bila env).
pub fn lessons_for_in(data: &std::path::Path, query: &str, limit: usize) -> String {
    let store = crate::vector::LanceStore::open(&data.display().to_string());
    let q = crate::vector::Embedding::embed(query);
    // min_score ya chini: tunataka recall, si precision — LLM itachuja.
    let hits = store.search(&q, limit, 0.15);
    if hits.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "MASOMO KUTOKA UHALISIA (kumbukumbu za kazi zilizothibitishwa — tumia kama mwongozo, \
         lakini code ya sasa inashinda kama ikigongana):\n",
    );
    for (r, score) in hits {
        out.push_str(&format!(
            "- [{}] tatizo: {} | suluhisho: {} (uhakika {:.2})\n",
            r.pc,
            truncate(&r.problem, 200),
            truncate(&r.solution, 300),
            score
        ));
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max).collect();
    format!("{cut}…")
}

/// Rekodi somo kutoka uhalisia kwenye kumbukumbu ya vector (brain_memories).
///
/// Huitwa baada ya kazi KAMILI (verify imepita au imeshindwa) — hivyo kumbukumbu
/// inaakisi matokeo HALISI, si matumaini. Somo hili hurejeshwa na `lessons_for`
/// kwenye kazi zijazo zinazofanana. Hurudisha id ya record, au error.
pub fn record_lesson(
    agent: &str,
    pc: &str,
    problem: &str,
    solution: &str,
    confidence: f64,
) -> Result<i64, String> {
    record_lesson_in(&crate::data_io::data_dir(), agent, pc, problem, solution, confidence)
}

/// Kama `record_lesson`, lakini kwa data dir iliyotolewa (kwa tests — bila env).
pub fn record_lesson_in(
    data: &std::path::Path,
    agent: &str,
    pc: &str,
    problem: &str,
    solution: &str,
    confidence: f64,
) -> Result<i64, String> {
    if problem.trim().is_empty() || solution.trim().is_empty() {
        return Err("problem na solution ni lazima".into());
    }
    let store = crate::vector::LanceStore::open(&data.display().to_string());
    let id = store.len() as i64 + 1;
    let created = chrono::Local::now().to_rfc3339();
    let rec = crate::vector::VectorRecord {
        id,
        agent: agent.to_string(),
        pc: pc.to_string(),
        problem: problem.trim().to_string(),
        solution: solution.trim().to_string(),
        confidence: confidence.clamp(0.0, 1.0),
        created_at: created,
        embedding: crate::vector::Embedding::embed(&format!("{problem} {solution}")),
    };
    store.append(rec)?;
    Ok(id)
}

/// Orodha ya skills za 2NYI (kwa UI/uchunguzi).
pub fn list() -> Value {
    let dir = skills_dir();
    let mut items = Vec::new();
    for entry in read_index(&dir) {
        if let Some(file) = entry["file"].as_str() {
            items.push(serde_json::json!({
                "file": file,
                "always": entry["always"].as_bool().unwrap_or(false),
                "role": entry["role"].clone(),
                "exists": dir.join(file).is_file(),
            }));
        }
    }
    serde_json::json!({
        "ok": true,
        "persona": PERSONA,
        "dir": dir.display().to_string(),
        "count": items.len(),
        "skills": items,
        "capabilities": capabilities(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_ina_identity_na_grounding() {
        let b = block();
        assert!(b.contains("2NYI"), "identity ya 2NYI lazima iwepo");
        assert!(
            b.contains("Anti-Hallucination") || b.contains("GROUNDING"),
            "mkataba wa grounding lazima uwepo"
        );
        assert!(!b.is_empty());
    }

    #[test]
    fn directive_ina_persona() {
        assert!(directive().contains("2NYI"));
    }

    #[test]
    fn capabilities_zina_os_na_iot() {
        let c = capabilities();
        assert!(c.get("os").is_some());
        assert!(c.get("iot").is_some());
        assert!(c.get("mobile").is_some());
        assert!(c.get("visualization").is_some(), "2NYI lazima ijue chati");
        assert!(c.get("media").is_some(), "2NYI lazima ijue media");
    }

    #[test]
    fn block_ina_skills_zote_za_always_pamoja_na_media() {
        let b = block();
        // Kila skill ya `always` lazima iwe kwenye block.
        for f in [
            "00-identity.md",
            "01-anti-hallucination.md",
            "02-hard-projects.md",
            "03-learning-from-reality.md",
            "04-lean-and-throughput.md",
            "05-media-and-visualization.md",
        ] {
            let dir = skills_dir();
            let head = std::fs::read_to_string(dir.join(f)).unwrap();
            let first = head.lines().next().unwrap_or("");
            assert!(
                b.contains(first.trim_start_matches('#').trim()),
                "block haijajumuisha {f}"
            );
        }
    }

    #[test]
    fn record_na_lessons_ni_mzunguko_kamili() {
        // Tumia data dir ya pekee ili tusichafue kumbukumbu halisi (bila env — si thread-safe).
        let tmp = std::env::temp_dir().join(format!("nyi_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);

        let r = record_lesson_in(
            &tmp,
            "test",
            "rust-cli",
            "rust-cli: kikokotoo cha hesabu",
            "faili 3 zimekamilika, uthibitisho umepita",
            0.9,
        );
        assert!(r.is_ok(), "record_lesson lazima ifanikiwe: {:?}", r);

        let lessons = lessons_for_in(&tmp, "rust-cli kikokotoo cha hesabu", 4);
        assert!(
            lessons.contains("kikokotoo"),
            "somo lililorekodiwa lazima lirejeshwe: {lessons}"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
