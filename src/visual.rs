// ABOUTME: Visual testing support for macroquad-based rendering.
// ABOUTME: Provides deterministic window configuration and frame capture for visual regression testing.

use macroquad::texture::{get_screen_data, Image};
use macroquad::window::next_frame;

use std::path::{Path, PathBuf};

/// Errors that can occur during visual regression testing.
#[derive(Debug)]
pub enum VisualTestError {
    Io(std::io::Error),
    ImageSave(String),
    ImageLoad(String),
    ReferenceNotFound { path: PathBuf, hint: String },
    DimensionMismatch { expected: (u32, u32), actual: (u32, u32) },
    PixelMismatch { differing_pixels: u32, total_pixels: u32, threshold: f32 },
}

impl std::fmt::Display for VisualTestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::ImageSave(msg) => write!(f, "Failed to save image: {}", msg),
            Self::ImageLoad(msg) => write!(f, "Failed to load image: {}", msg),
            Self::ReferenceNotFound { path, hint } => {
                write!(f, "Reference image not found at {}: {}", path.display(), hint)
            }
            Self::DimensionMismatch { expected, actual } => {
                write!(
                    f,
                    "Dimension mismatch: expected {}x{}, got {}x{}",
                    expected.0, expected.1, actual.0, actual.1
                )
            }
            Self::PixelMismatch { differing_pixels, total_pixels, threshold } => {
                let fraction = *differing_pixels as f64 / *total_pixels as f64;
                write!(
                    f,
                    "Pixel mismatch: {}/{} pixels differ ({:.2}%), threshold {:.2}%",
                    differing_pixels, total_pixels, fraction * 100.0, threshold * 100.0
                )
            }
        }
    }
}

impl From<std::io::Error> for VisualTestError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<image::ImageError> for VisualTestError {
    fn from(e: image::ImageError) -> Self {
        Self::ImageSave(e.to_string())
    }
}

/// Saves a macroquad `Image` as PNG to the given path.
///
/// Creates parent directories if they don't exist.
pub fn save_image(image: &Image, path: &Path) -> Result<(), VisualTestError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let rgba: image::RgbaImage = image::ImageBuffer::from_raw(
        image.width() as u32,
        image.height() as u32,
        image.bytes.clone(),
    )
    .ok_or_else(|| VisualTestError::ImageSave("buffer size mismatch".into()))?;
    rgba.save(path)?;
    Ok(())
}

/// Compares a captured image against a reference file on disk.
///
/// If `UPDATE_SNAPSHOTS=1` env var is set, saves actual to reference_path and returns `Ok(())`.
/// `threshold` is a f32 from 0.0 to 1.0 representing the max allowed fraction of differing pixels.
pub fn compare_images(
    actual: &Image,
    reference_path: &Path,
    threshold: f32,
) -> Result<(), VisualTestError> {
    if std::env::var("UPDATE_SNAPSHOTS").as_deref() == Ok("1") {
        save_image(actual, reference_path)?;
        return Ok(());
    }

    if !reference_path.exists() {
        return Err(VisualTestError::ReferenceNotFound {
            path: reference_path.to_path_buf(),
            hint: "Run with UPDATE_SNAPSHOTS=1 to create reference images".into(),
        });
    }

    let reference = image::open(reference_path)
        .map_err(|e| VisualTestError::ImageLoad(e.to_string()))?
        .to_rgba8();

    let actual_w = actual.width() as u32;
    let actual_h = actual.height() as u32;
    let ref_w = reference.width();
    let ref_h = reference.height();

    if actual_w != ref_w || actual_h != ref_h {
        return Err(VisualTestError::DimensionMismatch {
            expected: (ref_w, ref_h),
            actual: (actual_w, actual_h),
        });
    }

    let total_pixels = actual_w * actual_h;
    let ref_bytes = reference.as_raw();
    let mut differing_pixels = 0u32;

    for i in 0..total_pixels as usize {
        let offset = i * 4;
        if actual.bytes[offset] != ref_bytes[offset]
            || actual.bytes[offset + 1] != ref_bytes[offset + 1]
            || actual.bytes[offset + 2] != ref_bytes[offset + 2]
            || actual.bytes[offset + 3] != ref_bytes[offset + 3]
        {
            differing_pixels += 1;
        }
    }

    let fraction = differing_pixels as f32 / total_pixels as f32;

    if fraction > threshold {
        // Save actual image alongside reference for debugging
        let actual_path = actual_debug_path(reference_path);
        let _ = save_image(actual, &actual_path);

        return Err(VisualTestError::PixelMismatch {
            differing_pixels,
            total_pixels,
            threshold,
        });
    }

    Ok(())
}

/// Derives the `.actual.png` debug path from a reference path.
fn actual_debug_path(reference_path: &Path) -> PathBuf {
    let stem = reference_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    reference_path.with_file_name(format!("{}.actual.png", stem))
}

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
