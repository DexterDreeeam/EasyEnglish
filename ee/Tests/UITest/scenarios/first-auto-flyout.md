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
4. Verify the app starts one GitHub version check for the process.
5. If that check reports a newer remote version, trigger the next flyout after
   the result is known and verify an independent update banner appears above the
   input bar.
6. Verify the update banner auto-hides after 2 seconds.
7. Dismiss the flyout with Escape or by clicking outside it.
8. Wait long enough to confirm the flyout does not reappear by itself.
9. Trigger the flyout with Alt + backtick or tray `Show Flyout`.
10. Verify the manual wake still shows and focuses the flyout.
11. Verify the update banner does not appear a second time in the same process.

## Expected result

- The first process launch triggers exactly one automatic flyout.
- The automatic flyout uses the same placement, focus, and fade-in behavior as a
  normal wake.
- Dismissing the first auto flyout does not cause another automatic wake in the
  same process.
- Manual hotkey and tray wake behavior still works after dismissal.
- The update banner is shown only after a successful version mismatch result,
  appears above the input bar as a separate panel, auto-hides after 2 seconds,
  and appears at most once per process.
- Matching versions and failed network checks do not show the banner.
