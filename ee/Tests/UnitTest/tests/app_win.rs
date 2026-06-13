#![cfg(target_os = "windows")]

mod dict {
    use std::path::PathBuf;

    pub(crate) fn scan_for_highest_db_version(_prefix: &str) -> Option<PathBuf> {
        None
    }

    pub(crate) fn load_highest_version_word_list(_prefix: &str) -> Vec<String> {
        Vec::new()
    }
}

#[allow(dead_code)]
#[path = "..\\..\\..\\App\\Win\\src\\focus.rs"]
mod focus;

mod logging {
    pub(crate) fn log_message(_msg: &str) {}
}

#[allow(dead_code)]
#[path = "..\\..\\..\\App\\Win\\src\\signals.rs"]
mod signals;

#[allow(dead_code)]
#[path = "..\\..\\..\\App\\Win\\src\\startup.rs"]
mod startup;

mod win32 {
    pub(crate) fn wide_null(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub(crate) fn cursor_monitor_rect() -> (f32, f32, f32, f32) {
        (0.0, 0.0, 1920.0, 1080.0)
    }

    pub(crate) fn find_flyout_window() -> isize {
        0
    }

    pub(crate) fn flyout_is_foreground() -> Option<bool> {
        Some(true)
    }

    pub(crate) unsafe fn focus_flyout_and_clear_alt(_hwnd: isize) {}

    #[cfg(debug_assertions)]
    pub(crate) fn focus_debug_snapshot() -> String {
        "test snapshot".to_string()
    }
}

mod focus_tests {
    use super::focus::{evaluate_focus_hide, AnimationState, FocusHideDecision};

    #[test]
    fn focus_hide_waits_when_unfocused_in_grace_never_focused() {
        let decision = evaluate_focus_hide(
            AnimationState::Visible,
            Some(false),
            false,
            false,
            false,
            true,
        );
        assert_eq!(decision, FocusHideDecision::WaitForFocus);
    }

    #[test]
    fn focus_hide_hides_when_grace_expired_without_focus() {
        let decision = evaluate_focus_hide(
            AnimationState::Visible,
            Some(false),
            false,
            false,
            true,
            true,
        );
        assert_eq!(decision, FocusHideDecision::Hide);
    }

    #[test]
    fn focus_hide_hides_after_losing_acquired_focus_when_sustained() {
        let decision = evaluate_focus_hide(
            AnimationState::Visible,
            Some(false),
            false,
            true,
            false,
            true,
        );
        assert_eq!(decision, FocusHideDecision::Hide);
    }

    #[test]
    fn focus_hide_waits_on_transient_focus_blip() {
        let decision = evaluate_focus_hide(
            AnimationState::Visible,
            Some(false),
            false,
            true,
            true,
            false,
        );
        assert_eq!(decision, FocusHideDecision::WaitForFocus);
    }

    #[test]
    fn focus_hide_keeps_while_focused() {
        let decision =
            evaluate_focus_hide(AnimationState::Visible, Some(true), false, true, true, true);
        assert_eq!(decision, FocusHideDecision::Keep);
    }

    #[test]
    fn focus_hide_keeps_when_focus_unknown() {
        let decision = evaluate_focus_hide(AnimationState::Visible, None, false, true, true, true);
        assert_eq!(decision, FocusHideDecision::Keep);
    }

    #[test]
    fn focus_hide_keeps_while_composing_ime() {
        let decision =
            evaluate_focus_hide(AnimationState::Visible, Some(false), true, true, true, true);
        assert_eq!(decision, FocusHideDecision::Keep);
    }

    #[test]
    fn focus_hide_keeps_when_not_visible() {
        let decision = evaluate_focus_hide(
            AnimationState::FadingIn,
            Some(false),
            false,
            true,
            true,
            true,
        );
        assert_eq!(decision, FocusHideDecision::Keep);
    }
}

mod signals_tests {
    use super::signals::VISIBLE_REQUESTED;
    use std::sync::atomic::Ordering;

    #[test]
    fn test_global_keyboard_hook_wakeup() {
        VISIBLE_REQUESTED.store(false, Ordering::SeqCst);
        assert!(!VISIBLE_REQUESTED.load(Ordering::SeqCst));
    }
}

mod startup_tests {
    use super::startup::launch_on_startup_run_value;
    use std::path::Path;

