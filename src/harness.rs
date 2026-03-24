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

impl<A: TestableApp + 'static> TestHarness<A> {
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
