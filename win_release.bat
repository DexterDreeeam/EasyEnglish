@echo off
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
cd /d "%~dp0"

echo Stopping any running EasyEnglish instances...
taskkill /F /IM ee-win.exe >nul 2>nul

echo Building EasyEnglish (Release)...
cargo build -p ee-win --release || exit /b %ERRORLEVEL%

echo Starting EasyEnglish as a detached process...
start "" "%~dp0.target\release\ee-win.exe"
