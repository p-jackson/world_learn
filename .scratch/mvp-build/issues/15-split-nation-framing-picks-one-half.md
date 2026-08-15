# 15 — Split-nation framing picks one half (Malaysia)

Status: needs-triage

**What's wrong:** A country split into two comparable landmasses where the
larger holds a bare majority of the area frames off that half alone, cropping
the rest. Malaysia is the case: its largest polygon is East Malaysia (Borneo,
~60% of area), so `framingBbox` frames `[109.54, -6.99, 119.27, -0.86]` —
Sabah/Sarawak — and excludes Peninsular Malaysia, where the capital and most
of the population sit. Surfaced by the issue-10/14 spec review.

Source spec: `.scratch/mvp-spec/spec.md` §4.2. Same class as: 14. Follows up: 14.

## Root cause

`framingPolygons` (`tools/geometry/lib.mjs`, added in issue 14) routes a
feature to a **single dominant polygon** when the largest holds more than
`DOMINANT_AREA_FRACTION` (0.5) of the area, and only unions the major islands
below that. Malaysia's larger half is ~60% > 50%, so it takes the
dominant-mainland branch and frames one half.

The threshold can't simply be raised to catch Malaysia: Japan's mainland
(Honshu) is ~62% of its area and its current single-polygon framing is the
agreed "looks good" reference (issue 10). Any `DOMINANT_AREA_FRACTION` above
~0.6 flips Japan into the archipelago branch and regresses that reference, so
area-share alone can't separate Malaysia (frame both halves) from Japan (frame
Honshu).

## Notes toward a fix

Area share is the wrong axis. What distinguishes "frame one part" (Japan,
USA→Alaska) from "frame both" (Malaysia, Indonesia) is closer to *how much of
the country the framed part omits* combined with *how far the omitted part
is*: Honshu omits only small nearby islands; Borneo omits a peninsula a third
of the country and hundreds of km away. Options to weigh:

- Union parts whose omission would leave out more than some fraction of total
  area, capped by a distance/among-cluster guard so distant exclaves
  (Alaska, French Guiana) still drop.
- A per-nation override table for the handful of genuinely-split states.

Not in scope for issues 10/14 (no regression there — Malaysia framed Borneo
under the old `largestPolygon` too). File-and-park until the split-nation set
is worth generalising.

## Acceptance

- [ ] Malaysia frames both Peninsular Malaysia and Borneo
- [ ] Japan still frames Honshu (issue-10 reference unchanged)
- [ ] USA/France still drop their distant exclaves (no Alaska/Guiana blow-out)
- [ ] Unit test in `tools/geometry/build-geometry.test.mjs` for the split-nation
      case; regenerate `assets/geometry.json`
- [ ] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`, `cargo check --target aarch64-apple-ios`;
      `cd tools/geometry && npm test`
