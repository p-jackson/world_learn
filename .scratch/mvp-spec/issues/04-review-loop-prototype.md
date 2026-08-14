# Review-loop & screen interaction spec

Type: prototype
Status: resolved

## Question

Nail down how the app looks and feels across its three screens, cheaply, by
building a throwaway prototype to react to.

- **Review screen** (the crux): the regional-zoom map with one entity
  highlighted; the front (think the answer) → reveal (true name shown) →
  grade (4 buttons) flow; where the buttons sit; what "reveal" shows (name only?
  name + a pin/marker?); how the zoom framing feels at bbox × ~3–4.
- **Home screen**: due-today / new-remaining counts + a Start affordance.
- **Settings screen**: the new-cards/day control.
- Transitions between them; the "done for today" end state.

Output: a prototype (linked as an asset) + the interaction decisions it settles.
Feeds the review-state model (06) and graduates the onboarding / done-for-today
fog on the map.

## Answer

Settled by a self-contained HTML prototype (3 review layouts A/B/C, real NE 50m
geometry, same d3-geo equirectangular pipeline as ticket 03). Asset:
`.scratch/mvp-spec/prototype/` (`index.html`, `generate.mjs`, `shots/`).

**Review screen — WINNER: immersive map + vertically-stacked grades.**

- Regional-zoom map is **full-bleed / hero** (edge-to-edge, no card chrome).
- **Boundary lines always visible** on every entity, both states (non-scaling
  stroke) — explicit user requirement.
- **Front**: thin top status strip (`N left` · progress bar · `i/total`) + a
  single "Tap to reveal" pill; tapping the pill *or the map* reveals.
- **Reveal**: short name **+ formal / long name** (e.g. "France / French
  Republic"; "Kosovo / Republic of Kosovo (disputed)"), a dropped **📍 pin** on
  the entity centroid, and the 4 grade buttons **vertically stacked, full-width**
  (Again/Hard/Good/Easy — red/orange/green/blue). Stacked, not a row, so all four
  are reachable by either thumb (chosen over the immersive layout's original row).
- **No FSRS interval previews** on the buttons in MVP — just the 4 labels.
- Rejected: neighbour/context chips on reveal (Variant C); bottom-sheet layout
  (Variant B); interval previews (B/C).

**Framing / geometry** (refines ticket 03 for the render path):

- viewBox = target **mainland bbox × ~3.4** padding, square, centred; enforce a
  **min span (~6° → ~20° window)** so tiny/island entities aren't over-zoomed.
- Frame on the entity's **largest polygon** (mainland) — ignoring scattered
  overseas parts — else multi-part features (France + French Guiana) blow the
  bbox out to the whole Atlantic. Production pipeline must do the same.
- Per-card **cos(lat) horizontal correction** (one factor per card) via a group
  transform — matches ticket 03.
- **50m** geometry confirmed (not 110m): 110m loses peninsula/small-island
  distinctness under zoom (user-flagged); 50m fixes it. 2-decimal coord rounding.

**Home**: title + tagline, two stat tiles (reviews-due / new-today), primary
`Start · N cards`, Settings link. When nothing's due: Home shows 0/0 and Start is
absent/disabled — no special re-entry screen.

**Settings**: **new-cards/day stepper** (default 10; big→obscure intro order) is
the only interactive control; read-only rows for Scheduler (FSRS) and Deck (~250,
incl. contested).

**Done-for-today**: ✓ + "N reviewed · next batch unlocks tomorrow" + Back to home.
No come-back-later / extra-practice state in MVP.

**Onboarding**: **none**. First launch = Home with the full new-card backlog, the
daily cap applies. (Ruled out of scope on the map.)

Transitions: Home → Review (per-card front→reveal→grade loop) → Done → Home;
Home ⇄ Settings. Feeds review-state model (06): per-card lifecycle new/learning/
review, session new-introduced-today vs cap, and the settings shape above.
