# 22 — Back-to-home button broken

Status: wontfix

**What's wrong:** The "Back to home" button doesn't return to the Home screen.
It appears on the Done screen (issue 09, `Route::Done`) and/or Settings.

**Resolution (2026-08-16): cannot reproduce.** Both back-to-home controls
navigate to `Route::Home` correctly on the iOS simulator (iPhone 16e / iOS 26.2),
confirmed by real taps and by a programmatic proof. No code change — the
navigation wiring is already correct.

## Investigation (2026-08-16, iOS simulator iPhone 16e / iOS 26.2)

Both controls run the identical call: `nav.push(Route::Home {})` in an `onclick`
(`Done` back button in `src/review.rs`, Settings `‹` in `src/settings.rs`).

1. **Route wiring is correct.** `Route::Home {}` serialises to `/` and
   round-trips (`from_str("/")` → `Home`). `Navigator::push` to `/` is a normal
   internal navigation; the `MemoryHistory` (desktop/mobile back-end) only
   no-ops a push when the target equals the *current* route, and `/settings` /
   `/done/N` ≠ `/`.
2. **The push actually returns to Home.** A temporary mount-effect auto-drove
   Home → Settings → `nav.push(Route::Home {})` and logged the Home component
   re-mounting afterwards — the exact back-button call lands on Home.
3. **Real taps work.** With a temporary `DEBUG: go to Done` shortcut on Home,
   tapping "Back to home" on the Done screen returned to Home; tapping the `‹`
   arrow on Settings returned to Home. Both verified by hand on the simulator.

No CSS blocks the tap either: nothing sets `pointer-events`/`touch-action` on an
overlay, and `-webkit-tap-highlight-color: transparent` only suppresses the tap
flash (it does not stop `click` firing). All diagnostic instrumentation was
reverted; the tree is unchanged from the committed code.

Revisit only if the failure resurfaces with a concrete repro (device, build,
navigation sequence, and whether *any* button on that screen responds).
