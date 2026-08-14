# FSRS integration & scheduling API

Type: research
Status: resolved

## Question

Confirm `fsrs-rs` is viable for this app and surface what the spec must state
about it.

- Does `fsrs-rs` build/link cleanly for an iOS Dioxus target? Any native deps?
- What per-card memory state does it persist (stability, difficulty, etc.)?
- What's the scheduling API: given card state + a grade (Again/Hard/Good/Easy)
  + review time, what does it return (next interval / due date / new state)?
- Default parameters — ship stock defaults, or is per-user optimisation needed?
  Is optimisation in-scope-able later or a dependency now?
- How much simpler is the SM-2 fallback if `fsrs-rs` proves painful?

Recommend: proceed with `fsrs-rs`, or fall back to SM-2 — with the reason.

## Answer

**Decision: proceed with the `fsrs` crate (v6+). SM-2 fallback not needed.**

The ticket's fear — that the optimizer drags in the `burn` ML stack — is stale.
In current `fsrs` (crate name **`fsrs`**, not `fsrs-rs`; repo
`open-spaced-repetition/fsrs-rs`, v6.6.2, `edition=2024`), **`burn` is a
dev-dependency only**. The whole production dep set is pure Rust (`ndarray`,
`rayon`, `serde`, …), no C linkage, cross-compiles to `aarch64-apple-ios` as an
ordinary crate. No scheduler/optimizer split to engineer.

Facts for the spec:

- **Dependency**: `fsrs = "6"`, default features (`default = []`). Confirm ≥6.x
  (pre-rewrite versions did depend on burn).
- **Per-card FSRS state = `MemoryState { stability: f32, difficulty: f32 }`** —
  that's all the crate persists. The app owns the rest: `due`, `last_review`,
  `scheduled_days`, plus optional `reps`/`lapses` for our own stats. New card =
  `memory_state: None`.
- **Scheduling API** (one call gives all four grades):
  `FSRS::default().next_states(current: Option<MemoryState>, desired_retention:
  f32, days_elapsed: u32) -> Result<NextStates, _>` where
  `NextStates { again, hard, good, easy: ItemState }` and
  `ItemState { memory: MemoryState, interval: f32 /*days*/ }`.
  On grade: take that field's `.memory` (new state) + `.interval`; app computes
  `due = now + interval`, stores `last_review = now`.
- **Params**: ship stock `DEFAULT_PARAMETERS` (FSRS-6, Anki's defaults) for the
  MVP; no training needed. Per-user `compute_parameters` optimization is a clean,
  additive later phase — still pure Rust, no schema change beyond storing a
  weight vector.
- (Alternative lighter crate `rs-fsrs` exists with a fuller built-in `Card`
  struct, but `fsrs` is the reference impl kept in lockstep with Anki — prefer it.)
