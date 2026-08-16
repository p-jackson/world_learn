# 23 — Error reporting to Sentry (free tier)

Status: done

**Goal:** ship app errors to a Sentry dashboard so failures in the wild are
visible, without changing how the app renders failures to the user.

## Background

Follow-up from issue 20 / ADR-0001. Two axes are separate and both matter:

- **Render a fallback** (user-facing) — already done: each mount-time load
  failure surfaces via `match` → `ui::Failure`, logging the chain once with
  `error!("{e:#}")` (`ui::log_and_display`). This stays; Sentry does not touch it.
- **Report** (dev-facing) — not done. Sentry adds this, alongside the existing
  render.

## Approach (recommended)

Wire Sentry into the `tracing` subscriber via `sentry-tracing`, so the `error!`
calls we already emit at the boundary become Sentry events. Keeps ADR-0001
intact (no `ErrorBoundary` restructuring), and captures **all** logged errors
app-wide — including the Review grade-path error (`error!("{e:#}")`), which
never throws into render and so no error boundary would ever catch.

Rejected alternative: report from an `ErrorBoundary` `handle_error` via
`sentry-anyhow::capture_anyhow`. Only catches what `?`s into render, still needs
a separate path for non-throwing handler errors, and would double-report if
combined with `sentry-tracing`. Pick one report site.

## Open questions / risks

- **iOS build is the real risk, verify first.** We ship `aarch64-apple-ios`
  (Dioxus mobile). The `sentry` crate's default transport pulls `reqwest` +
  native-TLS and a background thread — confirm the chosen transport feature
  builds and links for the iOS target (`cargo check --target aarch64-apple-ios`)
  before wiring anything else.
- **Chain fidelity.** `error!("{e:#}")` flattens the anyhow chain into a string;
  Sentry gets it as message text. If we want structured frames, an explicit
  `sentry-anyhow::capture_anyhow(&e)` at the log point preserves the chain as
  separate exceptions — decide whether the flat message is good enough.
- **Panics.** Consider the Sentry panic integration so crashes are reported too.
- **PII / opt-in.** Free tier + a learning app: confirm what (if anything) we
  send, and whether a user opt-in / DSN-gating is wanted. Keep the DSN out of
  source.
- **AGENTS.md boundary rule.** Reporting stays at the app boundary (the
  subscriber), modules stay logging-free — `sentry-tracing` fits this cleanly.

## Not in scope

- Error-boundary adoption for rendering fallbacks (decided against in ADR-0001).
- Retry/recovery UX via `ErrorBoundary::clear_errors` — a separate, third
  concern; file its own issue if wanted.

## Acceptance

- [x] Boundary `error!` failures forward to Sentry — via
      `observability::report` → `capture_anyhow` (structured exception frames,
      not the flat `{e:#}` string); `sentry-tracing` demoted to breadcrumbs so
      `capture_anyhow` is the sole event site. Verified: a two-level chain
      delivered to the dashboard (flush confirmed).
- [x] iOS target builds with the Sentry transport — `reqwest` + `rustls` (not
      `native-tls`); `cargo check --target aarch64-apple-ios` green.
- [x] DSN configured out of source — embedded at build time from the
      `SENTRY_DSN` env var or a gitignored `.env` (`build.rs`); not committed.
- [x] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`, plus the web gates.
