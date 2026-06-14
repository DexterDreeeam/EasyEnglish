@echo off
rem Build all EasyEnglish Windows language installers.
setlocal enabledelayedexpansion
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
cd /d "%~dp0..\.."

set "ISCC=%LOCALAPPDATA%\Programs\Inno Setup 6\ISCC.exe"
if not exist "%ISCC%" set "ISCC=%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe"
if not exist "%ISCC%" set "ISCC=%ProgramFiles%\Inno Setup 6\ISCC.exe"
if not exist "%ISCC%" (
    echo [error] ISCC.exe not found. Install Inno Setup 6.3+ ^(winget install JRSoftware.InnoSetup^).
    exit /b 1
)

set /p VERSION_MARKER=<version
set "APP_VERSION=!VERSION_MARKER:EasyEnglish-=!"

echo Building x64 release...
cargo build -p ee-win --release --target x86_64-pc-windows-msvc || exit /b 1

set "ARCH_ARGS="
echo Building ARM64 release...
set "VCVARSALL=%ProgramFiles(x86)%\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat"
if exist "%VCVARSALL%" (
    call "%VCVARSALL%" x64_arm64 >nul
)
cargo build -p ee-win --release --target aarch64-pc-windows-msvc
if errorlevel 1 (
    echo [warn] ARM64 build failed - installers will use x64 under emulation on ARM64.
) else (
    set "ARCH_ARGS=/DARM64BUILD=1"
)

call :package CN "Mandarin Chinese" word_en_cn_v1 word_en_cn word_cn_v1 word_cn || exit /b 1
call :package HK "Traditional Chinese" word_en_hk_v1 word_en_hk word_hk_v1 word_hk || exit /b 1
call :package ES "Spanish" word_en_es_v1 word_en_es word_es_v1 word_es || exit /b 1
call :package JP "Japanese" word_en_ja_v1 word_en_ja word_ja_v1 word_ja || exit /b 1
call :package KR "Korean" word_en_ko_v1 word_en_ko word_ko_v1 word_ko || exit /b 1
call :package PT-BR "Portuguese (Brazil)" word_en_pt_v1 word_en_pt word_pt_v1 word_pt || exit /b 1
call :package ID "Indonesian" word_en_id_v1 word_en_id word_id_v1 word_id || exit /b 1
call :package AR "Arabic" word_en_ar_v1 word_en_ar word_ar_v1 word_ar || exit /b 1
call :package VI "Vietnamese" word_en_vi_v1 word_en_vi word_vi_v1 word_vi || exit /b 1
call :package HI "Hindi" word_en_hi_v1 word_en_hi word_hi_v1 word_hi || exit /b 1
call :package FR "French" word_en_fr_v1 word_en_fr word_fr_v1 word_fr || exit /b 1

echo Cleaning old installer versions...
for %%F in ("Release\EasyEnglish-*.exe") do (
    set "INSTALLER_NAME=%%~nxF"
    echo !INSTALLER_NAME! | findstr /B /I /C:"EasyEnglish-%APP_VERSION%-" >nul
    if errorlevel 1 (
        echo Removing old installer: %%~nxF
        del /F /Q "%%~fF"
    )
)

echo.
echo Done. Installers written to Release\
exit /b 0

:package
set "PKG_SUFFIX=%~1"
set "PKG_NAME=%~2"
set "EN_BASE=%~3"
set "EN_PREFIX=%~4"
set "TARGET_BASE=%~5"
set "TARGET_PREFIX=%~6"
echo Compiling %PKG_SUFFIX% installer...
"%ISCC%" %ARCH_ARGS% /DAppVersion="%APP_VERSION%" /DPackageLanguageSuffix="%PKG_SUFFIX%" /DPackageLanguageName="%PKG_NAME%" /DEnglishDictBase="%EN_BASE%" /DEnglishDictPrefix="%EN_PREFIX%" /DTargetDictBase="%TARGET_BASE%" /DTargetDictPrefix="%TARGET_PREFIX%" "App\Win\installer\easyenglish.iss" || exit /b 1
exit /b 0
