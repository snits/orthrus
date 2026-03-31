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

#[cfg(feature = "visual")]
pub mod visual;

#[cfg(feature = "visual")]
pub use visual::ComparisonResult;

#[cfg(feature = "visual")]
pub use visual::VisualTestError;

#[cfg(feature = "kittest")]
mod harness;

#[cfg(feature = "kittest")]
pub use harness::TestHarness;

#[cfg(feature = "kittest")]
pub use accesskit;

#[cfg(feature = "kittest")]
pub use egui_kittest::SnapshotError;

/// Re-exports for writing tests against `TestHarness`.
///
/// # Querying widgets by role
///
/// Some egui widgets (like `ComboBox`) expose as a role in AccessKit rather than
/// a text label. Use `get_by_role()` with `Role` to query them:
///
/// ```ignore
/// use orthrus::kittest_prelude::{Queryable, Role};
///
/// let mut harness = TestHarness::<MyApp>::new();
/// harness.run();
/// let combo = harness.get_by_role(Role::ComboBox);
/// ```
#[cfg(feature = "kittest")]
pub mod kittest_prelude {
    pub use accesskit::Role;
    pub use egui_kittest::kittest::Queryable;
}
