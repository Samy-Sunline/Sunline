@echo off
setlocal
chcp 65001 >nul
title Sunline - build VIctory from source (optional)
rem Only for people who want to build VIctory themselves. The ready download already has it.
set "ROOT=%~dp0.."
set "REPO=%ROOT%\victory-src"
set "OUT=%ROOT%\victory"
set "LOGS=%ROOT%\logs"
set "PATCH=%ROOT%\patches\newbase-nvidia.patch"
set "PATH=%PATH%;%USERPROFILE%\.cargo\bin;C:\Program Files\Git\cmd"
if not exist "%LOGS%" mkdir "%LOGS%"

where git >nul 2>&1 || (echo Git is missing:   winget install Git.Git& pause& exit /b 1)
where cargo >nul 2>&1 || (echo Rust is missing:  winget install Rustlang.Rustup   and Visual Studio Build Tools with C++& pause& exit /b 1)

if not exist "%REPO%\.git" (
  echo === Downloading VIctory-7900XTX ===
  git clone https://github.com/avatarmanz/VIctory-7900XTX "%REPO%" || (echo Clone failed& pause& exit /b 1)
)
pushd "%REPO%"
git checkout -q 014d52dedf837d1ac660a0bd256c4704be9eeb40 || (echo Checkout failed& popd& pause& exit /b 1)
git apply --reverse --check "%PATCH%" >nul 2>&1 || git apply "%PATCH%" || (echo Patch failed& popd& pause& exit /b 1)
echo === Building - about 5-15 minutes ===
cargo build --locked --profile diag -p victory --features diagnostics > "%LOGS%\victory-build.log" 2>&1 || (echo Build failed - see logs\victory-build.log& popd& pause& exit /b 1)
if not exist "%OUT%" mkdir "%OUT%"
copy /y "target\diag\victory.exe" "%OUT%\victory.exe" >nul || (echo Close Sunline and the game, then run this again& popd& pause& exit /b 1)
popd
echo === Done. Open Sunline and press PLAY (it fetches the D3D12 runtime by itself) ===
pause
