@echo off
setlocal
if "%~1"=="" goto demo
set "first=%~1"
if "%first:~0,1%"=="-" goto options
"%~dp0ascii-3d-rust.exe" --model %*
if errorlevel 1 goto file_error
exit /b 0

:options
if /i "%~1"=="--model" if "%~2"=="" goto missing_model
"%~dp0ascii-3d-rust.exe" %*
exit /b %errorlevel%

:missing_model
echo Specify the OBJ file path after --model.
echo Example: Run --model models\cube.obj
exit /b 1

:demo
"%~dp0ascii-3d-rust.exe"
exit /b %errorlevel%

:file_error
echo Press any key to close...
pause >nul
exit /b 1
