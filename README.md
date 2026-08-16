# World Learn

An iOS flashcard app for learning **where every country is**. A regional-zoom
map highlights one country with its borders; you recall the name, tap to reveal
it (plus a dropped pin), then self-grade **Again / Hard / Good / Easy**.
[FSRS](https://github.com/open-spaced-repetition/fsrs-rs) schedules each review,
and new countries are introduced big→obscure at a configurable daily cap.

The deck is the full inhabited [Natural Earth](https://www.naturalearthdata.com/)
admin-0 set — **240 entities**, contested territories included. Built with
[Dioxus 0.7](https://dioxuslabs.com/) (Rust), rendering inline SVG maps, running
natively on iOS.

Domain terms are defined in `CONTEXT.md`.

## Layout

```
src/            # Rust app (Dioxus) — screens, deck, scheduling, persistence
assets/         # Shipped assets, incl. the generated geometry.json map data
tools/geometry/ # Dev-only d3-geo pipeline that builds assets/geometry.json
```

## Running

The app targets iOS. Serve it to the simulator (or a connected device) with:

```bash
dx serve --ios
```

## Tests

### Rust app

Verify any change under `src/**`, `Cargo.toml`, or `Cargo.lock` before
committing:

```bash
cargo test                                  # unit tests
cargo clippy --all-targets -- -D warnings   # lint gate (warnings fail)
cargo fmt --check                            # formatting
```

The dev host is Apple (`target_vendor = "apple"`), so these compile the
iOS-only `objc2` code paths as well. If you touch anything platform-specific,
also confirm the ship target builds:

```bash
cargo check --target aarch64-apple-ios
```

CI runs the same lint/format/test gate on every push/PR touching those paths,
on a macOS runner so the Apple paths and the iOS target are exercised (see
`.github/workflows/rust.yml`).

### Geometry pipeline

The dev-only geometry pipeline (`tools/geometry` → `assets/geometry.json`) has
unit tests plus invariants asserted against the committed asset. Run them before
committing any change to `tools/geometry/**` or `assets/geometry.json`:

```bash
cd tools/geometry
npm ci        # first time / after dependency changes
npm test      # unit tests + produced-asset invariants
```

CI runs the same `npm test` on every push/PR that touches those paths (see
`.github/workflows/geometry.yml`). If you regenerate the asset, run
`npm run build` and commit the updated `assets/geometry.json` so the invariants
match. See `tools/geometry/README.md` for details.
