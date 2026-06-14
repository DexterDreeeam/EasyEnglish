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

if not defined PACKAGE_LANGUAGE_SUFFIX set "PACKAGE_LANGUAGE_SUFFIX=CN"
if not defined PACKAGE_LANGUAGE_NAME set "PACKAGE_LANGUAGE_NAME=Mandarin Chinese"
if not defined ENGLISH_DICT_BASE set "ENGLISH_DICT_BASE=word_en_cn_v1"
if not defined ENGLISH_DICT_PREFIX set "ENGLISH_DICT_PREFIX=word_en_cn"
if not defined TARGET_DICT_BASE set "TARGET_DICT_BASE=word_cn_v1"
if not defined TARGET_DICT_PREFIX set "TARGET_DICT_PREFIX=word_cn"
if not defined APP_VERSION (
    set /p VERSION_MARKER=<version
    set "APP_VERSION=!VERSION_MARKER:EasyEnglish-=!"
)

set "ISCC_ARGS=/DAppVersion="%APP_VERSION%" /DPackageLanguageSuffix="%PACKAGE_LANGUAGE_SUFFIX%" /DPackageLanguageName="%PACKAGE_LANGUAGE_NAME%" /DEnglishDictBase="%ENGLISH_DICT_BASE%" /DEnglishDictPrefix="%ENGLISH_DICT_PREFIX%" /DTargetDictBase="%TARGET_DICT_BASE%" /DTargetDictPrefix="%TARGET_DICT_PREFIX%""
echo Building ARM64 release...
set "VCVARSALL=%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
if exist "%VCVARSALL%" (
    call "%VCVARSALL%" x64_arm64 >nul
)
cargo build -p ee-win --release --target aarch64-pc-windows-msvc
if errorlevel 1 (
    echo [warn] ARM64 build failed - producing an x64-only installer.
    echo [warn] To ship a native ARM64 binary, install the "VC++ ARM64 build tools"
    echo [warn] Visual Studio component, then re-run this script.
) else (
    set "ISCC_ARGS=%ISCC_ARGS% /DARM64BUILD=1"
)

echo Compiling installer...
"%ISCC%" %ISCC_ARGS% "App\Win\installer\easyenglish.iss" || exit /b 1

echo.
echo Done. Installer written to Release\
