# 20 — Investigate Dioxus error boundaries vs. manual `log_and_display`

Status: needs-triage

**What's wrong:** `src/ui.rs` has a `log_and_display(&anyhow::Error) -> String`
helper, called at three sites that each manually match a `Result` and render
`Failure { message }` on `Err`:

- `src/main.rs` `App` — `app_setup()` result
- `src/home.rs` `Home` — `store.load()` result
- `src/review.rs` `start_session` (called from `Review`)

Dioxus 0.7 ships `ErrorBoundary` and lets components return
`Result<Element, RenderError>` (propagate with `?`, caught by the nearest
boundary) — see `dioxus-core`'s `error_boundary.rs` / `render_error.rs`. That's
the framework's built-in mechanism for exactly this "load failed, show a
failure UI" shape. Nothing in this codebase uses it; the three call sites
hand-roll the same catch/log/render dance instead.

## Question to answer

Should these three sites use `ErrorBoundary` + `?` instead of the manual
`match`/`log_and_display`/`Failure` pattern? Specifically:

- Does `ErrorBoundary` give a clean way to log the full `anyhow` chain at the
  boundary (AGENTS.md: modules stay logging-free, log once at the app
  boundary) before rendering the fallback — or does it fight that requirement?
- Is one `ErrorBoundary` wrapping `AppRouter` (single catch-all) preferable to
  the current per-screen handling, or do the three sites want distinct
  fallback UI/copy that argues for keeping them separate regardless of
  mechanism?
- Any interaction with `use_hook`/`use_signal`-held state on error (does an
  `ErrorBoundary` remount clear it in a way that matters for `Review`'s
  in-progress session)?

## Task

- Prototype `ErrorBoundary` over one call site (suggest `Home`, the simplest)
  and compare against the current pattern.
- Decide: adopt for all three, adopt for some, or keep `log_and_display` as-is
  with the rationale written down.
- If adopting, migrate the other sites and delete `log_and_display` /
  `Failure`'s manual call sites accordingly.

## Acceptance

- [ ] Decision recorded here (or in CONTEXT.md if it's a standing convention)
- [ ] If adopted: all three sites migrated, `log_and_display` removed or
      narrowed to what's still needed
- [ ] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`
