//! Pure overlay interaction helpers.

use super::{
    FLYOUT_BOTTOM_MARGIN, FLYOUT_INPUT_PANEL_HEIGHT, FLYOUT_MAX_WINDOW_HEIGHT, FLYOUT_WINDOW_WIDTH,
};
use eframe::egui;

/// Compute the flyout window size and outer top-left position so it is
/// horizontally centred on the given monitor and vertically placed at the
/// monitor's mid-line. All inputs/outputs are in the window's logical points;
/// `mon_left`/`mon_top` are the monitor's origin so the result lands on that
/// monitor. Pure (no Win32 / egui context) so it is unit-testable.
pub(crate) fn centered_on_monitor(
    mon_left: f32,
    mon_top: f32,
    mon_w: f32,
    mon_h: f32,
) -> (egui::Vec2, egui::Pos2) {
    let top_y = (mon_h - FLYOUT_INPUT_PANEL_HEIGHT) / 2.0;
    let size = egui::vec2(
        FLYOUT_WINDOW_WIDTH,
        FLYOUT_MAX_WINDOW_HEIGHT
            .min((mon_h - top_y - FLYOUT_BOTTOM_MARGIN).max(FLYOUT_INPUT_PANEL_HEIGHT)),
    );
    let pos = egui::pos2(
        mon_left + (mon_w - FLYOUT_WINDOW_WIDTH) / 2.0,
        mon_top + top_y,
    );
    (size, pos)
}

/// Whether two monitor rects (physical left/top/width/height) refer to the same
/// monitor. Comparing the origin is sufficient because distinct monitors never
/// share a top-left corner; the small tolerance absorbs any rounding.
pub(crate) fn same_monitor(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    (a.0 - b.0).abs() < 1.0 && (a.1 - b.1).abs() < 1.0
}

/// Parse the (already trimmed, lower-cased) overlay input into the effective
/// search key and whether an exact-only lookup was requested. A leading `!`
/// requests exact mode: `!apple` -> ("apple", true); `apple` -> ("apple", false).
pub(crate) fn parse_query_input(raw: &str) -> (String, bool) {
    if let Some(rest) = raw.strip_prefix('!') {
        (rest.trim().to_string(), true)
    } else {
        (raw.to_string(), false)
    }
}

/// Build the overlay input that selects `word` for an exact lookup.
///
/// Selecting a card preview fills the search box with `! <word>` (note the
/// space after `!`). The next frame's instant-search feeds this back through
/// [`parse_query_input`], which strips the `!` and surrounding space to yield
/// an exact-only query for `word`.
pub(crate) fn exact_query_for(word: &str) -> String {
    format!("! {}", word.trim())
}

/// Whether the input should be treated as a Chinese -> English search: true when
/// it contains at least one CJK ideograph.
pub(crate) fn input_is_chinese(input: &str) -> bool {
    input
        .chars()
        .any(|c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
}

/// TextEdit must occupy the full inner search-box width so clicks anywhere in
/// the visible input bar focus the text field, not just the left text extent.
pub(crate) fn input_text_edit_width(available_width: f32) -> f32 {
    available_width.max(0.0)
}

/// A moving pointer hover selects focusable rows/cards, while a stationary
/// pointer left over from a layout move does not steal keyboard focus.
pub(crate) fn should_focus_on_pointer_hover(hovered: bool, pointer_moving: bool) -> bool {
    hovered && pointer_moving
}

/// Focus index to land on after dispatching a fresh query. A Card Preview jump
/// arms `arm_card_focus` so focus lands directly on the exact-match Card (index
/// 1); every other (manual) query focuses the input box (index 0).
pub(crate) fn focus_for_new_query(arm_card_focus: bool) -> usize {
    if arm_card_focus {
        1
    } else {
        0
    }
}

/// Arrow keys handled by the two-level Chinese focus model.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CnNavKey {
    /// Move to the previous focus row.
    Up,
    /// Move to the next focus row.
    Down,
    /// Move left among row buttons, or back to whole-row focus.
    Left,
    /// Enter or advance among row buttons.
    Right,
}

/// Pure transition for the Chinese results' two-level focus.
///
/// Focus index layout: `0` = input box, `1..=num_rows` = Chinese preview rows,
/// `num_rows + 1` = the "Search on Bing" entry. `active_button` is the focused
/// English button within the focused row, or `None` when the whole row is
/// selected. Up/Down move between rows and always drop back to row selection;
/// Right enters / advances the English buttons; Left retreats and, from the
/// first button, returns to row selection.
pub(crate) fn cn_focus_step(
    key: CnNavKey,
    focus_index: usize,
    active_button: Option<usize>,
    num_rows: usize,
    buttons_in_focused_row: usize,
) -> (usize, Option<usize>) {
    let last = num_rows + 1; // "Search on Bing" index
    let on_row = focus_index >= 1 && focus_index <= num_rows;
    match key {
        CnNavKey::Down => ((focus_index + 1).min(last), None),
        CnNavKey::Up => (focus_index.saturating_sub(1), None),
        CnNavKey::Right => {
            if on_row && buttons_in_focused_row > 0 {
                let next = match active_button {
                    None => 0,
                    Some(b) => (b + 1).min(buttons_in_focused_row - 1),
                };
                (focus_index, Some(next))
            } else {
                (focus_index, active_button)
            }
        }
        CnNavKey::Left => {
            if on_row {
                match active_button {
                    Some(b) if b > 0 => (focus_index, Some(b - 1)),
                    _ => (focus_index, None),
                }
            } else {
                (focus_index, active_button)
            }
        }
    }
}

/// Decide which English candidate a Space/Enter keypress should activate on the
/// currently focused Chinese row.
///
/// When a specific English button is focused (`active_button = Some(b)`), that
/// word is activated. As a shortcut, when the whole row is selected (no button
/// focused) and the row exposes exactly one candidate, that lone word is
/// activated directly. Returns `None` when there is nothing to activate.
pub(crate) fn cn_row_activation_index(
    active_button: Option<usize>,
    row_selected: bool,
    num_english: usize,
) -> Option<usize> {
    match active_button {
        Some(b) if b < num_english => Some(b),
        None if row_selected && num_english == 1 => Some(0),
        _ => None,
    }
}
