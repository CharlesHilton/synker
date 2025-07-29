@echo off
REM Synker Server Build and Deployment Script for Windows

echo Building Synker Server...

REM Build the project
cargo build --release

if %ERRORLEVEL% neq 0 (
    echo Build failed!
    exit /b 1
)

echo Build completed successfully!

REM Create necessary directories
if not exist storage mkdir storage
if not exist temp mkdir temp

echo Setting up database...
REM Initialize database
target\release\synker-server.exe --init-db

echo Setup complete!
echo.
echo To start the server:
echo   target\release\synker-server.exe
echo.
echo To create an admin user:
echo   target\release\synker-server.exe --create-admin
echo.
echo Remember to edit config.toml with your MyCloud settings before starting!
