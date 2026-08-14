# Review-state & session data model

Type: grilling
Status: resolved
Blocked by: 01, 02, 04

## Question

Define the persisted data model, given FSRS's required fields (02), the chosen
persistence layer (01), and the interaction states the prototype settles (04).

- **Per-card state**: FSRS memory fields + due date + lifecycle status
  (new / learning / review), keyed by Entity id.
- **Deck / session state**: how "new cards introduced today" and "reviews due
  today" are computed and tracked against the new-cards/day cap.
- **Settings state**: new-cards/day and anything else the prototype surfaced.
- Serialisation shape for the persistence layer.

Output: the data model the spec will state.

## Answer

**One sparse serde-JSON file (per ticket 01); `cards` keyed by `ADM0_A3`, holding only cards that have left "new". Lifecycle status is derived, not stored.**

### Persisted shape

```jsonc
{
  "schema_version": 1,
  "settings": { "new_cards_per_day": 10 },   // the only interactive setting
  "cards": {
    "FRA": {
      "stability": 3.17,          // FSRS MemoryState.stability (f32)
      "difficulty": 5.20,         // FSRS MemoryState.difficulty (f32)
      "due": "2026-08-16",        // local date; ≤ today ⇒ due
      "last_review": "2026-08-14",// local date of last grade
      "introduced_on": "2026-08-10" // set once, on first grade; drives the daily-new cap
    }
    // …only seen cards. Absent key = a new, not-yet-introduced card.
  }
}
```

- Flat card records — memory fields inline, not nested. ISO **local-date** strings (`YYYY-MM-DD`), day precision. No other top-level keys.
- **Not stored** (out on purpose): lifecycle enum, `reps`/`lapses` (no stats screen in MVP — out of scope), `desired_retention` (code constant `0.9`), FSRS optimized weights (additive later behind `schema_version`, per ticket 02).

### Derived at runtime, never persisted

- **Deck membership + intro order** — from the static geometry asset (ticket 03/05: `LABELRANK` asc, tiebreak `POP_EST` desc).
- **new backlog** = deck keys − `cards` keys, in intro order.
- **new allowance remaining** = `new_cards_per_day − count(cards where introduced_on == today)`.
- **due set** = `{ cards where due ≤ today }`.
- **status** — `new` if key absent; else `due` if `due ≤ today`, else `scheduled`. ("Learning" is not a persisted state — just a seen card sitting at `due = today`.)
- **session queue** (transient, in-memory) = due set ++ up-to-allowance new cards in intro order.

### Scheduling & session rules (the model behind the shape)

- **Day boundary** = local device midnight; not configurable in MVP.
- **Intervals rounded to whole days**, min 1 (`interval.round().max(1.0)`). `due` is a date, not a timestamp — no sub-day session loop. (`fsrs` returns raw fractional days with no built-in same-day step scheduler; the app owns granularity — confirmed by research on ticket 06.)
- **On grade**: call `next_states(current_memory_state, 0.9, days_elapsed)`; `days_elapsed = max(0, today − last_review)` as `u32`; a new card's first grade passes `current = None, days_elapsed = 0`.
  - **Again** → update memory_state, set `due = today`, `last_review = today`; requeue to the back of the session queue (re-drills until a pass). Persisted `due = today` survives a mid-session quit, so a re-drill is never lost.
  - **Hard / Good / Easy** (passes) → persist memory_state, `last_review = today`, `due = today + round(interval)`; card exits the session.
  - **First grade of a new card** (any grade) → create the record and set `introduced_on = today` (counts against the daily cap from that moment).
- `last_review` updates on **every** grade, so same-day re-drills feed `days_elapsed = 0` (FSRS `delta_t == 0` short-term path).
- Persist atomically after each grade (tiny file, per ticket 01).
