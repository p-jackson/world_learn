# 12 — White "forehead" / "chin" in the safe areas

Status: ready-for-agent

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

- [ ] No white bands: safe-area top/bottom match the app's dark background on-device
- [ ] Status-bar clock/indicators remain legible over the dark background
- [ ] Decide the fate of the unused `assets/main.css` (delete or repurpose)
- [ ] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`, `cargo check --target aarch64-apple-ios`
