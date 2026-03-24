// ABOUTME: Acceptance tests for Orthrus using a minimal counter app.
// ABOUTME: Proves the TestableApp + TestHarness pattern works end-to-end.

use orthrus::kittest_prelude::Queryable;
use orthrus::{TestHarness, TestableApp};

struct CounterApp;

struct CounterState {
    count: i32,
    panel_visible: bool,
}

impl TestableApp for CounterApp {
    type State = CounterState;

    fn build_ui(ctx: &egui::Context, state: &mut CounterState) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(format!("Count: {}", state.count));
            if ui.button("Increment").clicked() {
                state.count += 1;
            }
            if ui.button("Toggle Panel").clicked() {
                state.panel_visible = !state.panel_visible;
            }
            if state.panel_visible {
                ui.label("Panel is visible");
            }
        });
    }

    fn new_test_state() -> CounterState {
        CounterState {
            count: 0,
            panel_visible: false,
        }
    }
}

#[test]
fn click_increment() {
    let mut harness = TestHarness::<CounterApp>::new();

    // Run initial frame to render widgets
    harness.run();

    // Click the increment button
    harness.get_by_label("Increment").click();
    harness.run();

    assert_eq!(harness.state().count, 1);
}

#[test]
fn toggle_panel_visible() {
    let mut harness = TestHarness::<CounterApp>::new();
    harness.run();

    // Panel should not be visible initially
    assert!(!harness.state().panel_visible);

    // Click toggle
    harness.get_by_label("Toggle Panel").click();
    harness.run();

    // Panel should now be visible
    assert!(harness.state().panel_visible);

    // Verify the label appears in the widget tree
    harness.get_by_label("Panel is visible");
}
