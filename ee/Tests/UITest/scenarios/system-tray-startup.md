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

## Start Menu re-launch behavior

1. Start EasyEnglish and leave it running in the tray.
2. Hide the flyout.
3. Launch EasyEnglish again from the Start Menu.
4. Verify no second resident process stays alive.
5. Verify the existing tray instance wakes and shows the flyout.

Expected result: launching EasyEnglish from the Start Menu while it is already
running behaves like `Show Flyout`, not like a silent no-op.

## Start Menu first-launch behavior

1. Exit EasyEnglish completely.
2. Launch EasyEnglish from the Start Menu.
3. Verify the process starts.
4. Verify the flyout is shown immediately.
5. Verify the tray icon remains available after hiding the flyout.

Expected result: manual Start Menu launches pass `--show` and wake the flyout
even when there was no resident tray process.
