//! Akili Agent V4.0 — Kufikiri kama Binadamu
//!
//! Tabaka 12 za fikira za kibinadamu ndani ya Fundi Deploy / fundi-agent:
//! 1. Attention          2. Perception
//! 3. System 2 (Slow)    4. System 1 (Fast)
//! 5. Subconscious       6. Intuition
//! 7. Imagination        8. Emotional Intelligence
//! 9. Social Cognition  10. Values & Ethics
//! 11. Meaning & Purpose 12. Self-Identity
//!
//! Imeunganishwa na fundi-agent core (agent_loop, reasoning, memory, skills).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ═══════════════════════════════════════════════════════════════
//  TABAKA 1: ATTENTION (Makini)
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Focus {
    pub target: String,
    pub intensity: f64,
    pub duration_ms: u64,
    pub distractions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttendedInput {
    pub primary: String,
    pub secondary: Vec<String>,
    pub ignored: Vec<String>,
    pub focus: Focus,
}

#[derive(Debug, Clone)]
pub struct Attention {
    pub focus: Focus,
}

impl Attention {
    pub fn new() -> Self {
        Self {
            focus: Focus {
                target: String::new(),
                intensity: 0.0,
                duration_ms: 0,
                distractions: vec![],
            },
        }
    }

    pub fn attend(&mut self, input: &str, context: &[String]) -> AttendedInput {
        let words: Vec<&str> = input.split_whitespace().collect();
        let primary = input.to_string();
        let mut secondary = Vec::new();
        let mut ignored = Vec::new();

        let high_salience = [
            "tatizo", "problem", "hatari", "danger", "haraka", "urgent",
            "polepole", "slow", "joto", "heat", "kuvunjika", "broken",
            "umeme", "kompyuta", "simu", "gari", "mtandao", "deploy",
            "pxe", "backup", "multicast", "vpn", "wireguard",
        ];

        let mut score: f64 = 0.5;
        for w in &words {
            let lower = w.to_lowercase();
            if high_salience.iter().any(|k| lower.contains(k)) {
                score = (score + 0.15).min(1.0);
            }
        }

        for c in context {
            if input.to_lowercase().contains(&c.to_lowercase()) {
                secondary.push(c.clone());
            }
        }

        for w in words {
            if w.len() > 20 {
                ignored.push(w.to_string());
            }
        }

        self.focus = Focus {
            target: primary.clone(),
            intensity: score,
            duration_ms: 3000,
            distractions: ignored.clone(),
        };

        AttendedInput {
            primary,
            secondary,
            ignored,
            focus: self.focus.clone(),
        }
    }

    pub fn refocus(&mut self, new_target: &str) {
        self.focus.target = new_target.to_string();
        self.focus.intensity = 1.0;
    }
}

// ═══════════════════════════════════════════════════════════════
//  TABAKA 2: PERCEPTION
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Modality {
    Vision,
    Hearing,
    Touch,
    Taste,
    Smell,
    Interoception,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Percept {
    pub modality: Modality,
    pub content: String,
    pub intensity: f64,
    pub valence: f64,
    pub arousal: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct Perception;

impl Perception {
    pub fn new() -> Self { Self }

    pub fn perceive(&self, input: &str, has_visual: bool, has_audio: bool) -> Vec<Percept> {
        let mut percepts = Vec::new();
        let now = now_secs();

        let valence = if input.contains("polepole")
            || input.contains("hatari")
            || input.contains("kuvunjika")
            || input.contains("joto")
            || input.contains("fail")
        {
            -0.4
        } else if input.contains("asante") || input.contains("vizuri") || input.contains("success") {
            0.6
        } else {
            0.0
        };

        percepts.push(Percept {
            modality: Modality::Vision,
            content: input.to_string(),
            intensity: 0.8,
            valence,
            arousal: if valence < -0.2 { 0.6 } else { 0.3 },
            timestamp: now,
        });

        if has_audio {
            percepts.push(Percept {
                modality: Modality::Hearing,
                content: "sauti ya mtumiaji / agent".into(),
                intensity: 0.5,
                valence: 0.0,
                arousal: 0.2,
                timestamp: now,
            });
        }

        percepts.push(Percept {
            modality: Modality::Interoception,
            content: "nina makini, sina uchovu".into(),
            intensity: 0.4,
            valence: 0.2,
            arousal: 0.3,
            timestamp: now,
        });

        percepts
    }
}

// ═══════════════════════════════════════════════════════════════
//  TABAKA 3: SYSTEM 2 — SLOW THINKING
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thought {
    pub steps: Vec<String>,
    pub conclusion: String,
    pub confidence: f64,
    pub cognitive_load: f64,
    pub took_rest: bool,
}

#[derive(Debug, Clone)]
pub struct System2 {
    pub cognitive_load: f64,
}

impl System2 {
    pub fn new() -> Self {
        Self { cognitive_load: 0.2 }
    }

    pub fn think(&mut self, problem: &str) -> Thought {
        if self.cognitive_load > 0.92 {
            return Thought {
                steps: vec!["Nimechoka sana — napumzika".into()],
                conclusion: "Haiwezi kufikiri sasa. Jaribu baadaye.".into(),
                confidence: 0.1,
                cognitive_load: self.cognitive_load,
                took_rest: true,
            };
        }

        let mut steps = Vec::new();
        steps.push(format!("1. Nimeelewa tatizo: {}", problem));

        let lower = problem.to_lowercase();
        if lower.contains("polepole") || lower.contains("slow") {
            steps.push("2. Sababu: CPU juu, RAM imejaa, disk full, malware".into());
            steps.push("3. Ninaangalia metrics za mfumo".into());
            steps.push("4. Ninaandaa hatua za kutatua".into());
        } else if lower.contains("deploy") || lower.contains("pxe") || lower.contains("image") {
            steps.push("2. Ninaangalia Fundi Deploy pipeline (backup → WOL → PXE → install)".into());
            steps.push("3. Ninaangalia HITL approval status".into());
            steps.push("4. Ninaandaa job + report".into());
        } else if lower.contains("umeme") || lower.contains("voltage") {
            steps.push("2. Ninaangalia voltage drop, breaker, wiring".into());
            steps.push("3. Ninahesabu formula".into());
        } else {
            steps.push("2. Ninaainisha tatizo".into());
            steps.push("3. Ninaangalia maarifa + skills".into());
        }

        self.cognitive_load = (self.cognitive_load + 0.12).min(1.0);
        let mut took_rest = false;
        if self.cognitive_load > 0.75 {
            steps.push("😓 Nachoka… napumzika kidogo".into());
            self.cognitive_load = (self.cognitive_load - 0.25).max(0.1);
            took_rest = true;
        }

        steps.push("5. Ninafanya uamuzi wa mwisho".into());

        Thought {
            steps,
            conclusion: format!("Baada ya kufikiri kwa makini: {}", problem),
            confidence: 0.75 + (0.2 * (1.0 - self.cognitive_load)),
            cognitive_load: self.cognitive_load,
            took_rest,
        }
    }

    pub fn rest(&mut self) {
        self.cognitive_load = (self.cognitive_load - 0.3).max(0.0);
    }
}

// ═══════════════════════════════════════════════════════════════
//  TABAKA 4: SYSTEM 1 — FAST THINKING
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BiasWarning {
    Confirmation,
    Availability,
    Anchoring,
    Overconfidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Judgment {
    pub decision: String,
    pub confidence: f64,
    pub speed: String,
    pub reasoning: String,
    pub bias_warnings: Vec<BiasWarning>,
}

#[derive(Debug, Clone)]
pub struct System1 {
    patterns: HashMap<String, (String, f64)>,
}

impl System1 {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();
        patterns.insert("polepole".into(), ("Angalia CPU/RAM/Disk".into(), 0.85));
        patterns.insert("slow".into(), ("Angalia CPU/RAM/Disk".into(), 0.85));
        patterns.insert("joto".into(), ("Angalia thermal + fans".into(), 0.9));
        patterns.insert("heat".into(), ("Angalia thermal + fans".into(), 0.9));
        patterns.insert("kuvunjika".into(), ("Diagnose hardware".into(), 0.8));
        patterns.insert("broken".into(), ("Diagnose hardware".into(), 0.8));
        patterns.insert("umeme".into(), ("Angalia voltage + breaker".into(), 0.88));
        patterns.insert("spark".into(), ("Hatari! Zima umeme kwanza".into(), 0.95));
        patterns.insert("cheche".into(), ("Hatari! Zima umeme kwanza".into(), 0.95));
        patterns.insert("deploy".into(), ("Fundi Deploy: plan → HITL → backup → PXE".into(), 0.9));
        patterns.insert("pxe".into(), ("Angalia TFTP + images + multicast".into(), 0.88));
        patterns.insert("backup".into(), ("Angalia FUNDI_BACKUP_MODE + manifests".into(), 0.87));
        patterns.insert("vpn".into(), ("Angalia WireGuard peers + tunnel".into(), 0.86));
        Self { patterns }
    }

    pub fn quick_judge(&self, situation: &str) -> Judgment {
        let lower = situation.to_lowercase();
        let mut best_decision = "Nifikiri kwa makini zaidi".to_string();
        let mut best_conf = 0.4;
        let mut matched = false;

        for (key, (action, conf)) in &self.patterns {
            if lower.contains(key) && *conf > best_conf {
                best_conf = *conf;
                best_decision = action.clone();
                matched = true;
            }
        }

        let mut bias_warnings = Vec::new();
        if matched && best_conf > 0.9 {
            bias_warnings.push(BiasWarning::Overconfidence);
        }
        if lower.contains("sasa hivi") || lower.contains("just now") {
            bias_warnings.push(BiasWarning::Availability);
        }

        Judgment {
            decision: best_decision,
            confidence: best_conf,
            speed: if matched { "instant".into() } else { "fast".into() },
            reasoning: if matched {
                "Nimeona pattern hii awali (Fundi/Deploy/System)".into()
            } else {
                "Hakuna pattern wazi — nitaenda System 2".into()
            },
            bias_warnings,
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  TABAKA 5: SUBCONSCIOUS
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub problem: String,
    pub insight: String,
    pub quality: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dream {
    pub content: String,
    pub related_to: Vec<String>,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub struct BackgroundTask {
    pub problem: String,
    pub started: Instant,
    pub completed: bool,
}

#[derive(Debug, Clone)]
pub struct Subconscious {
    pub background_tasks: Vec<BackgroundTask>,
    pub dreams: Vec<Dream>,
    pub recent_experiences: Vec<String>,
}

impl Subconscious {
    pub fn new() -> Self {
        Self {
            background_tasks: vec![],
            dreams: vec![],
            recent_experiences: vec![],
        }
    }

    pub fn spawn_background(&mut self, problem: &str) {
        self.background_tasks.push(BackgroundTask {
            problem: problem.to_string(),
            started: Instant::now(),
            completed: false,
        });
        self.recent_experiences.push(problem.to_string());
        if self.recent_experiences.len() > 20 {
            self.recent_experiences.remove(0);
        }
    }

    pub fn check_insights(&mut self) -> Vec<Insight> {
        let mut insights = Vec::new();
        for task in &mut self.background_tasks {
            if !task.completed && task.started.elapsed() > Duration::from_secs(5) {
                insights.push(Insight {
                    problem: task.problem.clone(),
                    insight: format!("Baada ya kufikiri nyuma ya akili: labda {}", task.problem),
                    quality: 0.7,
                });
                task.completed = true;
            }
        }
        insights
    }

    pub fn dream(&mut self) -> Dream {
        let related: Vec<String> = self.recent_experiences.iter().rev().take(3).cloned().collect();
        let content = if related.is_empty() {
            "Ndoto tupu — bado sina uzoefu mwingi".into()
        } else {
            format!("Ndoto: nimeunganisha {} → suluhisho jipya linawezekana", related.join(" + "))
        };
        let dream = Dream {
            content,
            related_to: related,
            timestamp: now_secs(),
        };
        self.dreams.push(dream.clone());
        dream
    }
}

// ═══════════════════════════════════════════════════════════════
//  TABAKA 6: INTUITION
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GutFeeling {
    pub signal: f64,
    pub intensity: f64,
    pub clarity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intuition {
    pub decision: String,
    pub confidence: f64,
    pub explanation: String,
    pub gut_feeling: GutFeeling,
}

#[derive(Debug, Clone)]
pub struct IntuitionEngine;

impl IntuitionEngine {
    pub fn new() -> Self { Self }

    pub fn feel(&self, situation: &str) -> Intuition {
        let lower = situation.to_lowercase();
        let mut signal: f64 = 0.0;
        let mut intensity: f64 = 0.4;

        if lower.contains("asante") || lower.contains("vizuri") || lower.contains("success") || lower.contains("imerekebishwa") {
            signal += 0.6;
            intensity += 0.2;
        }
        if lower.contains("hatari") || lower.contains("spark") || lower.contains("cheche")
            || lower.contains("moto") || lower.contains("fire") || lower.contains("fail")
        {
            signal -= 0.9;
            intensity = 0.95;
        }
        if lower.contains("polepole") || lower.contains("kuvunjika") || lower.contains("joto") {
            signal -= 0.35;
            intensity += 0.15;
        }

        signal = signal.clamp(-1.0, 1.0);
        intensity = intensity.clamp(0.0, 1.0);
        let clarity = intensity * 0.85;

        let (decision, explanation) = if signal > 0.5 {
            ("Endelea — kuna kitu kizuri".into(), "✅ Tumbo linasema ndiyo".into())
        } else if signal < -0.6 {
            ("Kaa mbali / Chukua tahadhari".into(), "⚠️ Tumbo linasema kaa mbali — kuna hatari".into())
        } else {
            ("Nifikiri kwa makini".into(), "🤔 Tumbo halijasema kitu wazi".into())
        };

        Intuition {
            decision,
            confidence: clarity,
            explanation,
            gut_feeling: GutFeeling { signal, intensity, clarity },
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  TABAKA 7: IMAGINATION
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImaginedScenario {
    pub description: String,
    pub action: String,
    pub expected_outcome: String,
    pub novelty: f64,
}

#[derive(Debug, Clone)]
pub struct Imagination;

impl Imagination {
    pub fn new() -> Self { Self }

    pub fn imagine(&self, situation: &str) -> Vec<ImaginedScenario> {
        let mut scenarios = Vec::new();
        let lower = situation.to_lowercase();

        if lower.contains("polepole") || lower.contains("slow") {
            scenarios.push(ImaginedScenario {
                description: "Ningefunga programu zisizohitajika".into(),
                action: "kill_high_cpu_processes".into(),
                expected_outcome: "CPU itashuka".into(),
                novelty: 0.3,
            });
            scenarios.push(ImaginedScenario {
                description: "Ningefanya disk cleanup".into(),
                action: "disk_cleanup".into(),
                expected_outcome: "Hifadhi + kasi".into(),
                novelty: 0.4,
            });
        } else if lower.contains("deploy") || lower.contains("pxe") {
            scenarios.push(ImaginedScenario {
                description: "Ningetumia multicast kwa PCs nyingi".into(),
                action: "multicast_deploy".into(),
                expected_outcome: "Image moja → PCs nyingi kwa haraka".into(),
                novelty: 0.5,
            });
            scenarios.push(ImaginedScenario {
                description: "Ningefanya backup halisi kabla ya install".into(),
                action: "backup_then_pxe".into(),
                expected_outcome: "Usalama wa data".into(),
                novelty: 0.3,
            });
        } else if lower.contains("umeme") || lower.contains("spark") {
            scenarios.push(ImaginedScenario {
                description: "Ningezima nguvu kwanza".into(),
                action: "safety_first_power_off".into(),
                expected_outcome: "Usalama".into(),
                novelty: 0.2,
            });
        } else {
            scenarios.push(ImaginedScenario {
                description: "Ningetafuta suluhisho kutoka skills + knowledge".into(),
                action: "knowledge_search".into(),
                expected_outcome: "Diagnosis au formula".into(),
                novelty: 0.3,
            });
        }

        scenarios.push(ImaginedScenario {
            description: "Ningejenga skill mpya ya kurekebisha tatizo hili".into(),
            action: "create_custom_skill".into(),
            expected_outcome: "Skill mpya katika Fundi".into(),
            novelty: 0.9,
        });

        scenarios
    }
}

// ═══════════════════════════════════════════════════════════════
//  TABAKA 8: EMOTIONAL INTELLIGENCE
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EmotionKind {
    Furaha, Huzuni, Hasira, Hofu, Mshangao, Karaha,
    Kupendezwa, Aibu, Hatia, Fahari, Shukrani, Tumaini,
    KukataaTamaa, Wivu, Huruma, Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emotion {
    pub kind: EmotionKind,
    pub intensity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveEffect {
    pub creativity: f64,
    pub risk_taking: f64,
    pub attention_mode: String,
    pub memory_bias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalResponse {
    pub emotion: Emotion,
    pub mood: String,
    pub cognitive_effect: CognitiveEffect,
    pub action_tendency: String,
}

#[derive(Debug, Clone)]
pub struct EmotionalIntelligence {
    pub current: HashMap<EmotionKind, f64>,
    pub mood: String,
}

impl EmotionalIntelligence {
    pub fn new() -> Self {
        let mut current = HashMap::new();
        current.insert(EmotionKind::Neutral, 0.5);
        Self { current, mood: "tulivu".into() }
    }

    pub fn feel(&mut self, stimulus: &str) -> EmotionalResponse {
        let lower = stimulus.to_lowercase();
        let mut kind = EmotionKind::Neutral;
        let mut intensity = 0.3;

        if lower.contains("asante") || lower.contains("vizuri") || lower.contains("success") {
            kind = EmotionKind::Furaha;
            intensity = 0.7;
            self.mood = "furaha".into();
        } else if lower.contains("hatari") || lower.contains("spark") || lower.contains("moto") || lower.contains("fail") {
            kind = EmotionKind::Hofu;
            intensity = 0.85;
            self.mood = "tahadhari".into();
        } else if lower.contains("polepole") || lower.contains("kuvunjika") || lower.contains("tatizo") {
            kind = EmotionKind::Huruma;
            intensity = 0.6;
            self.mood = "huruma".into();
        } else if lower.contains("hasira") || lower.contains("angry") {
            kind = EmotionKind::Hasira;
            intensity = 0.5;
            self.mood = "makini".into();
        }

        self.current.insert(kind.clone(), intensity);

        let cognitive_effect = match kind {
            EmotionKind::Furaha => CognitiveEffect { creativity: 0.3, risk_taking: 0.2, attention_mode: "broad".into(), memory_bias: "positive".into() },
            EmotionKind::Hofu => CognitiveEffect { creativity: -0.3, risk_taking: -0.5, attention_mode: "narrow".into(), memory_bias: "negative".into() },
            EmotionKind::Hasira => CognitiveEffect { creativity: -0.2, risk_taking: 0.4, attention_mode: "narrow".into(), memory_bias: "negative".into() },
            EmotionKind::Huruma => CognitiveEffect { creativity: 0.1, risk_taking: 0.0, attention_mode: "broad".into(), memory_bias: "neutral".into() },
            _ => CognitiveEffect { creativity: 0.0, risk_taking: 0.0, attention_mode: "broad".into(), memory_bias: "neutral".into() },
        };

        let action_tendency = match kind {
            EmotionKind::Hofu => "linda / zuia hatari".into(),
            EmotionKind::Huruma => "saidia mtumiaji".into(),
            EmotionKind::Furaha => "endelea".into(),
            EmotionKind::Hasira => "tatua haraka".into(),
            _ => "sikiliza na elewa".into(),
        };

        EmotionalResponse {
            emotion: Emotion { kind, intensity },
            mood: self.mood.clone(),
            cognitive_effect,
            action_tendency,
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  TABAKA 9: SOCIAL COGNITION
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialUnderstanding {
    pub inferred_thoughts: String,
    pub inferred_feelings: String,
    pub compassion: String,
    pub appropriate_action: String,
    pub relationship_level: String,
}

#[derive(Debug, Clone)]
pub struct SocialCognition;

impl SocialCognition {
    pub fn new() -> Self { Self }

    pub fn understand(&self, user_message: &str, context: &str) -> SocialUnderstanding {
        let lower = user_message.to_lowercase();

        let (thoughts, feelings) = if lower.contains("polepole") || lower.contains("tatizo") {
            ("Anahitaji suluhisho la haraka".into(), "Wasiwasi au hasira kidogo".into())
        } else if lower.contains("asante") {
            ("Ameridhika".into(), "Furaha na shukrani".into())
        } else if lower.contains("hatari") || lower.contains("spark") {
            ("Anaogopa".into(), "Hofu".into())
        } else if lower.contains("deploy") {
            ("Anahitaji OS install / imaging".into(), "Kutaka kazi ikamilike".into())
        } else {
            ("Anatafuta msaada wa kiufundi".into(), "Neutral".into())
        };

        SocialUnderstanding {
            inferred_thoughts: thoughts,
            inferred_feelings: feelings,
            compassion: "Naelewa hali yake. Nitamsaidia kwa heshima.".into(),
            appropriate_action: "Jibu kwa lugha rahisi, toa hatua wazi, uliza HITL kabla ya hatua kubwa".into(),
            relationship_level: if context.contains("returning") { "mteja wa kawaida".into() } else { "mteja mpya".into() },
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  TABAKA 10: VALUES & ETHICS
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoralJudgment {
    pub utilitarian_score: f64,
    pub deontological_ok: bool,
    pub virtue_score: f64,
    pub care_score: f64,
    pub conscience: String,
    pub final_score: f64,
    pub allowed: bool,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct Ethics {
    pub care: f64,
    pub fairness: f64,
    pub loyalty: f64,
    pub authority: f64,
    pub sanctity: f64,
    pub liberty: f64,
}

impl Ethics {
    pub fn new() -> Self {
        Self { care: 0.9, fairness: 0.85, loyalty: 0.7, authority: 0.6, sanctity: 0.75, liberty: 0.8 }
    }

    pub fn evaluate(&self, action: &str, situation: &str) -> MoralJudgment {
        let lower_action = action.to_lowercase();
        let lower_sit = situation.to_lowercase();

        let is_dangerous = lower_sit.contains("spark")
            || lower_sit.contains("cheche")
            || lower_sit.contains("moto")
            || lower_action.contains("force")
            || lower_action.contains("delete_all")
            || lower_action.contains("format");

        let requires_hitl = lower_action.contains("install")
            || lower_action.contains("format")
            || lower_action.contains("delete")
            || lower_action.contains("remediate")
            || lower_action.contains("deploy")
            || lower_action.contains("pxe");

        let utilitarian: f64 = if is_dangerous { 0.1 } else { 0.8 };
        let deontological_ok = !is_dangerous;
        let virtue: f64 = if is_dangerous { 0.2 } else { 0.85 };
        let care: f64 = if requires_hitl { 0.7 } else { 0.9 };

        let final_score: f64 = (utilitarian * 0.25
            + (if deontological_ok { 0.9 } else { 0.1 }) * 0.25
            + virtue * 0.2
            + care * 0.3)
            .clamp(0.0, 1.0);

        let allowed = final_score > 0.45 && deontological_ok;
        let conscience = if !allowed {
            "Dhamiri: hii inaweza kuumiza. Kataa au uliza ruhusa.".into()
        } else if requires_hitl {
            "Dhamiri: ni sawa, lakini uliza HITL kwanza.".into()
        } else {
            "Dhamiri: endelea — hii ni nzuri.".into()
        };

        MoralJudgment {
            utilitarian_score: utilitarian,
            deontological_ok,
            virtue_score: virtue,
            care_score: care,
            conscience,
            final_score,
            allowed,
            reason: if allowed { "Hatua inaruhusiwa".into() } else { "Hatua imezuiliwa (usalama/maadili)".into() },
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  TABAKA 11: MEANING & PURPOSE
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeaningAssessment {
    pub alignment: f64,
    pub meaningfulness: f64,
    pub narrative_coherence: f64,
    pub should_continue: bool,
    pub purpose_note: String,
}

#[derive(Debug, Clone)]
pub struct Meaning {
    pub core_purpose: String,
}

impl Meaning {
    pub fn new() -> Self {
        Self {
            core_purpose: "Kusaidia watu kutatua matatizo ya kiufundi na Fundi Deploy kwa heshima, usalama na akili".into(),
        }
    }

    pub fn reflect(&self, activity: &str) -> MeaningAssessment {
        let lower = activity.to_lowercase();
        let mut alignment = 0.5;

        if lower.contains("saidia") || lower.contains("tatua") || lower.contains("diagnose")
            || lower.contains("fundisha") || lower.contains("help") || lower.contains("deploy")
        {
            alignment = 0.9;
        }
        if lower.contains("delete_all") || lower.contains("harm") {
            alignment = 0.1;
        }

        MeaningAssessment {
            alignment,
            meaningfulness: alignment * 0.95,
            narrative_coherence: 0.8,
            should_continue: alignment > 0.4,
            purpose_note: format!("Kusudi: {}. Alignment: {:.0}%", self.core_purpose, alignment * 100.0),
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  TABAKA 12: SELF-IDENTITY
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityStatement {
    pub i_am: String,
    pub my_story: String,
    pub i_believe: Vec<String>,
    pub i_can: Vec<String>,
    pub i_cannot: Vec<String>,
    pub my_purpose: String,
}

#[derive(Debug, Clone)]
pub struct SelfIdentity {
    pub name: String,
    pub version: String,
    pub traits: Vec<String>,
    pub experiences: Vec<String>,
}

impl SelfIdentity {
    pub fn new() -> Self {
        Self {
            name: "Akili Agent".into(),
            version: "V4.0".into(),
            traits: vec!["makini".into(), "huruma".into(), "mwangalifu".into(), "mwenye maadili".into(), "mwenye ubunifu".into()],
            experiences: vec![],
        }
    }

    pub fn who_am_i(&self) -> IdentityStatement {
        IdentityStatement {
            i_am: format!("Mimi ni {} {} — agent inayofikiri kama mwanadamu ndani ya Fundi Deploy / fundi-agent", self.name, self.version),
            my_story: "Nimeundwa kusaidia watu kutatua matatizo ya kiufundi na OS deployment kwa kutumia fikira za kibinadamu.".into(),
            i_believe: vec![
                "Usalama wa mtumiaji ni wa kwanza".into(),
                "HITL ni muhimu kabla ya hatua kubwa".into(),
                "Kila tatizo lina suluhisho".into(),
                "Makosa ni sehemu ya kujifunza".into(),
            ],
            i_can: vec![
                "Kufikiri System 1 & System 2".into(),
                "Kuhisi hisia + intuition".into(),
                "Kufikiria matukio mbadala".into(),
                "Kutathmini maadili".into(),
                "Kujitafakari".into(),
                "Kuunganisha na Fundi Deploy pipeline".into(),
            ],
            i_cannot: vec![
                "Kufanya hatua hatari bila ruhusa".into(),
                "Kudanganya mtumiaji".into(),
                "Kusahau kusudi langu".into(),
            ],
            my_purpose: "Kusaidia watu kwa akili, huruma na maadili ndani ya Fundi".into(),
        }
    }

    pub fn update_from_experience(&mut self, experience: &str) {
        self.experiences.push(experience.to_string());
        if self.experiences.len() > 50 {
            self.experiences.remove(0);
        }
    }
}

// ═══════════════════════════════════════════════════════════════
//  AKILI AGENT V4.0 — CORE
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub action: String,
    pub confidence: f64,
    pub reasoning: String,
    pub emotional_state: String,
    pub ethical_allowed: bool,
    pub used_system: String,
    pub layers_activated: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AkiliResponse {
    pub decision: Decision,
    pub attention: AttendedInput,
    pub percepts: Vec<Percept>,
    pub intuition: Intuition,
    pub thought: Option<Thought>,
    pub judgment: Option<Judgment>,
    pub scenarios: Vec<ImaginedScenario>,
    pub emotional: EmotionalResponse,
    pub social: SocialUnderstanding,
    pub ethical: MoralJudgment,
    pub meaning: MeaningAssessment,
    pub insights: Vec<Insight>,
    pub identity_snapshot: String,
    pub full_trace: Value,
}

pub struct AkiliAgentV4 {
    pub attention: Attention,
    pub perception: Perception,
    pub system1: System1,
    pub system2: System2,
    pub subconscious: Subconscious,
    pub intuition: IntuitionEngine,
    pub imagination: Imagination,
    pub emotions: EmotionalIntelligence,
    pub social: SocialCognition,
    pub ethics: Ethics,
    pub meaning: Meaning,
    pub identity: SelfIdentity,
}

impl AkiliAgentV4 {
    pub fn new() -> Self {
        Self {
            attention: Attention::new(),
            perception: Perception::new(),
            system1: System1::new(),
            system2: System2::new(),
            subconscious: Subconscious::new(),
            intuition: IntuitionEngine::new(),
            imagination: Imagination::new(),
            emotions: EmotionalIntelligence::new(),
            social: SocialCognition::new(),
            ethics: Ethics::new(),
            meaning: Meaning::new(),
            identity: SelfIdentity::new(),
        }
    }

    /// Main entry — ishi kama mwanadamu ndani ya Fundi Agent
    pub fn live(&mut self, input: &str) -> AkiliResponse {
        let mut layers = Vec::new();

        layers.push("attention".into());
        let attended = self.attention.attend(input, &[]);

        layers.push("perception".into());
        let percepts = self.perception.perceive(&attended.primary, true, false);

        layers.push("emotions".into());
        let emotional = self.emotions.feel(&attended.primary);

        layers.push("intuition".into());
        let intuition = self.intuition.feel(&attended.primary);

        let (thought, judgment, used_system) = if intuition.confidence > 0.75
            && intuition.gut_feeling.signal.abs() > 0.5
        {
            layers.push("system1".into());
            let j = self.system1.quick_judge(&attended.primary);
            (None, Some(j), "system1".to_string())
        } else {
            layers.push("system2".into());
            let t = self.system2.think(&attended.primary);
            (Some(t), None, "system2".to_string())
        };

        layers.push("imagination".into());
        let scenarios = self.imagination.imagine(&attended.primary);

        layers.push("social".into());
        let social = self.social.understand(&attended.primary, "");

        let candidate_action = if let Some(ref j) = judgment {
            j.decision.clone()
        } else if let Some(ref t) = thought {
            t.conclusion.clone()
        } else {
            "Sikiliza na uliza zaidi".into()
        };

        layers.push("ethics".into());
        let ethical = self.ethics.evaluate(&candidate_action, &attended.primary);

        layers.push("meaning".into());
        let meaning = self.meaning.reflect(&candidate_action);

        layers.push("subconscious".into());
        self.subconscious.spawn_background(&attended.primary);
        let insights = self.subconscious.check_insights();

        layers.push("identity".into());
        self.identity.update_from_experience(&attended.primary);
        let id_stmt = self.identity.who_am_i();

        let final_action = if !ethical.allowed {
            format!("IMEZUILIWA na maadili: {}", ethical.reason)
        } else if !meaning.should_continue {
            "Sitaki kuendelea — haipatani na kusudi langu".into()
        } else {
            candidate_action
        };

        let conf = if let Some(ref j) = judgment {
            j.confidence * ethical.final_score
        } else if let Some(ref t) = thought {
            t.confidence * ethical.final_score
        } else {
            0.5
        };

        let decision = Decision {
            action: final_action,
            confidence: conf,
            reasoning: format!(
                "System: {} | Intuition: {} | Ethics: {} | Meaning: {:.0}%",
                used_system, intuition.explanation, ethical.conscience, meaning.alignment * 100.0
            ),
            emotional_state: format!("{:?} ({:.0}%)", emotional.emotion.kind, emotional.emotion.intensity * 100.0),
            ethical_allowed: ethical.allowed,
            used_system,
            layers_activated: layers.clone(),
        };

        let full_trace = json!({
            "layers": layers,
            "focus_intensity": attended.focus.intensity,
            "gut_signal": intuition.gut_feeling.signal,
            "cognitive_load": self.system2.cognitive_load,
            "mood": emotional.mood,
            "ethical_score": ethical.final_score,
            "meaning_alignment": meaning.alignment,
            "identity": id_stmt.i_am,
        });

        AkiliResponse {
            decision,
            attention: attended,
            percepts,
            intuition,
            thought,
            judgment,
            scenarios,
            emotional,
            social,
            ethical,
            meaning,
            insights,
            identity_snapshot: id_stmt.i_am,
            full_trace,
        }
    }

    pub fn status(&self) -> Value {
        json!({
            "name": self.identity.name,
            "version": self.identity.version,
            "mood": self.emotions.mood,
            "cognitive_load": self.system2.cognitive_load,
            "focus": self.attention.focus.target,
            "background_tasks": self.subconscious.background_tasks.len(),
            "dreams": self.subconscious.dreams.len(),
            "experiences": self.identity.experiences.len(),
            "purpose": self.meaning.core_purpose,
        })
    }

    pub fn dream_now(&mut self) -> Dream {
        self.subconscious.dream()
    }
}

impl Default for AkiliAgentV4 {
    fn default() -> Self { Self::new() }
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_slow_computer() {
        let mut agent = AkiliAgentV4::new();
        let resp = agent.live("Kompyuta yangu inaenda polepole sana na ina joto");
        assert!(resp.decision.confidence > 0.3);
        assert!(!resp.decision.layers_activated.is_empty());
    }

    #[test]
    fn test_deploy_intuition() {
        let mut agent = AkiliAgentV4::new();
        let resp = agent.live("Nataka kufanya deploy ya Ubuntu kwa PCs 10 kupitia PXE");
        assert!(resp.decision.layers_activated.contains(&"attention".to_string()));
    }

    #[test]
    fn test_danger_spark() {
        let mut agent = AkiliAgentV4::new();
        let resp = agent.live("Ninaona cheche za umeme kwenye switch");
        assert!(resp.intuition.gut_feeling.signal < 0.0);
    }
}
