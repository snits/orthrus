# Orthrus MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Orthrus kittest head — a `TestHarness<A>` wrapper around `egui_kittest::Harness` that lets any `TestableApp` implementor write headless egui widget tests.

**Architecture:** Trait-based marker type pattern. `TestableApp` defines a `State` type, a static `build_ui` function, and a `new_test_state` constructor. `TestHarness<A>` wraps `egui_kittest::Harness<'static, A::State>` and uses `Deref`/`DerefMut` to expose the inner harness's widget query and execution methods directly.

**Tech Stack:** Rust (edition 2024), egui 0.31, egui_kittest 0.31

**Spec:** `docs/superpowers/specs/2026-03-23-orthrus-mvp-design.md`

**API Research:** `.claude/scratchpad/egui-kittest-api-research.md`, `.claude/scratchpad/egui-kittest-features-research.md`

---

## File Structure

```
orthrus/
├── Cargo.toml              # MODIFY — add egui + egui_kittest deps, kittest feature
├── src/
│   ├── lib.rs              # REPLACE — TestableApp trait, conditional harness module
│   └── harness.rs          # CREATE — TestHarness<A> implementation
└── tests/
    └── counter_tests.rs    # CREATE — CounterApp + acceptance tests
```

**Design notes:**
- `CounterApp` lives in the test file, not in `examples/`. Integration tests cannot import from examples, and the counter is only ~20 lines. An example can be added later if needed.
- `TestHarness` implements `Deref<Target = egui_kittest::Harness<'static, A::State>>` and `DerefMut`. This means `run()`, `get_by_label()`, `state()`, `state_mut()`, etc. all work directly on `TestHarness` via deref coercion. The wrapper only adds typed constructors (`new()`, `with_state()`).

---

## Task 1: Project Setup

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Update Cargo.toml with dependencies**

Replace the entire contents of `Cargo.toml` with:

```toml
[package]
name = "orthrus"
version = "0.1.0"
edition = "2024"

[features]
kittest = ["dep:egui_kittest"]

[dependencies]
egui = "0.31"

[dependencies.egui_kittest]
version = "0.31"
optional = true
```

**Why no `wgpu`/`snapshot` features yet:** Those are for the stretch snapshot goal (Task 6). Adding them now would pull in heavy GPU dependencies. Add when needed.

- [ ] **Step 2: Verify compilation**

Run: `cargo check --features kittest`
Expected: Compiles with warnings about unused default lib.rs code. No errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -s -m "deps: add egui and egui_kittest dependencies"
```

---

## Task 2: TestableApp Trait

**Files:**
- Replace: `src/lib.rs`

- [ ] **Step 1: Replace src/lib.rs with trait definition**

Replace the entire contents of `src/lib.rs` with:

```rust
// ABOUTME: Orthrus — dual-headed GUI test harness for macroquad+egui applications.
// ABOUTME: Provides the TestableApp trait and test harness for headless widget testing.

/// Defines a testable egui application.
///
/// Implemented on a zero-sized marker type (e.g., `struct CounterApp;`).
/// The marker type serves as a namespace associating a State type with
/// its UI function. All data lives in `State`, not in the marker.
pub trait TestableApp {
    /// The application's state type — single source of truth for all UI state.
    type State;

    /// Builds the UI given an egui context and mutable state reference.
    /// Both test heads call this same function.
    fn build_ui(ctx: &egui::Context, state: &mut Self::State);

    /// Creates a deterministic state suitable for testing.
    fn new_test_state() -> Self::State;
}

#[cfg(feature = "kittest")]
mod harness;

#[cfg(feature = "kittest")]
pub use harness::TestHarness;
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check`
Expected: Compiles successfully. The `harness` module doesn't exist yet, but it's behind the `kittest` feature which isn't enabled by default.

Run: `cargo check --features kittest`
Expected: Compile error about missing `harness` module. This is expected — we'll create it in Task 3.

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -s -m "feat: define TestableApp trait"
```

---

## Task 3: Click Increment Acceptance Test (TDD)

This is the core TDD cycle. Write the test first, then implement `TestHarness` to make it pass.

