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
