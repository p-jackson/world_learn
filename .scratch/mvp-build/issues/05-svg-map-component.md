# 05 — SVG map component

Status: done

**What to build:** The foundational map view: an inline-SVG world rendered once from the geometry asset, with one Entity highlighted. Given an `ADM0_A3`, that entity fills a highlight color, everyone else is base fill, and boundary lines are visible on all entities. Stands up the minimal app scaffold needed to see it on screen (no zoom, no grading yet). This is the render primitive every later Review ticket builds on.

Source spec: `.scratch/mvp-spec/spec.md` §3.3, §4.1 (render path only).

**Blocked by:** 01 (geometry asset).

- [x] Minimal app scaffold replacing the Dioxus starter template, routing to a screen that shows the map for a chosen `ADM0_A3`
- [x] All ~240 `<path>` elements rendered **once**; highlight varies the `fill` **attribute** per Card — do NOT swap SVG child nodes (Dioxus #2274 workaround)
- [x] Boundary lines visible on every entity in both states, **non-scaling stroke**
- [x] Full-bleed / edge-to-edge, no card chrome
- [x] Demoable: passing different `ADM0_A3` values highlights the correct entity; verify a multi-part feature (e.g. France) and an antimeridian one (e.g. Russia) render sanely at world scale

## Comments

**Implemented.** `src/map.rs` + `src/main.rs`, styled with Tailwind
(`tailwind.css` `@theme` palette; no bespoke CSS).

- `WorldMap { deck, highlighted }` (`map.rs`): renders every Deck path once in
  fixed intro order with stable `key: "{code}"`; only `fill_for(code,
  highlighted)` varies the per-path `fill` **attribute** — the #2274 workaround
  (no child swaps). `WORLD_VIEW_BOX = "-180 -90 360 180"` is the whole
  equirectangular world (x=lon, y=−lat from issue 01). `SharedDeck` = `Rc<Deck>`
  with ptr-eq `PartialEq` for cheap prop-diffing.
- Boundary stroke is uniform, non-scaling, applied to every child `<path>` via
  one Tailwind child-variant on the `<svg>` (`[&_path]:[vector-effect:non-scaling-stroke]`,
  etc.) — independent of `fill`, so it shows on every entity in both states.
  Map fills its container; the screen makes it full-bleed over an ocean ground.
- `main.rs` replaces the starter template: loads the Deck once (logs+surfaces a
  load failure), renders `MapScreen`. The screen carries a **throwaway** dev bar
  (step/jump to any `ADM0_A3`) purely to satisfy "demoable" — real
  Home/Settings routing is issue 09.

Verified France (mainland + French Guiana, one multipart `d`) and Russia
(antimeridian + Kaliningrad + Chukotka, cut correctly by d3) render sanely at
world scale via a standalone-HTML screenshot of the shipped asset. 39 tests
green (4 new); clippy `-D warnings`, fmt, `aarch64-apple-ios` check all clean.

Two-axis `/code-review`: Spec faithful (core render-once + fill-attribute +
non-scaling stroke correct; "routing" and strict "edge-to-edge" partial only
because of the acknowledged throwaway dev bar). Standards clean — no hard
violations; judgement-call smells confined to the throwaway scaffold.
