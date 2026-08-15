# ADR-0001 — Load-failure UI: manual `log_and_display`, not `ErrorBoundary`

Status: accepted (2026-08-16)

## Context

Three screens run one fallible setup step at mount and must surface a failure
rather than hide it:

- `App` — `app_setup()` (Deck + Store)
- `Home` — `store.load()`
- `Review` — `Session::start`

Each matches the `Result` and renders `ui::Failure` on `Err`, logging the full
`anyhow` chain once via `ui::log_and_display` (`error!("{e:#}")` + return the
flattened string). AGENTS.md: modules stay logging-free; log once at the app
boundary with the `{:#}` chain.

Dioxus 0.7 ships `ErrorBoundary` + `?` (components return
`Result<Element, RenderError>`, errors caught by the nearest boundary). Issue 20
asked whether the three sites should adopt it.

## Decision

Keep the manual `match` / `log_and_display` / `Failure` pattern. Do not adopt
`ErrorBoundary` for these sites.

## Rationale

Prototyped `ErrorBoundary` over `Home` (compiled, then reverted). Findings:

- **Chain logging works but moves into the render path.** `CapturedError` wraps
  the real `anyhow::Error` and derefs to it, so `?` preserves the chain and the
  handler can `error!("{:#}", &*e)`. But that log then lives in `handle_error`,
  which runs on every render of the boundary while in the error state — a
  side-effect in render that can re-log. The current code logs exactly once,
  from a `use_hook` / `use_signal` init closure. ErrorBoundary makes the
  "log once at the boundary" rule *harder*, not easier.
- **`use_hook` needs `Clone`; `anyhow::Error` isn't.** To memoize a mount-time
  load you must convert the error to `RenderError`/`CapturedError` before
  storing it, then `.clone()?` in render — extra ceremony versus
  `.map_err(|e| log_and_display(&e))`, which yields a `Clone` `String` and logs
  in the same step.
- **Distinct copy + placement favour per-site handling.** The three want
  distinct framing ("Failed to start" / "…load" / "…start review"). A single
  catch-all boundary can't distinguish sites without pushing the framing into
  `.context(...)`, and `app_setup` runs in `App` *above* the router and context
  providers, so a router-level boundary can't catch it without restructuring
  setup into a child component.
- **No in-progress state is at risk.** Only `Review` holds session state, and
  its failure is at `Session::start` (mount), so there is nothing in-flight for
  a boundary remount to clear. This point is neutral, not a reason to adopt.

`ErrorBoundary` earns its keep when many descendant components can throw into a
single fallback. This app has three shallow, single-shot, mount-time setups with
distinct copy, one of them above the router — the manual pattern fits that shape
and keeps logging to exactly one call at the boundary.

## Consequences

- `ui::log_and_display` and `ui::Failure` stay. New load-failure sites follow
  the same `match` + `log_and_display` + `Failure` pattern.
- Revisit if failures start being thrown from deep within a screen's subtree
  (many points, one fallback) rather than from a single mount-time step.
