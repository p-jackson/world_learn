# 25 — Playwright UI tests with easy state mocking

Status: done
Blocked by: 24

**Goal:** a Playwright suite that drives the real app in a browser, with a
simple way to seed store state so tests start from a known deck/schedule, plus
CI wiring. Depends on web being a runnable target (issue 24).

## State mocking

The point is deterministic starts without clicking through days of reviews.
Tests need to inject a full `StoreState` (cards, due dates, settings,
schema_version) before the app reads it. Options to choose from:

- Seed the web persistence backend directly (e.g. `localStorage` key) before
  navigating — clean if issue 24 lands a localStorage backend. This is the
  preferred path if it's simpler: no in-app hook, Playwright just writes the key
  and reloads. If it tips the persistence decision, feed that back to issue 24.
- A test-only injection hook: a `window.__wl_seed(json)` global, or a
  `?seed=<url-encoded-json>` query param the store reads on first load under a
  debug cfg. Needed if the web backend stays ephemeral.

Pick one, keep it debug/test-gated (not in ship builds), and give tests a
typed helper to build states (empty deck, all-due, all-done, mid-review).

## Flows worth testing (audit)

Happy paths:

- Home → start review; front shows country framing, reveal shows name/pin.
- Grade queue: front → reveal → grade (Again/Hard/Good/Easy) advances to next
  card; queue drains to a done/empty state.
- Session completion: all-due deck graded → "done for today" state; new-cards
  cap honored (`new_cards_per_day`).
- Settings: change new-cards-per-day; **Clear all memory** button resets store
  (issue: `Store::clear`).
- Regional zoom / framing renders for representative countries (large: Canada/
  Australia; split: Indonesia; the framing-regression cases from issues
  10/14/15/21).
- Back-to-home navigation from a review (issue 22 was a reported break — guard
  it with a test).

Error cases:

- Store load failure → `ui::Failure` fallback renders (the `log_and_display`
  path from issue 20 / ADR-0001), app doesn't blank-screen.
- Corrupt / wrong-schema stored state → handled per the schema-version guard,
  not a panic.
- Missing/failed geometry asset load → graceful failure, not a crash.
- Grade-path error surfaces (the non-throwing `error!("{e:#}")` path) without
  losing the session.

Triage each into "worth a Playwright test" vs "already covered by `cargo test`"
— don't duplicate unit-level coverage in the browser. Prefer a small number of
high-value UI flows over exhaustive re-testing of scheduler/store logic already
unit-tested.

## CI (required)

The suite must run in CI, not just locally — a green PR means the e2e suite
passed on the runner.

- New workflow (or job) installs Playwright browsers, builds/serves web, runs
  the suite headless. Keep it separate from `rust.yml` / `geometry.yml`; gate
  PRs on it.
- Suite lives in `tests/e2e/`; document the run command in AGENTS.md "Verifying
  changes".

## Not in scope

- iOS/device UI automation (web only).
- Visual-regression / screenshot-diffing (could be a follow-up).

## Acceptance

- [x] Playwright installed and configured; `npm test` (or documented command)
      runs the suite headless locally and in CI. (`tests/e2e/`,
      `playwright.config.ts` — `webServer` boots `dx serve --web`.)
- [x] State-seeding mechanism works and is test/debug-gated out of ship builds;
      a typed helper builds known `StoreState`s. (Seeds the `localStorage`
      key directly — the web dev backend, gated out of the iOS ship build;
      typed builders in `tests/e2e/helpers/state.ts`.)
- [x] Audited flow list turned into tests: the high-value happy paths + the
      error cases above that aren't better left to `cargo test`. (Home/Review,
      grading→Done + persistence, new-card session admits the cap + stamps
      introductions, Again re-drill, back-to-home (issue 22), new-card cap on the
      Home counts, settings stepper + Clear-all-memory, corrupt/wrong-schema
      failure fallback. Framing is a **render smoke** — one highlighted country,
      zoomed off the world view — for the representative shapes (large: Canada/
      Australia; split: Indonesia); the framing-correctness regressions
      themselves stay guarded by `map.rs` unit tests, not re-tested in the
      browser. Triage of what was left to `cargo test` is documented in
      `tests/e2e/tests/errors.spec.ts`.)
- [x] CI job runs the suite green on PRs; documented in AGENTS.md.
      (`.github/workflows/e2e.yml`.)
