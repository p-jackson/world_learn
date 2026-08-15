# 04 — FSRS scheduling + session/grading core

**What to build:** The scheduler brain, pure Rust and fully unit-testable without any UI. Given the persisted store and today's date it derives what's due, what new Cards are allowed today, and builds the session queue; given a grade it advances FSRS state, computes the next due date, requeues or exits the Card, and persists. This is the contract the Review loop drives.

Source spec: `.scratch/mvp-spec/spec.md` §5.2, §5.3, §5.4.

**Blocked by:** 02 (persistence), 03 (deck/order).

- [x] `fsrs = "6"` wired: `FSRS::default().next_states(current: Option<MemoryState>, 0.9, days_elapsed)`; ship stock `DEFAULT_PARAMETERS`, no training; `desired_retention = 0.9` as a constant
- [x] Per-card state persisted is only `MemoryState { stability, difficulty }`; app owns `due` / `last_review` / `introduced_on`
- [x] Derived (never persisted): new backlog = deck keys − card keys in intro order; new-allowance = `new_cards_per_day − count(introduced_on == today)`; due set = `due ≤ today`; status = new/due/scheduled
- [x] Session queue (in-memory) = due set ++ up-to-allowance new Cards in intro order
- [x] Intervals whole-day, min 1: `interval.round().max(1.0)`; `due` is a date
- [x] Grade rules: **Again** → update memory, `due = today`, `last_review = today`, requeue to **back** of session queue, persist (survives quit); **Hard/Good/Easy** → persist memory, `last_review = today`, `due = today + round(interval)`, Card exits; **first grade of a new Card** (any grade) → create record, stamp `introduced_on = today`
- [x] `days_elapsed = max(0, today − last_review)` as `u32`; new Card's first grade passes `current = None, days_elapsed = 0`; same-day re-drills feed `days_elapsed = 0`
- [x] Persist atomically after each grade (via 02)
- [x] Unit tests cover: Again requeue + persisted `due=today`, pass exits + next due, new-cap enforcement across a simulated day boundary, first-grade `introduced_on` stamping

## Comments

**Implemented.** `src/scheduler.rs` + `src/session.rs` (commit `e6349e3`).

- `Scheduler::review(prior, grade, today) -> CardRecord`: pure FSRS step over
  `fsrs=6` (`FSRS::default()` = stock `DEFAULT_PARAMETERS`, `DESIRED_RETENTION =
  0.9`). Whole-day intervals (`round().max(1)`), Again pins `due=today`, passes
  `due=today+interval`, `last_review=today` every grade, `introduced_on` stamped
  once. No I/O — persistence/requeue live in `session`.
- `session.rs`: pure derivations (`status`, `new_backlog`, `new_allowance`,
  `due_cards`, `build_queue`) + `Session` orchestrator (start → front→grade loop,
  atomic persist per grade via 02, Again → back of queue).

31 tests green (11 new); clippy `-D warnings`, fmt, iOS target all clean.
Two-axis `/code-review`: Spec faithful (no gaps/creep); Standards clean after
adding card-naming context to the grade boundary.

Status: ready-for-human
