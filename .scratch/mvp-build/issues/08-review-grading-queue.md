# 08 — Review grading + queue advance

**What to build:** Make the Review loop real end to end. The four grade buttons now drive the scheduler: grading a Card advances FSRS state, persists, pulls the next Card from the session queue (re-drilling Again cards later in the session), and lands on the done-for-today state when the queue empties. This is the tracer bullet that connects UI to the scheduling core.

Source spec: `.scratch/mvp-spec/spec.md` §4.1, §4.5, §5.4.

**Blocked by:** 04 (scheduling core), 07 (front/reveal presentation).

- [x] Session queue built from the scheduling core (04): due set ++ up-to-allowance new Cards in intro order
- [x] Each grade calls the core's grade path: FSRS `next_states`, `due`/`last_review` update, atomic persist
- [x] **Again** requeues the Card to the back of the session queue and re-shows it later in the session; passes exit
- [x] After a grade, the next Card loads (map reframes, state resets to front); status strip counts (`N left`, `i/total`, progress) reflect real queue position
- [x] Empty queue → **Done-for-today** screen: ✓ + "N reviewed · next batch unlocks tomorrow" + Back to home
- [x] Mid-session quit is safe: a re-drill (Again, `due = today`) is not lost on reload
- [x] Demoable: run a full session over several real Cards from a fresh store, grading through to Done

## Comments

**Implemented** by wiring the existing scheduling core (`Session`, issue 04) to
the front/reveal presentation (`Review`, issue 07) through a new `ReviewSession`
driver in `src/review.rs`; `main.rs` opens the real store and runs it.

- **Ownership refactor (to live in a signal):** `Session` borrowed `&Deck`/`&Store`,
  which can't sit in a Dioxus `Signal` (needs `'static`). It now owns cheap `Clone`
  handles — `SharedDeck` (moved map.rs→deck.rs, its natural home) and `Store`
  (made `Clone`) by value. Behaviour unchanged; the issue-04 tests guard it.
- **`ReviewSession`** holds `Signal<Result<Session, String>>` + a `revealed`
  signal, snapshots to a `Screen` enum (drops the read guard before handlers write
  back), and renders `Review` until the queue drains, then `DoneForToday`. Each
  grade → `Session::grade` (FSRS advance + atomic persist + Again-requeue) then
  resets `revealed` to front. `revealed` is lifted to the driver (not keyed on the
  card) so the next Card resets to front **without** remounting the 240-path SVG
  (keeps the #2274 render-once-vary-fill map intact).
- **Status strip:** new `Session::total()` (fixed at start) / `reviewed()` (passes,
  not grade taps — an Again is a re-drill, not progress) drive `QueuePosition`.
- **Done-for-today** shows ✓ + "N reviewed · next batch unlocks tomorrow" + a
  Back-to-home button; with no Home yet (issue 09), it restarts the session, which
  re-derives from the persisted store and lands straight back on Done — proof the
  passes persisted.
- **`From<review::Grade> for scheduler::Grade`** keeps the view's grade enum
  independent of the core; unit-tested for the 1:1 mapping.
- Reviewed via `/code-review` (Standards + Spec). Applied: shared `start_session`
  helper (dedupe + consistent error surfacing on the restart path), Done glyph
  ✅→✓ to match the spec literal.
- **Eyeballed in the iOS simulator** (2026-08-15, `dx serve --ios`, iPhone 16e).
  All six acceptance checks confirmed on-device: front (China 1/10, boundaries +
  amber highlight + progress strip), reveal (Russia: common + long name, 📍 on the
  entity, four red/orange/green/blue grade buttons), advance/reframe (Russia→Japan
  resets to front + reframes + strip advances), Again re-drill (Japan Again kept
  the counter at 2/3 and re-showed Japan after Mexico), Done ("3 reviewed" — passes
  not taps, despite 4 taps incl. an Again), and quit-safety (Again'd Russia
  persisted `due=today` and returned at the front after kill+relaunch). No fixes
  needed for issue 08 itself.
- **Framing defect found & logged (not issue 08):** huge/high-latitude countries
  (Russia the clear case) render as a near-global, horizontally-squished world
  instead of a regional zoom — a `Frame::for_bbox` (issue 06) limitation surfaced
  by this pass. Written up with repro + root cause + proposed fix in
  `10-large-country-framing-distortion.md` (`ready-for-agent`).
- `cargo test` (52), `clippy -D warnings`, `fmt --check`, and
  `cargo check --target aarch64-apple-ios` all green.
