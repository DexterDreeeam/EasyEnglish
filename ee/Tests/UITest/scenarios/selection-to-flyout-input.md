⬆️ [UI Test Specifications](../README.md)

# Scenario — Selection to Flyout Input

## Goal

Verify that the Windows global hotkey can wake the flyout with text selected in
another application prefilled into the input bar, and that the prefilled text is
fully selected for immediate replacement.

## Preconditions

- Run on the local Windows desktop.
- EasyEnglish is running and the flyout can be triggered with Alt + backtick.
- A browser, editor, or document application with UI Automation text selection
  support is open.
- The system clipboard contains a known text sentinel value, such as
  `easyenglish-clipboard-sentinel`.

## Steps

1. In the external application, select the text `apple`.
2. Press Alt + backtick without changing focus manually.
3. Verify the EasyEnglish flyout appears and the input box contains `apple`.
4. Type `b`.
5. Verify the input box now contains only `b`, not `appleb`.
6. Dismiss the flyout.
7. Verify the system clipboard still contains the sentinel value.
8. Select text in an application or control that does not expose UI Automation
   text selection.
9. Press Alt + backtick.

## Expected result

- Hotkey wake reads the selected text before the flyout takes focus.
- The selected text is prefilled into the input box.
- The prefilled input is fully selected, so typing replaces it immediately.
- The system clipboard is restored to its original content after temporary copy
  extraction.
- Unsupported source controls fall back to the normal empty-input flyout wake.
