@echo off
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

echo Stopping any running EasyEnglish instances...
taskkill /F /IM ee-win.exe >nul 2>nul

echo Building and running EasyEnglish (Release)...
cargo run -p ee-win --release
