# 13 — Home screen title says "Wayfinder", should be "World Learn"

Status: done

**What's wrong:** The Home screen `h1` displays "Wayfinder" instead of the app's
actual name, "World Learn". Surfaced during the issue-09 demo eyeball
(Home/Settings/routing).

## Where

`src/home.rs:53`

```
h1 { class: "text-[34px] font-[750] tracking-[-0.02em]", "Wayfinder" }
```

Change the string to "World Learn".

Grepped the repo for other user-facing "Wayfinder" occurrences — found none.
The only hit is this `h1`; all other "wayfinder" matches are unrelated agent-
tooling docs (`/wayfinder` skill, `.agents/skills/wayfinder/`). Scope stays
just this title string.

## Acceptance

- [x] Home screen `h1` reads "World Learn"
- [x] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`, `cargo check --target aarch64-apple-ios`
