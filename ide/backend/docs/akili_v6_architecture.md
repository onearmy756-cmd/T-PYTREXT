# Akili Agent V6.0 — Architecture & Data Flow

**Mahali:** `server/agent/` (Fundi Deploy)  
**Engine:** `src/akili_v6.rs` · **Bridge:** `src/akili_bridge.rs` · **R:** Rscript  
**Tabaka:** 200 · **Makundi:** 20

---

## Architecture kuu

```
INPUT (dunia / user / tools)
    ↓
[1–10]   Perception
[11–20]  Memory
[21–30]  Thinking
[31–40]  Creativity
[41–50]  Emotion
[51–60]  Social
[61–70]  Ethics
[71–80]  Self
[81–90]  Language
[91–100] Embodiment
[101–110] Time
[111–120] World Model
[121–130] Learning
[131–140] Meta-Cognition
[141–150] Executive
[151–160] Personality / Habits
[161–170] Motivation
[171–180] Adaptation
[181–190] Wisdom
[191–200] Transcendence
    ↓
OUTPUT (uamuzi · HITL · tool · jibu) + R analysis
```

---

## Kundi 1 — Perception (1–10)

```
INPUT → Sensation → Detection → Attention → Perception
      → Recognition → Discrimination → Localization
      → Tracking → Filtering → Salience → MEMORY
```

| # | Tabaka | Kazi | Output |
|---|--------|------|--------|
| 1 | Sensation | Hisia ghafi | Signal |
| 2 | Detection | Kuna kitu? | Boolean |
| 3 | Attention | Chagua focus | Focus |
| 4 | Perception | Elewa ni nini | Concept |
| 5 | Recognition | Tambua | Identity |
| 6 | Discrimination | Tofautisha | Difference |
| 7 | Localization | Wapi? | Position |
| 8 | Tracking | Fuata | Path |
| 9 | Filtering | Ondoa kelele | Clean |
| 10 | Salience | Kipaumbele | Ranked |

---

## Kundi 2 — Memory (11–20)

```
Perception → Working → Short-Term → Long-Term
              ├── Episodic · Semantic · Procedural
              ├── Autobiographical · Flashbulb
              └── Prospective · Muscle → THINKING
```

| # | Tabaka | Muda | Uwezo |
|---|--------|------|-------|
| 11 | Working Memory | ~30s | 7±2 |
| 12 | Short-Term | Dakika–saa | 100+ |
| 13 | Long-Term | Milele | ∞ |
| 14–20 | Episodic…Muscle | Milele | ∞ |

---

## Kundi 3 — Thinking (21–30)

| # | Tabaka | Aina |
|---|--------|------|
| 21 | System 1 | Haraka, intuitive |
| 22 | System 2 | Pole, analytical |
| 23 | Reasoning | Hoja |
| 24 | Logic | Mantiki |
| 25 | Analysis | Gawanya |
| 26 | Synthesis | Unganisha |
| 27 | Evaluation | Pima |
| 28 | Abstraction | Dhana |
| 29 | Generalization | Jumlisha |
| 30 | Specialization | Bainisha |

---

## Kundi 4 — Creativity (31–40)

Imagination · Creativity · Divergent · Convergent · Brainstorming ·  
Incubation · Insight · Flow · Play · Improvisation

---

## Kundi 5 — Emotion (41–50)

Primary · Secondary · Mood · Affect · Empathy · Sympathy ·  
Compassion · Regulation · Emotional Memory · Sentiment

---

## Kundi 6 — Social (51–60)

Theory of Mind · Social Cognition · Perspective Taking · Social Empathy ·  
Trust · Reciprocity · Cooperation · Competition · Negotiation · Leadership

---

## Kundi 7 — Ethics (61–70)

Moral Reasoning · Conscience · Values · Virtues · Rights ·  
Duties · Justice · Care · Integrity · Accountability

**Fundi:** deploy/PXE/format → **HITL**; wipe/spark → **BLOCK**.

---

## Kundi 8 — Self (71–80)

Self-Awareness · Self-Concept · Self-Esteem · Self-Efficacy · Identity ·  
Agency · Autonomy · Authenticity · Self-Narrative · Ego Boundary

---

## Kundi 9 — Language (81–90)

Phonology · Morphology · Syntax · Semantics · Pragmatics ·  
Discourse · Narrative · Metaphor · Translation · Expression

---

## Kundi 10 — Embodiment (91–100)

Interoception · Proprioception · Motor Planning · Action · Habit Body ·  
Gesture · Fatigue · Arousal · Pain Signal · Comfort

---

## Kundi 11 — Time (101–110)

Present · Past Recall · Future Projection · Duration · Sequence ·  
Rhythm · Deadline · Patience · Urgency · Temporal Continuity

---

## Kundi 12 — World Model (111–120)