**Files:**
- Create: `tests/counter_tests.rs`
- Create: `src/harness.rs`

- [ ] **Step 1: Write the failing test**

Create `tests/counter_tests.rs`:

```rust
// ABOUTME: Acceptance tests for Orthrus using a minimal counter app.
// ABOUTME: Proves the TestableApp + TestHarness pattern works end-to-end.

use orthrus::{TestHarness, TestableApp};

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
        CounterState {
            count: 0,
            panel_visible: false,
        }
    }
}

#[test]
fn click_increment() {
    let mut harness = TestHarness::<CounterApp>::new();

    // Run initial frame to render widgets
    harness.run();

    // Click the increment button
    harness.get_by_label("Increment").click();
    harness.run();

    assert_eq!(harness.state().count, 1);
}
```

**Note on widget query API:** The `get_by_label("Increment").click()` pattern is the expected kittest API. If the exact method names differ (e.g., `query_by_label`, `find_by_label`), check `docs.rs/egui_kittest/0.31.0` and adjust. The key methods to look for are widget querying by label text and simulated click interaction.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --features kittest click_increment`
Expected: Compile error — `TestHarness` does not exist (harness module is missing).

- [ ] **Step 3: Implement TestHarness**

Create `src/harness.rs`:

```rust
// ABOUTME: TestHarness wraps egui_kittest::Harness for TestableApp implementors.
// ABOUTME: Provides typed constructors; delegates all other methods via Deref.

use std::ops::{Deref, DerefMut};

use crate::TestableApp;

/// Wraps `egui_kittest::Harness` pre-configured for a `TestableApp` implementor.
///
/// Construct via `new()` (uses default test state) or `with_state()` (custom state).
/// All `egui_kittest::Harness` methods (widget queries, `run()`, `state()`, etc.)
/// are accessible directly through `Deref` coercion.
pub struct TestHarness<A: TestableApp> {
    inner: egui_kittest::Harness<'static, A::State>,
}

impl<A: TestableApp> TestHarness<A> {
    /// Creates a harness with the default test state from `A::new_test_state()`.
    pub fn new() -> Self {
        Self::with_state(A::new_test_state())
    }

    /// Creates a harness with custom state.
    pub fn with_state(state: A::State) -> Self {
        let inner = egui_kittest::Harness::new_state(A::build_ui, state);
        Self { inner }
    }
}

impl<A: TestableApp> Deref for TestHarness<A> {
    type Target = egui_kittest::Harness<'static, A::State>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<A: TestableApp> DerefMut for TestHarness<A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --features kittest click_increment`
Expected: PASS

**If the widget query API doesn't match** (e.g., `get_by_label` doesn't exist), check `docs.rs/egui_kittest/0.31.0` for the correct method names. Common alternatives: `query_by_label`, `find_by_label`. The `Harness` may also expose queries through a `node()` or `tree()` method that returns a kittest query handle.

- [ ] **Step 5: Commit**

```bash
git add src/harness.rs tests/counter_tests.rs
git commit -s -m "feat: implement TestHarness with click increment test"
```

---

## Task 4: Toggle Panel Acceptance Test

**Files:**
- Modify: `tests/counter_tests.rs`

- [ ] **Step 1: Add the toggle panel test**

Add to `tests/counter_tests.rs`:

```rust
#[test]
fn toggle_panel_visible() {
    let mut harness = TestHarness::<CounterApp>::new();
    harness.run();

    // Panel should not be visible initially
    assert!(!harness.state().panel_visible);

    // Click toggle
    harness.get_by_label("Toggle Panel").click();
    harness.run();

    // Panel should now be visible
    assert!(harness.state().panel_visible);

    // Verify the label appears in the widget tree
    harness.get_by_label("Panel is visible");
}
```

**Note:** `get_by_label("Panel is visible")` will panic if the label doesn't exist, which serves as our assertion. If you need a non-panicking check, look for `try_get_by_label` or similar.

- [ ] **Step 2: Run the test to verify it passes**

