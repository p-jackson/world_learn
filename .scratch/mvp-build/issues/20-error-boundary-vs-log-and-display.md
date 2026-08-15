# 20 — Investigate Dioxus error boundaries vs. manual `log_and_display`

Status: done

## Decision

Keep the manual `match` / `log_and_display` / `Failure` pattern; do not adopt
`ErrorBoundary`. Prototyped it over `Home` (compiled, reverted). Rationale in
`docs/adr/0001-load-failure-ui-manual-not-error-boundary.md`:

- `CapturedError` derefs to the real `anyhow::Error`, so `?` preserves the chain
  and the handler *can* log `{:#}` — but that log lives in the render-path
  `handle_error`, which can re-log; the current code logs exactly once from a
  `use_hook`/`use_signal` init.
- `use_hook` needs `Clone`; `anyhow::Error` isn't, so a memoized mount-time load
  must be converted to `RenderError` first — more ceremony than `log_and_display`.
- The three sites want distinct copy, and `app_setup` sits above the router +
  context providers, so a single catch-all boundary doesn't fit without
  restructuring. Per-site handling is the better fit.
- No in-progress state is at risk: `Review`'s only failure is at mount.

No code change beyond recording the decision.

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

- [x] Decision recorded here (+ standing convention in `docs/adr/0001-…`)
- [x] Not adopted: `log_and_display` / `Failure` kept as-is
- [x] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`
