# 16 — Validate codebase against Rust best practices skill

Status: done

**What's wrong:** No pass has run the `rust-best-practices` skill (or equivalent
Rust-idioms review) over `src/**`. Want a dedicated audit rather than relying on
ad hoc review during feature issues.

## Task

- Run the Rust best-practices skill/review over `src/**` (error handling,
  ownership, iterator use, `clippy::pedantic`-style idioms, module boundaries).
- File follow-up issues for anything non-trivial found; fix trivial stuff inline.

## Acceptance

- [x] Audit run and findings triaged (fixed inline or filed as new issues)
- [x] Gate green: `cargo test`, `cargo clippy --all-targets -- -D warnings`,
      `cargo fmt --check`
