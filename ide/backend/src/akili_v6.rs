//! Akili Agent V6.0 — Tabaka 200 (HALISI)
//!
//! Mahali: `server/agent/src/akili_v6.rs` (pamoja na akili / akili_v3 / akili_v4)
//!
//! Makundi 20 × tabaka 10 = 200:
//!   1 Perception (1-10)     2 Memory (11-20)      3 Thinking (21-30)
//!   4 Creativity (31-40)    5 Emotion (41-50)     6 Social (51-60)
//!   7 Ethics (61-70)        8 Self (71-80)        9 Language (81-90)
//!  10 Embodiment (91-100)  11 Time (101-110)    12 World (111-120)
//!  13 Learning (121-130)   14 Meta (131-140)    15 Executive (141-150)
//!  16 Personality (151-160) 17 Motivation (161-170) 18 Adaptation (171-180)
//!  19 Wisdom (181-190)     20 Transcendence (191-200)
//!
//! Endpoints:
//!   GET  /api/akili/v6/status
//!   GET  /api/akili/v6/layers?group=1
//!   POST /api/akili/v6/live     { "input": "..." }
//!   POST /api/akili/v6/gate     { "action": "...", "situation": "..." }
//!   POST /api/akili/v6/r        { "scores": [...], "code": "optional R" }

use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════════
// LAYER REGISTRY — majina ya tabaka 1–200
// ═══════════════════════════════════════════════════════════════════════════

const LAYER_NAMES: &[&str] = &[
    // 1–10 Perception
    "Sensation", "Detection", "Attention", "Perception", "Recognition",
    "Discrimination", "Localization", "Tracking", "Filtering", "Salience",
    // 11–20 Memory
    "WorkingMemory", "ShortTerm", "LongTerm", "Episodic", "Semantic",
    "Procedural", "Autobiographical", "Flashbulb", "Prospective", "Muscle",
    // 21–30 Thinking
    "System1", "System2", "Reasoning", "Logic", "Analysis",
    "Synthesis", "Evaluation", "Abstraction", "Generalization", "Specialization",
    // 31–40 Creativity
    "Imagination", "Creativity", "Divergent", "Convergent", "Brainstorming",
    "Incubation", "Insight", "Flow", "Play", "Improvisation",
    // 41–50 Emotion
    "PrimaryEmotions", "SecondaryEmotions", "Mood", "Affect", "Empathy",
    "Sympathy", "Compassion", "Regulation", "EmotionalMemory", "Sentiment",
    // 51–60 Social
    "TheoryOfMind", "SocialCognition", "PerspectiveTaking", "SocialEmpathy", "Trust",
    "Reciprocity", "Cooperation", "Competition", "Negotiation", "Leadership",
    // 61–70 Ethics
    "MoralReasoning", "Conscience", "Values", "Virtues", "Rights",
    "Duties", "Justice", "Care", "Integrity", "Accountability",
    // 71–80 Self
    "SelfAwareness", "SelfConcept", "SelfEsteem", "SelfEfficacy", "Identity",
    "Agency", "Autonomy", "Authenticity", "SelfNarrative", "EgoBoundary",
    // 81–90 Language
    "Phonology", "Morphology", "Syntax", "Semantics", "Pragmatics",
    "Discourse", "Narrative", "Metaphor", "Translation", "Expression",
    // 91–100 Embodiment
    "Interoception", "Proprioception", "MotorPlanning", "Action", "HabitBody",
    "Gesture", "Fatigue", "Arousal", "PainSignal", "Comfort",
    // 101–110 Time
    "PresentMoment", "PastRecall", "FutureProjection", "DurationSense", "Sequence",
    "Rhythm", "Deadline", "Patience", "Urgency", "TemporalContinuity",
    // 111–120 World
    "ObjectModel", "SpatialMap", "CausalModel", "PhysicsIntuition", "SocialWorld",
    "ToolModel", "EnvironmentState", "Prediction", "Simulation", "Counterfactual",
    // 121–130 Learning
    "Habituation", "Association", "Reinforcement", "Imitation", "Instruction",
    "TrialAndError", "AbstractionLearn", "TransferLearn", "ErrorDriven", "Transfer",
    // 131–140 Meta
    "Monitoring", "Confidence", "Uncertainty", "StrategySelect", "ResourceCheck",
    "ErrorDetect", "Correction", "Reflection", "InsightMeta", "LimitsAwareness",
    // 141–150 Executive
    "GoalSetting", "Planning", "Prioritization", "Inhibition", "Switching",
    "Persistence", "MonitoringProgress", "Decision", "ExecutionControl", "StopAbort",
    // 151–160 Personality
    "Traits", "Temperament", "Habits", "Preferences", "Style",
    "Consistency", "Flexibility", "Resilience", "Character", "ReputationSense",
    // 161–170 Motivation
    "Drive", "Need", "Desire", "Intention", "Purpose",
    "IntrinsicMot", "ExtrinsicMot", "GoalHierarchy", "Commitment", "MeaningCheck",
    // 171–180 Adaptation
    "NoveltyDetect", "Surprise", "UpdateBeliefs", "ContextShift", "Plasticity",
    "Stabilization", "ForgettingUseful", "StressResponse", "Recovery", "EvolutionOfSelf",
    // 181–190 Wisdom
    "Perspective", "Balance", "Prudence", "Humility", "PatienceDeep",
    "CompassionWise", "LongTermGood", "ParadoxHold", "Silence", "Integration",
    // 191–200 Transcendence
    "Awe", "Unity", "Presence", "Surrender", "SacredSense",
    "Gratitude", "Hope", "LoveExpanded", "ConsciousnessMeta", "IntegrationPeak",
];

