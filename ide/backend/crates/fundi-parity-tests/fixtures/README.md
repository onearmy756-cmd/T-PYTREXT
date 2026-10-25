# Parity fixtures

## Layout

- **`registry.json`** — lists **active** golden modules only; larger roadmap items stay in `PARITY_PLAN.md` until fixtures exist.
- **`<module>/*.json`** — one file per focus area; each file has `schema_version`, `cases[]`.
- **`pending/`** — excluded from `run_all_active_fixtures` (scaffolding / future work).

## Case schema

Each case may include:

- `id`, `op`, `input`, `expected`
- **`skip`: `true`** — ignored by the harness (placeholder until implementations exist).

## Running

```bash
cargo test -p fundi-parity-tests
```

Recursive run over all non-`pending` JSON: `fundi_parity_tests::run_all_active_fixtures`.

**Machine run / skip counts:** [`../PARITY_FIXTURE_STATUS.md`](../PARITY_FIXTURE_STATUS.md) (refresh per footer there before committing).

Rust 侧 **模块 C**（Bedrock / 插件 / Dashboard）进度与缺口说明见 [`deploy/PARITY_MODULE_C.md`](../../../deploy/PARITY_MODULE_C.md)。

## Recording (one-off)

```bash
# Fixtures were captured from the legacy Python reference; MTECHOS now runs Rust only.
```

Works without a reference checkout for **`checkpoint_shadow_dir_id`**; the
Anthropic adapter goldens came from the legacy Python adapter, captured once.
