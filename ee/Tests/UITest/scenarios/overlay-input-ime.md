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
- The packaged `version` file is present next to the app executable.

## Steps

1. Start `ee-win.exe`.
2. Trigger the flyout with Alt + backtick.
3. Verify the input box is focused and a caret is visible.
4. On the first wake after process start, allow the background version check to
   finish.
5. If the remote `version` differs from the packaged local `version`, verify an
   English update toast appears above the input bar.
6. Click the left side of the visible input bar and verify the text field keeps
   focus.
7. Click the right side of the visible input bar and verify the text field also
   keeps focus.
8. Type `apple`.
9. Verify characters appear in the input box without system error beeps.
10. Clear the input.
11. Switch to a Chinese IME.
12. Type and commit `苹果`.
13. Verify the IME candidate UI can appear and the flyout stays visible during
   composition.
14. Verify the committed Chinese text appears in the input box.

## Expected result

- The flyout gains focus after wake.
- The caret is visible.
- Version mismatch is reported as a non-blocking English update toast.
- The whole visible input bar, including its right side, focuses the text field
  when clicked.
- English input reaches the text field.
- Chinese IME composition and commit work.
- No system error beep occurs during normal typing.
- The flyout does not hide while the IME candidate window is active.
