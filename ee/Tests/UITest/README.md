⬇️ [First Auto Flyout](scenarios/first-auto-flyout.md) · [Overlay input and IME](scenarios/overlay-input-ime.md) · [Selection to Flyout Input](scenarios/selection-to-flyout-input.md) · [Card preview navigation](scenarios/card-preview-navigation.md) · [English pronunciation playback](scenarios/english-pronunciation-playback.md) · [Chinese to English](scenarios/chinese-to-english.md) · [System tray startup](scenarios/system-tray-startup.md)

# EasyEnglish UI Test Specifications

This folder contains markdown-only UI automation specifications. Rust unit and
integration tests live separately in `Tests/UnitTest/`.

## Scope

UI tests cover Windows desktop behavior that cannot be validated by pure Rust
unit tests:

- flyout wake and focus acquisition;
- first auto flyout on process launch;
- caret visibility and keyboard input;
- Chinese IME composition and commit behavior;
- selection-to-flyout input prefill and select-all replacement;
- card preview keyboard/mouse activation;
- English exact-card pronunciation playback;
- Chinese to English two-level preview navigation;
- Bing fallback row visibility and activation.
- system tray launch-on-startup toggling.

## Required execution target

Run UI automation on the local Windows desktop only when the user explicitly
asks to test, run, or verify UI behavior.
