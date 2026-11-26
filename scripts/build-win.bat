@echo off
setlocal
cd /d "%~dp0.."

rem Make NSIS (makensis) discoverable if installed in the default location.
if exist "C:\Program Files (x86)\NSIS\makensis.exe" set "PATH=C:\Program Files (x86)\NSIS;%PATH%"

echo.
echo ==== Building natus (release) ====
echo.

cargo build --release
if errorlevel 1 (
  echo.
  echo BUILD FAILED.
  echo.
  pause
  exit /b 1
)

echo.
echo ==== Packaging installer (NSIS) ====
echo.

cargo packager --release --formats nsis
if errorlevel 1 (
  echo.
  echo PACKAGING FAILED.
  echo.
  pause
  exit /b 1
)

echo.
echo BUILD OK: "%CD%\target\release\natus.exe"
echo INSTALLER: "%CD%\target\release\natus_0.1.0_x64-setup.exe"
echo.
pause