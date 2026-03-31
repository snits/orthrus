// ABOUTME: Visual testing support for macroquad-based rendering.
// ABOUTME: Provides deterministic window configuration, frame capture, and screenshot comparison.

use macroquad::texture::{get_screen_data, Image};
use macroquad::window::next_frame;

/// Returns a macroquad `Conf` configured for deterministic, reproducible rendering.
///
/// Disables MSAA, vsync, high-DPI scaling, and window resizing so that
/// screenshot captures produce identical pixel output across runs.
pub fn test_window_conf(width: i32, height: i32) -> macroquad::window::Conf {
    macroquad::window::Conf {
        window_title: "orthrus-test".to_string(),
        window_width: width,
        window_height: height,
        high_dpi: false,
        fullscreen: false,
        sample_count: 1,
        window_resizable: false,
        platform: macroquad::miniquad::conf::Platform {
            swap_interval: Some(0),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Captures a rendered frame after optional warm-up iterations.
///
/// Runs `stabilize_frames` warm-up iterations (each calling `render_fn` followed
/// by a frame swap), then renders one final frame and captures it before the
/// swap. The capture frame is always frame N+1, not counted in `stabilize_frames`.
///
/// `stabilize_frames = 0` skips warm-up and captures immediately.
///
/// `render_fn` must produce deterministic output. Caller is responsible for
/// freezing time sources and disabling animation state. `capture_frame` does
/// not enforce determinism.
///
/// # Example
///
/// ```ignore
/// let image = capture_frame(2, || {
///     clear_background(BLACK);
///     egui_macroquad::ui(|ctx| MyApp::build_ui(ctx, &mut state));
///     egui_macroquad::draw();
/// }).await;
/// ```
pub async fn capture_frame(stabilize_frames: usize, mut render_fn: impl FnMut()) -> Image {
    for _ in 0..stabilize_frames {
        render_fn();
        next_frame().await;
    }

    render_fn();
    get_screen_data()
}
