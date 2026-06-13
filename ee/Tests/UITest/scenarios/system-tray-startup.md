⬆️ [UI Test Specifications](../README.md)

# Scenario — System Tray Launch on Startup

## Goal

Verify the Windows system tray menu exposes a checked `Launch on Startup` item
that controls the current user's autostart entry.

## Preconditions

- Run inside `vm-ee-test`.
- EasyEnglish is running and the tray icon is visible.
- The test user can inspect `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

## Steps

1. Right-click the EasyEnglish tray icon.
2. Verify the context menu contains `Show Flyout`, `Launch on Startup`, and
   `Exit`.
3. Verify `Launch on Startup` is checked on first run.
4. Click `Launch on Startup`.
5. Right-click the tray icon again.
6. Verify `Launch on Startup` is unchecked.
7. Verify the `EasyEnglish` value is removed from
   `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
8. Click `Launch on Startup` again.
9. Verify the menu item is checked.
10. Verify the `EasyEnglish` value is restored under the HKCU Run key and points
    at the current `ee-win.exe`.

## Expected result

- Launch-on-startup defaults to enabled.
- Toggling the tray menu item updates both the menu check state and HKCU Run
  registration.
- The setting persists across app restarts.

