# 10 — Large-country regional-zoom distortion (Russia)

Status: ready-for-agent

**What's wrong:** For a physically huge, high-latitude entity (Russia is the
clear case), the regional-zoom frame (issue 06, `src/map.rs`) degenerates into a
near-global, horizontally-squished world map instead of a regional view. The
country reads as "jank and skewed, not like other countries": continents look
thin/vertically-stretched, the landmass is shoved to the left, and a dead ocean
band sits on the right. Surfaced during the issue-08 simulator eyeball.

Source spec: `.scratch/mvp-spec/spec.md` §3.2, §4.2. Follows up: 06.

## Repro

Seed the store so today's new-card batch starts at Russia (first 7 intro cards
CHN, IND, USA, IDN, BRA, PAK, NGA marked introduced today, due future), launch,
observe card 1 (Russia) front state. The whole world renders, compressed to ~half
width. Compare with any small/mid country (France, Japan) which frames cleanly.

## Root cause

`Frame::for_bbox` on Russia's mainland bbox `[27.35, -77.73, 180, -41.2]`
(x = lon, y = −lat; lat 41.2°N–77.7°N, lon 27.35°E–180°E):

- `center_y = -59.465` → `scale_x = cos(59.5°) ≈ 0.51`. This is applied as
  `scale(0.51 1)` to the **entire** map `<g>`, so the whole world is squished
  horizontally by half about the pivot at lon 95°E.
- `corrected_width = 152.65 × 0.51 ≈ 77.6`; `span = 77.6 × FRAME_PADDING(3.4)
  ≈ 264°` → a viewBox spanning ~264° (near-global) rather than a regional window.

Two compounding effects: (a) the global horizontal cos-scale only looks natural
when the visible content is near the frame's centre latitude — at a ~130°-tall
frame it distorts everything; (b) `FRAME_PADDING 3.4` blows an already-huge
country out to global scale (even China frames to ~172°, visibly under-zoomed).
Small entities hide both because their frame is small, centred, and floored by
`MIN_WINDOW_DEG`.

## Proposed fix (recommended)

Cap the frame span: add `MAX_WINDOW_DEG` (~90°) and clamp `span` to it in
`for_bbox`. Big countries then zoom to a regional view (Russia's corrected width
77.6° < 90° fits; USA/China fit too), so the cos-correction only spans content
near the country's latitude and reads naturally — like every other card. Small /
island cards are unaffected (already floored by `MIN_WINDOW_DEG`).

Add unit tests: a large-bbox entity clamps to `MAX_WINDOW_DEG`; a mid entity is
unchanged; `MIN_WINDOW_DEG < span ≤ MAX_WINDOW_DEG` holds across the real deck.
Verify Russia + China visually in the simulator before closing.

Note: Russia spans 41°–78°N, so a single `cos(centre_lat)` factor still can't be
perfectly conformal across its own height — capping the span makes it *look like
the other cards*, which is the goal here; a per-latitude correction is out of
scope unless the capped result still reads wrong.

## Acceptance

- [ ] `MAX_WINDOW_DEG` clamp added to `Frame::for_bbox`, with the reasoning as a
      doc comment (match issue-06 comment density)
- [ ] Unit tests for the clamp (large clamps, mid unchanged, real-deck invariant)
- [ ] Russia and China eyeballed in the simulator: regional view, no global squish
- [ ] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`, `cargo check --target aarch64-apple-ios`
