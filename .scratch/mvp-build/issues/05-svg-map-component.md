# 05 — SVG map component

**What to build:** The foundational map view: an inline-SVG world rendered once from the geometry asset, with one Entity highlighted. Given an `ADM0_A3`, that entity fills a highlight color, everyone else is base fill, and boundary lines are visible on all entities. Stands up the minimal app scaffold needed to see it on screen (no zoom, no grading yet). This is the render primitive every later Review ticket builds on.

Source spec: `.scratch/mvp-spec/spec.md` §3.3, §4.1 (render path only).

**Blocked by:** 01 (geometry asset).

- [ ] Minimal app scaffold replacing the Dioxus starter template, routing to a screen that shows the map for a chosen `ADM0_A3`
- [ ] All ~240 `<path>` elements rendered **once**; highlight varies the `fill` **attribute** per Card — do NOT swap SVG child nodes (Dioxus #2274 workaround)
- [ ] Boundary lines visible on every entity in both states, **non-scaling stroke**
- [ ] Full-bleed / edge-to-edge, no card chrome
- [ ] Demoable: passing different `ADM0_A3` values highlights the correct entity; verify a multi-part feature (e.g. France) and an antimeridian one (e.g. Russia) render sanely at world scale
