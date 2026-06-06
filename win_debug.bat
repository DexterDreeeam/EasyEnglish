@echo off
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
echo Building and running EasyEnglish (Debug)...
cargo run -p ee-win
