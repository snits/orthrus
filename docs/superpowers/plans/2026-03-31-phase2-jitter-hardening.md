# Phase 2 Jitter Hardening Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Harden the visual regression module against GPU rendering jitter, add comparison telemetry, and prepare for cross-platform testing.

**Architecture:** Refactor comparison functions to return a `ComparisonResult` struct with jitter telemetry (per-channel max diff, pixels absorbed by tolerance). Pin `pixels_per_point` in egui setup. Validate warm-up frame convergence. All changes are in the orthrus crate's `visual` module.

**Tech Stack:** Rust, macroquad 0.4, egui 0.31, image 0.24

**Design meeting report:** `.claude/scratchpad/meetings/gpu-jitter-review/report.md`

---

## File Structure

| File | Responsibility | Action |
|------|---------------|--------|
| `src/visual.rs` | All visual testing helpers | Modify: add ComparisonResult, refactor comparison functions, add egui pixels_per_point helper |
| `src/lib.rs` | Crate root re-exports | Modify: re-export ComparisonResult |
| `tests/visual_tests.rs` | Unit tests for visual module | Modify: add tests for ComparisonResult, tolerance telemetry |
| `tests/capture_frame_test.rs` | Integration test for capture | Modify: add warm-up convergence verification |

---

### Task 1: Add ComparisonResult struct

**Files:**
- Modify: `src/visual.rs`
- Modify: `src/lib.rs`
- Modify: `tests/visual_tests.rs`

The comparison functions currently return `Result<(), VisualTestError>` — on success you get nothing, on failure you get error details. This loses valuable telemetry: how much jitter was observed, how close we were to the threshold, per-channel breakdown. Add a `ComparisonResult` struct that the comparison functions return on success.

- [ ] **Step 1: Write failing test for ComparisonResult on identical images**

Add to `tests/visual_tests.rs` inside the `mod visual` block:

```rust
#[test]
fn compare_images_returns_result_with_stats() {
    let _lock = ENV_LOCK.lock().unwrap();
    unsafe { std::env::remove_var("UPDATE_SNAPSHOTS") };
    let img = make_image(4, 4, [100, 100, 100, 255]);
    let dir = test_dir();
    let ref_path = dir.join("result_stats_ref.png");

    save_image(&img, &ref_path).unwrap();
    let result = compare_images(&img, &ref_path, 0.0).unwrap();

    assert_eq!(result.differing_pixels, 0);
    assert_eq!(result.total_pixels, 16);
    assert_eq!(result.max_channel_diff, [0, 0, 0, 0]);
    assert_eq!(result.pixels_absorbed_by_tolerance, 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features visual --test visual_tests compare_images_returns_result_with_stats -- --exact`
Expected: FAIL — `compare_images` currently returns `Result<(), VisualTestError>`, not a struct.

- [ ] **Step 3: Define ComparisonResult and update compare_images return type**

In `src/visual.rs`, add the struct after `VisualTestError`:

```rust
/// Telemetry from a screenshot comparison.
///
/// Returned on success to provide jitter diagnostics, drift tracking,
/// and calibration data. Check `max_channel_diff` to see observed jitter
/// and `pixels_absorbed_by_tolerance` to gauge how much tolerance is doing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonResult {
    /// Max per-channel difference observed across all pixels (R, G, B, A).
    pub max_channel_diff: [u8; 4],
    /// Pixels that differed but were within per_channel_tolerance.
    pub pixels_absorbed_by_tolerance: u32,
    /// Pixels that exceeded per_channel_tolerance.
    pub differing_pixels: u32,
    /// Total pixel count.
    pub total_pixels: u32,
}
```

Change `compare_images` return type from `Result<(), VisualTestError>` to `Result<ComparisonResult, VisualTestError>`. Same for `compare_images_with_tolerance`.

Update the comparison loop in `compare_images_with_tolerance` to track per-channel max diff and absorbed pixels:

