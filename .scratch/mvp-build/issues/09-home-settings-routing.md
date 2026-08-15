# 09 — Home / Settings / Done + routing

**What to build:** The surrounding app shell that makes the three-screen MVP whole. Home is the launch surface with due/new counts and a Start button; Settings exposes the one interactive control (new-cards/day); the router ties Home, Review, Settings, and Done together with the specified transitions. First launch drops straight into Home with the full new-Card backlog — no onboarding.

Source spec: `.scratch/mvp-spec/spec.md` §4.3, §4.4, §4.6, §4.7.

**Blocked by:** 08 (review loop reaches Done).

- [ ] `Router` with Home / Review / Settings / Done routes and an outlet layout
- [ ] **Home**: title + tagline; two stat tiles — reviews-due and new-today (from the scheduling core); primary `Start · N cards`; Settings link
- [ ] When nothing's due: Home shows **0/0** and Start is absent/disabled — no special re-entry screen
- [ ] **Settings**: new-cards/day stepper (default 10) as the only interactive control, persisted via the store; read-only rows — Scheduler (FSRS), Deck (240, incl. contested)
- [ ] Transitions: Home → Review → Done → Home; Home ⇄ Settings
- [ ] No onboarding/first-run flow: first launch = Home with the full new-Card backlog, daily cap applied
- [ ] Demoable: cold launch → Home counts correct → Start → review to Done → Home; change new-cards/day in Settings and see the new-today allowance change
