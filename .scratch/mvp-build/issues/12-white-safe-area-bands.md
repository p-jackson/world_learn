# 12 — White "forehead" / "chin" in the safe areas

Status: done

**What's wrong:** On-device there are white bands at the top (status-bar area, the
"forehead") and bottom (home-indicator area, the "chin"). The app's dark ocean
background stops at the safe-area edges instead of bleeding to the screen edges,
so white shows through. Surfaced during the issue-08 simulator eyeball (visible in
every screenshot).

Source spec: `.scratch/mvp-spec/spec.md` §3.3/§4.1 (full-bleed presentation).

## Root cause (likely)

The webview `body` has **no background colour**, so it defaults to white, and that
white is what shows in the safe-area insets the app content doesn't paint:

- `src/main.rs` links only `TAILWIND_CSS`. The starter `assets/main.css` — which
  *does* set `body { background-color: #0f1116 }` — is **not linked** and never
  loads. (It's leftover Dioxus scaffolding, `#hero`/`#links` styles the app doesn't
  use; consider deleting it rather than wiring it in.)
- The app root is `div { class: "fixed inset-0" }` with `bg-ocean` only on the
  inner Review/Done divs. Nothing paints the `body` or the area behind the safe
  insets.

Two things worth checking together:
1. **Paint the body** the ocean colour (`--color-ocean: #0f1720`) so any exposed
   safe-area is dark, not white — e.g. a `bg-ocean` (or explicit background) on the
   `html`/`body`, or on the fixed root.
2. **Full-bleed the webview** into the safe areas: confirm the generated index
   carries `<meta name="viewport" content="viewport-fit=cover">` so content extends
   under the status bar / home indicator (the Review strip already pads with
   `env(safe-area-inset-*)`, which only helps once the webview draws edge-to-edge).

Verify on-device: the top/bottom bands should match the ocean background, with the
status-bar glyphs still legible over it.

## Acceptance

- [x] No white bands: safe-area top/bottom match the app's dark background on-device
- [x] Status-bar clock/indicators remain legible over the dark background
- [x] Decide the fate of the unused `assets/main.css` (delete or repurpose)
- [x] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`, `cargo check --target aarch64-apple-ios`

## Resolution

Two fixes, found in two passes:

1. `src/main.rs`: deleted the unused `assets/main.css`, and injected a second
   viewport meta (`viewport-fit=cover`) plus a `html, body { background-color:
   #0f1720 }` rule via `dioxus::mobile::Config::with_custom_head` — closes the
   white-safe-area root cause this ticket diagnosed.
2. On-device eyeball after (1) still showed a "chin" — a different bug wearing
   the same symptom. `WorldMap`'s SVG (`src/map.rs`) used a square `viewBox`
   with `preserveAspectRatio="xMidYMid meet"`; on a portrait phone that
   letterboxes, leaving a fixed-height flat `bg-ocean` band top and bottom.
   Switched `meet` → `slice` so the map crops to fill the container
   edge-to-edge instead of letterboxing (spec: "immersive full-bleed ...
   edge-to-edge, no card chrome"). Pin-size math is unaffected — both `meet`
   and `slice` scale the viewBox uniformly in x/y, only the crop differs.

3. Simulator screenshot after (2) confirmed the map is now genuinely full-bleed,
   but a dark band still sat at the bottom — the reveal scrim
   `bg-[linear-gradient(transparent,#000c_28%)]` on the bottom overlay
   (`src/review.rs`), which was applied in *both* card states. In the front
   state the "Tap to reveal" pill is already opaque and self-legible, so the
   scrim served no purpose there and painted a black band over the map /
   home-indicator safe area. Gated the scrim gradient behind a conditional
   `class:` on `revealed()`; the front state now
   shows the map edge-to-edge with just the floating pill. Reveal-state
   legibility (country name + grade buttons over the map) is unchanged.
   Verified on the iOS simulator (2026-08-15): UK front state, full-bleed, no
   chin.
