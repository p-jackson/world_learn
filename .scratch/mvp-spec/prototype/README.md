# Prototype — review-loop & screen interaction (wayfinder ticket 04)

**Throwaway.** Answered: how should the review screen look & feel across the
three MVP screens? Built as a self-contained HTML file because the real render
target is WKWebView (inline SVG), so this is faithful to production.

## Run

`index.html` is fully self-contained (real NE **50m** geometry inlined) — open
it directly, or serve the folder (`python3 -m http.server`) and open on a phone.
Switch review layouts with the floating bar, `←`/`→`, or `?variant=A|B|C`:

- **A ★ — Immersive + stacked** — the winner (see verdict).
- **B — Bottom sheet** — map card + solid sheet, 2×2 grades w/ intervals.
- **C — Info rail** — smaller map + neighbour chips + vertical grade stack.

## Regenerate geometry

`node generate.mjs` (needs `npm i topojson-client d3-geo` + `countries-50m.json`
from `world-atlas`). Mirrors the production pipeline (ticket 03): topojson →
d3-geo equirectangular → per-feature `{id,name,d,bbox}`, mainland-only bbox.
Then re-inline into `index.html` (replace the `<script id="geo">` body).

## Verdict (settled, see ticket 04 `## Answer`)

Winner = **A** immersive full-bleed map + dropped pin, but with **C's
vertically-stacked** grade buttons (reachable by either thumb). Reveal shows
**short name + formal name + pin**, **no interval previews**. Boundary lines
always visible; 50m geometry (110m lost island/peninsula detail).

`shots/` holds the screenshots the decision was made from.
