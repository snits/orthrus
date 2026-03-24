# Dual-Headed GUI Test Harness: macroquad + egui + kittest

## Overview

A test architecture that provides two complementary testing "heads" for Rust GUI
applications built with macroquad (game rendering) + egui (UI widgets):

- **Head 1 (kittest):** Headless egui widget testing — query widgets, simulate clicks,
  assert on state via AccessKit. Fast, no GPU needed.
- **Head 2 (macroquad):** Full visual render captured as screenshots for visual regression
  testing. Needs Xvfb on CI.

Both heads consume the same UI functions and shared application state.

## Primary Use Case

Enable an AI coding assistant (Claude) to develop and test GUI applications with a real
feedback loop — writing tests that drive UI programmatically, capturing visual output for
review, and running assertions on widget state — without needing to see or interact with
a live window.

Applicable to desert-island/phoenix and any macroquad+egui project.

## Dependencies (Version Conflict Resolved)

```toml
[dependencies]
macroquad = "0.4"
egui-macroquad = "0.17.3"       # → egui 0.31.1

[dev-dependencies]
egui_kittest = { version = "0.31.0", features = ["snapshot", "wgpu"] }  # → egui 0.31.x
```

No Cargo [patch], no forking, no workspace splitting. Both crates resolve to egui 0.31.x.

### What egui-macroquad Does

egui-macroquad is ~120 lines of glue code that:
1. Forwards macroquad input events into egui (via egui-miniquad)
2. Renders egui's output on top of macroquad's GL context

It does NOT own egui logic or modify egui's behavior. kittest is completely independent
of it — kittest creates its own egui::Context and never touches macroquad.

## Architecture: Three-Layer Separation

```
┌─────────────────────────────────────────────────────┐
│  Layer 3: Test Heads                                │
│  ┌───────────────────┐   ┌────────────────────────┐ │
│  │ Head 1: kittest   │   │ Head 2: macroquad      │ │
│  │ Widget testing     │   │ Visual regression      │ │
│  │ (headless, fast)  │   │ (Xvfb, screenshots)   │ │
│  └────────┬──────────┘   └───────────┬────────────┘ │
│           │                          │               │
├───────────┼──────────────────────────┼───────────────┤
│  Layer 2: UI Functions (egui 0.31 only)              │
│           │                          │               │
│     fn build_game_ui(ctx, state)     │               │
│     fn build_hex_overlay(ctx, state) │               │
│           │                          │               │
├───────────┼──────────────────────────┼───────────────┤
│  Layer 1: Application Core (no rendering deps)       │
│                                                      │
│     GameState, InputAction, game logic               │
│     fn apply(state, action) -> ()                    │
└──────────────────────────────────────────────────────┘
```

Each head creates its own egui::Context. They share application state and UI functions.
Since egui is immediate mode, both heads calling the same UI functions with the same
state produce equivalent widget trees.

## Desirable Constraint: Explicit UI State

**All UI state must live in GameState, not inside egui's Context.**

egui internally tracks things like which windows are open, scroll positions, and
collapsing header state. Since each head has its own egui::Context, that internal state
does NOT transfer between heads. A window opened via kittest interaction won't appear
open in the macroquad screenshot head.

The solution — and this is a *desirable* constraint, not a limitation:

- Which view/panel/screen is active → `state.active_view: ActiveView`
- Whether a dialog is open → `state.show_inventory: bool`
- Scroll positions (if needed) → explicit fields in state

This makes the application better for testing:
- Every test starts from a fully known, reproducible state
- No hidden egui internals to set up or replay click sequences to reach
- Tests can directly assert on state fields (e.g., `state.active_view == Inventory`)
- Both heads are always perfectly synchronized by definition
- Any scenario can be constructed by just building a GameState

```rust
pub enum ActiveView { Map, Inventory, Combat }

fn build_game_ui(ctx: &egui::Context, state: &mut GameState) {
    // View switching driven by state, not egui internals
    if ui.button("Open Inventory").clicked() {
        state.active_view = ActiveView::Inventory;
    }

    match state.active_view {
        ActiveView::Map => build_map_ui(ctx, state),
        ActiveView::Inventory => build_inventory_ui(ctx, state),
        ActiveView::Combat => build_combat_ui(ctx, state),
    }
}
```

