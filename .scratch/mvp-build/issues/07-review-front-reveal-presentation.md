# 07 — Review front/reveal presentation

**What to build:** The two visual states of the Review screen for a single Card, driven by a local reveal toggle — no scheduler wiring yet. Front shows the framed map with a thin status strip and a Tap-to-reveal pill; tapping the pill or the map flips to reveal, which shows the names, a pin on the entity, and the four grade buttons. The buttons render and are styled but are inert this ticket. This is the greenfield component/interaction layer the loop will later wire to the scheduler.

Source spec: `.scratch/mvp-spec/spec.md` §4.1.

**Blocked by:** 06 (regional-zoom framing).

- [x] **Front state**: thin top status strip (`N left` · progress bar · `i/total`) + a single "Tap to reveal" pill; tapping the pill **or the map** reveals
- [x] **Reveal state**: short common name **+** formal/long name; a dropped **📍 pin** at the entity centroid (from the asset); four grade buttons
- [x] Grade buttons **vertically stacked, full-width**, Again / Hard / Good / Easy, colored red / orange / green / blue (stacked so all four are thumb-reachable) — no FSRS interval previews on them
- [x] Reveal toggle is local component state; buttons inert (no scheduling) this ticket
- [x] No neighbour/context chips, no bottom-sheet layout, no interval previews (explicitly rejected)
- [x] Demoable on a single hardcoded Card: front ⇄ reveal by tapping map or pill; buttons visible and styled

## Comments

**Implemented** as a new `src/review.rs` (`Review` component) plus a small `pin`
addition to `WorldMap` in `src/map.rs`; `main.rs`'s throwaway map scaffold is
replaced by a `ReviewDemo` on a hardcoded Card (France).

- **Front/reveal** is a local `use_signal(bool)`. The pill reveals; tapping the
  map layer **toggles** (front⇄reveal) so the state is demoable both ways while
  grading is inert — issue 08 swaps that for advance-on-grade.
- **Pin**: rendered as an SVG `<text>` 📍 *outside* the cos-correction `<g>` (so
  the glyph isn't horizontally squished), carrying the correction on its own x via
  `Frame::correct_x`. The node is always present for a highlighted Card and only
  its opacity toggles with `pin`, so front⇄reveal never restructures the SVG
  children (Dioxus #2274). Size is span-relative (`PIN_SIZE_FRACTION`) so its
  apparent size stays constant across Cards under `meet`. No DOM measurement (the
  prototype's `getBoundingClientRect` pin-placement is unnecessary in-SVG).
- **Grades**: `Again/Hard/Good/Easy` stacked full-width, coloured via new palette
  tokens `--color-again/hard/good/easy` (red/orange/green/blue); buttons have no
  `onclick` (inert). No interval previews.
- **Status strip**: `N left` · progress bar · `i/total` from a `QueuePosition`
  prop the demo hardcodes (issue 08 owns the real session queue).
- Pure seams unit-tested: `Frame::correct_x`/`pin_font_size` (map.rs) and
  `QueuePosition`/`Grade` (review.rs). `cargo test` (50), `clippy -D warnings`,
  `fmt --check`, and `cargo check --target aarch64-apple-ios` all green.
- **Not yet eyeballed in the simulator** — pin vertical anchoring and gradient
  legibility are best confirmed with `dx serve --ios`.
