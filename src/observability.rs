//! App-boundary observability setup: the tracing subscriber, and — on native
//! targets — Sentry error reporting layered onto it.
//!
//! AGENTS.md keeps modules logging-free and logs once at the boundary with
//! `error!("{e:#}")`. Reporting stays at that same boundary: `sentry-tracing`
//! turns those `error!` events into Sentry events, so the report site is the
//! subscriber, not scattered `capture_*` calls (issue 23; the boundary-logging
//! rule is AGENTS.md's, ADR-0001 is the related load-failure-UI decision).
//!
//! Web is a dev-only target and the default Sentry transport does not build for
//! wasm, so the wasm build keeps `dioxus-logger`'s subscriber untouched and
//! reports nothing. iOS (the ship target) and any other native build own the
//! subscriber so the Sentry layer can be composed in.

/// Initialise logging, and Sentry when a DSN was embedded at build time. The
/// returned guard flushes queued events on drop, so the caller must hold it for
/// the life of the process; dropping it early stops reporting.
///
/// A DSN-less build (no `SENTRY_DSN` at compile time — see `build.rs`) still
/// installs the subscriber and logs normally; it just reports nothing.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn init() -> Option<sentry::ClientInitGuard> {
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{fmt, EnvFilter};

    // Mirror dioxus-logger's native defaults (debug in dev, info in release,
    // `RUST_LOG` overrides, quieten hyper), then add the Sentry layer. We set
    // the global subscriber ourselves so the layer composes; `dioxus::launch`'s
    // auto-init then no-ops because one is already installed.
    let default_level = if cfg!(debug_assertions) {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };
    let filter = EnvFilter::builder()
        .with_default_directive(default_level.into())
        .from_env_lossy()
        .add_directive("hyper_util=warn".parse().expect("static directive parses"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .with(sentry_tracing::layer())
        .init();

    let Some(dsn) = option_env!("SENTRY_DSN") else {
        dioxus::logger::tracing::warn!(
            "SENTRY_DSN not embedded at build time; error reporting disabled"
        );
        return None;
    };

    // `panic` feature installs a panic hook so crashes report too; `release`
    // tags events with the crate version so dashboard issues group by build.
    // `send_default_pii = false` is the default — set explicitly so the "we send
    // no PII" posture is a stated decision, not an inherited one (issue 23).
    let mut options = sentry::ClientOptions::default();
    options.release = sentry::release_name!();
    options.send_default_pii = false;
    Some(sentry::init((dsn, options)))
}

/// Web (wasm) dev target: keep dioxus-logger's subscriber, no Sentry. Returns
/// `()` so the [`init`] call site holds a guard binding on every target.
#[cfg(target_arch = "wasm32")]
pub fn init() {
    dioxus::logger::initialize_default();
}
