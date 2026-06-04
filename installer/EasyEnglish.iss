; EasyEnglish — Inno Setup script
; Build via tools/build_installer.ps1 (requires Inno Setup 6 + a Release build).
; Output: installer/dist/EasyEnglishSetup-<version>.exe

#define MyAppName "EasyEnglish"
#define MyAppVersion "0.2.0"
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

[Files]
; The "staging" directory is produced by tools/build_installer.ps1 — it contains
; EasyEnglish.exe, Qt DLLs (via windeployqt), and mini_dict.sqlite.
Source: "staging\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
