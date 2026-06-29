⬆️ [UI Test Specifications](../README.md)

# Scenario — English Pronunciation Playback

## Goal

Verify that a selected English exact-match Card can play Windows built-in TTS
pronunciation with a single-playback lock.

## Preconditions

- Run on the local Windows desktop.
- EasyEnglish is running and the flyout can be triggered.
- The bundled English dictionary is available.
- Windows system audio output is audible.

## Steps

1. Trigger the flyout.
2. Type an English word with an exact dictionary Card, such as `description`.
3. Verify the exact Card is shown.
4. Move keyboard focus to the exact Card if it is not already selected.
5. Press Enter.
6. Verify Windows TTS speaks `description`.
7. While speech is still playing, press Enter or Space repeatedly.
8. Verify no overlapping duplicate speech starts.
9. Wait until speech completes.
10. Press Space while the exact Card is still selected.
11. Verify a new pronunciation playback starts.
12. Move focus to the input box, a Card Preview row, or the Bing row.
13. Press Enter or Space.

## Expected result

- Enter or Space plays pronunciation only when the first large English exact
  Card exists and is selected.
- Playback uses Windows built-in TTS for the exact English headword.
- Playback requests made while speech is active are dropped.
- After speech completes, a later Enter or Space request can start playback
  again.
- Input, Card Preview, Bing, and Chinese preview rows do not trigger
  pronunciation playback.
