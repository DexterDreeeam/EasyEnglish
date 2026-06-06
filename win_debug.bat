@echo off
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

echo Stopping any running EasyEnglish instances...
taskkill /F /IM ee-win.exe >nul 2>nul

echo Ensuring font assets exist...
if not exist "App\Assets\segoeui.ttf" (
    echo Copying segoeui.ttf from Windows Fonts...
    copy "C:\Windows\Fonts\segoeui.ttf" "App\Assets\segoeui.ttf" >nul
)
if not exist "App\Assets\msyh.ttc" (
    echo Copying msyh.ttc from Windows Fonts...
    copy "C:\Windows\Fonts\msyh.ttc" "App\Assets\msyh.ttc" >nul
)

echo Building and running EasyEnglish (Debug)...
cargo run -p ee-win
