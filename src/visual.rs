// ABOUTME: Visual testing support for macroquad-based rendering.
// ABOUTME: Provides deterministic window configuration for reproducible screenshot capture.

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