Run: `cargo test --features kittest toggle_panel`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add tests/counter_tests.rs
git commit -s -m "test: add toggle panel acceptance test"
```

---

## Task 5: Panel Absent by Default

**Files:**
- Modify: `tests/counter_tests.rs`

- [ ] **Step 1: Add the panel-absent test**

This test verifies that "Panel is visible" does NOT appear in the default state. This requires a non-panicking query method. If `try_get_by_label` doesn't exist, use an alternative approach.

Add to `tests/counter_tests.rs`:

```rust
#[test]
fn panel_absent_by_default() {
    let mut harness = TestHarness::<CounterApp>::new();
    harness.run();

    // Panel label should NOT be present in default state
    // Use try_get_by_label if available, otherwise check state directly
    assert!(!harness.state().panel_visible);

    // If a try_get_by_label or similar method exists:
    // assert!(harness.try_get_by_label("Panel is visible").is_none());
}
```

**Implementor note:** The state assertion is the minimum. If you can find a non-panicking widget query method (check the kittest/egui_kittest docs), also assert that the label widget is absent from the tree. This tests both state correctness AND UI rendering correctness.

- [ ] **Step 2: Run the test to verify it passes**

Run: `cargo test --features kittest panel_absent`
Expected: PASS

- [ ] **Step 3: Run all tests together**

Run: `cargo test --features kittest`
Expected: All 3 tests pass.

- [ ] **Step 4: Commit**

```bash
git add tests/counter_tests.rs
git commit -s -m "test: add panel-absent-by-default acceptance test"
```

---

## Task 6: (Stretch) Snapshot Test

**This task is a stretch goal.** It requires `wgpu` and `snapshot` features, which pull in GPU dependencies. It may not work in a container without GPU passthrough. Attempt it, but if it fails due to GPU/rendering issues, document the failure and defer to post-MVP.

**Files:**
- Modify: `Cargo.toml`
- Modify: `tests/counter_tests.rs`
- Create: `tests/snapshots/` (auto-created by snapshot framework)

- [ ] **Step 1: Add wgpu and snapshot features to Cargo.toml**

Update the `egui_kittest` dependency:

```toml
[dependencies.egui_kittest]
version = "0.31"
optional = true
features = ["wgpu", "snapshot"]
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check --features kittest`
Expected: Compiles. May take a while — wgpu pulls in many dependencies.

- [ ] **Step 3: Write the snapshot test**

Add to `tests/counter_tests.rs`:

```rust
#[test]
fn snapshot_default_state() {
    let mut harness = TestHarness::<CounterApp>::new();
    harness.run();

    // Capture a snapshot of the rendered UI
    // First run creates the reference image; subsequent runs compare against it
    harness.try_snapshot("counter_default")
        .expect("snapshot comparison failed");
}
```

**Note:** The first run will always pass (creates the reference snapshot). Run twice to verify comparison works. Reference images are saved to `tests/snapshots/counter_default.png`.

- [ ] **Step 4: Run the snapshot test**

Run: `cargo test --features kittest snapshot_default`
Expected: PASS on first run (creates reference image).

If it fails with a GPU/wgpu error (e.g., "no suitable adapter found"), this is a container GPU issue. Document and defer.

- [ ] **Step 5: Add snapshot artifacts to .gitignore**

Add to `.gitignore`:

```
**/tests/snapshots/**/*.diff.png
**/tests/snapshots/**/*.new.png
```

**Do** commit the reference `.png` files — those are the "expected" outputs.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml tests/counter_tests.rs .gitignore
git add tests/snapshots/ 2>/dev/null  # may not exist if snapshot failed
git commit -s -m "feat: add snapshot test for default counter state"
```

---

## Completion Criteria

**MVP is proven when Tasks 1-5 pass:** `cargo test --features kittest` runs 3 acceptance tests (click increment, toggle panel, panel absent by default) and all pass.

**Stretch is proven when Task 6 also passes:** Snapshot test creates and compares reference images.

## Post-MVP

After MVP is proven, the next steps (separate plans) are:
- Ergonomic API polish (forwarding helpers if Deref proves insufficient)
- Macroquad head design decision (library-owned vs pattern-documented)
- Consumer integration (Alpha Prime, Phoenix)
