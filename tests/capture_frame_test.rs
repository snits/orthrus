// ABOUTME: Integration test binary for capture_frame().
// ABOUTME: Runs inside macroquad's event loop to validate call count and image capture.

use macroquad::prelude::*;
use orthrus::visual::{capture_frame, setup_deterministic_egui, test_window_conf};
use std::cell::Cell;

fn window_conf() -> Conf {
    test_window_conf(64, 64)
}

#[macroquad::main(window_conf)]
async fn main() {
    // Test 1: stabilize_frames = 2 should call render_fn 3 times (2 warm-up + 1 capture)
    let call_count = Cell::new(0u32);
    let image = capture_frame(2, || {
        call_count.set(call_count.get() + 1);
        clear_background(RED);
    })
    .await;

    assert_eq!(
        call_count.get(),
        3,
        "render_fn should be called stabilize_frames + 1 times"
    );
    assert_eq!(image.width(), 64, "image width should match window width");
    assert_eq!(
        image.height(),
        64,
        "image height should match window height"
    );

    // Test 2: stabilize_frames = 0 should call render_fn exactly once
    let call_count2 = Cell::new(0u32);
    let image2 = capture_frame(0, || {
        call_count2.set(call_count2.get() + 1);
        clear_background(BLUE);
    })
    .await;

    assert_eq!(
        call_count2.get(),
        1,
        "stabilize_frames=0 should call render_fn exactly once"
    );
    assert_eq!(image2.width(), 64);
    assert_eq!(image2.height(), 64);

    // Test: Verify warm-up frame convergence
    // Capture with 2 stabilization frames, then capture again with 3.
    // The rendered output should be pixel-identical, confirming 2 frames is enough.
    let image_2frames = capture_frame(2, || {
        clear_background(RED);
    })
    .await;

    let image_3frames = capture_frame(3, || {
        clear_background(RED);
    })
    .await;

    assert_eq!(
        image_2frames.bytes, image_3frames.bytes,
        "Frames after 2 and 3 stabilization frames should be identical"
    );

    // Test: setup_deterministic_egui sets pixels_per_point
    let ctx = egui::Context::default();
    setup_deterministic_egui(&ctx);
    assert_eq!(
        ctx.pixels_per_point(),
        1.0,
        "pixels_per_point should be pinned to 1.0"
    );
    next_frame().await;

    println!("PASS");
}
