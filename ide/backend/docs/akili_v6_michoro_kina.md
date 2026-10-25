# Akili Agent V6.0 — Michoro ya Kina (Makundi + Data Flow)

**Engine:** `src/akili_v6.rs` · **Bridge:** `src/akili_bridge.rs`  
**Pia:** `akili_v6_architecture.md` (muhtasari 1–20) · `akili_v6_layers.json` · `akili_v6_dataflow.json`

> Paste ya kina ilikatika kwenye Kundi 9. Makundi 10–20 kamili: `akili_v6_architecture.md`.

---

## KUNDI 1: CORE PERCEPTION (Tabaka 1–10)

```
INPUT (Dunia) → Sensation → Detection → Attention → Perception
  → Recognition → Discrimination → Localization → Tracking
  → Filtering → Salience → MEMORY
```

| # | Tabaka | Input | Process | Output | Muda |
|---|--------|-------|---------|--------|------|
| 1 | Sensation | Mwanga, sauti… | Kupokea | Signal ghafi | 10ms |
| 2 | Detection | Signal | Threshold | Boolean | 20ms |
| 3 | Attention | Signals | Salience (novelty/urgency/relevance…) | Focus | 50ms |
| 4 | Perception | Focus | Pattern match | Percept | 100ms |
| 5 | Recognition | Percept | Memory lookup | Identity / Novel | 150ms |
| 6 | Discrimination | Identity | Compare | Difference | 50ms |
| 7 | Localization | Object | Spatial | Position | 30ms |
| 8 | Tracking | Moving | Path | Trajectory | 40ms |
| 9 | Filtering | Noisy | Denoise | Clean | 60ms |
| 10 | Salience | Multiple | Rank | Priority | 20ms |

**Attention weights:** Novelty 0.20 · Urgency 0.25 · Relevance 0.20 · Emotion 0.15 · Size 0.10 · Movement 0.10  
**Filter:** PRIMARY (focus) · SECONDARY (buffer) · IGNORED

---

## KUNDI 2: MEMORY (11–20)

```
Perception → Working (7±2, 30s) → Short-Term / Long-Term
  Long-Term: Episodic · Semantic · Procedural
             Autobiographical · Flashbulb · Prospective · Muscle
  → THINKING
```

| # | Tabaka | Muda | Uwezo |
|---|--------|------|-------|
| 11 | Working | ~30s | 7±2 |
| 12 | Short-Term | Dakika–saa | 100+ |
| 13 | Long-Term | Milele | ∞ |
| 14 | Episodic | Milele | matukio |
| 15 | Semantic | Milele | ukweli |
| 16 | Procedural | Milele | ujuzi |
| 17 | Autobiographical | Milele | hadithi |
| 18 | Flashbulb | Milele | mshtuko |
| 19 | Prospective | Siku | reminders |
| 20 | Muscle | Milele | reflexes |

---

## KUNDI 3: THINKING (21–30)

```
Memory → System1 (haraka) + System2 (pole)
       → Reasoning (deductive/inductive/abductive)
       → Logic (AND/OR/NOT/IMPLY)
       → Analysis · Synthesis · Evaluation
       → Abstraction · Generalization · Specialization
       → CREATIVITY
```

| # | Tabaka | Muda |
|---|--------|------|
| 21 | System 1 | ~100ms |
| 22 | System 2 | ~10s |
| 23 | Reasoning | ~1s |
| 24 | Logic | ~50ms |
| 25 | Analysis | ~500ms |
| 26 | Synthesis | ~800ms |
| 27 | Evaluation | ~300ms |
| 28 | Abstraction | ~1s |
| 29 | Generalization | ~500ms |
| 30 | Specialization | ~300ms |

---

## KUNDI 4: CREATIVITY (31–40)

Imagination → Creativity (combinatorial / exploratory / transformational / bisociation)  
→ Divergent + Convergent → Brainstorm · Incubation · Insight · Flow · Play · Improvisation

---

## KUNDI 5: EMOTION (41–50)

Primary (furaha, huzuni, hofu, hasira, mshangao, karaha)  
→ Secondary (wivu, nostalgia, hatia…) · Mood · Affect  
→ Empathy · Sympathy · Compassion · Regulation · Emotional Memory · Sentiment

---

## KUNDI 6: SOCIAL (51–60)

Theory of Mind (levels 1–4) · Social Cognition · Perspective Taking · Social Empathy  
→ Trust · Reciprocity · Cooperation · Competition · Negotiation · Leadership

---

## KUNDI 7: ETHICS (61–70)

Moral Reasoning (utilitarian / deontological / virtue / care)  
→ Conscience · Values · Virtues · Rights · Duties · Justice · Care · Integrity · Accountability

**Fundi Deploy:** block wipe/spark; HITL kwa deploy/PXE/format.

---

## KUNDI 8: SELF (71–80)

Self-Awareness · Self-Concept · Self-Esteem  
→ Self-Regulation · Self-Reflection · Self-Identity  
→ Self-Narrative · Self-Image · Self-Actualization · Self-Transcendence

---

## KUNDI 9: TIME (81–90) — sehemu

Time Perception · Temporal Ordering · Duration  
→ Prospective · Retrospective · Planning · Scheduling · Prioritization (Urgent×Important matrix)  
→ Deadlines · Patience

---

## Makundi 10–20 (muhtasari)

Tazama **`akili_v6_architecture.md`** na **`akili_v6_layers.json`**:

| Kundi | Tabaka | Jina |
|-------|--------|------|
| 10 | 91–100 | Embodiment |
| 11 | 101–110 | Time (kamili) |
| 12 | 111–120 | World Model |
| 13 | 121–130 | Learning |
| 14 | 131–140 | Meta |
| 15 | 141–150 | Executive |
| 16 | 151–160 | Personality |
| 17 | 161–170 | Motivation |
| 18 | 171–180 | Adaptation |
| 19 | 181–190 | Wisdom |
| 20 | 191–200 | Transcendence |

---

## Data flow kuu

```
INPUT → Perception → Memory → Thinking → Creativity
     → Emotion → Social → Ethics → Self → Language
     → Embodiment → Time → World → Learning → Meta
     → Executive → Personality → Motivation → Adaptation
     → Wisdom → Transcendence → DECISION
                                    ├─ Tools (+ HITL)
                                    ├─ Brain
                                    └─ Rscript
```

Feedback: outcome → Learning → Memory → Meta → Adaptation
