// ABOUTME: Tests for the visual testing module's deterministic window configuration.

#[cfg(feature = "visual")]
mod visual {
    use orthrus::visual::test_window_conf;

    #[test]
    fn test_window_conf_sets_requested_dimensions() {
        let conf = test_window_conf(800, 600);
        assert_eq!(conf.window_width, 800);
        assert_eq!(conf.window_height, 600);
    }

    #[test]
    fn test_window_conf_disables_resizing() {
        let conf = test_window_conf(800, 600);
        assert!(!conf.window_resizable);
    }

    #[test]
    fn test_window_conf_disables_high_dpi() {
        let conf = test_window_conf(800, 600);
        assert!(!conf.high_dpi);
    }

    #[test]
    fn test_window_conf_disables_msaa() {
        let conf = test_window_conf(800, 600);
        assert_eq!(conf.sample_count, 1);
    }

    #[test]
    fn test_window_conf_disables_fullscreen() {
        let conf = test_window_conf(800, 600);
        assert!(!conf.fullscreen);
    }

    #[test]
    fn test_window_conf_sets_window_title() {
        let conf = test_window_conf(800, 600);
        assert_eq!(conf.window_title, "orthrus-test");
    }

    #[test]
    fn test_window_conf_disables_vsync() {
        let conf = test_window_conf(800, 600);
        assert_eq!(conf.platform.swap_interval, Some(0));
    }

    use macroquad::texture::Image;
    use orthrus::visual::{compare_images, save_image};
    use orthrus::VisualTestError;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    // compare_images reads UPDATE_SNAPSHOTS env var, which is process-global.
    // Tests that manipulate this env var must hold this lock to avoid races.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn test_dir() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir()
            .join("orthrus_visual_tests")
            .join(format!("{}_{}", std::process::id(), id));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn make_image(width: u16, height: u16, pixel: [u8; 4]) -> Image {
        let pixel_count = width as usize * height as usize;
        let mut bytes = Vec::with_capacity(pixel_count * 4);
        for _ in 0..pixel_count {
            bytes.extend_from_slice(&pixel);
        }
        Image {
            bytes,
            width,
            height,
        }
    }

    #[test]
    fn save_image_writes_valid_png() {
        let img = make_image(2, 2, [255, 0, 0, 255]);
        let path = test_dir().join("save_test.png");

        save_image(&img, &path).unwrap();

        assert!(path.exists());
        // Verify the file can be loaded back as a valid PNG
        let loaded = image::open(&path).unwrap().to_rgba8();
        assert_eq!(loaded.width(), 2);
        assert_eq!(loaded.height(), 2);
        assert_eq!(loaded.get_pixel(0, 0).0, [255, 0, 0, 255]);
    }

    #[test]
    fn compare_images_passes_for_identical() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("UPDATE_SNAPSHOTS") };
        let img = make_image(4, 4, [0, 128, 255, 255]);
        let dir = test_dir();
        let ref_path = dir.join("identical_ref.png");

        save_image(&img, &ref_path).unwrap();
        compare_images(&img, &ref_path, 0.0).unwrap();
    }

    #[test]
    fn compare_images_fails_for_different() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("UPDATE_SNAPSHOTS") };
        let img_a = make_image(2, 2, [255, 0, 0, 255]);
        let img_b = make_image(2, 2, [0, 0, 255, 255]);
        let dir = test_dir();
        let ref_path = dir.join("different_ref.png");

        save_image(&img_a, &ref_path).unwrap();
        let result = compare_images(&img_b, &ref_path, 0.0);

        match result {
            Err(VisualTestError::PixelMismatch {
                differing_pixels,
                total_pixels,
                ..
            }) => {
                assert_eq!(total_pixels, 4);
                assert_eq!(differing_pixels, 4);
            }
            other => panic!("Expected PixelMismatch, got {:?}", other),
        }
    }

    #[test]
    fn compare_images_update_snapshots_creates_reference() {
        let _lock = ENV_LOCK.lock().unwrap();
        let img = make_image(3, 3, [10, 20, 30, 255]);
        let dir = test_dir();
        let ref_path = dir.join("update_ref.png");

        assert!(!ref_path.exists());

        unsafe { std::env::set_var("UPDATE_SNAPSHOTS", "1") };
        let result = compare_images(&img, &ref_path, 0.0);
        unsafe { std::env::remove_var("UPDATE_SNAPSHOTS") };

        result.unwrap();
        assert!(ref_path.exists());
    }

    #[test]
    fn compare_images_reference_not_found() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("UPDATE_SNAPSHOTS") };
        let img = make_image(2, 2, [0, 0, 0, 255]);
        let dir = test_dir();
        let ref_path = dir.join("nonexistent_ref.png");

        let result = compare_images(&img, &ref_path, 0.0);
        match result {
            Err(VisualTestError::ReferenceNotFound { .. }) => {}
            other => panic!("Expected ReferenceNotFound, got {:?}", other),
        }
    }

    #[test]
    fn compare_images_dimension_mismatch() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("UPDATE_SNAPSHOTS") };
        let img_small = make_image(2, 2, [255, 255, 255, 255]);
        let img_big = make_image(4, 4, [255, 255, 255, 255]);
        let dir = test_dir();
        let ref_path = dir.join("dim_mismatch_ref.png");

        save_image(&img_small, &ref_path).unwrap();
        let result = compare_images(&img_big, &ref_path, 0.0);

        match result {
            Err(VisualTestError::DimensionMismatch {
                expected,
                actual,
            }) => {
                assert_eq!(expected, (2, 2));
                assert_eq!(actual, (4, 4));
            }
            other => panic!("Expected DimensionMismatch, got {:?}", other),
        }
    }

    #[test]
    fn compare_images_threshold_allows_some_differences() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("UPDATE_SNAPSHOTS") };
        // Create a 10x10 image (100 pixels)
        let mut img_a = make_image(10, 10, [100, 100, 100, 255]);
        // Modify 5 pixels (5% difference)
        for i in 0..5 {
            let offset = i * 4;
            img_a.bytes[offset] = 200; // change red channel
        }

        let img_b = make_image(10, 10, [100, 100, 100, 255]);
        let dir = test_dir();
        let ref_path = dir.join("threshold_ref.png");

        save_image(&img_b, &ref_path).unwrap();

        // 10% threshold should pass (only 5% differ)
        compare_images(&img_a, &ref_path, 0.10).unwrap();

        // 1% threshold should fail (5% differ)
        let result = compare_images(&img_a, &ref_path, 0.01);
        match result {
            Err(VisualTestError::PixelMismatch {
                differing_pixels,
                total_pixels,
                threshold,
            }) => {
                assert_eq!(differing_pixels, 5);
                assert_eq!(total_pixels, 100);
                assert!((threshold - 0.01).abs() < f32::EPSILON);
            }
            other => panic!("Expected PixelMismatch, got {:?}", other),
        }
    }

    #[test]
    fn compare_images_saves_actual_on_failure() {
        let _lock = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("UPDATE_SNAPSHOTS") };
        let img_a = make_image(2, 2, [255, 0, 0, 255]);
        let img_b = make_image(2, 2, [0, 0, 255, 255]);
        let dir = test_dir();
        let ref_path = dir.join("failure_ref.png");
        let actual_path = dir.join("failure_ref.actual.png");

        save_image(&img_a, &ref_path).unwrap();
        let _ = compare_images(&img_b, &ref_path, 0.0);

        // The actual image should have been saved for debugging
        assert!(actual_path.exists());
    }
}