```rust
let mut max_diff = [0u8; 4];
let mut pixels_absorbed = 0u32;
let mut differing_pixels = 0u32;

let tol = per_channel_tolerance as i16;
for i in 0..total_pixels as usize {
    let offset = i * 4;
    let mut pixel_max = 0i16;
    for c in 0..4 {
        let diff = (actual.bytes[offset + c] as i16 - ref_bytes[offset + c] as i16).abs();
        pixel_max = pixel_max.max(diff);
        let d = diff as u8;
        if d > max_diff[c] {
            max_diff[c] = d;
        }
    }
    if pixel_max > tol {
        differing_pixels += 1;
    } else if pixel_max > 0 {
        pixels_absorbed += 1;
    }
}

let fraction = differing_pixels as f32 / total_pixels as f32;

if fraction > threshold {
    let actual_path = actual_debug_path(reference_path);
    let _ = save_image(actual, &actual_path);

    return Err(VisualTestError::PixelMismatch {
        differing_pixels,
        total_pixels,
        threshold,
    });
}

Ok(ComparisonResult {
    max_channel_diff: max_diff,
    pixels_absorbed_by_tolerance: pixels_absorbed,
    differing_pixels,
    total_pixels,
})
```

Also update the UPDATE_SNAPSHOTS early return to return a result:

```rust
if std::env::var("UPDATE_SNAPSHOTS").as_deref() == Ok("1") {
    save_image(actual, reference_path)?;
    let total = actual.width() as u32 * actual.height() as u32;
    return Ok(ComparisonResult {
        max_channel_diff: [0; 4],
        pixels_absorbed_by_tolerance: 0,
        differing_pixels: 0,
        total_pixels: total,
    });
}
```

Add re-export in `src/lib.rs`:

```rust
#[cfg(feature = "visual")]
pub use visual::ComparisonResult;
```

- [ ] **Step 4: Fix existing tests that use `.unwrap()` on compare_images**

Existing tests call `compare_images(...).unwrap()` which now returns `ComparisonResult`. These still compile (unwrap discards the Ok value). But tests that check `Err(VisualTestError::PixelMismatch { ... })` should still work since the error path is unchanged.

Review all existing `compare_images` calls in `tests/visual_tests.rs` and verify they still compile. The main changes:
- `compare_images_passes_for_identical`: currently calls `.unwrap()` — still works, discards result
- `compare_images_fails_for_different`: checks `Err(PixelMismatch)` — still works
- Others: should be fine

- [ ] **Step 5: Run all tests to verify they pass**

Run: `cargo test --features visual --test visual_tests --test capture_frame_test`
Expected: All 16 tests pass (15 existing + 1 new)

- [ ] **Step 6: Commit**

```bash
git add src/visual.rs src/lib.rs tests/visual_tests.rs
git commit -s -m "feat: add ComparisonResult telemetry to comparison functions

Return per-channel max diff, pixels absorbed by tolerance, and
pixel counts on successful comparison. Enables jitter drift
tracking and tolerance calibration."
```

---

### Task 2: Add telemetry tests for tolerance scenarios

**Files:**
- Modify: `tests/visual_tests.rs`

Now that ComparisonResult exists, add tests that verify the telemetry values are accurate for various jitter scenarios.

- [ ] **Step 1: Write test for tolerance absorbing differences**

```rust
#[test]
fn comparison_result_reports_absorbed_pixels() {
    let _lock = ENV_LOCK.lock().unwrap();
    unsafe { std::env::remove_var("UPDATE_SNAPSHOTS") };

    // Create a 10x10 reference image (100 pixels)
    let reference = make_image(10, 10, [100, 100, 100, 255]);
    let dir = test_dir();
    let ref_path = dir.join("absorbed_ref.png");
    save_image(&reference, &ref_path).unwrap();

    // Create actual with 5 pixels differing by 10 in the red channel
    let mut actual = make_image(10, 10, [100, 100, 100, 255]);
    for i in 0..5 {
        actual.bytes[i * 4] = 110; // red +10
    }

    // With tolerance=15, those 5 pixels should be absorbed
    let result = compare_images_with_tolerance(&actual, &ref_path, 0.0, 15).unwrap();
    assert_eq!(result.pixels_absorbed_by_tolerance, 5);
    assert_eq!(result.differing_pixels, 0);
    assert_eq!(result.max_channel_diff[0], 10); // red
    assert_eq!(result.max_channel_diff[1], 0);  // green
    assert_eq!(result.max_channel_diff[2], 0);  // blue
    assert_eq!(result.max_channel_diff[3], 0);  // alpha
}
```

- [ ] **Step 2: Write test for per-channel max diff tracking**

