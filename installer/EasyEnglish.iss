; EasyEnglish — Inno Setup script
; Build via tools/build_installer.ps1 (requires Inno Setup 6 + a Release build).
; Output: installer/dist/EasyEnglishSetup-<version>.exe

#define MyAppName "EasyEnglish"
#define MyAppVersion "0.3.0"
#define MyAppPublisher "EasyEnglish Contributors"
#define MyAppExeName "EasyEnglish.exe"

[Setup]
AppId={{0AC9D5E2-1F1A-4F4E-8E0A-EE7C9D2A4FE2}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=dist
OutputBaseFilename=EasyEnglishSetup-{#MyAppVersion}
Compression=lzma
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64
WizardStyle=modern
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "autostart"; Description: "Start EasyEnglish when Windows starts"; GroupDescription: "Startup:"; Flags: checkedonce

[Files]
; The "staging" directory is produced by tools/build_installer.ps1 — it contains
; EasyEnglish.exe, runtime DLLs, mini_dict.sqlite, and fonts/.
Source: "staging\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
; Optional autostart entry; the value disappears with the uninstall.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "EasyEnglish"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
