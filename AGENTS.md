# Launching application

```sh
dx serve --ios
```

# Error handling

App code returns `anyhow::Result<T>`; propagate with `?`. Do not hand-roll error
enums — reach for `anyhow`, not `thiserror`, unless a caller genuinely needs to
branch on the error kind.

Attach context at every fallible boundary so a failure reads as a chain, not a
bare OS message. Say what the operation was and name the thing it acted on
(path, id, value):

```rust
fs::write(&tmp, &json)
    .with_context(|| format!("writing temp store file {}", tmp.display()))?;
serde_json::to_vec_pretty(state).context("serializing review state")?;
anyhow::ensure!(v <= MAX, "unsupported schema version {v}; max is {MAX}");
```

Use `.context(...)` for a static string, `.with_context(|| ...)` when the message
needs formatting (the closure runs only on the error path), `ensure!`/`bail!` for
invariant/early-out failures.

Modules stay logging-free: they return errors, they don't log them. Log once at
the app boundary (launch, event handlers) with the full chain via the `{:#}`
alternate form:

```rust
if let Err(e) = store.load() {
    tracing::error!("{e:#}"); // whole context chain, not just the outer message
}
```

Logging is set up by `dioxus-logger` (a default `dioxus` feature). `main` calls
`dioxus::logger::initialize_default()`; on iOS the subscriber writes to stdout,
which `dx serve --ios` and Xcode capture. The `error!`/`warn!`/`info!` macros are
re-exported from `dioxus::prelude`.

# Verifying changes

Before committing, run the tests for whatever you touched and confirm they pass.
CI runs these same commands, so a red run locally is a red run in CI.

- **Rust app** — any change under `src/**`, `Cargo.toml`, or `Cargo.lock`:

  ```sh
  cargo test                                  # unit tests
  cargo clippy --all-targets -- -D warnings   # lint gate (warnings fail)
  cargo fmt --check                            # formatting
  ```

  Host is Apple (`target_vendor = "apple"`), so this compiles the iOS-only
  `objc2` path too. Touched anything platform-specific? Also confirm the ship
  target builds: `cargo check --target aarch64-apple-ios`. CI:
  `.github/workflows/rust.yml`.

- **Geometry pipeline** — any change under `tools/geometry/**` or to
  `assets/geometry.json`:

  ```sh
  cd tools/geometry && npm test   # unit tests + produced-asset invariants
  ```

  Regenerated the asset? Run `npm run build` and commit the updated
  `assets/geometry.json` so the invariants match. CI:
  `.github/workflows/geometry.yml`.

# Agent skills

## Issue tracker

Issues and specs live as markdown files under `.scratch/<feature>/` in this
repo. See `docs/agents/issue-tracker.md`.

## Triage labels

Default canonical roles (`needs-triage`, `needs-info`, `ready-for-agent`,
`ready-for-human`, `wontfix`), recorded as a `Status:` line in each issue file.
See `docs/agents/triage-labels.md`.

## Domain docs

Single-context: one `CONTEXT.md` + `docs/adr/` at the repo root. See
`docs/agents/domain.md`.

# Conventions

- Do NOT add any verbose code comments that narrate what the change does or why
it was made (e.g. `// Added this to fix X`, `// Changed from Y to Z`, restating
the code in prose). Such explanation belongs in the PR description, not the
source. Only keep comments that a future reader genuinely needs — non-obvious
rationale, gotchas, links to context — and match the comment density and style
of the surrounding code.

# Dioxus

