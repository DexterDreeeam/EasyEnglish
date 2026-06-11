@echo off
rem Build the EasyEnglish Windows installer (single setup .exe for x64 + ARM64).
rem
rem Compiles the release binaries for each available architecture and runs the
rem Inno Setup compiler. The ARM64 binary is included only when its build
rem succeeds (it needs the "MSVC v143/v144 ARM64 build tools" Visual Studio
rem component, because the bundled SQLite is compiled from C for the target
rem architecture). Without it the installer ships the x64 binary, which also
rem runs on ARM64 Windows under emulation.
setlocal enabledelayedexpansion
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
cd /d "%~dp0..\.."

rem --- Locate the Inno Setup compiler (winget installs it per-user) ----------
set "ISCC=%LOCALAPPDATA%\Programs\Inno Setup 6\ISCC.exe"
if not exist "%ISCC%" set "ISCC=%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe"
if not exist "%ISCC%" set "ISCC=%ProgramFiles%\Inno Setup 6\ISCC.exe"
if not exist "%ISCC%" (
    echo [error] ISCC.exe not found. Install Inno Setup 6.3+ ^(winget install JRSoftware.InnoSetup^).
    exit /b 1
)

echo Building x64 release...
cargo build -p ee-win --release --target x86_64-pc-windows-msvc || exit /b 1

set "ISCC_ARGS="
echo Building ARM64 release...
cargo build -p ee-win --release --target aarch64-pc-windows-msvc
if errorlevel 1 (
    echo [warn] ARM64 build failed - producing an x64-only installer.
    echo [warn] To ship a native ARM64 binary, install the "VC++ ARM64 build tools"
    echo [warn] Visual Studio component, then re-run this script.
) else (
    set "ISCC_ARGS=/DARM64BUILD=1"
)

echo Compiling installer...
"%ISCC%" %ISCC_ARGS% "App\Win\installer\easyenglish.iss" || exit /b 1

echo.
echo Done. Installer written to Release\
