# MVP Spec — Country-location flashcard app

Handoff-ready spec for an iOS app (Dioxus 0.7, Rust) that drills **"where is
this?"**: a regional-zoom map with one Entity highlighted, self-reveal + 4-button
grading, FSRS scheduling, over the full inhabited Natural Earth admin-0 set
(incl. contested territories).

This document is the synthesis of the wayfinding map at `map.md`. Every product
decision is settled here; a build session should be able to execute it without
further product calls. Where a choice was made, the fallback is noted so an
implementation snag has a known escape hatch.

Glossary terms (**Entity**, **Card**, **Review**, **Deck**, **Regional zoom**)
are defined in `CONTEXT.md` and used with that meaning throughout.

---

## 1. Product overview

The learner sees a regional-zoom map with one Entity highlighted and boundary
lines visible. They recall its name mentally, tap to reveal the true name + a
dropped pin, then self-grade **Again / Hard / Good / Easy**. FSRS schedules the
next Review. New Cards are introduced big→obscure at a configurable daily cap.

**MVP surface = three screens**: Home, Review loop, Settings. Plus a
done-for-today end state. Nothing more — no accounts, no onboarding, no stats.

### Fixed premises (settled while charting — treat as given)

- **Self-reveal recall**: mental guess → tap → reveal → grade. **4-button** FSRS
  grades (Again/Hard/Good/Easy).
- **Regional zoom**, medium framing, fixed frame per Card.
- Scope = inhabited Natural Earth admin-0 features **incl. contested** — **240
  Cards** (see §2).
- **FSRS** scheduler (`fsrs` crate v6+); SM-2 was the fallback, not needed.
- New-Cards/day cap (configurable, default 10), fixed intro order big→obscure.
- Render = **inline SVG in RSX** (iOS renders via WKWebView; feasibility
  confirmed while charting).

---

## 2. The Deck — Entity set & derivation rules

Derived from Natural Earth **50m admin-0** (241 features) by rule — never a
hand-typed 240-row list. Every Entity is keyed on **`ADM0_A3`** (populated and
unique for every feature incl. disputed ones; **never `ISO_A3`**, which is `-99`
for France/Norway/many disputed entities).

### 2.1 Inclusion rule → 240 Cards

**Include every feature EXCEPT uninhabited dependencies:**

- `TYPE ∈ {Sovereign country, Country, Disputed, Indeterminate}` → **211**
  (all sovereign states, semi-independent constituents like Greenland, and all
  6 contested entities — Kosovo, Western Sahara, Northern Cyprus, Somaliland,
  Taiwan, Palestine; their `TYPE` is inconsistent, so they ride in on these
  classes, **not** on a `TYPE="Disputed"` filter).
- **PLUS** `TYPE = Dependency AND POP_EST > 0` → **28** of the 30 dependencies.
- **Drop only** the two `POP_EST = 0` dependencies: **Heard I. & McDonald Is.**
  and **Ashmore & Cartier Is.**
- **PLUS Tuvalu**, a sovereign state NE omits from 50m for being too small
  (26 km²) — supplemented from the **10m** source (§3.3). It has no land
  neighbours, so 10m geometry seams with nothing.

Total = **211 + 28 from 50m + Tuvalu = 240 Cards.** Criterion is **inhabited**,
not "big enough to locate" — tiny-but-inhabited entities (Niue, Anguilla,
Pitcairn, Bermuda) are in. `POP_EST > 0` is a single data-derived rule, no
curated allow/deny list. Small sovereign states (Vatican, Nauru, Tuvalu) are
always in regardless of size. No area field exists in NE; inclusion needs none.

### 2.2 Palestine patch

NE folds Palestine into Israel at admin-0. Source Palestine from Natural Earth's
**`ps` point-of-view file** (`ne_10m_admin_0_countries_ps`): it has a clean
Palestine polygon *and* an Israel drawn without it. Swap **both** in over the
default merged Israel — no boolean geometry. Fallback: union West Bank + Gaza
from the disputed-areas layer.

### 2.3 Display name (reveal)

Reveal shows **curated common name (primary) + `NAME_LONG` (formal, secondary)**
— e.g. "France / French Republic", "Kosovo / Republic of Kosovo (disputed)".

- Common name derived from NE `NAME` **by rule**, with a **small curated
  override table (~15–25 entries)** fixing NE abbreviations to natural forms:
  `W. Sahara`→Western Sahara, `Dem. Rep. Congo`→DR Congo, `N. Cyprus`→Northern
  Cyprus, `Bosnia and Herz.`→Bosnia & Herzegovina, etc.
