# Orthrus MVP Design Spec

## Purpose

Orthrus is a Rust library crate that enables AI-driven GUI development and testing
for macroquad+egui applications. It provides a dual-headed test architecture:

- **Head 1 (kittest):** Headless egui widget testing — query widgets, simulate
  interactions, assert on state. Fast, no GPU. **This is the MVP.**
- **Head 2 (macroquad):** Visual regression via screenshot capture. Deferred to
  post-MVP. Decision on library-owned vs pattern-documented will be made after
  Head 1 is proven.

Named after the two-headed dog of Greek mythology.

**First consumers:** Alpha Prime (`~/devel/alpha-prime`), Phoenix (`~/desert-island/phoenix`).

## Core Invariant

All UI state lives in the consumer's state type, never in egui's internal `Context`.
Both heads share the same state and UI functions. Since egui is immediate mode,
both heads calling the same UI function with the same state produce equivalent
widget trees.

Each head creates its own `egui::Context`. Any state stored in egui internals
(window open/closed, scroll positions) does not transfer between heads. This is a
desirable constraint — it forces explicit, testable state.

## Trait Contract

```rust
pub trait TestableApp {
    /// The application's state type — single source of truth for all UI state.
    type State;

    /// Builds the UI given an egui context and mutable state reference.
    /// Both heads call this same function.
    fn build_ui(ctx: &egui::Context, state: &mut Self::State);

    /// Creates a deterministic state suitable for testing.
    fn new_test_state() -> Self::State;
}
```

**What the trait enforces:**
- `State` is an associated type owned by the consumer, not egui-internal
- `build_ui` is framework-agnostic — both heads can call it with their own Context
- `new_test_state()` guarantees reproducible starting state for every test

**Design choice — marker type pattern:** `build_ui` and `new_test_state` are
static methods with no `&self` receiver. The trait is implemented on a zero-sized
marker type (e.g., `struct CounterApp;`), not on a type carrying configuration or
resources. This is intentional: the app type is a namespace for associating a
State type with its UI function, not an instance that holds data. All data lives
in `State`.

