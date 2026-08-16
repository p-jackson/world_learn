# 24 — Make web an officially supported dev target

Status: done

## Decision: `localStorage` backend (not ephemeral)

Chose the real-persistence `localStorage` backend over ephemeral in-memory. Two
reasons pushed past the issue's "start ephemeral" default:

- **It's synchronous.** `localStorage` matches the existing blocking `Store` API
  (`load`/`save`/`clear`) with no async refactor — the exact thing IndexedDB would
  have forced. The web `Store` is a zero-sized token; app code is unchanged.
- **Issue 25 seeding is trivial.** Playwright seeds a known state by writing one
  key and reloading — no in-app injection hook (`window.__wl_seed` / `?seed=`)
  needed. Per issue 25 that was the preferred path if simpler; it is, so this
  tips the decision here as anticipated.

The split lives behind the `Store` seam in `src/store.rs`: `#[cfg(feature =
"web")]` selects the `localStorage` backend, `not(feature = "web")` the filesystem
one. The persisted JSON is byte-identical across backends. Key:
`world_learn.review_state`. No follow-up for web persistence is needed — this *is*
real persistence.

Two wasm-only runtime deps surfaced (both would panic/fail to link, not fail to
compile the host check — the `dx serve --web` smoke caught the jiff one):
`getrandom` needs its `wasm_js` backend (`fsrs → rand`), and `jiff` needs its `js`
feature for `Zoned::now()`. Wired in `Cargo.toml` +`.cargo/config.toml`.

Verified live: `dx serve --web` → Home renders (0 due / 10 new) → Start → reveal
(China) → grade Good → queue advances 1/10→2/10 and `localStorage` holds the CHN
FSRS record.

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

- [x] `dx serve --web` builds and serves; app renders, a review can be started
      and graded in the browser. (Verified live via Playwright.)
- [x] Web persistence decision made and implemented behind the `Store` seam
      (`localStorage`), documented in the issue.
- [x] `objc2` / Apple-only code excluded from the web build (`cargo tree` on the
      wasm target confirms); assets resolve (favicon + tailwind copied,
      `geometry.json` embedded via `include_str!`).
- [x] AGENTS.md "Launching application" documents `dx serve --web` as a
      supported dev target, iOS still the default.
- [x] Gate green for both targets: existing iOS/host gate **plus** `cargo clippy
      --no-default-features --features web --all-targets -- -D warnings` and a
      `wasm32-unknown-unknown` check. Both added to `.github/workflows/rust.yml`.
