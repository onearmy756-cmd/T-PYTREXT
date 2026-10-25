# Akili Agent V6.0 — Fundi Deploy

Mfumo wa **tabaka 200** (makundi 20) kwa maamuzi, ethics/HITL, na uchambuzi wa R.

## Faili

| Faili | Kazi |
|-------|------|
| `src/akili_v6.rs` | Engine — pipeline 200 + Rscript |
| `src/akili_bridge.rs` | Unganisho kwa agents zote |
| `src/agent_engine.rs` | Autopilot STEP 0 = Akili V6 |
| `src/akili_v3.rs` | V3 (memory, MCTS, R) |
| `src/akili_v4.rs` | V4 (tabaka 12) |
| `docs/akili_v6_architecture.md` | Michoro + data flow |
| `docs/akili_v6_layers.json` | Registry ya tabaka |
| `docs/akili_v6_dataflow.json` | Pipeline JSON |

## Stack

- **Rust** — engine, API, HITL
- **R** — `Rscript` analysis ya activation scores
- **JSON** — layers + dataflow config

## Wire `main.rs` (lazima)

```bash
# Rejesha main.rs kama imeharibiwa
git show 50ee55616bf2433cf18dfd37bd0e655d846bb4cd:server/agent/src/main.rs > server/agent/src/main.rs

# Unganisha V4/V6/bridge kwa mkono, kisha:
cd server/agent && cargo build -p fundi-deploy --release
```

## API

```bash
curl -s localhost:8080/api/akili/v6/status
curl -s -X POST localhost:8080/api/akili/v6/live \
  -H 'Content-Type: application/json' \
  -d '{"input":"Nataka deploy Ubuntu"}'
curl -s -X POST localhost:8080/api/akili/v6/r \
  -H 'Content-Type: application/json' \
  -d '{"input":"Kompyuta inaenda polepole"}'
```

## Ukweli

Inafanya: maamuzi, ethics, HITL deploy, R stats, autopilot gate.  
Si AGI / fahamu ya binadamu — ni production agent architecture.