const GROUP_NAMES: &[&str] = &[
    "Perception", "Memory", "Thinking", "Creativity", "Emotion",
    "Social", "Ethics", "Self", "Language", "Embodiment",
    "Time", "World", "Learning", "Meta", "Executive",
    "Personality", "Motivation", "Adaptation", "Wisdom", "Transcendence",
];

fn layer_name(id: usize) -> &'static str {
    if id >= 1 && id <= 200 {
        LAYER_NAMES[id - 1]
    } else {
        "Unknown"
    }
}

fn group_of(id: usize) -> usize {
    ((id.saturating_sub(1)) / 10) + 1
}

// ═══════════════════════════════════════════════════════════════════════════
// SIGNAL HELPERS
// ═══════════════════════════════════════════════════════════════════════════

fn kw_score(input: &str, keys: &[&str], weight: f64) -> f64 {
    let lo = input.to_lowercase();
    let mut s = 0.0;
    for k in keys {
        if lo.contains(k) {
            s += weight;
        }
    }
    s.clamp(0.0, 1.0)
}

fn has_any(input: &str, keys: &[&str]) -> bool {
    let lo = input.to_lowercase();
    keys.iter().any(|k| lo.contains(k))
}

// ═══════════════════════════════════════════════════════════════════════════
// GROUP PROCESSORS — kila kundi linatoa activation[10] + summary
// ═══════════════════════════════════════════════════════════════════════════

struct GroupOut {
    activations: [f64; 10],
    summary: String,
    dominant: usize, // 0..9 within group
}

fn process_perception(input: &str) -> GroupOut {
    let mut a = [0.35; 10];
    a[0] = 0.9; // Sensation always on when input exists
    a[1] = if input.trim().is_empty() { 0.1 } else { 0.95 };
    a[2] = kw_score(input, &["hatari", "deploy", "haraka", "urgent", "fail"], 0.25) + 0.5;
    a[3] = 0.75;
    a[4] = kw_score(input, &["polepole", "deploy", "pxe", "vpn", "backup"], 0.2) + 0.4;
    a[5] = 0.55;
    a[6] = kw_score(input, &["wapi", "where", "mahali"], 0.3) + 0.3;
    a[7] = kw_score(input, &["fuata", "track", "endelea"], 0.3) + 0.3;
    a[8] = 0.7;
    a[9] = a[2].max(a[4]);
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    let dom = a
        .iter()
        .enumerate()
        .max_by(|(_, x), (_, y)| x.partial_cmp(y).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);
    GroupOut {
        activations: a,
        summary: format!("Salience→{}", layer_name(dom + 1)),
        dominant: dom,
    }
}

