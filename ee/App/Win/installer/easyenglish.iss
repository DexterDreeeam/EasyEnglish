; EasyEnglish — Windows installer (Inno Setup).
;
; Produces a single per-user setup .exe that installs the correct native binary
; on both AMD64 (x86_64) and ARM64 machines. A single installer is possible
; because the script bundles both architecture builds and selects the matching
; `ee-win.exe` at install time via the `IsArm64` check.
;
; Build it through `App\Win\win_package.bat`, which compiles the release
; binaries and invokes ISCC. When the ARM64 release is available the build
; script passes `/DARM64BUILD=1`, which switches in the native ARM64 binary;
; without it the installer ships the x64 binary for every architecture (it runs
; under emulation on ARM64).
;
; Requires Inno Setup 6.3 or newer (native ARM64 support / `IsArm64`).
;
; Silent local install:
;   Release\EasyEnglish-{version}.exe /VERYSILENT /SUPPRESSMSGBOXES /NORESTART

#define AppName "EasyEnglish"
#define AppVersion "1.0.0-alpha.2"
#define AppPublisher "EasyEnglish"
#define AppExeName "ee-win.exe"
#define AppId "{{B7F4C2E1-9A3D-4E58-9C1F-EE0A11C0FFEE}"

; SourcePath is the directory containing this .iss (App\Win\installer\), with a
; trailing backslash. The repository root is three levels up.
#define RepoRoot SourcePath + "..\..\.."
#define TargetDir RepoRoot + "\.target"
#define X64Exe TargetDir + "\x86_64-pc-windows-msvc\release\" + AppExeName
#define Arm64Exe TargetDir + "\aarch64-pc-windows-msvc\release\" + AppExeName
#define DictDir RepoRoot + "\Dict"

[Setup]
AppId={#AppId}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\{#AppExeName}
SetupIconFile={#SourcePath}easyenglish.ico
OutputDir={#RepoRoot}\Release
OutputBaseFilename=EasyEnglish-{#AppVersion}
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
; Per-user install: no administrator rights required, which keeps the tray
; daemon's autostart entry in HKCU and lets the same package install on locked
; down machines.
PrivilegesRequired=lowest
; Allow installing on x64 and on ARM64 (natively, or x64-under-emulation), and
; place files under the 64-bit Program Files on both.
ArchitecturesAllowed=x64compatible arm64
ArchitecturesInstallIn64BitMode=x64compatible arm64

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "startup"; Description: "Start {#AppName} automatically when I sign in"
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
; --- Native executable, one per architecture ----------------------------------
; On a machine where the ARM64 binary is bundled it is installed only on ARM64;
; the x64 binary covers every other (x64-compatible) machine. Without an ARM64
; build the x64 binary is installed everywhere.
Source: "{#X64Exe}"; DestDir: "{app}"; DestName: "{#AppExeName}"; Check: InstallX64Binary; Flags: ignoreversion
#ifdef ARM64BUILD
Source: "{#Arm64Exe}"; DestDir: "{app}"; DestName: "{#AppExeName}"; Check: IsArm64; Flags: ignoreversion
#endif

; --- Architecture-neutral runtime data ----------------------------------------
; `ee-win` discovers the highest-version dictionary by walking up from the
; executable to find a `Dict\` folder, so the data lives in {app}\Dict.
Source: "{#DictDir}\word_en_v1"; DestDir: "{app}\Dict"; Flags: ignoreversion
Source: "{#DictDir}\word_en_v1.sqlite"; DestDir: "{app}\Dict"; Flags: ignoreversion
Source: "{#DictDir}\word_cn_v1"; DestDir: "{app}\Dict"; Flags: ignoreversion
Source: "{#DictDir}\word_cn_v1.sqlite"; DestDir: "{app}\Dict"; Flags: ignoreversion
Source: "{#DictDir}\ECDICT-LICENSE.txt"; DestDir: "{app}\Dict"; Flags: ignoreversion

; Icon used by the Start Menu / desktop shortcuts.
Source: "{#SourcePath}easyenglish.ico"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\easyenglish.ico"
Name: "{group}\Uninstall {#AppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\easyenglish.ico"; Tasks: desktopicon

[Registry]
; Autostart the tray daemon for the current user.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "{#AppName}"; ValueData: """{app}\{#AppExeName}"""; Tasks: startup; Flags: uninsdeletevalue

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch {#AppName}"; WorkingDir: "{app}"; Flags: nowait postinstall skipifsilent

[Code]
{ Decide whether the x64 binary should be installed on this machine.
  When a native ARM64 build is bundled, the x64 binary is used only on
  non-ARM64 (i.e. real x64) machines; otherwise it is installed everywhere. }
function InstallX64Binary: Boolean;
begin
#ifdef ARM64BUILD
  Result := not IsArm64;
#else
  Result := True;
#endif
end;
