# 03 — Deck derivation + intro order

**What to build:** The in-memory Deck the rest of the app iterates. It loads the static geometry asset and exposes the 240 Cards in the fixed big→obscure introduction order, so new Cards always enter the same way and every screen shares one ordering. Pure Rust, no UI.

Source spec: `.scratch/mvp-spec/spec.md` §2.4, §5.2.

**Blocked by:** 01 (geometry asset).

- [x] Loads the asset via `include_str!` + `serde_json` into a typed Entity/Card structure keyed by `ADM0_A3`
- [x] Deck ordered by `LABELRANK` ascending, tiebreak `POP_EST` descending — the fixed new-Card intro sequence
- [x] Deck membership + order derived at runtime from the asset only; nothing persisted (§5.2)
- [x] Unit tests: count = 240; contested entities land mid-to-late (Taiwan LR 3; Palestine/Somaliland 5; Kosovo/N. Cyprus 6; W. Sahara 7); a famous-small nation (e.g. Vatican/Nauru) is not buried at the tail as a pure-population sort would do

## Comments

**Implemented.** `src/deck.rs` (commit `1ea4d16`). `Deck::load()` parses the
embedded asset into `Card { code, entity }`, sorted `LABELRANK` asc / `POP_EST`
desc (full ties break alphabetically by code, stable). 7 unit tests; full gate
green (`cargo test`, clippy `-D warnings`, fmt, iOS target).

Two data-reality notes:

- **Count = 240, not 239** (§2.1 + Tuvalu supplement; matches the shipped
  asset and `DECK_COUNT`).
- **Famous-small test uses Iceland/New Zealand, not Vatican/Nauru.** The issue's
  example doesn't hold against the data: VAT and NRU are both `LABELRANK 6`, so
  intro order buries them at the tail as hard as pure-population would. Iceland
  (small + famous) is the genuine rescue case; the test documents this.
