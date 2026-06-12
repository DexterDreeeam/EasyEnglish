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
4. Click the left side of the visible input bar and verify the text field keeps
   focus.
5. Click the right side of the visible input bar and verify the text field also
   keeps focus.
6. Type `apple`.
7. Verify characters appear in the input box without system error beeps.
8. Clear the input.
9. Switch to a Chinese IME.
10. Type and commit `苹果`.
11. Verify the IME candidate UI can appear and the flyout stays visible during
   composition.
12. Verify the committed Chinese text appears in the input box.

## Expected result

- The flyout gains focus after wake.
- The caret is visible.
- The whole visible input bar, including its right side, focuses the text field
  when clicked.
- English input reaches the text field.
- Chinese IME composition and commit work.
- No system error beep occurs during normal typing.
- The flyout does not hide while the IME candidate window is active.
