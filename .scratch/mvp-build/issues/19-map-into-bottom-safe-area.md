# 19 — Map does not render into the bottom home-indicator safe area

Status: wontfix

**What's wrong:** The full-bleed map reaches every screen edge *except* the bottom
home-indicator strip (~34pt), which is filled by the `body` background colour
(ocean `#0f1720`, set in `src/main.rs`). Because the body colour matches the
map's ocean it blends in over open sea, but any coastline / border / dropped pin
near the bottom is clipped ~34pt early. Cosmetic, ocean-on-ocean.

**Resolution (2026-08-16): deferred.** The fix this ticket proposed is a no-op,
and the underlying capability (`viewport-fit=cover`) is not supported by the
WKWebView that wry/tao build on iOS. Every remaining fix costs more than the
polish is worth. Details below; revisit if wry/dioxus gains `viewport-fit=cover`
support.

## Investigation (2026-08-16, iOS simulator iPhone 16e / iOS 26.2)

Feedback loop: temporary magenta `body`, `dx serve --ios`, then
`xcrun simctl io booted screenshot` + a JS `document::eval` metrics probe. Magenta
in the bottom strip = bug present (root does not reach the physical bottom).

### The proposed native fix is disproven

Setting `scrollView.contentInsetAdjustmentBehavior = .never` has **no effect** on
this symptom:

- The mount effect runs and the property changes (probe logged `3 → 2`). Note the
  default is `.Always` (3), **not** `.automatic` (0) as this ticket assumed.
- The magenta strip persists unchanged. The inset that shortens the layout
  viewport is **not** driven by the scroll view's content inset, so `.never`
  cannot move it.

### `viewport-fit=cover` is not honoured by this WKWebView

Measured with the probe (dpr 3, physical height 844 CSS px):

- As the **sole** viewport meta (served via `Config::with_custom_index`, so no
  two-meta ordering issue): `env(safe-area-inset-bottom)` = **0** and
  `window.innerHeight` = **763** (= 844 − 47 top − 34 bottom). The layout viewport
  is still inset — cover did not activate.
- Same result with `contentInsetAdjustmentBehavior` at `.never` **and** at the
  default `.Always`. The inset behaviour is irrelevant.
- A one-shot `location.reload()` after setting `.never` (the "env stuck until
  reload" theory) also leaves the strip.

The view hierarchy is otherwise correct (`UIWindow → TaoUIViewController →
TaoView → WKWebView` subview) and the webview *frame* does fill to the physical
bottom (that's why the `body` background paints the strip). WebKit simply never
exposes the safe area to CSS nor extends fixed content into it — i.e. it behaves
as if `viewport-fit=cover` were absent. This is an upstream limitation of
wry `0.53.5` / tao `0.34.8` (dioxus `0.7.10` wraps them); other stacks
(Tauri / Ionic / Cordova / Capacitor) assume cover works and so hit env-based
recipes that do not apply here.

### A web/RSX workaround is impossible

The entire web layout region is hard-capped at the inset viewport:
`documentElement.clientHeight` **and** `scrollHeight` = 763, and a test element at
`position:absolute` on `<html>` **and** one at `position:fixed` both bottom out at
763. Nothing — fixed, absolute, or in-flow — can reach past 763 into the strip.
The **only** thing that paints there is the root element's background (WebKit
canvas-background propagation), which cannot carry the interactive map SVG.

### Corrected assumption: `env(safe-area-inset-*)` is 0 here

Because cover is off, `env(safe-area-inset-bottom)` resolves to **0**. The
`env(safe-area-inset-bottom)` padding on the bottom overlays in `src/review.rs`
is therefore currently a no-op: the overlays clear the home indicator only
because the whole layout viewport already ends above it. Any real fix that grows
the viewport into the strip must also re-plumb that padding off `env()`.

## Remaining options (all rejected for now)

- **Native `additionalSafeAreaInsets`** — set a negative bottom inset on
  `TaoUIViewController` so WebKit grows the viewport into the strip. Untested.
  Cost: more `objc2` (reach the view controller, pass a `UIEdgeInsets` by value)
  **and** `env()` stays 0, so `review.rs`'s overlay padding must move to a
  hardcoded / Rust-fed ~34pt inset. Too much machinery for ocean-on-ocean polish.
- **Fork / patch wry, or file upstream** and wait for `viewport-fit=cover`
  support. Highest effort; benefits others but out of scope for the MVP.

## Superseded — original proposed fix (INVALID, kept for the record)

The earlier proposal was to set, from a mount-time `objc2` hook:

```objc
webView.scrollView.contentInsetAdjustmentBehavior = UIScrollViewContentInsetAdjustmentNever;
```

Disproven above: the property changes but the strip does not, because the layout
viewport is not inset via the scroll view's content inset. `viewport-fit=cover`
— the piece that would actually decouple the viewport from the safe area while
keeping `env()` non-zero — is not honoured by this WKWebView, so the native tweak
has nothing to complete.

## Acceptance (not pursued)

- [ ] ~~Map SVG renders under the home indicator~~ — not achievable in-app without
      the native `additionalSafeAreaInsets` workaround above.
- [ ] ~~Bottom overlays clear the home indicator via `env(safe-area-inset-bottom)`~~
      — premise invalid: `env()` is 0 in this webview.
- [ ] Swipe-up-to-home gesture unaffected (strip is a non-interactive gesture area).
