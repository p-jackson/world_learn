# 17 — Stop mixing Tailwind and hand-written CSS

Status: ready-for-agent

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

- [ ] Only one CSS system in use (Tailwind); `assets/main.css` deleted or
      folded into Tailwind
- [ ] No visual regression (spot-check Home, Review front/reveal, Done)
- [ ] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`
