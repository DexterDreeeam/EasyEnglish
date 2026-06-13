@echo off
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
cd /d "%~dp0..\.."

echo Stopping any running EasyEnglish instances...
taskkill /F /IM ee-win.exe >nul 2>nul

echo Building EasyEnglish (Debug)...
cargo build -p ee-win || exit /b %ERRORLEVEL%

echo Starting EasyEnglish as a detached process...
start "" ".target\debug\ee-win.exe"