```rust
#[test]
fn comparison_result_tracks_per_channel_max() {
    let _lock = ENV_LOCK.lock().unwrap();
    unsafe { std::env::remove_var("UPDATE_SNAPSHOTS") };

    let reference = make_image(4, 4, [100, 100, 100, 255]);
    let dir = test_dir();
    let ref_path = dir.join("perchannel_ref.png");
    save_image(&reference, &ref_path).unwrap();

    // Pixel 0: red differs by 20
    // Pixel 1: green differs by 15
    // Pixel 2: blue differs by 8
    // Pixel 3: alpha differs by 3
    let mut actual = make_image(4, 4, [100, 100, 100, 255]);
    actual.bytes[0] = 120;  // pixel 0: R +20
    actual.bytes[5] = 115;  // pixel 1: G +15
    actual.bytes[10] = 108; // pixel 2: B +8
    actual.bytes[15] = 252; // pixel 3: A -3

    let result = compare_images_with_tolerance(&actual, &ref_path, 1.0, 25).unwrap();
    assert_eq!(result.max_channel_diff[0], 20);
    assert_eq!(result.max_channel_diff[1], 15);
    assert_eq!(result.max_channel_diff[2], 8);
    assert_eq!(result.max_channel_diff[3], 3);
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --features visual --test visual_tests -- --exact`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add tests/visual_tests.rs
git commit -s -m "test: add ComparisonResult telemetry verification tests

Verify per-channel max diff tracking and pixels-absorbed-by-tolerance
counting for calibration scenarios."
```

---

### Task 3: Pin egui pixels_per_point in capture_frame

**Files:**
- Modify: `src/visual.rs`
- Modify: `tests/visual_tests.rs`
- Modify: `tests/capture_frame_test.rs`

The design meeting identified that egui's `pixels_per_point` defaults to system DPI, which can vary across machines. Pin it to 1.0 during capture to eliminate this variable. Since `capture_frame` runs the render closure (which includes `egui_macroquad::ui`), we need to set it during the egui context setup. However, `capture_frame` doesn't have access to the egui context — the caller's closure does.

The pragmatic approach: add a `setup_deterministic_egui` helper that callers invoke inside their render closure before `egui_macroquad::ui`. This keeps capture_frame simple and gives callers explicit control.

- [ ] **Step 1: Write test for setup_deterministic_egui**

This helper sets `pixels_per_point` on the egui context. Since we can't unit-test egui context manipulation without macroquad, add a test to the `capture_frame_test.rs` integration binary.

Add to `tests/capture_frame_test.rs` after the existing tests:

```rust
// Test 3: setup_deterministic_egui sets pixels_per_point
egui_macroquad::ui(|ctx| {
    orthrus::visual::setup_deterministic_egui(ctx);
    assert_eq!(ctx.pixels_per_point(), 1.0, "pixels_per_point should be pinned to 1.0");
});
next_frame().await;
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features visual --test capture_frame_test`
Expected: FAIL — `setup_deterministic_egui` doesn't exist yet

- [ ] **Step 3: Implement setup_deterministic_egui**

In `src/visual.rs`, add:

```rust
/// Configures egui for deterministic rendering in visual tests.
///
/// Call this inside `egui_macroquad::ui()` before building your UI.
/// Pins `pixels_per_point` to 1.0 to eliminate DPI variation across machines.
pub fn setup_deterministic_egui(ctx: &egui::Context) {
    ctx.set_pixels_per_point(1.0);
}
```

Add `egui` to the imports at the top of `visual.rs` — it's already a dependency of the crate (non-optional).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --features visual --test capture_frame_test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/visual.rs tests/capture_frame_test.rs
git commit -s -m "feat: add setup_deterministic_egui helper

Pins pixels_per_point to 1.0 to eliminate DPI variation across
machines in visual tests. Callers invoke inside egui_macroquad::ui()."
```

---

### Task 4: Verify warm-up frame convergence

**Files:**
- Modify: `tests/capture_frame_test.rs`

The design meeting recommended verifying that 2 stabilization frames are sufficient — frames 2 and 3 should produce identical output. This is a one-time validation test.

- [ ] **Step 1: Write convergence test**

Add to `tests/capture_frame_test.rs`:

