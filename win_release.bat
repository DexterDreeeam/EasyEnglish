@echo off
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
echo Building and running EasyEnglish (Release)...
cargo run -p ee-win --release
