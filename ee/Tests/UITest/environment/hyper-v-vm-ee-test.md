⬆️ [UI Test Specifications](../README.md)

# Hyper-V VM Environment — `vm-ee-test`

## Purpose

`vm-ee-test` is the dedicated Windows UI automation machine for EasyEnglish.
Using a VM keeps focus, hotkey, IME, and desktop-window tests isolated from the
developer's host desktop.

## Provisioning rules

1. Use Hyper-V as the preferred virtualization backend.
2. If Hyper-V is unavailable or disabled, ask the user to confirm before enabling
   it because it can require a reboot and can affect other virtualization tools.
3. If `vm-ee-test` does not exist, create it before running UI automation.
4. Before downloading a Windows ISO, search the user's Downloads directory and
   reuse a suitable existing ISO when possible.
5. If no suitable ISO exists, download an official Windows ISO into Downloads and
   use that file for VM setup.
6. Do not reboot the host, enable Hyper-V, create or modify VMs, or download
   large ISO files silently.
7. Do not bypass Windows licensing or activation requirements.

## Minimum VM capabilities

- Windows 11 guest OS.
- Desktop session reachable by the agent through PowerShell/RDP or equivalent
  automation channel.
- Rust toolchain and project build dependencies available, or a documented setup
  step that installs them.
- EasyEnglish repository checkout or synced build artifact.
- Ability to run `ee-win.exe` interactively in the guest desktop.
- Ability to collect logs from the guest after each run.

## Required report fields

Every UI automation run should report:

- VM name;
- guest Windows version;
- EasyEnglish commit or working tree state;
- test scenario files executed;
- pass/fail result;
- screenshots or logs captured when a scenario fails;
- any missing setup step that blocked execution.

