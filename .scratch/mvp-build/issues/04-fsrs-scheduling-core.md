# 04 — FSRS scheduling + session/grading core

**What to build:** The scheduler brain, pure Rust and fully unit-testable without any UI. Given the persisted store and today's date it derives what's due, what new Cards are allowed today, and builds the session queue; given a grade it advances FSRS state, computes the next due date, requeues or exits the Card, and persists. This is the contract the Review loop drives.

Source spec: `.scratch/mvp-spec/spec.md` §5.2, §5.3, §5.4.

**Blocked by:** 02 (persistence), 03 (deck/order).

- [ ] `fsrs = "6"` wired: `FSRS::default().next_states(current: Option<MemoryState>, 0.9, days_elapsed)`; ship stock `DEFAULT_PARAMETERS`, no training; `desired_retention = 0.9` as a constant
- [ ] Per-card state persisted is only `MemoryState { stability, difficulty }`; app owns `due` / `last_review` / `introduced_on`
- [ ] Derived (never persisted): new backlog = deck keys − card keys in intro order; new-allowance = `new_cards_per_day − count(introduced_on == today)`; due set = `due ≤ today`; status = new/due/scheduled
- [ ] Session queue (in-memory) = due set ++ up-to-allowance new Cards in intro order
- [ ] Intervals whole-day, min 1: `interval.round().max(1.0)`; `due` is a date
- [ ] Grade rules: **Again** → update memory, `due = today`, `last_review = today`, requeue to **back** of session queue, persist (survives quit); **Hard/Good/Easy** → persist memory, `last_review = today`, `due = today + round(interval)`, Card exits; **first grade of a new Card** (any grade) → create record, stamp `introduced_on = today`
- [ ] `days_elapsed = max(0, today − last_review)` as `u32`; new Card's first grade passes `current = None, days_elapsed = 0`; same-day re-drills feed `days_elapsed = 0`
- [ ] Persist atomically after each grade (via 02)
- [ ] Unit tests cover: Again requeue + persisted `due=today`, pass exits + next due, new-cap enforcement across a simulated day boundary, first-grade `introduced_on` stamping
