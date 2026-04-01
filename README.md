# Orthrus

Dual-headed GUI test harness for [macroquad](https://macroquad.rs)+[egui](https://docs.rs/egui) applications.

Orthrus provides two independent testing approaches for applications built with macroquad and egui:

- **kittest head** — Headless widget testing via [egui_kittest](https://docs.rs/egui_kittest). Query widgets by text or accessibility role, simulate clicks, and assert state changes without a GPU.

- **visual head** — Screenshot-based visual regression testing for macroquad-rendered frames. Capture frames, compare against reference images with configurable per-channel tolerance, and track jitter telemetry.

Both heads share the `TestableApp` trait. Implement it once and test from either angle.

## Feature Flags

| Feature | What it enables |
|---------|----------------|
| `kittest` | `TestHarness`, headless widget testing, accesskit queries |
| `visual` | `capture_frame`, `compare_images`, screenshot comparison |

Enable one or both depending on your testing needs:

```toml
[dev-dependencies]
orthrus = { version = "0.1", features = ["kittest", "visual"] }
```

## Usage

### Define a TestableApp

```rust
use orthrus::TestableApp;

struct MyApp;

impl TestableApp for MyApp {
    type State = MyState;

    fn build_ui(ctx: &egui::Context, state: &mut MyState) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Increment").clicked() {
                state.count += 1;
            }
            ui.label(format!("Count: {}", state.count));
        });
    }

    fn new_test_state() -> MyState {
        MyState { count: 0 }
    }
}
```

### Headless Widget Testing (kittest)

```rust
use orthrus::{TestHarness, TestableApp};
use orthrus::kittest_prelude::Queryable;

let mut harness = TestHarness::<MyApp>::new();
harness.run();
harness.get_by_label("Increment").click();
harness.run();
assert_eq!(harness.state().count, 1);
```

### Visual Regression Testing (visual)

Visual tests run as standalone binaries inside macroquad's event loop:

```rust
use orthrus::visual::{capture_frame, compare_images_with_tolerance, test_window_conf};

fn window_conf() -> macroquad::window::Conf {
    test_window_conf(800, 600)
}

#[macroquad::main(window_conf)]
async fn main() {
    let state = MyApp::new_test_state();

    let image = capture_frame(2, || {
        clear_background(BLACK);
        render_scene(&state);
        egui_macroquad::ui(|ctx| MyApp::build_ui(ctx, &mut state));
        egui_macroquad::draw();
    }).await;

    let ref_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/snapshots/my_scene.png");

    if let Err(e) = compare_images_with_tolerance(&image, &ref_path, 0.02, 25) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
```

Generate or update reference images:

```bash
UPDATE_SNAPSHOTS=1 cargo test --test my_visual_test
```

### Comparison Telemetry

`compare_images` and `compare_images_with_tolerance` return a `ComparisonResult` on success with per-channel max diff, pixels absorbed by tolerance, and pixel counts — useful for monitoring jitter drift across platforms.

## CI Setup

Visual tests require a display server. Use Xvfb with Mesa's llvmpipe software renderer for deterministic, GPU-independent results:

```bash
# Install on Ubuntu/Debian
sudo apt-get install xvfb

# Install on Fedora
sudo dnf install xorg-x11-server-Xvfb

# Run visual tests
xvfb-run -a cargo test --test my_visual_test
```

llvmpipe produces near-deterministic output (max per-channel jitter of ~3), making it ideal for CI. Hardware GPUs show higher jitter (20-50+ depending on vendor), which is handled by the per-channel tolerance parameter.

See [.github/workflows/ci.yml](.github/workflows/ci.yml) for a complete GitHub Actions example.

## Design Notes

The two heads are independent — you cannot render with macroquad and then query widget state with kittest in a single test. This is by design: macroquad owns the event loop for visual tests, while kittest runs headlessly. Choose the head that matches what you're testing:

- Use **kittest** for widget behavior (clicks, state changes, accessibility)
- Use **visual** for rendered appearance (layout, colors, regression detection)

## License

MIT
