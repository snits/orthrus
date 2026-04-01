// ABOUTME: Orthrus — dual-headed GUI test harness for macroquad+egui applications.
// ABOUTME: Provides the TestableApp trait and test harness for headless widget testing.

//! Dual-headed GUI test harness for [macroquad](https://macroquad.rs)+[egui](https://docs.rs/egui) applications.
//!
//! Orthrus provides two independent testing approaches ("heads") for applications
//! built with macroquad and egui:
//!
//! - **kittest head** (`feature = "kittest"`) — Headless widget testing via
//!   [`egui_kittest`](https://docs.rs/egui_kittest). Query widgets by text or
//!   accessibility role, simulate clicks, and assert state changes without a GPU.
//!
//! - **visual head** (`feature = "visual"`) — Screenshot-based visual regression
//!   testing for macroquad-rendered frames. Capture frames, compare against
//!   reference images with configurable per-channel tolerance, and track jitter
//!   telemetry via [`ComparisonResult`].
//!
//! Both heads share the [`TestableApp`] trait, which defines a `State` type,
//! a `build_ui` function, and a `new_test_state` constructor. Implement this
//! trait once and test from either angle.
//!
//! # Feature Flags
//!
//! | Feature | What it enables |
//! |---------|----------------|
//! | `kittest` | [`TestHarness`], headless widget testing, accesskit queries |
//! | `visual` | [`capture_frame`](visual::capture_frame), [`compare_images`](visual::compare_images), screenshot comparison |
//!
//! Enable one or both depending on your testing needs.
//!
//! # Quick Start (kittest)
//!
//! ```ignore
//! use orthrus::{TestHarness, TestableApp};
//! use orthrus::kittest_prelude::Queryable;
//!
//! let mut harness = TestHarness::<MyApp>::new();
//! harness.run();
//! harness.get_by_label("Increment").click();
//! harness.run();
//! assert_eq!(harness.state().count, 1);
//! ```
//!
//! # Quick Start (visual)
//!
//! ```ignore
//! use orthrus::visual::{capture_frame, compare_images_with_tolerance, test_window_conf};
//!
//! let image = capture_frame(2, || {
//!     clear_background(BLACK);
//!     render_scene(&state);
//! }).await;
//!
//! compare_images_with_tolerance(&image, &ref_path, 0.02, 25)?;
//! ```
//!
//! For visual tests, run with `UPDATE_SNAPSHOTS=1` to generate or update
//! reference images. In CI, use `xvfb-run` with llvmpipe for deterministic
//! software rendering.

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
