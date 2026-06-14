⬆️ [UI Test Specifications](../README.md)

# Scenario — First Auto Flyout

## Goal

Verify EasyEnglish automatically shows the flyout once when a new process starts,
then returns to normal manual wake behavior after the first dismiss.

## Preconditions

- Run on the local Windows desktop.
- EasyEnglish is built and `ee-win.exe` is available.
- No other EasyEnglish process is already running.

## Steps

1. Start `ee-win.exe`.
2. Verify the flyout appears automatically without pressing the hotkey or using
   the tray menu.
3. Verify the input box is focused and ready for typing.
4. Dismiss the flyout with Escape or by clicking outside it.
5. Wait long enough to confirm the flyout does not reappear by itself.
6. Trigger the flyout with Alt + backtick or tray `Show Flyout`.
7. Verify the manual wake still shows and focuses the flyout.

## Expected result

- The first process launch triggers exactly one automatic flyout.
- The automatic flyout uses the same placement, focus, and fade-in behavior as a
  normal wake.
- Dismissing the first auto flyout does not cause another automatic wake in the
  same process.
- Manual hotkey and tray wake behavior still works after dismissal.