- Rules + overrides, never a hand-typed 240-row list.
- Grading is self-assessed, so the displayed string never gates correctness — it
  only has to read naturally.

### 2.4 Introduction order (big→obscure)

Fixed sequence new Cards enter = **sort by `LABELRANK` ascending** (lower = more
prominent), **tiebreak `POP_EST` descending**. LABELRANK is NE's curated
label-prominence signal (populated for all entities incl. contested), so famous
countries lead and lesser-known ones trail. Contested entities land mid-to-late
(Taiwan LR 3; Palestine/Somaliland 5; Kosovo/N.Cyprus 6; W. Sahara 7) — the
desired feel.

*Rejected: pure `POP_EST` desc (buries famous-small nations). Rejected for
inclusion: `MIN_ZOOM` cutoff (would drop Palestine at 7.0).*

---

## 3. Geometry asset pipeline

**Decision: 50m base geometry, equirectangular projection, build-time static
asset keyed by `ADM0_A3`.**

### 3.1 Source & simplification

- **50m** NE admin-0 (not 110m): we zoom *in*, so coastline detail matters; 110m
  looks jagged magnified and drops small nations. 10m is overkill.
- **Source from full-attribute NE data** (martynafford's GeoJSON or raw NE),
  **not** the attribute-stripped `world-atlas` TopoJSON (which keeps only a
  numeric id + `name` — we need `ADM0_A3`, `TYPE`, `POP_EST`, `LABELRANK`,
  `NAME`, `NAME_LONG`).
- Round projected coordinates at build time (~2 decimal places) to shrink the
  shipped asset. ~240 paths is well within the render budget.

### 3.2 Projection

**Equirectangular (Plate Carrée)** with a **per-entity `cos(lat_center)`
horizontal correction** (latitude midpoint of the entity's frame). Lon/lat map
linearly → bounding boxes are trivial and zoom is a pure `viewBox` swap, no
per-card re-projection. The cos-scale removes high-latitude horizontal stretch;
because each Card shows a small latitude band, one factor per entity looks
locally correct. Mercator is rejected (its conformality is worthless here and
vertical exaggeration hurts).

### 3.3 Build-time generation

A small **d3-geo Node script** (dev-only, never ships):

1. Fetch NE 50m (full attributes) → `topojson.feature`.
2. Apply the Palestine `ps`-file swap (§2.2) — guarded: only if the source
   folds Palestine into Israel (martynafford's 50m already separates them, so it
   no-ops in practice).
3. Supplement Tuvalu from 10m (§2.1) — guarded: only if absent from the base.
4. Filter to the 240 Deck features (§2.1).
5. Per feature: `d3.geoBounds` + `d3.geoPath(geoEquirectangular)` →
   `{ name (common), name_long, d (SVG path), bbox:[minx,miny,maxx,maxy],
   labelrank, pop_est, centroid }`.
6. Emit JSON (or a generated `.rs`) map: **`ADM0_A3 → { … }`**.

App loads via `include_str!` + `serde_json`, renders all ~240 `<path>` once,
**varies the `fill` attribute per Card** (Dioxus #2274 workaround — vary `fill`,
not SVG child nodes), sets `viewBox` per Card (§4.2).

**All-Rust fallback**: `topojson` + `geo` + `geo-svg` + hand-rolled
equirectangular affine.

**Antimeridian caveat**: Russia / Fiji / Alaska cross ±180°. d3-geo cuts them
correctly; a naive min/max-lon bbox will wrap the globe — verify those few.

---

## 4. Review loop & screens

Settled by a throwaway HTML prototype (asset: `prototype/` — `index.html`,
`generate.mjs`, `shots/`). **WINNER: immersive full-bleed map + vertically-
stacked grades.**

### 4.1 Review screen

- Regional-zoom map is **full-bleed / hero** — edge-to-edge, no card chrome.
- **Boundary lines always visible** on every entity, both states (non-scaling
  stroke) — explicit requirement.
- **Front state**: thin top status strip (`N left` · progress bar · `i/total`)
  + a single "Tap to reveal" pill. Tapping the pill **or the map** reveals.
- **Reveal state**: short name **+ formal/long name**, a dropped **📍 pin** on
  the entity centroid, and the **4 grade buttons vertically stacked, full-width**
  — Again / Hard / Good / Easy, colored **red / orange / green / blue**. Stacked
  (not a row) so all four are reachable by either thumb.
- **No FSRS interval previews** on the buttons in MVP — just the 4 labels.
- Rejected: neighbour/context chips on reveal; bottom-sheet layout; interval
  previews.

### 4.2 Framing / geometry (refines §3 for the render path)

- `viewBox` = target **mainland bbox × ~3.4** padding, square, centred.
- Enforce a **minimum span (~6° → ~20° window)** so tiny/island entities aren't
  over-zoomed.
- Frame on the entity's **largest polygon** (mainland), ignoring scattered
  overseas parts — else multi-part features (France + French Guiana) blow the
  bbox out to the whole Atlantic. The production pipeline must compute the
  mainland-polygon bbox, not the whole-feature bbox.
- Per-Card **cos(lat) horizontal correction** via a group transform (§3.2).

### 4.3 Home screen

- Title + tagline; two stat tiles: **reviews-due** and **new-today**.
- Primary `Start · N cards`; Settings link.
- When nothing's due: Home shows **0/0** and Start is absent/disabled — no
  special re-entry screen.

### 4.4 Settings screen

- **New-cards/day stepper** (default 10) — the **only** interactive control.
- Read-only rows: Scheduler (FSRS), Deck (240, incl. contested).

### 4.5 Done-for-today

✓ + "N reviewed · next batch unlocks tomorrow" + Back to home. No
come-back-later / extra-practice state in MVP.

### 4.6 Onboarding

**None.** First launch = Home with the full new-Card backlog; the daily cap
applies.

### 4.7 Transitions

Home → Review (per-Card front→reveal→grade loop) → Done → Home; Home ⇄ Settings.

---

## 5. Data model & scheduling

**One sparse serde-JSON file. `cards` keyed by `ADM0_A3`, holding only Cards
that have left "new". Lifecycle status is derived, not stored.**

### 5.1 Persisted shape

```jsonc
{
  "schema_version": 1,
  "settings": { "new_cards_per_day": 10 },   // the only interactive setting
  "cards": {
    "FRA": {
      "stability": 3.17,           // FSRS MemoryState.stability (f32)
      "difficulty": 5.20,          // FSRS MemoryState.difficulty (f32)
      "due": "2026-08-16",         // local date; ≤ today ⇒ due
      "last_review": "2026-08-14", // local date of last grade
      "introduced_on": "2026-08-10"// set once, on first grade; drives daily-new cap
    }
    // …only seen cards. Absent key = a new, not-yet-introduced Card.
  }
}
```

- Flat card records — memory fields inline, not nested. ISO **local-date**
  strings (`YYYY-MM-DD`), day precision. No other top-level keys.
- **Not stored** (on purpose): lifecycle enum, `reps`/`lapses` (no stats screen
  — out of scope), `desired_retention` (code constant `0.9`), FSRS optimized
  weights (additive later behind `schema_version`).

### 5.2 Derived at runtime, never persisted

- **Deck membership + intro order** — from the static geometry asset (§2/§3:
  `LABELRANK` asc, tiebreak `POP_EST` desc).
- **new backlog** = deck keys − `cards` keys, in intro order.
- **new allowance remaining** = `new_cards_per_day − count(cards where
  introduced_on == today)`.
- **due set** = `{ cards where due ≤ today }`.
- **status** — `new` if key absent; else `due` if `due ≤ today`, else
  `scheduled`. ("Learning" is not persisted — just a seen Card at `due = today`.)
- **session queue** (transient, in-memory) = due set ++ up-to-allowance new
  Cards in intro order.

### 5.3 FSRS scheduling

Crate **`fsrs = "6"`** (default features; confirm ≥6.x — `burn` is a
dev-dependency only in v6+, whole prod dep set is pure Rust, cross-compiles to
`aarch64-apple-ios` cleanly).

- Per-card FSRS state = **`MemoryState { stability: f32, difficulty: f32 }`** —
  all the crate persists. App owns `due` / `last_review` / `introduced_on`.
- API: `FSRS::default().next_states(current: Option<MemoryState>,
  desired_retention: f32, days_elapsed: u32) -> Result<NextStates, _>`, where
  `NextStates { again, hard, good, easy: ItemState }` and
  `ItemState { memory: MemoryState, interval: f32 /* days */ }`.
- Ship stock **`DEFAULT_PARAMETERS`** (FSRS-6 / Anki defaults); no training.
  Per-user `compute_parameters` optimization is a clean additive later phase.
- `desired_retention = 0.9` (code constant).

### 5.4 Session & grading rules

- **Day boundary** = local device midnight; not configurable in MVP.
- **Intervals rounded to whole days, min 1**: `interval.round().max(1.0)`. `due`
  is a date, not a timestamp — no sub-day session loop. (`fsrs` returns raw
  fractional days with no built-in same-day step scheduler; the app owns
  granularity.)
- **On grade**: `next_states(current_memory_state, 0.9, days_elapsed)` where
  `days_elapsed = max(0, today − last_review)` as `u32`; a new Card's first
  grade passes `current = None, days_elapsed = 0`.
  - **Again** → update memory_state, `due = today`, `last_review = today`;
    requeue to the **back of the session queue** (re-drills until a pass).
    Persisted `due = today` survives a mid-session quit — a re-drill is never
    lost.
  - **Hard / Good / Easy** (passes) → persist memory_state, `last_review =
    today`, `due = today + round(interval)`; Card exits the session.
  - **First grade of a new Card** (any grade) → create the record, set
    `introduced_on = today` (counts against the daily cap from that moment).
- `last_review` updates on **every** grade, so same-day re-drills feed
  `days_elapsed = 0` (FSRS `delta_t == 0` short-term path).
- Persist **atomically** after each grade (tiny file).

---

## 6. Persistence & platform

**A single serde-JSON file in `Library/Application Support/<app>/`.** SQLite
(`rusqlite`, `features=["bundled"]`) is the fallback if the dataset grows or
formal migrations are wanted (over-engineered for 240 records).

- Dioxus mobile runs **Rust natively** (it drives, but does not run inside, the
  WKWebView). This is a **native-filesystem** problem — `localStorage`/IndexedDB
  are irrelevant.
- ~240 records + settings is tens of KB. Hold in a `Signal`/struct, persist on
  mutation. Write **atomically** (temp file in same dir → `rename`). Include
  `schema_version: u32`; migrate by load-all/save-all.
- **The one unavoidable platform-specific piece**: obtaining the iOS sandbox
  path. `dirs`/`directories`/`dioxus-sdk` all resolve iOS **wrong** (they fall
  back to Linux/XDG). Write a ~15-line **`objc2` + `objc2-foundation`** helper
  calling `NSSearchPathForDirectoriesInDomains(NSApplicationSupportDirectory,
  NSUserDomainMask, true)`; `create_dir_all` on first launch. Dioxus 0.7 ships
  no helper for this.
- Use **Application Support** (app-managed, backed up, hidden) — not Caches
  (purgeable) for review history.

---

## 7. Tech stack summary

| Concern | Choice |
|---|---|
| Framework | Dioxus 0.7, iOS target (`dx serve --ios`) |
| Rendering | Inline SVG in RSX; vary `fill` per Card (Dioxus #2274) |
| Scheduler | `fsrs = "6"` (default params, `desired_retention = 0.9`) |
| Geometry | NE 50m admin-0, equirectangular, build-time d3-geo Node script → static `ADM0_A3 → {…}` asset |
| Persistence | serde-JSON file in iOS Application Support; `objc2` path helper |
| Entity key | `ADM0_A3` |

---

## 8. Suggested build phases

Not a product decision — a suggested execution order for the build session(s).

1. **Geometry asset pipeline** (§3): d3-geo script producing the 240-entry
   `ADM0_A3 → {…}` asset, incl. Palestine swap and mainland-bbox framing. Verify
   antimeridian entities.
2. **Persistence layer** (§6): `objc2` path helper, atomic JSON load/save,
   `schema_version`.
3. **Domain + scheduling core** (§2, §5): deck derivation from the asset, FSRS
   `next_states` wiring, session-queue + grading rules — pure Rust, unit-testable
   without UI.
4. **Review screen** (§4.1–4.2): full-bleed SVG map, front→reveal→grade loop.
5. **Home / Settings / Done** (§4.3–4.5) + transitions (§4.7).

---

## 9. Out of scope (MVP)

Ruled beyond this MVP; each returns only as a fresh effort.

- Capitals / flags / reverse-lookup quiz modes.
- Accounts, cloud sync, multi-device.
- Streaks / gamification / achievements.
- Audio (pronunciation) & disputed-status context blurb on reveal.
- Android / desktop / web polish (iOS-only spec).
- First-run / onboarding flow — first launch drops into Home with the full
  new-Card backlog.
- "Come back later" / extra-practice re-entry when nothing's due — Home shows
  0/0, Start disabled; done-for-today is a plain ✓ screen.
- Stats screen; `reps`/`lapses` tracking; per-user FSRS parameter optimization
  (all additive later behind `schema_version`).