**What the trait does not mandate:**
- Internal state structure (flat, nested, ECS — consumer's choice)
- Input/action patterns (`apply(action)` is optional, not required)
- Scenario loading or visual diffing

## Kittest Head (TestHarness)

Orthrus wraps `egui_kittest::Harness` into a `TestHarness` pre-configured for
any `TestableApp` implementor:

```rust
pub struct TestHarness<A: TestableApp> {
    inner: egui_kittest::Harness<'static, A::State>,
}

impl<A: TestableApp> TestHarness<A> {
    /// Creates a harness with the default test state.
    pub fn new() -> Self { ... }

    /// Creates a harness with custom state.
    pub fn with_state(state: A::State) -> Self { ... }

    /// Access the underlying kittest harness for widget queries.
    pub fn harness(&self) -> &egui_kittest::Harness<A::State> { ... }
    pub fn harness_mut(&mut self) -> &mut egui_kittest::Harness<A::State> { ... }

    /// Access the state directly.
    pub fn state(&self) -> &A::State { ... }
}
```

**Lifetime note:** The `'static` bound works because `A::build_ui` is a static
function pointer (no borrows from the environment). `TestHarness::new()` constructs
the inner harness by passing `A::build_ui` as the closure — since it captures
nothing, it satisfies `'static`. If `Harness::new_state` in egui_kittest 0.31
requires a non-`'static` closure, the wrapper design will need adjustment (see
Verification Items).

Ergonomics (Deref to inner harness or forwarding helpers for common operations
like `get_by_label`, `click`, `run`) will be decided during implementation once
the actual egui_kittest 0.31 API is confirmed.

## Crate Structure

```
orthrus/
├── Cargo.toml
├── src/
│   ├── lib.rs          # TestableApp trait, re-exports
│   └── harness.rs      # TestHarness implementation (behind feature flag)
├── examples/
│   └── counter.rs      # Example app (CounterApp)
└── tests/
    └── counter_tests.rs # MVP acceptance tests
```

## Dependencies

```toml
[features]
kittest = ["dep:egui_kittest"]

[dependencies]
egui = "0.31"

[dependencies.egui_kittest]
version = "0.31"
optional = true
# Features TBD — "wgpu" and "snapshot" are desired but must be verified
# against egui_kittest 0.31 before adding (see Verification Items).
```

`egui` is a regular dependency (the trait references `egui::Context`).
`egui_kittest` is optional, gated behind the `kittest` feature flag.

Consumers add Orthrus like:
```toml
[dependencies]
orthrus = { path = "../orthrus" }

[dev-dependencies]
orthrus = { path = "../orthrus", features = ["kittest"] }
```

Cargo merges features when a crate appears in both sections — during `cargo test`,
Orthrus is compiled with `kittest` enabled; in release builds, the test
infrastructure is absent.

This keeps runtime clean (just `egui`) and test infrastructure opt-in.

**AccessKit note:** kittest depends on AccessKit for widget querying. The `egui`
crate has `accesskit` as an optional feature (not in `default`). `eframe` enables
it by default, but `egui-macroquad` does not. For Orthrus's kittest head, this is
not an issue — `egui_kittest::Harness` creates its own `egui::Context` with
AccessKit enabled internally. However, this needs verification (see Verification
Items).

## MVP Example App

A minimal counter app proving the pattern works:

```rust
struct CounterApp;

struct CounterState {
    count: i32,
    panel_visible: bool,
}

impl TestableApp for CounterApp {
    type State = CounterState;

    fn build_ui(ctx: &egui::Context, state: &mut CounterState) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("Count: {}", state.count));
            if ui.button("Increment").clicked() {
                state.count += 1;
            }
            if ui.button("Toggle Panel").clicked() {
                state.panel_visible = !state.panel_visible;
            }
            if state.panel_visible {
                ui.label("Panel is visible");
            }
        });
    }

    fn new_test_state() -> CounterState {
        CounterState { count: 0, panel_visible: false }
    }
}
```

## MVP Acceptance Criteria

These tests must pass using `TestHarness<CounterApp>`:

1. Click "Increment", assert `state.count == 1`
2. Click "Toggle Panel", query for "Panel is visible" label, assert it exists
3. Assert "Panel is visible" is absent in default state
4. (Stretch) Snapshot test of the rendered UI

If all tests pass, the kittest head works and Orthrus is proven.

## What Orthrus Does NOT Do (MVP)

- No macroquad dependency — pure egui
- No visual diffing or pixel comparison
- No scenario loading
- No input action abstraction
- No CI helpers or Xvfb wrappers

## Verification Items (Prerequisite — Before Writing Wrapper Code)

These must be confirmed before implementing `TestHarness`. If any item invalidates
an assumption, the design must be adjusted before proceeding.

1. `Harness::new_state` signature in egui_kittest 0.31 — confirm it exists, confirm
   the closure lifetime requirements, and confirm `harness.state()` returns a
   reference after running
2. Snapshot API in egui_kittest 0.31 — confirm method name, behavior, and which
   feature flags enable it (the `TestRenderer` trait was added in 0.33, so 0.31
   may differ)
3. Whether `accesskit` feature needs explicit enabling on the `egui` crate, or
   whether `egui_kittest::Harness` handles this internally
4. Whether `egui_kittest` 0.31 features `wgpu` and `snapshot` exist — if not,
   determine what's available and adjust dependencies accordingly

## Future Work (Post-MVP)

- **Macroquad head:** Screenshot capture for visual regression testing.
  Decision on library-owned vs consumer-implemented deferred until after MVP.
- **Ergonomic API polish:** Deref, forwarding methods, builder patterns
- **Optional trait extensions:** `ScenarioLoadable`, `ActionDriven`, etc.
- **Snapshot management:** Reference image workflows
- **CI integration:** Xvfb helpers, visual diff tooling

## Design Origin

- Concept originated from Jerry asking "can we do a siamese twin thing" (2026-03-23)
- Architecture validated by agent team brainstorming session
- Prior research: egui_kittest investigation (Oct 2025), AccessKit analysis (Aug 2025)