## Layer 1: Application Core

```rust
/// All mutable game state — no rendering types
pub struct GameState {
    pub map: HexMap,
    pub selected_hex: Option<HexCoord>,
    pub active_view: ActiveView,
    pub turn: u32,
    pub camera_pos: Vec2,
}

/// Abstracted input — NOT macroquad types
pub enum InputAction {
    SelectHex(HexCoord),
    EndTurn,
    PanCamera(Vec2),
    ZoomCamera(f32),
}

impl GameState {
    pub fn new_test() -> Self { /* known seed, deterministic */ }
    pub fn for_scenario(name: &str) -> Self { /* load scenario */ }

    pub fn apply(&mut self, action: InputAction) {
        match action {
            InputAction::SelectHex(coord) => self.selected_hex = Some(coord),
            InputAction::EndTurn => self.turn += 1,
            // ...
        }
    }
}
```

The main game loop reads macroquad globals and translates to InputAction.
Tests create InputAction directly. For egui widget clicks, kittest handles
that via AccessKit.

## Layer 2: UI Functions

```rust
/// Context-agnostic egui UI — both heads call this
pub fn build_game_ui(ctx: &egui::Context, state: &mut GameState) {
    egui::SidePanel::left("info_panel").show(ctx, |ui| {
        ui.heading("Game Info");
        ui.label(format!("Turn: {}", state.turn));

        if let Some(hex) = state.selected_hex {
            ui.label(format!("Selected: {:?}", hex));
            let terrain = state.map.terrain_at(hex);
            ui.label(format!("Terrain: {terrain}"));
        }

        if ui.button("End Turn").clicked() {
            state.apply(InputAction::EndTurn);
        }
    });
}
```

Zero dependency on macroquad. Only egui::Context + &mut GameState.

## Head 1: kittest (Widget/Interaction Tests)

```rust
#[cfg(test)]
mod ui_tests {
    use egui_kittest::Harness;
    use crate::state::{GameState, HexCoord};
    use crate::ui::build_game_ui;

    #[test]
    fn end_turn_button_increments_turn() {
        let state = GameState::new_test();

        let mut harness = Harness::new_state(state, |ctx, state| {
            build_game_ui(ctx, state);
        });

        harness.get_by_label("End Turn").unwrap().click();
        harness.run();

        assert_eq!(harness.state().turn, 2);
    }

    #[test]
    fn selecting_hex_shows_terrain_info() {
        let mut state = GameState::new_test();
        state.selected_hex = Some(HexCoord::new(3, 5));

        let harness = Harness::new_state(state, |ctx, state| {
            build_game_ui(ctx, state);
        });

        assert!(harness.get_by_label("Terrain: Forest").is_some());
    }

    #[test]
    fn info_panel_snapshot() {
        let mut state = GameState::new_test();
        state.selected_hex = Some(HexCoord::new(0, 0));

        let harness = Harness::new_state(state, |ctx, state| {
            build_game_ui(ctx, state);
        });

        // egui-only snapshot via wgpu — verify exact API in Phase 1
        harness.wgpu_snapshot("info_panel_with_selection");
    }
}
```

Characteristics: normal #[test], no window/GPU/Xvfb, milliseconds per test, any platform.

### What kittest Can Test
- egui widgets exist, have correct text, are enabled/disabled
- Button clicks trigger state changes
- Layout and widget relationships (via AccessKit tree)
- Snapshot images of egui-rendered content (with wgpu feature)

### What kittest Cannot See
- Anything drawn with macroquad's draw_* APIs (sprites, hex maps, shapes)
- Custom macroquad shaders
- macroquad's own macroquad::ui widgets

## Head 2: macroquad (Visual Regression)

