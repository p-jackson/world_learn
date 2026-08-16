//! App-boundary observability setup: the tracing subscriber, and — on native
//! targets — Sentry error reporting layered onto it.
//!
//! AGENTS.md keeps modules logging-free and logs once at the boundary with
//! `error!("{e:#}")`. Reporting stays at that same boundary: [`report`] both
//! logs the chain and captures it to Sentry. It captures the `anyhow::Error`
//! value (via `capture_anyhow`), not the flattened `{e:#}` string, so the
//! anyhow chain reaches the dashboard as separate exception frames rather than
//! one opaque line. `capture_anyhow` is the sole event site — the tracing layer
//! only feeds breadcrumbs — so nothing double-reports (issue 23; the
//! boundary-logging rule is AGENTS.md's, ADR-0001 the related UI decision).
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
    use dioxus::logger::tracing::Level;
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{fmt, EnvFilter};

    // Make `anyhow` capture a backtrace when each error is created, so the
    // Sentry event carries a stack trace (not just the message chain):
    // `sentry-anyhow` attaches frames only when the backtrace status is
    // `Captured`, which needs one of these env vars set. Do this before any
    // error can be created (std caches the enable-decision on first capture),
    // and only if the dev hasn't set their own value (e.g. `RUST_BACKTRACE=0`).
    // `RUST_LIB_BACKTRACE` targets library backtraces without turning on
    // panic-print. Release/iOS builds are stripped, so frames symbolicate fully
    // only with debug symbols uploaded to Sentry — see issue 23.
    if std::env::var_os("RUST_LIB_BACKTRACE").is_none()
        && std::env::var_os("RUST_BACKTRACE").is_none()
    {
        std::env::set_var("RUST_LIB_BACKTRACE", "1");
    }

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

    // Breadcrumbs only: demote ERROR from its default Event mapping so the sole
    // event site is `report`'s `capture_anyhow` (structured frames), not this
    // layer's flattened string — and nothing double-reports. The trail of
    // info/warn/error breadcrumbs still rides along on the captured event.
    let sentry_layer = sentry_tracing::layer().event_filter(|md| match *md.level() {
        Level::ERROR | Level::WARN | Level::INFO => sentry_tracing::EventFilter::Breadcrumb,
        Level::DEBUG | Level::TRACE => sentry_tracing::EventFilter::empty(),
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer())
        .with(sentry_layer)
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

/// The boundary error action: log the full `anyhow` chain, then (on native)
/// report it to Sentry. `capture_anyhow` preserves the chain as structured
/// exception frames. Call this instead of a bare `error!("{e:#}")` at any
/// boundary that wants a failure reported — it is the one Sentry event site.
#[cfg(not(target_arch = "wasm32"))]
pub fn report(e: &anyhow::Error) {
    dioxus::logger::tracing::error!("{e:#}");
    sentry::integrations::anyhow::capture_anyhow(e);
}

/// Web (wasm): log the chain; nothing to report (Sentry is native-only).
#[cfg(target_arch = "wasm32")]
pub fn report(e: &anyhow::Error) {
    dioxus::logger::tracing::error!("{e:#}");
}
