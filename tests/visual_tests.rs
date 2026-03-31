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
}
