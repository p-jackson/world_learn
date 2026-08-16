//! Embed the Sentry DSN at build time without committing it to source.
//!
//! The DSN is not a secret the way a signing key is — it ships inside the built
//! app either way — but we still keep it out of the git history. It reaches the
//! binary as the `SENTRY_DSN` compile-time env var, read here from either the
//! build environment (CI / GitHub config) or a gitignored local `.env`. When
//! neither supplies it, nothing is emitted and `option_env!("SENTRY_DSN")` is
//! `None`, so a DSN-less build simply does not report (see `observability.rs`).

use std::{env, fs};

fn main() {
    // Rebuild when the DSN source changes either way it can be provided.
    println!("cargo:rerun-if-env-changed=SENTRY_DSN");
    println!("cargo:rerun-if-changed=.env");

    if let Some(dsn) = env::var("SENTRY_DSN").ok().or_else(dsn_from_dotenv) {
        println!("cargo:rustc-env=SENTRY_DSN={dsn}");
    }
}

/// Pull `SENTRY_DSN` from a local `.env` (gitignored). Absent file or key → `None`.
fn dsn_from_dotenv() -> Option<String> {
    let contents = fs::read_to_string(".env").ok()?;
    contents.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        (key.trim() == "SENTRY_DSN").then(|| value.trim().to_string())
    })
}