    #[test]
    fn launch_on_startup_run_value_quotes_exe_path() {
        assert_eq!(
            launch_on_startup_run_value(Path::new("C:\\Program Files\\EasyEnglish\\ee-win.exe")),
            "\"C:\\Program Files\\EasyEnglish\\ee-win.exe\""
        );
    }
}

#[allow(dead_code)]
#[path = "..\\..\\..\\App\\Win\\src\\overlay.rs"]
mod overlay;

mod overlay_tests {
    use super::overlay::{
        centered_on_monitor, cn_focus_step, cn_row_activation_index, draw_growing_results_panel,
        exact_query_for, focus_for_new_query, input_is_chinese, input_text_edit_width,
        parse_query_input, same_monitor, should_focus_on_pointer_hover, smooth_damp, CnNavKey,
        FLYOUT_INPUT_PANEL_HEIGHT, FLYOUT_MAX_WINDOW_HEIGHT, FLYOUT_WINDOW_WIDTH,
        RESULTS_ANIM_SMOOTH_TIME,
    };

    #[test]
    fn centered_on_primary_origin_matches_legacy_formula() {
        let (size, pos) = centered_on_monitor(0.0, 0.0, 1920.0, 1080.0);
        let top_y = (1080.0 - FLYOUT_INPUT_PANEL_HEIGHT) / 2.0;
        assert_eq!(size.x, FLYOUT_WINDOW_WIDTH);
        assert!((pos.x - (1920.0 - FLYOUT_WINDOW_WIDTH) / 2.0).abs() < f32::EPSILON);
        assert!((pos.y - top_y).abs() < f32::EPSILON);
    }

    #[test]
    fn centered_on_offset_monitor_shifts_by_origin() {
        let (_, base) = centered_on_monitor(0.0, 0.0, 1920.0, 1080.0);
        let (_, shifted) = centered_on_monitor(1920.0, 0.0, 1920.0, 1080.0);
        assert!((shifted.x - (base.x + 1920.0)).abs() < f32::EPSILON);
        assert!((shifted.y - base.y).abs() < f32::EPSILON);
    }

    #[test]
    fn same_size_monitor_yields_same_window_size() {
        let (base_size, _) = centered_on_monitor(0.0, 0.0, 1920.0, 1080.0);
        let (offset_size, _) = centered_on_monitor(-1920.0, 200.0, 1920.0, 1080.0);
        assert_eq!(base_size, offset_size);
    }

    #[test]
    fn same_monitor_matches_by_origin() {
        let a = (1920.0, 0.0, 1920.0, 1080.0);
        assert!(same_monitor(a, (1920.0, 0.0, 2560.0, 1440.0)));
        assert!(same_monitor(a, (1920.4, -0.3, 1920.0, 1080.0)));
        assert!(!same_monitor(a, (0.0, 0.0, 1920.0, 1080.0)));
        assert!(!same_monitor(a, (1920.0, 1080.0, 1920.0, 1080.0)));
    }

    #[test]
    fn parse_query_plain_is_fuzzy() {
        assert_eq!(parse_query_input("apple"), ("apple".to_string(), false));
    }

    #[test]
    fn parse_query_bang_is_exact() {
        assert_eq!(parse_query_input("!apple"), ("apple".to_string(), true));
    }

    #[test]
    fn parse_query_bang_trims_inner_space() {
        assert_eq!(parse_query_input("!  apple"), ("apple".to_string(), true));
    }

    #[test]
    fn parse_query_bang_only_is_empty_exact() {
        assert_eq!(parse_query_input("!"), (String::new(), true));
    }

    #[test]
    fn parse_query_empty_is_plain() {
        assert_eq!(parse_query_input(""), (String::new(), false));
    }

    #[test]
    fn exact_query_prefixes_bang_and_space() {
        assert_eq!(exact_query_for("apple"), "! apple");
        assert_eq!(exact_query_for("new york"), "! new york");
    }

    #[test]
    fn exact_query_round_trips_to_exact_lookup() {
        let raw = exact_query_for("Apple").to_lowercase();
        assert_eq!(parse_query_input(&raw), ("apple".to_string(), true));
    }

    #[test]
    fn preview_jump_focuses_card_others_focus_input() {
        assert_eq!(focus_for_new_query(true), 1);
        assert_eq!(focus_for_new_query(false), 0);
    }

    #[test]
    fn input_is_chinese_detects_cjk() {
        assert!(input_is_chinese("苹果"));
        assert!(input_is_chinese("a苹果"));
        assert!(!input_is_chinese("apple"));
        assert!(!input_is_chinese(""));
    }

    #[test]
    fn input_text_edit_width_fills_available_inner_width() {
        assert_eq!(input_text_edit_width(420.0), 420.0);
        assert_eq!(input_text_edit_width(0.0), 0.0);
        assert_eq!(input_text_edit_width(-12.0), 0.0);
    }

    #[test]
    fn pointer_hover_focus_requires_hover_and_motion() {
        assert!(should_focus_on_pointer_hover(true, true));
        assert!(!should_focus_on_pointer_hover(true, false));
        assert!(!should_focus_on_pointer_hover(false, true));
        assert!(!should_focus_on_pointer_hover(false, false));
    }