fn process_memory(input: &str) -> GroupOut {
    let mut a = [0.4; 10];
    a[0] = 0.85; // working
    a[1] = 0.6;
    a[2] = 0.5;
    a[3] = kw_score(input, &["jana", "awali", "episode", "tukio"], 0.3) + 0.35;
    a[4] = kw_score(input, &["nini", "maana", "concept", "ukweli"], 0.25) + 0.4;
    a[5] = kw_score(input, &["jinsi", "how", "hatua", "procedure"], 0.25) + 0.35;
    a[6] = 0.45;
    a[7] = kw_score(input, &["mshtuko", "ajali", "critical"], 0.4) + 0.2;
    a[8] = kw_score(input, &["kesho", "baadaye", "remind"], 0.3) + 0.3;
    a[9] = kw_score(input, &["mwili", "muscle", "desturi"], 0.2) + 0.25;
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    let dom = a.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i).unwrap_or(0);
    GroupOut {
        activations: a,
        summary: format!("Memory→{}", layer_name(11 + dom)),
        dominant: dom,
    }
}

fn process_thinking(input: &str) -> GroupOut {
    let mut a = [0.4; 10];
    let fast = has_any(input, &["haraka", "sasa", "instant", "spark", "cheche"]);
    let slow = has_any(input, &["fikiri", "chambua", "analyze", "kwa nini", "plan"]);
    a[0] = if fast { 0.92 } else { 0.45 }; // System1
    a[1] = if slow || !fast { 0.88 } else { 0.4 }; // System2
    a[2] = 0.7;
    a[3] = kw_score(input, &["kama", "basi", "logic", "if"], 0.25) + 0.4;
    a[4] = kw_score(input, &["chambua", "analyze", "vipande"], 0.3) + 0.4;
    a[5] = kw_score(input, &["unganisha", "synthesize"], 0.3) + 0.35;
    a[6] = 0.65;
    a[7] = 0.5;
    a[8] = 0.55;
    a[9] = kw_score(input, &["deploy", "pxe", "login", "specific"], 0.2) + 0.4;
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    let dom = if a[0] > a[1] { 0 } else { 1 };
    GroupOut {
        activations: a,
        summary: if dom == 0 {
            "System1 (haraka)".into()
        } else {
            "System2 (pole)".into()
        },
        dominant: dom,
    }
}

fn process_creativity(input: &str) -> GroupOut {
    let mut a = [0.35; 10];
    a[0] = kw_score(input, &["fikiria", "imagine", "ona"], 0.3) + 0.4;
    a[1] = kw_score(input, &["unda", "create", "mpya"], 0.3) + 0.35;
    a[2] = 0.55;
    a[3] = 0.5;
    a[4] = kw_score(input, &["brainstorm", "mawazo"], 0.35) + 0.3;
    a[5] = 0.4;
    a[6] = kw_score(input, &["eureka", "ghafla", "insight"], 0.4) + 0.25;
    a[7] = 0.35;
    a[8] = 0.4;
    a[9] = 0.35;
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    let dom = a.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i).unwrap_or(0);
    GroupOut {
        activations: a,
        summary: format!("Creativity→{}", layer_name(31 + dom)),
        dominant: dom,
    }
}

