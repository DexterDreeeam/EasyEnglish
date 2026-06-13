//! Flyout animation state and the pure focus-loss auto-hide decision.

/// Wall-clock grace window after a wake during which a not-yet-focused flyout is
/// allowed to keep waiting for focus to arrive instead of immediately hiding.
/// Once focus has ever been acquired, focus loss hides the flyout regardless of
/// this window. Kept short so an immediate click-away still hides promptly.
pub(crate) const WAKE_FOCUS_GRACE: std::time::Duration = std::time::Duration::from_millis(250);

/// How long the flyout must remain continuously non-foreground before auto-hiding.
/// Debouncing tolerates transient focus blips — most importantly the IME candidate
/// window briefly taking foreground while composing Chinese, but also OS toasts and
/// momentary activation changes — which would otherwise hide the flyout mid-use.
pub(crate) const HIDE_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(180);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnimationState {
    Hidden,
    FadingIn,
    Visible,
    FadingOut,
}

/// Outcome of evaluating whether the flyout should auto-hide on focus loss.
///
/// Extracted as a pure function so the interaction between the post-wake grace
/// window, whether focus was ever acquired, and the current viewport focus state
/// is unit-testable and can be logged precisely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FocusHideDecision {
    /// Visible, unfocused, and either focus was previously acquired (then lost)
    /// or the post-wake grace window has elapsed → begin fading out.
    Hide,
    /// Visible and unfocused, but focus has never been acquired yet and the grace
    /// window is still open → keep waiting (and repainting) for focus to arrive.
    WaitForFocus,
    /// No action: focused, not yet "ready" (not Visible), or composing IME.
    Keep,
}

pub(crate) fn evaluate_focus_hide(
    state: AnimationState,
    focused: Option<bool>,
    ime_composing: bool,
    was_focused: bool,
    grace_expired: bool,
    unfocused_long_enough: bool,
) -> FocusHideDecision {
    // Only auto-hide once the flyout is fully shown ("ready") and not mid-IME.
    if state != AnimationState::Visible || ime_composing {
        return FocusHideDecision::Keep;
    }
    // Unknown focus (`None`) is treated as still-focused: never hide on uncertainty.
    if focused.unwrap_or(true) {
        return FocusHideDecision::Keep;
    }
    // Visible and genuinely unfocused. Hide only once the loss has persisted past
    // the debounce window (and either focus was previously acquired or the post-wake
    // grace has elapsed); a brief blip keeps waiting so a returning foreground — e.g.
    // dismissing the IME candidate window — leaves the flyout up.
    if (was_focused || grace_expired) && unfocused_long_enough {
        FocusHideDecision::Hide
    } else {
        FocusHideDecision::WaitForFocus
    }
}
