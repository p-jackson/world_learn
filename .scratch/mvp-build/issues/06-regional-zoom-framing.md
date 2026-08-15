# 06 — Regional-zoom framing

Status: done

**What to build:** Turn the world map into the Review's regional zoom. For the highlighted Entity, the viewport frames its mainland with padding so neighbours show as location cues, tiny islands aren't over-zoomed, and high-latitude horizontal stretch is corrected. After this ticket, showing a Card presents the medium regional framing the product calls for — a pure `viewBox` swap per Card, no re-projection.

Source spec: `.scratch/mvp-spec/spec.md` §3.2, §4.2.

**Blocked by:** 05 (SVG map component).

- [x] `viewBox` = the entity's **mainland** bbox (from the asset) × ~3.4 padding, square, centred
- [x] Minimum span enforced (~6° → ~20° window) so tiny/island entities aren't over-zoomed
- [x] Per-Card `cos(lat_center)` horizontal correction applied via a group transform (latitude midpoint of the frame)
- [x] Zoom is a pure per-Card `viewBox` swap on the single rendered map — no per-card re-projection, no re-render of paths
- [x] Demoable: France frames on the European mainland (not the Atlantic out to French Guiana); a small island (e.g. Nauru/Niue) shows a sane window, not a pinpoint; a high-latitude entity is not horizontally stretched

## Comments

**Implemented** in `src/map.rs` — a pure `Frame` value plus a one-line wiring
change to `WorldMap`; no asset or pipeline change (the pipeline already emits the
**mainland** bbox via `largestPolygon`, so France's `bbox` is continental, and
the pin/centroid are mainland too — issue 01).

- `Frame::for_bbox([min_x, min_y, max_x, max_y])` computes a square, centred
  `view_box()` string and a `transform()` string. `FRAME_PADDING = 3.4`,
  `MIN_WINDOW_DEG = 20.0` (so ~6° bbox → ~20.4° > floor; smaller entities clamp
  to 20°).
- **cos(lat) correction** is a single group `transform` on the one `<g>` wrapping
  every path: `translate(cx·(1−k) 0) scale(k 1)` — scale x by `k = cos(frame
  midpoint lat)` about the frame centre, y untouched. `k` floored at
  `MIN_COS_SCALE = 0.1` (degenerate near-pole guard; no inhabited mainland
  reaches it).
- The square side is sized off the **cos-corrected** width `(max_x−min_x)·k`, not
  the raw lon span. The `view_box` lives in raw projected coords but the geometry
  inside is compressed by `k` about `cx`, so this frames the real-world footprint
  tightly and stays centred (view_box centre = transform's fixed point). A
  deliberate refinement past the literal "bbox × 3.4" wording.
- `WorldMap` looks up the highlighted Card's bbox, builds the `Frame`, and swaps
  only `view_box` + the `<g>` `transform` per Card — the path tree is untouched
  (Dioxus #2274). No highlight → whole world (`WORLD_VIEW_BOX`, no transform).
- Tests: framing (centre/square/min-window/high-lat compression) + real-deck
  invariants (France <40° over Europe, Niue floored to 20°, Iceland `scale_x`
  < 0.5). `cargo test` / `clippy -D warnings` / `fmt --check` all green.