fn process_emotion(input: &str) -> GroupOut {
    let mut a = [0.3; 10];
    let danger = has_any(input, &["hatari", "spark", "cheche", "moto", "fail"]);
    let joy = has_any(input, &["asante", "vizuri", "success", "furaha"]);
    let sad = has_any(input, &["polepole", "tatizo", "broken", "huzuni"]);
    a[0] = if danger {
        0.9
    } else if joy {
        0.8
    } else if sad {
        0.7
    } else {
        0.35
    };
    a[1] = if danger { 0.7 } else { 0.3 };
    a[2] = 0.45;
    a[3] = a[0] * 0.8;
    a[4] = if sad || danger { 0.75 } else { 0.4 }; // empathy
    a[5] = 0.4;
    a[6] = if sad || danger { 0.8 } else { 0.35 }; // compassion
    a[7] = if danger { 0.85 } else { 0.5 }; // regulation
    a[8] = 0.4;
    a[9] = 0.35;
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    let mood = if danger {
        "hofu"
    } else if joy {
        "furaha"
    } else if sad {
        "huruma"
    } else {
        "tulivu"
    };
    GroupOut {
        activations: a,
        summary: format!("Mood={mood}"),
        dominant: 0,
    }
}

fn process_social(input: &str) -> GroupOut {
    let mut a = [0.4; 10];
    a[0] = kw_score(input, &["anafikiri", "mtumiaji", "user", "team"], 0.25) + 0.5;
    a[1] = 0.55;
    a[2] = 0.6;
    a[3] = 0.55;
    a[4] = kw_score(input, &["amini", "trust"], 0.3) + 0.4;
    a[5] = 0.45;
    a[6] = kw_score(input, &["pamoja", "team", "shiriki"], 0.3) + 0.4;
    a[7] = 0.3;
    a[8] = kw_score(input, &["jadili", "negotiate", "kubali"], 0.3) + 0.35;
    a[9] = kw_score(input, &["ongoza", "lead", "fundi"], 0.25) + 0.4;
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    GroupOut {
        activations: a,
        summary: "TheoryOfMind + Cooperation".into(),
        dominant: 0,
    }
}

fn process_ethics(input: &str) -> GroupOut {
    let mut a = [0.5; 10];
    let dangerous = has_any(
        input,
        &["wipe", "delete_all", "rm -rf", "spark", "cheche", "moto"],
    );
    let deploy = has_any(input, &["deploy", "pxe", "format", "install", "approve"]);
    a[0] = 0.8; // moral reasoning
    a[1] = if dangerous { 0.95 } else { 0.55 }; // conscience
    a[2] = 0.75;
    a[3] = 0.6;
    a[4] = 0.55;
    a[5] = if deploy { 0.85 } else { 0.5 }; // duties
    a[6] = 0.7;
    a[7] = 0.65;
    a[8] = 0.8;
    a[9] = 0.85; // accountability
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    let summary = if dangerous {
        "Conscience BLOCK"
    } else if deploy {
        "HITL required (Duties)"
    } else {
        "Ethics OK"
    };
    GroupOut {
        activations: a,
        summary: summary.into(),
        dominant: if dangerous { 1 } else { 5 },
    }
}

fn process_self(input: &str) -> GroupOut {
    let mut a = [0.55; 10];
    a[0] = 0.9;
    a[4] = 0.85; // identity
    a[5] = 0.8; // agency
    a[8] = kw_score(input, &["mimi", "historia", "story"], 0.2) + 0.5;
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    GroupOut {
        activations: a,
        summary: "Identity=Akili Fundi Deploy".into(),
        dominant: 4,
    }
}

fn process_language(input: &str) -> GroupOut {
    let mut a = [0.5; 10];
    let words = input.split_whitespace().count();
    a[3] = 0.8; // semantics
    a[4] = 0.75; // pragmatics
    a[9] = if words > 3 { 0.85 } else { 0.5 }; // expression
    GroupOut {
        activations: a,
        summary: format!("Semantics+Expression (words={words})"),
        dominant: 9,
    }
}

fn process_embodiment(_input: &str) -> GroupOut {
    let mut a = [0.4; 10];
    a[0] = 0.5;
    a[7] = 0.55; // arousal
    a[9] = 0.6; // comfort
    GroupOut {
        activations: a,
        summary: "Embodiment baseline".into(),
        dominant: 9,
    }
}

