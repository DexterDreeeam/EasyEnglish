⬆️ [UI Test Specifications](../README.md)

# Scenario — Chinese to English Preview Navigation

## Goal

Verify Chinese input uses exact and prefix matching only, displays up to five
Chinese preview rows, and supports two-level keyboard navigation into English
word buttons.

## Preconditions

- Run inside `vm-ee-test`.
- EasyEnglish is running.
- The bundled Chinese to English dictionary is available.
- A Chinese IME is installed, or the automation can paste Chinese text directly.

## Steps

1. Trigger the flyout.
2. Enter `苹`.
3. Verify Chinese preview rows are shown for exact/prefix matches only.
4. Verify no edit-distance typo result is shown for a non-prefix typo.
5. Verify there are no more than five Chinese preview rows.
6. Verify each row shows the Chinese term on the left and up to three English
   word buttons on the right.
7. Use Up/Down to select a Chinese row.
8. Press Right.
9. Verify row selection is cleared and the first English word button is focused.
10. Use Left/Right to move between English buttons.
11. Use Up/Down while on a button.
12. Verify focus leaves the button level and moves between rows.
13. Press Enter or Space on an English button.
14. Verify the input becomes `! <english-word>`.
15. Verify the app switches to English exact lookup and selects the resulting
   Card directly.

## Expected result

- Chinese matching is exact plus prefix only.
- English equivalents are ordered by usage frequency.
- Button-level navigation is reachable and reversible by keyboard.
- Click, Enter, and Space on an English button all trigger the same exact English
  lookup behavior.

