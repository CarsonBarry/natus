@echo off
setlocal
cd /d "%~dp0.."

set "BIN=%CD%\target\release\natus.exe"

if not exist "%BIN%" (
  echo.
  echo ==== No release build found; building first ====
  echo.
  cargo build --release
  if errorlevel 1 (
    echo.
    echo BUILD FAILED.
    echo.
    pause
    exit /b 1
  )
)

echo.
echo ==== Launching natus ====
echo.
"%BIN%"
set "EXITCODE=%ERRORLEVEL%"
if not "%EXITCODE%"=="0" (
  echo.
  echo natus exited with code %EXITCODE%.
  echo.
  pause
)
