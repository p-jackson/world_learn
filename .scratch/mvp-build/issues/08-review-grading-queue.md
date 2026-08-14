# 08 — Review grading + queue advance

**What to build:** Make the Review loop real end to end. The four grade buttons now drive the scheduler: grading a Card advances FSRS state, persists, pulls the next Card from the session queue (re-drilling Again cards later in the session), and lands on the done-for-today state when the queue empties. This is the tracer bullet that connects UI to the scheduling core.

Source spec: `.scratch/mvp-spec/spec.md` §4.1, §4.5, §5.4.

**Blocked by:** 04 (scheduling core), 07 (front/reveal presentation).

- [ ] Session queue built from the scheduling core (04): due set ++ up-to-allowance new Cards in intro order
- [ ] Each grade calls the core's grade path: FSRS `next_states`, `due`/`last_review` update, atomic persist
- [ ] **Again** requeues the Card to the back of the session queue and re-shows it later in the session; passes exit
- [ ] After a grade, the next Card loads (map reframes, state resets to front); status strip counts (`N left`, `i/total`, progress) reflect real queue position
- [ ] Empty queue → **Done-for-today** screen: ✓ + "N reviewed · next batch unlocks tomorrow" + Back to home
- [ ] Mid-session quit is safe: a re-drill (Again, `due = today`) is not lost on reload
- [ ] Demoable: run a full session over several real Cards from a fresh store, grading through to Done
