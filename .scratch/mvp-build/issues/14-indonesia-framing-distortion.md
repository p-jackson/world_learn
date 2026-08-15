# 14 — Archipelago-nation framing distortion (Indonesia)

Status: ready-for-agent

**What's wrong:** The Indonesia review card's map framing looks off — zoomed/
centred wrong, same class of problem as issue 10 (Russia), but the opposite
failure mode: instead of zooming out to a near-global squish, it zooms in
tight on a single island and crops the rest of the archipelago. Surfaced
during the issue-09 demo eyeball.

Source spec: `.scratch/mvp-spec/spec.md` §3.2, §4.2. Same root cause class as: 10.

## Root cause

`assets/geometry.json`'s `IDN` entry has `bbox: [108.91, -4.37, 118.98, 4.17]`,
`centroid: [114.01, 0.19]` — a ~10°×8.5° box sitting over Kalimantan
(Indonesian Borneo). Real Indonesia spans roughly 95°E (Sumatra) to 141°E
(Papua), so this bbox excludes Sumatra, Java, Sulawesi, Maluku, and Papua
entirely.

Per `tools/geometry/lib.mjs`, `mainlandBbox`/`mainlandCentroid` frame every
country off `largestPolygon` — the single largest-area ring (spec §4.2, added
to keep exclaves like Alaska from blowing out the bbox, per issue 10's
comment). For a contiguous country that heuristic is fine. For an archipelago
nation whose largest single island (Kalimantan) is only a fraction of the
country, it picks one island and frames as if that's the whole country —
`Frame::for_bbox` (`src/map.rs`) then zooms tightly onto Kalimantan while the
rest of the rendered `d` path (which does include every island) sits outside
the frame or barely pokes in at the edges.

## Proposed fix (recommended)

`largestPolygon`-based framing is the wrong heuristic for multi-polygon
countries whose islands are comparably sized. Consider unioning the bbox
across all polygons above some area threshold (e.g. islands within an order
of magnitude of the largest, or cumulative area covering some % of the
country) rather than the single largest ring — this fixes Indonesia without
regressing Russia/Alaska (where the excluded exclaves are genuinely tiny by
comparison). Verify Indonesia's corrected bbox roughly spans Sumatra→Papua
before/after; re-run issue 10's Russia/China checks to confirm no regression.

Add a unit test in `tools/geometry/build-geometry.test.mjs` asserting IDN's
bbox covers a representative multi-island span (e.g. contains both a Sumatra
and a Papua point), and a `Frame::for_bbox`-level test if the fix also touches
`src/map.rs`.

## Acceptance

- [ ] Indonesia's bbox/centroid cover the archipelago, not just Kalimantan
- [ ] Indonesia eyeballed in the simulator: recognisable regional view, not a
      single-island crop
- [ ] Russia/China (issue 10) re-checked for no regression
- [ ] Unit test(s) added covering the archipelago bbox case
- [ ] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`, `cargo check --target aarch64-apple-ios`; if
      `tools/geometry` changes, also `cd tools/geometry && npm test` and
      regenerate/commit `assets/geometry.json`
