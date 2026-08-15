# 17 — Stop mixing Tailwind and hand-written CSS

Status: done

**What's wrong:** `assets/tailwind.css` is linked and used throughout
(`src/map.rs`, `src/review.rs`, `src/main.rs`), but `assets/main.css` also
exists with hand-written rules (`body { background-color: #0f1116 }`,
leftover `#hero`/`#links` scaffolding). Issue 12 flagged `main.css` as
possibly-unused leftover Dioxus boilerplate. Two styling systems side by side
is confusing and error-prone (e.g. issue 12's white safe-area bug traced back
to `main.css` not being linked).

## Task

- Pick one system: Tailwind utility classes only.
- Audit `assets/main.css` for anything actually load-bearing (e.g. body
  background) and port it to Tailwind classes / `tailwind.css` `@layer`, or
  delete it if unused.
- Sweep `src/**` for any inline `style:` attrs or raw CSS that should be
  Tailwind classes instead.

## Acceptance

- [x] Only one CSS system in use (Tailwind); `assets/main.css` deleted or
      folded into Tailwind
- [x] No visual regression (spot-check Home, Review front/reveal, Done)
- [x] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`

## Resolution

Already done by issue 12 (c5b78fc): `assets/main.css` deleted, body background
folded into a literal in `main.rs`'s pre-Tailwind `<style>` head tag. No
`assets/main.css` or other CSS file remains; `src/**` has no raw CSS. Two
`style:` attrs remain, both legitimate (not sweepable to Tailwind classes):
- `review.rs`: runtime-computed progress-bar width % — can't be a static
  Tailwind class.
- `main.rs`: inline `<style>` in the native WKWebView head, painted before
  the Tailwind stylesheet asset loads (avoids white flash on launch).

No code changes needed this pass; verified via grep sweep + gate rerun.