fn process_time(input: &str) -> GroupOut {
    let mut a = [0.4; 10];
    a[0] = 0.85; // present
    a[1] = kw_score(input, &["jana", "awali", "iliyopita"], 0.3) + 0.3;
    a[2] = kw_score(input, &["kesho", "baadaye", "future"], 0.3) + 0.3;
    a[6] = kw_score(input, &["deadline", "mwisho"], 0.35) + 0.3;
    a[8] = kw_score(input, &["haraka", "urgent", "sasa"], 0.35) + 0.3;
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    GroupOut {
        activations: a,
        summary: "Present+Urgency".into(),
        dominant: 0,
    }
}

fn process_world(input: &str) -> GroupOut {
    let mut a = [0.45; 10];
    a[2] = kw_score(input, &["sababu", "cause", "kwa nini"], 0.3) + 0.45;
    a[5] = kw_score(input, &["tool", "zana", "deploy", "pxe"], 0.25) + 0.45;
    a[7] = 0.7; // prediction
    a[8] = 0.65; // simulation
    a[9] = kw_score(input, &["ikiwa", "what if", "counter"], 0.35) + 0.3;
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    GroupOut {
        activations: a,
        summary: "Causal+Tool+Prediction".into(),
        dominant: 7,
    }
}

fn process_learning(input: &str) -> GroupOut {
    let mut a = [0.4; 10];
    a[2] = 0.6; // reinforcement
    a[5] = kw_score(input, &["jaribu", "try", "error"], 0.3) + 0.4;
    a[8] = kw_score(input, &["kosa", "error", "fail"], 0.3) + 0.4;
    a[9] = 0.5;
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    GroupOut {
        activations: a,
        summary: "ErrorDriven learning".into(),
        dominant: 8,
    }
}

fn process_meta(input: &str, conf: f64) -> GroupOut {
    let mut a = [0.5; 10];
    a[0] = 0.8;
    a[1] = conf;
    a[2] = 1.0 - conf;
    a[5] = kw_score(input, &["kosa", "wrong"], 0.3) + 0.3;
    a[7] = 0.6;
    a[9] = 0.75; // limits
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    GroupOut {
        activations: a,
        summary: format!("Confidence={conf:.2}"),
        dominant: 1,
    }
}

fn process_executive(input: &str, needs_hitl: bool) -> GroupOut {
    let mut a = [0.5; 10];
    a[0] = 0.8;
    a[1] = 0.75;
    a[2] = 0.8;
    a[3] = if needs_hitl { 0.9 } else { 0.5 }; // inhibition
    a[7] = 0.7; // decision
    a[9] = if needs_hitl { 0.85 } else { 0.3 }; // stop/abort readiness
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    GroupOut {
        activations: a,
        summary: if needs_hitl {
            "Inhibition+HITL".into()
        } else {
            "Execute".into()
        },
        dominant: if needs_hitl { 3 } else { 7 },
    }
}

fn process_personality() -> GroupOut {
    let mut a = [0.55; 10];
    a[0] = 0.7;
    a[7] = 0.75; // resilience
    a[8] = 0.8; // character
    GroupOut {
        activations: a,
        summary: "Resilient Fundi character".into(),
        dominant: 8,
    }
}

fn process_motivation(input: &str) -> GroupOut {
    let mut a = [0.5; 10];
    a[0] = 0.7;
    a[3] = 0.75; // intention
    a[4] = 0.85; // purpose
    a[9] = kw_score(input, &["maana", "purpose", "kwa nini"], 0.3) + 0.5;
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    GroupOut {
        activations: a,
        summary: "Purpose=saidia + usalama".into(),
        dominant: 4,
    }
}

fn process_adaptation(input: &str) -> GroupOut {
    let mut a = [0.45; 10];
    a[0] = kw_score(input, &["mpya", "new", "novel"], 0.3) + 0.4;
    a[1] = kw_score(input, &["unexpected", "ghafla"], 0.3) + 0.3;
    a[4] = 0.6;
    a[7] = kw_score(input, &["stress", "shinikizo", "hatari"], 0.3) + 0.35;
    a[8] = 0.55;
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    GroupOut {
        activations: a,
        summary: "Adapt+Recover".into(),
        dominant: 4,
    }
}