[Dioxus](https://dioxuslabs.com/learn/0.7) is a framework for building
cross-platform apps with the Rust programming language. With one codebase,
you can build apps that run on web, desktop, and mobile platforms. Dioxus 0.7
changed every api in Dioxus. Only use this up to date documentation. `cx`,
`Scope`, and `use_state` are gone.

## UI with RSX

```rust
rsx! {
	div {
		class: "container", // Attribute
		color: "red", // Inline styles
		width: if condition { "100%" }, // Conditional attributes
		"Hello, Dioxus!"
	}
	// Prefer loops over iterators
	for i in 0..5 {
		div { "{i}" } // use elements or components directly in loops
	}
	if condition {
		div { "Condition is true!" } // use elements or components directly in conditionals
	}

	{children} // Expressions are wrapped in brace
	{(0..5).map(|i| rsx! { span { "Item {i}" } })} // Iterators must be wrapped in braces
}
```

## Assets

The asset macro can be used to link to local files to use in your project. All
links start with `/` and are relative to the root of your project.

```rust
rsx! {
	img {
		src: asset!("/assets/image.png"),
		alt: "An image",
	}
}
```

## Styles

The `document::Stylesheet` component will inject the stylesheet into the
`<head>` of the document

```rust
rsx! {
	document::Stylesheet {
		href: asset!("/assets/styles.css"),
	}
}
```

## Components

Components are the building blocks of apps

* Component are functions annotated with the `#[component]` macro.
* The function name must start with a capital letter or contain an underscore.
* A component re-renders only under two conditions:
	1.  Its props change (as determined by `PartialEq`).
	2.  An internal reactive state it depends on is updated.

```rust
#[component]
fn Input(mut value: Signal<String>) -> Element {
	rsx! {
		input {
            value,
			oninput: move |e| {
				*value.write() = e.value();
			},
			onkeydown: move |e| {
				if e.key() == Key::Enter {
					value.write().clear();
				}
			},
		}
	}
}
```

Each component accepts function arguments (props)

* Props must be owned values, not references. Use `String` and `Vec<T>` instead
  of `&str` or `&[T]`.
* Props must implement `PartialEq` and `Clone`.
* To make props reactive and copy, you can wrap the type in `ReadOnlySignal`.
  Any reactive state like memos and resources that read `ReadOnlySignal` props
  will automatically re-run when the prop changes.

## State

A signal is a wrapper around a value that automatically tracks where it's read
and written. Changing a signal's value causes code that relies on the signal
to rerun.

### Local State

The `use_signal` hook creates state that is local to a single component. You can
call the signal like a function (e.g. `my_signal()`) to clone the value, or use
`.read()` to get a reference. `.write()` gets a mutable reference to the value.

Use `use_memo` to create a memoized value that recalculates when its
dependencies change. Memos are useful for expensive calculations that you don't
want to repeat unnecessarily.

```rust
#[component]
fn Counter() -> Element {
	let mut count = use_signal(|| 0);
	let mut doubled = use_memo(move || count() * 2); // doubled will re-run when count changes because it reads the signal

	rsx! {
		h1 { "Count: {count}" } // Counter will re-render when count changes because it reads the signal
		h2 { "Doubled: {doubled}" }
		button {
			onclick: move |_| *count.write() += 1, // Writing to the signal rerenders Counter
			"Increment"
		}
		button {
			onclick: move |_| count.with_mut(|count| *count += 1), // use with_mut to mutate the signal
			"Increment with with_mut"
		}
	}
}
```

### Context API

The Context API allows you to share state down the component tree. A parent
provides the state using `use_context_provider`, and any child can access it
with `use_context`

```rust
#[component]
fn App() -> Element {
	let mut theme = use_signal(|| "light".to_string());
	use_context_provider(|| theme); // Provide a type to children
	rsx! { Child {} }
}

#[component]
fn Child() -> Element {
	let theme = use_context::<Signal<String>>(); // Consume the same type
	rsx! {
		div {
			"Current theme: {theme}"
		}
	}
}
```

## Async

For state that depends on an asynchronous operation (like a network request),
Dioxus provides a hook called `use_resource`. This hook manages the lifecycle of
the async task and provides the result to your component.

* The `use_resource` hook takes an `async` closure. It re-runs this closure
  whenever any signals it depends on (reads) are updated
* The `Resource` object returned can be in several states when read:
  1. `None` if the resource is still loading
  2. `Some(value)` if the resource has successfully loaded

```rust
let mut dog = use_resource(move || async move {
	// api request
});

match dog() {
	Some(dog_info) => rsx! { Dog { dog_info } },
	None => rsx! { "Loading..." },
}
```

## Routing

All possible routes are defined in a single Rust `enum` that derives `Routable`.
Each variant represents a route and is annotated with `#[route("/path")]`.
Dynamic Segments can capture parts of the URL path as parameters by using
`:name` in the route string. These become fields in the enum variant.

The `Router<Route> {}` component is the entry point that manages rendering the
correct component for the current URL.

You can use the `#[layout(NavBar)]` to create a layout shared between pages and
place an `Outlet<Route> {}` inside your layout component. The child routes will
be rendered in the outlet.

```rust
#[derive(Routable, Clone, PartialEq)]
enum Route {
	#[layout(NavBar)] // This will use NavBar as the layout for all routes
		#[route("/")]
		Home {},
		#[route("/blog/:id")] // Dynamic segment
		BlogPost { id: i32 },
}

#[component]
fn NavBar() -> Element {
	rsx! {
		a { href: "/", "Home" }
		Outlet<Route> {} // Renders Home or BlogPost
	}
}

#[component]
fn App() -> Element {
	rsx! { Router::<Route> {} }
}
```

```toml
dioxus = { version = "0.7.1", features = ["router"] }
```

## UI Primitives

When adding a new base ui primitive (e.g. button, card, dialog, combobox, etc.) start from [Dioxus' component library](https://dioxuslabs.com/components).

```bash
# List what components are available for install
dx components list

# Add a component to the repo, e.g. switch
dx components add switch

# Other instructions for component library
dx components --help
```
