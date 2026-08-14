# 07 — Review front/reveal presentation

**What to build:** The two visual states of the Review screen for a single Card, driven by a local reveal toggle — no scheduler wiring yet. Front shows the framed map with a thin status strip and a Tap-to-reveal pill; tapping the pill or the map flips to reveal, which shows the names, a pin on the entity, and the four grade buttons. The buttons render and are styled but are inert this ticket. This is the greenfield component/interaction layer the loop will later wire to the scheduler.

Source spec: `.scratch/mvp-spec/spec.md` §4.1.

**Blocked by:** 06 (regional-zoom framing).

- [ ] **Front state**: thin top status strip (`N left` · progress bar · `i/total`) + a single "Tap to reveal" pill; tapping the pill **or the map** reveals
- [ ] **Reveal state**: short common name **+** formal/long name; a dropped **📍 pin** at the entity centroid (from the asset); four grade buttons
- [ ] Grade buttons **vertically stacked, full-width**, Again / Hard / Good / Easy, colored red / orange / green / blue (stacked so all four are thumb-reachable) — no FSRS interval previews on them
- [ ] Reveal toggle is local component state; buttons inert (no scheduling) this ticket
- [ ] No neighbour/context chips, no bottom-sheet layout, no interval previews (explicitly rejected)
- [ ] Demoable on a single hardcoded Card: front ⇄ reveal by tapping map or pill; buttons visible and styled