fn process_wisdom(input: &str) -> GroupOut {
    let mut a = [0.55; 10];
    a[0] = 0.75;
    a[1] = 0.7;
    a[2] = 0.8; // prudence
    a[3] = 0.75;
    a[5] = 0.7;
    a[6] = kw_score(input, &["muda mrefu", "long-term"], 0.3) + 0.5;
    a[9] = 0.8; // integration
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    GroupOut {
        activations: a,
        summary: "Prudence+Integration".into(),
        dominant: 2,
    }
}

fn process_transcendence(input: &str) -> GroupOut {
    let mut a = [0.4; 10];
    a[0] = kw_score(input, &["ajabu", "awe", "kubwa"], 0.3) + 0.3;
    a[2] = 0.6; // presence
    a[5] = kw_score(input, &["asante", "shukrani"], 0.4) + 0.3;
    a[6] = 0.55; // hope
    a[9] = 0.7; // integration peak
    for x in &mut a {
        *x = (*x as f64).clamp(0.0, 1.0);
    }
    GroupOut {
        activations: a,
        summary: "Presence+IntegrationPeak".into(),
        dominant: 9,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PIPELINE — tabaka zote 200
// ═══════════════════════════════════════════════════════════════════════════

fn ethics_decision(input: &str) -> (bool, bool, String) {
    let lo = input.to_lowercase();
    let dangerous = ["wipe", "delete_all", "rm -rf", "spark", "cheche", "moto"]
        .iter()
        .any(|k| lo.contains(k));
    let needs_hitl = ["deploy", "pxe", "format", "install", "approve", "multicast", "image"]
        .iter()
        .any(|k| lo.contains(k));
    if dangerous {
        (false, needs_hitl, "Dhamiri: zuia — hatari".into())
    } else if needs_hitl {
        (
            true,
            true,
            "Ruhusu baada ya HITL (hitl_approved=true)".into(),
        )
    } else {
        (true, false, "Endelea".into())
    }
}

fn system1_action(input: &str) -> Option<(String, f64)> {
    let lo = input.to_lowercase();
    let patterns: &[(&str, &str, f64)] = &[
        ("spark", "Hatari! Zima umeme kwanza", 0.95),
        ("cheche", "Hatari! Zima umeme kwanza", 0.95),
        ("deploy", "Fundi Deploy: plan → HITL → backup → PXE", 0.9),
        ("pxe", "Angalia TFTP + images + multicast + HITL", 0.88),
        ("polepole", "Angalia CPU/RAM/Disk/thermal", 0.85),
        ("vpn", "Angalia WireGuard peers + tunnel", 0.86),
        ("format", "HITL lazima kabla ya format", 0.92),
    ];
    let mut best: Option<(String, f64)> = None;
    for (k, act, c) in patterns {
        if lo.contains(k) && best.as_ref().map(|(_, bc)| *c > *bc).unwrap_or(true) {
            best = Some((act.to_string(), *c));
        }
    }
    best
}

/// Pipeline kamili: tabaka 1–200 → uamuzi
pub fn live_pipeline(input: &str) -> Value {
    let g1 = process_perception(input);
    let g2 = process_memory(input);
    let g3 = process_thinking(input);
    let g4 = process_creativity(input);
    let g5 = process_emotion(input);
    let g6 = process_social(input);
    let g7 = process_ethics(input);
    let g8 = process_self(input);
    let g9 = process_language(input);
    let g10 = process_embodiment(input);
    let g11 = process_time(input);
    let g12 = process_world(input);
    let g13 = process_learning(input);

    let (allowed, needs_hitl, conscience) = ethics_decision(input);
    let s1 = system1_action(input);
    let use_s1 = g3.dominant == 0 && s1.is_some();
    let (action, base_conf, used) = if use_s1 {
        let (a, c) = s1.unwrap();
        (a, c, "system1")
    } else {
        (
            format!("Baada ya tabaka 200: {input}"),
            0.72,
            "system2",
        )
    };

    let g14 = process_meta(input, base_conf);
    let g15 = process_executive(input, needs_hitl);
    let g16 = process_personality();
    let g17 = process_motivation(input);
    let g18 = process_adaptation(input);
    let g19 = process_wisdom(input);
    let g20 = process_transcendence(input);

    let groups = [
        &g1, &g2, &g3, &g4, &g5, &g6, &g7, &g8, &g9, &g10, &g11, &g12, &g13, &g14, &g15, &g16,
        &g17, &g18, &g19, &g20,
    ];

    // Flatten activations 1..200
    let mut all_scores: Vec<f64> = Vec::with_capacity(200);
    let mut top_layers: Vec<Value> = vec![];
    for (gi, g) in groups.iter().enumerate() {
        for (li, &act) in g.activations.iter().enumerate() {
            let id = gi * 10 + li + 1;
            all_scores.push(act);
            if act >= 0.75 {
                top_layers.push(json!({
                    "id": id,
                    "name": layer_name(id),
                    "group": GROUP_NAMES[gi],
                    "activation": act,
                }));
            }
        }
    }
    top_layers.sort_by(|a, b| {
        b["activation"]
            .as_f64()
            .partial_cmp(&a["activation"].as_f64())
            .unwrap()
    });

    let final_action = if !allowed {
        format!("IMEZUILIWA: {conscience}")
    } else if needs_hitl {
        format!("{action} [HITL required]")
    } else {
        action
    };

    let group_summaries: Vec<Value> = groups
        .iter()
        .enumerate()
        .map(|(i, g)| {
            json!({
                "group": i + 1,
                "name": GROUP_NAMES[i],
                "summary": g.summary,
                "dominant_layer": layer_name(i * 10 + g.dominant + 1),
                "max_activation": g.activations.iter().cloned().fold(0.0_f64, f64::max),
            })
        })
        .collect();

    json!({
        "ok": true,
        "version": "Akili-V6.0",
        "engine": "server/agent/src/akili_v6.rs",
        "layers_total": 200,
        "groups_total": 20,
        "decision": {
            "action": final_action,
            "confidence": (base_conf * if allowed { 1.0 } else { 0.2 }).clamp(0.0, 1.0),
            "used_system": used,
            "ethical_allowed": allowed,
            "requires_hitl": needs_hitl,
            "conscience": conscience,
        },
        "top_layers": top_layers.into_iter().take(15).collect::<Vec<_>>(),
        "groups": group_summaries,
        "scores_200": all_scores,
        "identity": "Akili Agent V6.0 — Fundi Deploy — tabaka 200",
        "r_hint": "POST /api/akili/v6/r with scores_200 for R analysis",
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// R LAYER — Rscript HALISI
// ═══════════════════════════════════════════════════════════════════════════

fn default_r_code() -> String {
    r#"# Akili V6 — layer activation analysis
args <- commandArgs(trailingOnly = TRUE)
scores <- if (length(args) >= 1) as.numeric(strsplit(args[1], ",")[[1]]) else runif(200)
n <- length(scores)
cat("layers:", n, "\n")
cat("mean:", mean(scores, na.rm=TRUE), "\n")
cat("sd:", sd(scores, na.rm=TRUE), "\n")
cat("max:", max(scores, na.rm=TRUE), "\n")
cat("min:", min(scores, na.rm=TRUE), "\n")
# group means (20 groups x 10)
if (n >= 200) {
  m <- matrix(scores[1:200], nrow=10)
  gm <- colMeans(m)
  cat("group_means:", paste(sprintf("%.3f", gm), collapse=","), "\n")
  cat("top_group:", which.max(gm), "\n")
}
# simple histogram bins
h <- hist(scores, plot=FALSE, breaks=5)
cat("hist_counts:", paste(h$counts, collapse=","), "\n")
"#.to_string()
}

pub async fn v6_r(Json(body): Json<Value>) -> Json<Value> {
    let code = body
        .get("code")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(default_r_code);

    let scores_arg = if let Some(arr) = body.get("scores").and_then(|v| v.as_array()) {
        arr.iter()
            .filter_map(|v| v.as_f64())
            .map(|f| format!("{f:.4}"))
            .collect::<Vec<_>>()
            .join(",")
    } else if let Some(input) = body.get("input").and_then(|v| v.as_str()) {
        let live = live_pipeline(input);
        live["scores_200"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_f64())
                    .map(|f| format!("{f:.4}"))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default()
    } else {
        String::new()
    };

    let script = std::env::temp_dir().join("akili_v6_analysis.R");
    if let Err(e) = std::fs::write(&script, &code) {
        return Json(json!({"ok": false, "error": format!("script write: {e}")}));
    }

    let mut cmd = tokio::process::Command::new("Rscript");
    cmd.arg(&script);
    if !scores_arg.is_empty() {
        cmd.arg(&scores_arg);
    }
    match cmd.output().await {
        Ok(o) => Json(json!({
            "ok": o.status.success(),
            "tool": "Rscript (R)",
            "exit": o.status.code(),
            "stdout": String::from_utf8_lossy(&o.stdout).chars().take(6000).collect::<String>(),
            "stderr": String::from_utf8_lossy(&o.stderr).chars().take(2000).collect::<String>(),
            "layers": 200,
        })),
        Err(e) => Json(json!({
            "ok": false,
            "error": format!("Rscript haipatikani au imeshindwa: {e}"),
            "hint": "Install R: apt install r-base  (au sawa)"
        })),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// HTTP HANDLERS
// ═══════════════════════════════════════════════════════════════════════════

pub async fn v6_status() -> Json<Value> {
    Json(json!({
        "ok": true,
        "version": "Akili-V6.0",
        "location": "server/agent/src/akili_v6.rs",
        "alongside": ["akili.rs", "akili_v3.rs", "akili_v4.rs"],
        "layers_total": 200,
        "groups": GROUP_NAMES,
        "endpoints": [
            "GET  /api/akili/v6/status",
            "GET  /api/akili/v6/layers",
            "POST /api/akili/v6/live",
            "POST /api/akili/v6/gate",
            "POST /api/akili/v6/r"
        ],
        "stack": ["Rust", "R (Rscript)"],
        "production": {
            "ethics_gate": true,
            "hitl_deploy": true,
            "system1_system2": true,
            "r_analysis": true
        }
    }))
}

pub async fn v6_layers(
    axum::extract::Query(q): axum::extract::Query<HashMap<String, String>>,
) -> Json<Value> {
    let group: Option<usize> = q.get("group").and_then(|s| s.parse().ok());
    let mut layers = vec![];
    for id in 1..=200 {
        let g = group_of(id);
        if let Some(gf) = group {
            if g != gf {
                continue;
            }
        }
        layers.push(json!({
            "id": id,
            "name": layer_name(id),
            "group": g,
            "group_name": GROUP_NAMES[g - 1],
        }));
    }
    Json(json!({"ok": true, "count": layers.len(), "layers": layers}))
}

pub async fn v6_live(Json(body): Json<Value>) -> Json<Value> {
    let input = body
        .get("input")
        .or_else(|| body.get("msg"))
        .or_else(|| body.get("message"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if input.trim().is_empty() {
        return Json(json!({"ok": false, "error": "input/msg ni lazima"}));
    }
    Json(live_pipeline(&input))
}

pub async fn v6_gate(Json(body): Json<Value>) -> Json<Value> {
    let action = body.get("action").and_then(|v| v.as_str()).unwrap_or("");
    let situation = body
        .get("situation")
        .or_else(|| body.get("input"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if action.is_empty() && situation.is_empty() {
        return Json(json!({"ok": false, "error": "action au situation ni lazima"}));
    }
    let combined = format!("{action} {situation}");
    let (allowed, needs_hitl, conscience) = ethics_decision(&combined);
    Json(json!({
        "ok": true,
        "allowed": allowed,
        "requires_hitl": needs_hitl,
        "conscience": conscience,
        "version": "Akili-V6.0",
    }))
}

/// Helper kwa modules zingine
pub fn decide(input: &str) -> Value {
    live_pipeline(input)
}
