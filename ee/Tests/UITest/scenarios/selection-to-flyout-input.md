⬆️ [UI Test Specifications](../README.md)

# Scenario — Hotkey Wake Without Selection Capture

## Goal

Verify that the Windows global hotkey wakes the flyout without reading selected
text from another application while clipboard-based selection capture is
disabled for stability.

## Preconditions

- Run on the local Windows desktop.
- EasyEnglish is running and the flyout can be triggered with Alt + backtick.
- The system clipboard contains a known text sentinel value, such as
  `easyenglish-clipboard-sentinel`.

## Steps

1. In an external application, select the text `apple`.
2. Press Alt + backtick without changing focus manually.
3. Verify the EasyEnglish flyout appears and the input box is empty.
4. Type `b`.
5. Verify the input box contains `b`.
6. Dismiss the flyout.
7. Verify the system clipboard still contains the sentinel value.

## Expected result

- Hotkey wake does not read selected text before the flyout takes focus.
- The flyout uses the normal empty-input wake path.
- The system clipboard is not modified by hotkey wake.
