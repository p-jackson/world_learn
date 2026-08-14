# Geometry asset pipeline (dev-only)

Turns Natural Earth admin-0 into `assets/geometry.json`, the static geometry
asset the app ships and renders from. **This tool never ships in the app build.**

Spec: `.scratch/mvp-spec/spec.md` §2, §3, §4.2.

## Run

```sh
cd tools/geometry
npm install
npm run build      # -> ../../assets/geometry.json
npm test           # unit tests + produced-asset invariants
```

Source data is fetched once and cached under `tmp/ne-cache/` (gitignored), so
repeat builds are offline.

## Output shape

Flat JSON map, one entry per Deck Entity, keyed by `ADM0_A3`:

```jsonc
{
  "FRA": {
    "name": "France",              // curated common name (§2.3)
    "name_long": "France",         // NE NAME_LONG (formal, reveal 2nd line)
    "d": "M1.71,-42.5L…Z",         // SVG path, equirectangular, whole feature
    "bbox": [-4.76,-51.1,8.14,-42.34], // MAINLAND [minx,miny,maxx,maxy]
    "labelrank": 2,                // intro-order signal (§2.4)
    "pop_est": 67106161,           // intro-order tiebreak
    "centroid": [2.46,-46.57]      // mainland centroid, for the reveal pin
  }
}
```

Coordinates are equirectangular with `x = lon`, `y = -lat` (degree units, SVG y
down), rounded to 2 dp. Bounding boxes are trivial and per-Card zoom is a pure
`viewBox` swap; the per-entity `cos(lat)` correction happens at render time
(§3.2), not here. `bbox`/`centroid` use the **largest (mainland) polygon** so
multi-part features (France + French Guiana, USA + Alaska) don't frame out to an
ocean-spanning box (§4.2). The `d` path is the whole feature, cut at ±180 by
d3-geo's antimeridian clipping (RUS/USA/FJI render as pieces per side, not a
globe-wrapping line).

Data sources are **pinned to specific commits** (`MARTYNAFFORD` / `NVKELSO` in
`build-geometry.mjs`) so a rebuild is reproducible.

## Deck size: 240, not the spec's 239

The inclusion rule over NE 50m yields **239**; **Tuvalu is supplemented from 10m
to make 240** (see below). `DECK_COUNT` in `overrides.mjs` is the single source
of that number, asserted by both the build guard and the tests.

## Two places data reality diverges from the spec's assumptions

Both are logged by the build and covered by tests — flagged here so they aren't
mistaken for bugs.

- **Palestine (§2.2).** The spec assumes NE folds Palestine into Israel and
  prescribes a `pse` point-of-view swap. The chosen 50m source
  (martynafford's GeoJSON) already ships a clean, separate `PSX` and an Israel
  polygon that excludes the West Bank/Gaza, so no swap is needed and none is
  applied (mixing the 10m `pse` file into 50m neighbours would create border
  seams). The swap is implemented as a **guarded fallback**: the build probes
  whether Israel still covers the West Bank and only then fetches the `pse` file
  and swaps `ISR`+`PSX` in. `palestine_handling` in the build report records
  which path ran (`source-already-separated` here).

- **Tuvalu.** The spec names Tuvalu as an always-in small state (§2.1), but the
  50m NE source has **no Tuvalu feature at all** (too small at 50m). It is
  **supplemented from the 10m source** (`SUPPLEMENT_CODES` in `overrides.mjs`),
  making the deck 240. Tuvalu has no land neighbours, so 10m geometry introduces
  no border seam with 50m. Same guarded pattern as Palestine: the 10m file is
  only fetched when a required code is actually missing. Rendering something this
  tiny is a later concern (min-span framing, §4.2). `supplemented` in the build
  report lists what was pulled (`TUV` here).

## Files

- `build-geometry.mjs` — CLI: fetch (cached) → build → write, with guardrails.
- `lib.mjs` — pure transforms (inclusion rule, names, mainland bbox/centroid,
  Palestine guard/swap, 10m supplements, `buildAsset`). Where the tests live.
- `overrides.mjs` — curated common-name table + rule constants (`DECK_COUNT`,
  `SUPPLEMENT_CODES`).
- `build-geometry.test.mjs` — unit tests + invariants on the produced asset.
