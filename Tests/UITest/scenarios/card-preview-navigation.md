⬆️ [UI Test Specifications](../README.md)

# Scenario — English Card Preview Navigation

## Goal

Verify the English search preview UX: one exact Card at most, up to five Card
Preview rows, exact lookup when a preview is activated, and a persistent Bing
fallback row for non-empty input.

## Preconditions

- Run inside `vm-ee-test`.
- EasyEnglish is running and the flyout can be triggered.
- The bundled English dictionary is available.

## Steps

1. Trigger the flyout.
2. Type a partial English query such as `appl`.
3. Verify at most one exact Card is shown.
4. Verify Card Preview contains no more than five dictionary suggestions.
5. Verify the bottom row offers Bing search for the typed query.
6. Use Up/Down to select a Card Preview row.
7. Press Enter or Space.
8. Verify the input becomes `! <word>` with a space after `!`.
9. Verify the app performs an exact lookup for that word.
10. Verify the exact Card is selected immediately after the preview jump.
11. Repeat activation with a mouse click on a Card Preview row.

## Expected result

- Preview activation always converts the query to exact lookup syntax.
- The exact Card receives focus only after a Card Preview jump.
- Manual typing keeps focus on the input.
- Bing search is visible for every non-empty input.

