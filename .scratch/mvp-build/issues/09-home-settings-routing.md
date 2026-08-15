# 09 — Home / Settings / Done + routing

**What to build:** The surrounding app shell that makes the three-screen MVP whole. Home is the launch surface with due/new counts and a Start button; Settings exposes the one interactive control (new-cards/day); the router ties Home, Review, Settings, and Done together with the specified transitions. First launch drops straight into Home with the full new-Card backlog — no onboarding.

Source spec: `.scratch/mvp-spec/spec.md` §4.3, §4.4, §4.6, §4.7.

**Blocked by:** 08 (review loop reaches Done).

- [x] `Router` with Home / Review / Settings / Done routes and an outlet layout
- [x] **Home**: title + tagline; two stat tiles — reviews-due and new-today (from the scheduling core); primary `Start · N cards`; Settings link
- [x] When nothing's due: Home shows **0/0** and Start is absent/disabled — no special re-entry screen
- [x] **Settings**: new-cards/day stepper (default 10) as the only interactive control, persisted via the store; read-only rows — Scheduler (FSRS), Deck (240, incl. contested)
- [x] Transitions: Home → Review → Done → Home; Home ⇄ Settings
- [x] No onboarding/first-run flow: first launch = Home with the full new-Card backlog, daily cap applied
- [ ] Demoable: cold launch → Home counts correct → Start → review to Done → Home; change new-cards/day in Settings and see the new-today allowance change

## Comments

- **Done in `af133bf`.** `Route` enum (Home/Review/Settings/Done) under a
  `#[layout(Shell)]` outlet; `AppRouter` provides Deck + Store via context so the
  route components take no props (read through `ui::use_app_context`). `Done`
  carries the reviewed count as a `/done/:reviewed` path segment.
- **Home** derives its two tiles + Start count from `session::counts` (new pure
  helper, `counts(state, deck, today) → {due, new_today}`, asserted equal to
  `build_queue` length). Start renders only when `total > 0`; at 0/0 it is absent.
- **Settings** stepper persists each `±` through `store::set_new_cards_per_day`
  (loads, sets, saves atomically). Floor 0, ceil 99 (unspecified cap — see Q). The
  store is the single source of truth: Home re-reads on remount, so the new
  allowance shows on return.
- **Refactor:** old `ReviewSession` → `Review` route reading context, draining to
  `Route::Done` via a `use_effect`; `DoneForToday` → `Done` route; `today_local`
  moved to `session`; shared `ui::Failure` for the three error screens.
- **Dependency wrinkle:** dioxus's `router` feature at 0.7.10 first failed to
  resolve on a stale index (`dioxus-router 0.7.10` not seen); a `cargo update`
  refreshed it and everything aligns at 0.7.10.
- `cargo test` (57), `clippy -D warnings`, `fmt --check`, and
  `cargo check --target aarch64-apple-ios` all green. Reviewed via `/code-review`
  (Standards + Spec): no hard violations / no missing requirements; applied its
  dedup + speculative-generality fixes.
- **Not yet done:** end-to-end simulator eyeball (the Demoable box) — verified by
  tests + builds only, not run in `dx serve --ios` this session.

### Open questions

- Stepper cap of 99 is invented (spec §4.4 gives no bound). Acceptable? A lower
  ceiling (e.g. deck-size 240, or ~40) may read better.