```rust
// src/bin/visual_test.rs
use macroquad::prelude::*;

#[macroquad::main("VisualRegression")]
async fn main() {
    let scenario = std::env::var("TEST_SCENARIO")
        .unwrap_or_else(|_| "default".into());
    let mut state = GameState::for_scenario(&scenario);

    // Render frames to stabilize layout/animations
    for _ in 0..5 {
        clear_background(Color::from_hex(0x1a1a2e));
        render_hex_map(&state);
        egui_macroquad::ui(|ctx| build_game_ui(ctx, &mut state));
        egui_macroquad::draw();
        next_frame().await;
    }

    // Capture after next_frame() — framebuffer is populated
    let image = get_screen_data();
    image.export_png(&format!("tests/snapshots/{scenario}_actual.png"));
}
```

### Test runner

```rust
// tests/visual_regression.rs
#[test]
#[ignore] // Run with: cargo test -- --ignored
fn visual_hex_map_default() {
    let output = std::process::Command::new("./target/debug/visual-test")
        .env("TEST_SCENARIO", "default_map")
        .output()
        .expect("failed to run visual test");

    assert!(
        output.status.success(),
        "Visual test failed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let actual = image::open("tests/snapshots/default_map_actual.png").unwrap();
    let reference = image::open("tests/snapshots/default_map_reference.png").unwrap();
    let diff = pixel_diff_percentage(&actual, &reference);
    assert!(diff < 0.5, "Visual regression: {diff:.2}% pixel difference");
}
```

Characteristics: separate binary, #[ignore], Xvfb on CI, captures full scene
(game rendering + egui overlays).

## CI/CD Strategy

```yaml
# Fast tier — every commit
test-widgets:
  runs-on: ubuntu-latest
  steps:
    - cargo test  # All kittest widget tests, no special setup

# Slow tier — nightly or manual
test-visual:
  runs-on: ubuntu-latest
  steps:
    - sudo apt-get install -y xvfb mesa-utils libegl1-mesa
    - cargo build --bin visual-test
    - xvfb-run cargo test -- --ignored
    - uses: actions/upload-artifact@v4
      with:
        name: visual-snapshots
        path: tests/snapshots/*_actual.png
```

## Verification Items for Phase 1

API details to confirm when writing the first test:

1. **Harness::new_state signature in kittest 0.31** — Confirm it exists and that
   harness.state() returns a reference after running. Fallback: Harness::new_ui
   with state captured in closure.

2. **Snapshot API in kittest 0.31** — Confirm whether it's harness.wgpu_snapshot("name")
   or a different method. TestRenderer trait was added in 0.33, so 0.31 may differ.

3. **egui_macroquad::ui() closure** — Takes FnMut (confirmed), so &mut state capture works.

4. **get_screen_data() timing** — Confirm correct framebuffer contents after next_frame().

## Risks

| Risk | Severity | Status |
|---|---|---|
| egui version conflict | RESOLVED | egui_kittest 0.31.0 + egui-macroquad 0.17.3 both use egui 0.31.x |
| kittest 0.31 missing features | LOW | Core functionality present; run() does 3 fixed steps |
| Snapshot cross-platform variance | MEDIUM | Use CI-generated references; epaint font rasterizer is deterministic |
| egui-macroquad maintenance | MEDIUM | Last updated for egui 0.31; fork is ~500 lines if it stalls |
| AccessKit label coverage | LOW | Only affects kittest queries; custom-drawn elements use InputAction |

## Implementation Phases

**Phase 1 — kittest widget tests (start here):**
- Add egui_kittest = "0.31.0" to dev-dependencies
- Extract GameState and UI functions into three-layer structure
- Verify the four API items above
- Write first widget interaction test

**Phase 2 — Visual regression (when needed):**
- Add visual test binary
- Set up Xvfb in CI
- Capture reference screenshots
- Simple pixel diff comparison

**Phase 3 — Polish (if needed):**
- Perceptual diff (dssim)
- Scenario matrix
- Reference image update workflow

## Design Origin

Produced by agent team brainstorming session (2026-03-23):
- egui-specialist: kittest internals, AccessKit, version compatibility
- macroquad-specialist: framebuffer capture, headless rendering, miniquad internals
- test-architect: architecture synthesis and API design