Object · Spatial · Causal · Physics Intuition · Social World ·  
Tool Model · Environment · Prediction · Simulation · Counterfactual

---

## Kundi 13 — Learning (121–130)

Habituation · Association · Reinforcement · Imitation · Instruction ·  
Trial & Error · Abstraction Learn · Transfer Learn · Error-Driven · Transfer

---

## Kundi 14 — Meta-Cognition (131–140)

Monitoring · Confidence · Uncertainty · Strategy · Resource Check ·  
Error Detect · Correction · Reflection · Insight Meta · Limits Awareness

---

## Kundi 15 — Executive (141–150)

Goal Setting · Planning · Prioritization · Inhibition · Switching ·  
Persistence · Progress Monitor · Decision · Execution Control · Stop/Abort

---

## Kundi 16 — Personality / Habits (151–160)

Traits · Temperament · Habits · Preferences · Style ·  
Consistency · Flexibility · Resilience · Character · Reputation Sense

---

## Kundi 17 — Motivation (161–170)

Drive · Need · Desire · Intention · Purpose ·  
Intrinsic · Extrinsic · Goal Hierarchy · Commitment · Meaning Check

---

## Kundi 18 — Adaptation (171–180)

Novelty · Surprise · Update Beliefs · Context Shift · Plasticity ·  
Stabilization · Useful Forgetting · Stress Response · Recovery · Evolution of Self

---

## Kundi 19 — Wisdom (181–190)

Perspective · Balance · Prudence · Humility · Patience Deep ·  
Compassion Wise · Long-term Good · Paradox Hold · Silence · Integration

---

## Kundi 20 — Transcendence (191–200)

Awe · Unity · Presence · Surrender · Sacred Sense ·  
Gratitude · Hope · Love Expanded · Consciousness Meta · Integration Peak

---

## Data flow kamili (1 → 200 → output)

```
User / Event / Tool
        │
        ▼
┌───────────────────┐
│ 1 Perception      │ → signal, focus, salience
└─────────┬─────────┘
          ▼
┌───────────────────┐
│ 2 Memory          │ → working / episodic / semantic
└─────────┬─────────┘
          ▼
┌───────────────────┐
│ 3–4 Thinking+Create│ → System1/2, idea
└─────────┬─────────┘
          ▼
┌───────────────────┐
│ 5–6 Emotion+Social │ → mood, ToM, trust
└─────────┬─────────┘
          ▼
┌───────────────────┐
│ 7 Ethics          │ → allow / HITL / block
└─────────┬─────────┘
          ▼
┌───────────────────┐
│ 8–12 Self…World   │ → identity, language, body, time, model
└─────────┬─────────┘
          ▼
┌───────────────────┐
│ 13–15 Learn+Meta+Exec │ → strategy, plan, inhibit
└─────────┬─────────┘
          ▼
┌───────────────────┐
│ 16–18 Person+Mot+Adapt │ → style, purpose, update
└─────────┬─────────┘
          ▼
┌───────────────────┐
│ 19–20 Wisdom+Trans │ → prudence, integration
└─────────┬─────────┘
          ▼
    DECISION + ACTION
          │
          ├─→ Tools (deploy / terminal / IDE) [HITL kama inahitajika]
          ├─→ Brain (shared memory)
          └─→ Rscript (scores_200 analysis)
```

### Feedback loop

```
Output / outcome → Learning (13) → Memory (2) → Meta (14) → Adaptation (18)
```

---

## API (baada ya wire main.rs)

| Method | Path | Kazi |
|--------|------|------|
| GET | `/api/akili/v6/status` | Status + groups |
| GET | `/api/akili/v6/layers?group=N` | Orodha tabaka |
| POST | `/api/akili/v6/live` | Pipeline 200 `{"input":"..."}` |
| POST | `/api/akili/v6/gate` | Ethics/HITL gate |
| POST | `/api/akili/v6/r` | Rscript analysis |

---

## Stack

| Sehemu | Teknolojia |
|--------|------------|
| Engine | Rust (`akili_v6.rs`) |
| Agents | `akili_bridge` → agent_engine, tools |
| Uchambuzi | R (`Rscript`) |
| Config / tools | JSON (catalog, patterns) |
| Memory pamoja | `brain.rs` (SQLite + vectors) |

---

## Ukweli wa production

- **Inafanya:** maamuzi, ethics, HITL deploy, R stats, autopilot gate.
- **Si AGI:** simulation ya tabaka 200, si fahamu ya binadamu.
- **Restore main:** `git show <commit-salama>:server/agent/src/main.rs > server/agent/src/main.rs`, kisha unganisha mods/routes kwa mkono (kuna `docs/akili_v6_*.json` za marejeo).

---

*Akili V6.0 — Fundi Deploy — server/agent*
