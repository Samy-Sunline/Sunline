@echo off
setlocal
title Sunline - build the launcher
set "PATH=%PATH%;%USERPROFILE%\.cargo\bin"
pushd "%~dp0"
cargo build --release --locked || (echo Build failed& popd& pause& exit /b 1)
copy /y "target\release\sunline.exe" "..\Sunline.exe" >nul || (echo Close Sunline first& popd& pause& exit /b 1)
echo === Done: Sunline.exe ===
popd
pause