    #[test]
    fn smooth_damp_moves_toward_target_without_overshoot() {
        let mut vel = 0.0;
        let next = smooth_damp(0.0, 300.0, &mut vel, RESULTS_ANIM_SMOOTH_TIME, 0.011);
        assert!((0.0..300.0).contains(&next), "got {next}");
        assert!(vel > 0.0, "velocity should build up, got {vel}");
    }

    #[test]
    fn smooth_damp_settles_and_stops_at_target() {
        let mut vel = 0.2;
        let out = smooth_damp(299.8, 300.0, &mut vel, RESULTS_ANIM_SMOOTH_TIME, 0.011);
        assert_eq!(out, 300.0);
        assert_eq!(vel, 0.0);
    }

    #[test]
    fn smooth_damp_velocity_is_continuous_across_a_target_jump() {
        let mut vel = 0.0;
        let mut h = 0.0;
        for _ in 0..30 {
            h = smooth_damp(h, 120.0, &mut vel, RESULTS_ANIM_SMOOTH_TIME, 0.011);
        }
        h = smooth_damp(h, 260.0, &mut vel, RESULTS_ANIM_SMOOTH_TIME, 0.011);
        assert!(h < 260.0, "should still be climbing, got {h}");
        assert!(vel > 0.0, "velocity should remain positive across the jump");
    }

    #[test]
    fn smooth_damp_shrinks_toward_zero() {
        let mut vel = 0.0;
        let next = smooth_damp(300.0, 0.0, &mut vel, RESULTS_ANIM_SMOOTH_TIME, 0.011);
        assert!((0.0..300.0).contains(&next), "got {next}");
        assert!(vel < 0.0, "velocity should be negative while shrinking");
    }

    #[test]
    fn growing_panel_renders_without_panic() {
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::vec2(FLYOUT_WINDOW_WIDTH, FLYOUT_MAX_WINDOW_HEIGHT),
            )),
            ..Default::default()
        };
        for display in [0.0_f32, 1.0, 25.0, 120.0, 400.0] {
            let _ = ctx.run(input.clone(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let _natural = draw_growing_results_panel(ui, 1.0, display, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label("alpha");
                        ui.label("beta");
                        ui.label("gamma");
                    });
                });
            });
        }
    }

    #[test]
    fn cn_focus_right_enters_and_advances_buttons() {
        assert_eq!(cn_focus_step(CnNavKey::Right, 1, None, 2, 3), (1, Some(0)));
        assert_eq!(
            cn_focus_step(CnNavKey::Right, 1, Some(0), 2, 3),
            (1, Some(1))
        );
        assert_eq!(
            cn_focus_step(CnNavKey::Right, 1, Some(2), 2, 3),
            (1, Some(2))
        );
        assert_eq!(cn_focus_step(CnNavKey::Right, 1, None, 2, 0), (1, None));
        assert_eq!(cn_focus_step(CnNavKey::Right, 0, None, 2, 3), (0, None));
    }

    #[test]
    fn cn_focus_left_retreats_then_returns_to_row() {
        assert_eq!(
            cn_focus_step(CnNavKey::Left, 1, Some(2), 2, 3),
            (1, Some(1))
        );
        assert_eq!(cn_focus_step(CnNavKey::Left, 1, Some(0), 2, 3), (1, None));
        assert_eq!(cn_focus_step(CnNavKey::Left, 1, None, 2, 3), (1, None));
    }

    #[test]
    fn cn_focus_up_down_move_rows_and_drop_buttons() {
        assert_eq!(cn_focus_step(CnNavKey::Down, 1, Some(1), 2, 3), (2, None));
        assert_eq!(cn_focus_step(CnNavKey::Up, 1, Some(1), 2, 3), (0, None));
        assert_eq!(cn_focus_step(CnNavKey::Down, 3, None, 2, 0), (3, None));
        assert_eq!(cn_focus_step(CnNavKey::Up, 0, None, 2, 0), (0, None));
    }

    #[test]
    fn cn_single_candidate_row_activates_on_space() {
        assert_eq!(cn_row_activation_index(None, true, 1), Some(0));
    }

    #[test]
    fn cn_multi_candidate_row_needs_explicit_button() {
        assert_eq!(cn_row_activation_index(None, true, 3), None);
        assert_eq!(cn_row_activation_index(Some(2), true, 3), Some(2));
    }

    #[test]
    fn cn_activation_ignores_unselected_or_empty_rows() {
        assert_eq!(cn_row_activation_index(None, false, 1), None);
        assert_eq!(cn_row_activation_index(None, true, 0), None);
        assert_eq!(cn_row_activation_index(Some(5), true, 3), None);
    }
}
