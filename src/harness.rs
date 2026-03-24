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

    /// Takes a snapshot, returning `Err` instead of panicking when no GPU is available.
    ///
    /// In headless or no-GPU environments (CI, dev containers), the underlying wgpu
    /// renderer cannot initialize. This method catches that failure and returns it as
    /// a `SnapshotError::RenderError` rather than panicking.
    pub fn try_snapshot(&mut self, name: &str) -> Result<(), egui_kittest::SnapshotError> {
        // egui_kittest's try_snapshot panics if the wgpu renderer can't be created
        // (e.g., no GPU adapter in headless environments). We catch that panic and
        // convert it to a SnapshotError::RenderError.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.inner.try_snapshot(name)
        }));

        match result {
            Ok(inner_result) => inner_result,
            Err(panic_payload) => {
                let msg = if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else {
                    "snapshot rendering panicked (likely no GPU adapter available)".to_string()
                };
                Err(egui_kittest::SnapshotError::RenderError { err: msg })
            }
        }
    }
}

impl<A: TestableApp + 'static> Default for TestHarness<A> {
    fn default() -> Self {
        Self::new()
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
