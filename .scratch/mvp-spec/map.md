# Map: MVP spec — Country-location flashcard app

Label: wayfinder:map

## Destination

A handoff-ready **MVP spec** for an iOS/Dioxus app that drills "where is this?":
a regional-zoom map with one entity highlighted, self-reveal + 4-button
grading, FSRS scheduling, over the full Natural Earth entity set (incl.
contested territories). Reaching the end = the spec is complete enough that a
build session (or series) can execute it without further product decisions.

## Notes

Domain glossary: `CONTEXT.md` (Entity / Card / Review / Deck). Every session
consults `/grilling` + `/domain-modeling`; prototype tickets use `/prototype`;
research tickets use `/research`.

**Fixed premises** (settled while charting — treat as given):

- Self-reveal recall (mental guess → tap → reveal → grade); **4-button** grades (FSRS: Again/Hard/Good/Easy)
- **Regional zoom**, medium framing (target bbox × ~3–4 padding), fixed frame per card
- Scope = major territories **incl. contested** (~250 Natural Earth admin-0 features)
- **FSRS** scheduler (`fsrs-rs`); SM-2 is the fallback
- New-cards/day cap (configurable, default ~10), fixed intro order big→obscure
- MVP surface = home + review loop + settings; nothing more
- Render = **inline SVG in RSX** (iOS = WKWebView; feasibility confirmed while charting)

## Decisions so far

<!-- one line per resolved ticket: gist + link -->

- [Local persistence approach](issues/01-local-persistence.md) — serde JSON file in iOS `Library/Application Support/` (atomic write, `schema_version`); path via a ~15-line `objc2` helper (no crate resolves iOS right); `rusqlite` bundled is the fallback.
- [FSRS integration](issues/02-fsrs-integration.md) — use crate `fsrs` v6+ (burn is dev-only now, pure-Rust, iOS-clean); per-card state = `MemoryState{stability,difficulty}` + app-owned due/last_review; `next_states()` returns all 4 grade branches; ship default params, optimise later.
- [Geometry asset pipeline](issues/03-geometry-asset-pipeline.md) — NE **50m**, **equirectangular + per-entity cos(lat)** correction, build-time d3-geo script → static `ADM0_A3 → {name,d,bbox}` asset; key on `ADM0_A3` not `ISO_A3`; Palestine from NE `ps` POV file; watch antimeridian (RUS/FJI/USA).
- [Deck-derivation & introduction-order rules](issues/05-deck-derivation.md) — Deck = **239 Cards**: include every NE 50m feature *except uninhabited dependencies* (`TYPE` sovereign/constituent/disputed/indeterminate = 211, plus 28 dependencies with `POP_EST>0`; drop only the 2 POP-0 ones); criterion is **inhabited**, tiny-dot visibility deferred to build. Reveal name = curated common name + `NAME_LONG`, via rule + ~15–25 override table. Intro order = **`LABELRANK` asc, tiebreak `POP_EST` desc** (recognizable→obscure; contested land mid-late).
- [Review-loop & screen interaction spec](issues/04-review-loop-prototype.md) — WINNER = **immersive full-bleed map + dropped pin + vertically-stacked** 4 grades (either-thumb reach); reveal shows **short + formal name**, **no interval previews**; **boundaries always visible**; frame on **mainland bbox × ~3.4** w/ ~6° min-span; Home (due/new tiles + Start), Settings (new-cards/day stepper), Done = ✓ screen; **no onboarding**. Prototype: `prototype/`.
- [Review-state & session data model](issues/06-review-state-model.md) — **one sparse JSON file**, `cards` keyed by `ADM0_A3` (absent = new); flat record `{stability, difficulty, due, last_review, introduced_on}`, ISO local-dates. **Status derived not stored** (learning = a seen card at `due=today`); daily-new cap **derived** from `introduced_on==today`. Intervals **rounded to whole days, min 1** (`due` is a date). **Again** keeps `due=today` + in-session re-drill; **Hard/Good/Easy** schedule `today+round(interval)`. `desired_retention=0.9` constant; no `reps`/`lapses`/weights (all deferred behind `schema_version`).
- [Author the MVP spec](issues/07-author-mvp-spec.md) — **DESTINATION REACHED**: handoff-ready spec at [`spec.md`](spec.md) synthesising all 6 decisions into 9 sections + build-phase order + out-of-scope. Every product decision settled; a build session can execute without further product calls. The map is complete.

## Not yet specified

<!-- empty: review-loop prototype (04) resolved the two remaining fog patches —
     done-for-today folded into 04's answer; onboarding ruled out of scope. -->

_(none — frontier fully specified into tickets)_

## Out of scope

<!-- ruled beyond this MVP; returns only as a fresh effort -->

- Capitals / flags / reverse-lookup quiz modes — future effort
- Accounts, cloud sync, multi-device — future effort
- Streaks / gamification / achievements — future effort
- Audio (pronunciation) & disputed-status context blurb on reveal — future effort
- Android / desktop / web polish (iOS-only spec) — future effort
- First-run / onboarding flow — none in MVP (settled resolving [04](issues/04-review-loop-prototype.md)); first launch drops into Home with the full new-card backlog
- "Come back later" / extra-practice re-entry when nothing's due — future effort (MVP: Home shows 0/0, Start disabled; done-for-today is a plain ✓ screen)
