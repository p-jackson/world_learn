# 24 — Make web an officially supported dev target

Status: needs-triage

**Goal:** `dx serve --web` is a first-class, documented way to run the app in
development. iOS (`dx serve --ios`) **stays the default and only ship target** —
web is a dev/test convenience, and the prerequisite for browser-driven UI tests
(issue 25).

## Background

The `web` feature already exists in `Cargo.toml` (`web = ["dioxus/web"]`) and
`Dioxus.toml` has a `[web.app]` block, but nothing exercises or documents the
web path. It has never been a maintained target: no build gate, no persistence
backend, no launch docs.

## The real work: persistence

`src/store.rs` is filesystem + Apple-only:

- `app_support_dir()` resolves the iOS Application Support dir via
  `NSSearchPathForDirectoriesInDomains`; the `#[cfg(not(target_vendor =
  "apple"))]` arm just `bail!`s.
- `Store` reads/writes a JSON file with atomic temp-file rename.

A browser has no filesystem sandbox, so web needs its own backend. Decide
between:

- **Ephemeral / in-memory** (simplest): web runs with a non-persisting store.
  Enough to render and drive UI tests (issue 25 seeds state itself). Lowest
  effort; loses state on reload.
- **`localStorage` / IndexedDB** backend: real persistence on web. More work
  (async for IndexedDB; serde round-trip through `web_sys`), but makes web a
  genuine usable client.

Recommend starting **ephemeral** to unblock issue 25, and filing a follow-up for
real web persistence if wanted. **Caveat:** if a `localStorage` backend makes
seeding test state (issue 25) meaningfully easier — Playwright writes a
`localStorage` key and reloads, no in-app injection hook needed — prefer it over
ephemeral. Let issue 25's seeding ergonomics drive the call. Whichever: keep the
platform split behind the existing `Store` seam so app code stays
backend-agnostic.

## Also verify

- **Assets** load on web: `assets/geometry.json` and the stylesheet resolve
  under `dx serve --web` (asset macro paths).
- **`objc2` deps** are already `[target.'cfg(target_vendor = "apple")']`-gated,
  so a wasm/web build must not pull them — confirm.
- **Lint/build gate**: `cargo clippy --no-default-features --features web
  --all-targets -- -D warnings` (web feature disables the default `mobile`).
  Note: clippy on the wasm target, not a full `dx` bundle, is the cheap CI gate.

## Not in scope

- Shipping web as a product target (iOS-only ship stands).
- Real web persistence backend if we choose ephemeral (file a follow-up).
- Playwright itself — that's issue 25 (this issue unblocks it).

## Acceptance

- [ ] `dx serve --web` builds and serves; app renders, a review can be started
      and graded in the browser.
- [ ] Web persistence decision made and implemented behind the `Store` seam
      (ephemeral or localStorage/IndexedDB), documented in the issue.
- [ ] `objc2` / Apple-only code excluded from the web build; assets resolve.
- [ ] AGENTS.md "Launching application" documents `dx serve --web` as a
      supported dev target, iOS still the default.
- [ ] Gate green for both targets: existing iOS/host gate **plus** `cargo clippy
      --no-default-features --features web --all-targets -- -D warnings`.