```rust
// Test 4: Verify warm-up frame convergence
// Capture with 2 stabilization frames, then capture again with 3.
// The rendered output should be pixel-identical, confirming 2 frames is enough.
let image_2frames = capture_frame(2, || {
    clear_background(RED);
}).await;

let image_3frames = capture_frame(3, || {
    clear_background(RED);
}).await;

assert_eq!(
    image_2frames.bytes, image_3frames.bytes,
    "Frames after 2 and 3 stabilization frames should be identical"
);
```

- [ ] **Step 2: Run test**

Run: `cargo test --features visual --test capture_frame_test`
Expected: PASS — solid color fill should converge immediately

- [ ] **Step 3: Commit**

```bash
git add tests/capture_frame_test.rs
git commit -s -m "test: verify warm-up frame convergence

Confirm that 2 stabilization frames produce identical output to 3,
validating the default stabilization count is sufficient."
```

---

### Task 5: Update Alpha Prime snapshot test to use ComparisonResult

**Files:**
- Modify: `/Users/jsnitsel/desert-island/alpha-prime/tests/alphaprime_snapshot.rs`

Now that comparison functions return `ComparisonResult`, update the Alpha Prime test to log the telemetry. This gives us baseline jitter measurements on each run.

- [ ] **Step 1: Update alphaprime_snapshot.rs to print telemetry**

```rust
match compare_images_with_tolerance(&image, &ref_path, 0.02, 25) {
    Ok(result) => {
        println!("PASS");
        println!("  max channel diff: R={} G={} B={} A={}",
            result.max_channel_diff[0], result.max_channel_diff[1],
            result.max_channel_diff[2], result.max_channel_diff[3]);
        println!("  pixels absorbed by tolerance: {}/{}",
            result.pixels_absorbed_by_tolerance, result.total_pixels);
        println!("  pixels exceeding tolerance: {}/{}",
            result.differing_pixels, result.total_pixels);
    }
    Err(e) => {
        eprintln!("Visual test failed: {}", e);
        std::process::exit(1);
    }
}
```

Also add `setup_deterministic_egui` inside the egui_macroquad::ui closure:

```rust
egui_macroquad::ui(|ctx| {
    orthrus::visual::setup_deterministic_egui(ctx);
    AlphaPrimeApp::build_ui(ctx, &mut state);
});
```

- [ ] **Step 2: Regenerate reference image** (pixels_per_point change may affect rendering)

```bash
cd /Users/jsnitsel/desert-island/alpha-prime
UPDATE_SNAPSHOTS=1 cargo test --test alphaprime_snapshot
```

- [ ] **Step 3: Verify test passes and review telemetry output**

```bash
cargo test --test alphaprime_snapshot -- --nocapture
```

Expected: PASS with telemetry showing observed jitter levels.

- [ ] **Step 4: Commit both repos**

Orthrus (if any changes needed):
```bash
cd /Users/jsnitsel/desert-island/orthrus
git add -A && git commit -s -m "..."
```

Alpha Prime:
```bash
cd /Users/jsnitsel/desert-island/alpha-prime
git add tests/alphaprime_snapshot.rs tests/snapshots/default_state.png
git commit -s -m "feat: use ComparisonResult telemetry and deterministic egui setup

Log per-channel jitter stats on each visual test run. Pin
pixels_per_point to 1.0 for cross-machine determinism."
```

---

## Summary

| Task | What | Where |
|------|------|-------|
| 1 | ComparisonResult struct + refactor comparison returns | orthrus |
| 2 | Telemetry verification tests | orthrus |
| 3 | setup_deterministic_egui helper | orthrus |
| 4 | Warm-up frame convergence validation | orthrus |
| 5 | Alpha Prime integration + telemetry logging | alpha-prime |

Tasks 1-4 are orthrus library changes. Task 5 is the consumer integration. Linear dependencies: 1 → 2, 1 → 5. Tasks 3 and 4 are independent of each other but both depend on orthrus compiling.

## Deferred (needs Linux box)

These items from the design meeting need the Linux/NVIDIA system and are out of scope for this plan:

- **Empirical NVIDIA jitter measurement** — run Alpha Prime snapshot test, measure jitter magnitude
- **Empirical llvmpipe determinism validation** — run same test under `xvfb-run`, verify tolerance=0 works
- **Per-platform baseline directories** — implement once multi-platform testing is validated
- **Two-tier testing strategy** — CI config once llvmpipe is confirmed deterministic
