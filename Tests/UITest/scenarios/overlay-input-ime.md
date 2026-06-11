⬆️ [UI Test Specifications](../README.md)

# Scenario — Overlay Input, Caret, and IME

## Goal

Verify that the Windows flyout accepts keyboard input reliably after wake,
shows the text caret, and supports Chinese IME composition without system error
beeps or unintended auto-hide.

## Preconditions

- Run inside `vm-ee-test`.
- EasyEnglish is built and `ee-win.exe` is available.
- A Chinese IME is installed and selectable in the guest OS.
- No other EasyEnglish process is already running.

## Steps

1. Start `ee-win.exe`.
2. Trigger the flyout with Alt + backtick.
3. Verify the input box is focused and a caret is visible.
4. Type `apple`.
5. Verify characters appear in the input box without system error beeps.
6. Clear the input.
7. Switch to a Chinese IME.
8. Type and commit `苹果`.
9. Verify the IME candidate UI can appear and the flyout stays visible during
   composition.
10. Verify the committed Chinese text appears in the input box.

## Expected result

- The flyout gains focus after wake.
- The caret is visible.
- English input reaches the text field.
- Chinese IME composition and commit work.
- No system error beep occurs during normal typing.
- The flyout does not hide while the IME candidate window is active.
