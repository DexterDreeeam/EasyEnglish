⬆️ [Repository](../../../README.md)

# Skill — local-install

Use this skill whenever the user asks to build, run, launch locally, compile and
run, or otherwise start EasyEnglish on the local machine. Trigger phrases include
Chinese requests such as `编译运行`, `运行`, `本地启动`, `启动`, `打开应用`, and
English equivalents such as `run`, `launch`, `start locally`, or `compile and
run`.

The required workflow is not a debug-run shortcut. Always build the release
package for the current operating system, write the installer/package under the
repository `ee\Release\` directory, then silently install it for the user.

Launch and UI verification run on the local machine only when the user
explicitly asks to run, launch, test, or verify the app.

## Windows workflow

Windows is currently the only fully implemented desktop target.

### 1. Stop any running EasyEnglish process

Use PID-based process termination only.

```powershell
$running = Get-Process ee-win -ErrorAction SilentlyContinue
foreach ($process in $running) {
    Stop-Process -Id $process.Id -Force
}
```

### 2. Build the release installer

Run the existing packaging script from the repository root.

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
cd C:\r\EasyEnglish\ee
.\App\Win\win_package.bat
```

Expected output:

- release binaries are built for the current Windows target;
- ARM64 is included when the ARM64 toolchain and MSVC ARM64 build tools are
  available;
- installer output is written under `ee\Release\`.

The installer name includes the app version, for example:

```text
ee\Release\EasyEnglish-1.0.0-CN.exe
```

If the packaging script fails because Inno Setup is missing, install Inno Setup
6.3+ visibly before retrying:

```powershell
winget install JRSoftware.InnoSetup
```

### 3. Silently install the package

The Windows installer is built with Inno Setup, which supports silent install
switches. Install the generated package silently:

```powershell
$installer = Get-ChildItem "C:\r\EasyEnglish\ee\Release" -Filter "EasyEnglish-*.exe" |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1 -ExpandProperty FullName
Start-Process -FilePath $installer `
    -ArgumentList "/VERYSILENT /SUPPRESSMSGBOXES /NORESTART" `
    -Wait
```

Notes:

- The installer is per-user (`PrivilegesRequired=lowest`), so admin elevation is
  not expected for normal local install.
- The `startup` task is selected by default, matching the app's default-on
  launch-on-startup behavior.
- The installer launches EasyEnglish at the end of non-silent installs only; in
  silent mode, do not start it unless the user explicitly asks to run or verify
  the app.

### 4. Verify installation metadata on the host

Verify the package installed files and registry state on the host without
launching the application.

```powershell
$installed = Join-Path $env:LOCALAPPDATA "Programs\EasyEnglish\ee-win.exe"
Test-Path $installed
Get-Content (Join-Path (Split-Path $installed) "version")
reg query "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v EasyEnglish
reg query "HKCU\Software\EasyEnglish" /v LaunchOnStartup
```

### 5. Optional local launch or UI verification

Only perform this step when the user explicitly asks to run, launch, test, or
verify the app. Use the local Windows desktop.

## macOS and Linux workflow

The platform crates exist, but packaging/install workflows are not implemented
yet. If the current OS is macOS or Linux, do not invent a package format. Report
that the `local-install` skill is blocked until the corresponding platform
installer exists.

## Reporting

When this skill completes, report:

- package path under `ee\Release\`;
- whether silent install succeeded;
- installed executable path;
- launch-on-startup registry/preference state on Windows;
- local launch/UI verification result, only when it was explicitly requested.

If any step fails, report the exact failed command and the blocker.
