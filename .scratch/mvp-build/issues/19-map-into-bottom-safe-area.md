# 19 — Map does not render into the bottom home-indicator safe area

Status: ready-for-agent

**What's wrong:** The full-bleed map reaches every screen edge *except* the bottom
home-indicator strip (~34pt). That strip is filled by the `body` background colour
(ocean `#0f1720`, set in `src/main.rs`), not by the map SVG. Because the body
colour matches the map's ocean, it blends in when the map's bottom edge happens
to be open sea, but any coastline / border / dropped pin near the bottom is clipped
~34pt early, and the map never truly bleeds under the home indicator.

Follows up on issue 12 (that one closed the white/mismatched safe-area bands by
painting `body` ocean; this is the finer point of getting the *map itself* down
there). The home-indicator strip is a gesture area but is not interactive in this
app, so drawing the map under it does not interfere with the swipe-up gesture.

## Root cause (diagnosed 2026-08-15, on the iOS simulator)

WKWebView (via wry/tao under Dioxus mobile) insets the **layout viewport** by the
bottom safe area, so `position: fixed; inset: 0` — the app root, and the map with
it — stops above the home indicator. The webview *frame* does extend to the
physical bottom (the `body` background paints the whole screen, confirmed with a
magenta-`body` test: magenta showed only in the bottom strip), but the layout
viewport that fixed/`100%` positioning resolves against is shorter.

`viewport-fit=cover` is the documented web-side fix, but it does **not** work here.
Verified every meta-tag variant on a clean (manually-rebuilt, styled) app — all
still left the bottom strip uncovered:

- `viewport-fit=cover` meta **appended** via `Config::with_custom_head` (after the
  template's default viewport meta) — no effect (WebKit honours the first meta).
- Same meta as the **only** viewport meta via `Config::with_custom_index` — no
  effect (rules out the two-meta ordering theory; `custom_index` does *not* break
  Tailwind — that earlier symptom was an unrelated `dx serve` hot-reload glitch
  that a manual rebuild clears).
- JS editing the viewport meta's `content` after load — no effect (`viewport-fit`
  is only honoured at initial parse in this WebKit).

So this is a **native** WKWebView behaviour, not something the served HTML can
override. Deferred deliberately: keep it out of the web/RSX layer.

## Proposed fix (native — later stage)

Set the WebView's scroll-view to stop insetting for the safe area:

```objc
webView.scrollView.contentInsetAdjustmentBehavior = UIScrollViewContentInsetAdjustmentNever;
```

Access path: wry 0.53 exposes the `WKWebView` — `wry::WryWebView` derefs to
`objc2_web_kit::WKWebView`, and Dioxus hands out the wry `WebView` via the desktop
context (`dioxus::desktop::window()` / `DesktopContext`). From Rust this is an
`objc2` `msg_send!` to `scrollView` then set `contentInsetAdjustmentBehavior`.
Timing: must run after the webview is attached (a mount-time hook / effect, not
`main` before launch). Confirm `env(safe-area-inset-bottom)` still returns 34pt
afterwards so the overlays keep padding content out of the gesture area.

Host is Apple, so gate the code `#[cfg(target_vendor = "apple")]` /
`target_os = "ios"` as the existing `objc2` paths do, and keep modules
logging-free per AGENTS.md.

## Acceptance

- [ ] Map SVG renders under the home indicator (no ~34pt ocean-`body` strip);
      verify with a temporary non-ocean `body` colour that the strip is gone
- [ ] Bottom overlays (reveal pill, grade buttons) still clear the home indicator
      via `env(safe-area-inset-bottom)` padding
- [ ] Swipe-up-to-home gesture still works normally
- [ ] Native code is `objc2` (not a web/RSX workaround) and Apple-gated
- [ ] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`, `cargo check --target aarch64-apple-ios`
