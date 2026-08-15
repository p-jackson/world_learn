# 01 — Geometry asset pipeline

**What to build:** A dev-only build step that turns Natural Earth admin-0 data into the static geometry asset the app ships and renders from. Running it produces a JSON map keyed by `ADM0_A3`, one entry per Deck Entity, each carrying everything a Card needs to render and frame: display names, an SVG path string, a bounding box, centroid, and the intro-order signals. This asset is the single source of Deck membership, geometry, and ordering for every downstream ticket. The script itself never ships.

Source spec: `.scratch/mvp-spec/spec.md` §2, §3.

**Blocked by:** None — can start immediately.

- [x] d3-geo Node script (dev-only, not in the app build) reads NE **50m** admin-0 with full attributes (not the attribute-stripped world-atlas TopoJSON)
- [x] Palestine `ps` point-of-view swap applied (§2.2): clean Palestine polygon + Israel-without-it swapped in over the merged Israel; fallback noted (union West Bank + Gaza)
- [x] Filtered to the **239** Deck features by rule (§2.1): `TYPE ∈ {Sovereign country, Country, Disputed, Indeterminate}` plus `TYPE = Dependency AND POP_EST > 0`; drop only the two `POP_EST = 0` dependencies. No hand-typed list
- [x] Per feature emit `{ name (common), name_long, d (SVG path, equirectangular), bbox:[minx,miny,maxx,maxy], labelrank, pop_est, centroid }`, keyed by `ADM0_A3`
- [x] `bbox` is the **mainland (largest) polygon** bbox, not whole-feature (§4.2) — multi-part features (e.g. France + French Guiana) must not blow out to an ocean-spanning box
- [x] Common name = NE `NAME` by rule + a small curated override table (~15–25 entries: `W. Sahara`→Western Sahara, `Dem. Rep. Congo`→DR Congo, etc., §2.3)
- [x] Projected coordinates rounded (~2 dp) to shrink the asset
- [x] Output is `include_str!`-loadable JSON (or generated `.rs`)
- [x] Verified: exactly 240 keys (239 from the 50m rule + Tuvalu, see Comments); Palestine present as its own entity; antimeridian entities (RUS, FJI, USA/Alaska) are cut correctly, not globe-wrapped

## Comments

**Implemented.** `tools/geometry/` → `assets/geometry.json`. Two data-reality
notes (see `tools/geometry/README.md`):

- **Deck = 240, not 239.** Rule over 50m yields 239; **Tuvalu** is absent from
  50m (too small) and supplemented from 10m per product call → 240. `DECK_COUNT`
  is the single source of that number.
- **Palestine swap not run.** The 50m source already separates Palestine from
  Israel (Israel excludes West Bank/Gaza, verified by point-in-polygon), so the
  guarded `pse` swap correctly no-ops. Kept as fallback.

Sources pinned to commits for reproducibility. `centroid` = mainland-polygon
centroid (keeps the reveal pin inside the framed view).

Follow-up (resolved): `spec.md`/`CONTEXT.md` now say 240.
