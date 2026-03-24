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

#[test]
fn panel_absent_by_default() {
    let mut harness = TestHarness::<CounterApp>::new();
    harness.run();

    assert!(!harness.state().panel_visible);

    // Verify the panel label is absent from the rendered widget tree
    assert!(harness.query_by_label("Panel is visible").is_none());
}

#[test]
fn snapshot_default_state() {
    let mut harness = TestHarness::<CounterApp>::new();
    harness.run();

    // Capture a snapshot of the rendered UI
    // First run creates the reference image; subsequent runs compare against it
    harness
        .try_snapshot("counter_default")
        .expect("snapshot comparison failed");
}

#[test]
fn accesskit_role_accessible_via_orthrus() {
    // Verify consumers can use accesskit::Role through Orthrus's re-export
    // without needing a direct accesskit dependency
    let _role = orthrus::accesskit::Role::Button;
}

#[test]
fn get_by_role_via_prelude() {
    use orthrus::kittest_prelude::Role;

    let mut harness = TestHarness::<CounterApp>::new();
    harness.run();

    // Buttons are accessible by role through the prelude
    let buttons = harness.query_all_by_role(Role::Button);
    // Counter app has two buttons: Increment and Toggle Panel
    assert_eq!(buttons.count(), 2);
}
