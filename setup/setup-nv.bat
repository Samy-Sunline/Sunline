@echo off
setlocal
chcp 65001 >nul
title Sunline - set up VIctory-NVIDIA
rem Builds VIctory-NVIDIA (DLSS, frame generation) with our fixes beside Sunline: Git and Rust are needed.
set "ROOT=%~dp0.."
set "REPO=%ROOT%\VIctory-NVIDIA"
rem Sunline passes the folder set in its settings
if not "%~1"=="" set "REPO=%~1"
set "COMMIT=63b902fe1720a5188ee79d94a88c8de0470f6349"
set "PATCH=%ROOT%\patches\nv-ours.patch"
set "LOGS=%ROOT%\logs"
set "PATH=%PATH%;%USERPROFILE%\.cargo\bin;C:\Program Files\Git\cmd"
if not exist "%LOGS%" mkdir "%LOGS%"
where git >nul 2>&1 || (echo Git is missing:   winget install Git.Git& pause& exit /b 1)
where cargo >nul 2>&1 || (echo Rust is missing:  winget install Rustlang.Rustup   and Visual Studio Build Tools with C++& pause& exit /b 1)
if not exist "%REPO%\.git" (
  git clone --no-checkout https://github.com/trustno1-coder/VIctory-NVIDIA.git "%REPO%" || (echo Clone failed. Choose an empty folder.& pause& exit /b 1)
  git -C "%REPO%" checkout -q --detach %COMMIT% || (echo Checkout failed& pause& exit /b 1)
)
pushd "%REPO%" || (echo Could not open the VIctory folder& pause& exit /b 1)
git rev-parse HEAD | findstr /x /i "%COMMIT%" >nul || (echo This folder has a different VIctory revision. Choose an empty folder to build the pinned version.& popd& pause& exit /b 1)
git apply --reverse --check "%PATCH%" >nul 2>&1 || (
  git apply --check "%PATCH%" && git apply "%PATCH%"
  if errorlevel 1 (echo Patch failed. Check the working tree or choose an empty folder.& popd& pause& exit /b 1)
)
echo === Building - about 10-20 minutes the first time ===
powershell -NoProfile -ExecutionPolicy Bypass -File "%ROOT%\setup\build-progress.ps1" -Repo "%REPO%" -Log "%LOGS%\nv-build.log" || (echo Build failed - see logs\nv-build.log& popd& pause& exit /b 1)
echo %COMMIT% p2> sunline-build.txt
popd
echo === Done. Press PLAY in Sunline ===
pause
