@echo off
setlocal EnableDelayedExpansion

echo.
echo  ============================================
echo   SkyMemory - Cognitive Memory Engine
echo   One-click Launcher
echo  ============================================
echo.

set "ROOT=%~dp0"
cd /d "%ROOT%"

:: ── Check prerequisites ─────────────────────────────────
where cargo >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo  [ERROR] cargo not found. Please install Rust: https://rustup.rs
    pause
    exit /b 1
)

where node >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo  [ERROR] node not found. Please install Node.js: https://nodejs.org
    pause
    exit /b 1
)

where npm >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo  [ERROR] npm not found. Please install Node.js: https://nodejs.org
    pause
    exit /b 1
)

:: ── Build backend ───────────────────────────────────────
echo  [1/3] Building backend ^(release^)...
cargo build --release
if %ERRORLEVEL% neq 0 (
    echo  [ERROR] Backend build failed.
    pause
    exit /b 1
)
echo  [1/3] Backend built successfully.
echo.

:: ── Install frontend dependencies ──────────────────────
if not exist "%ROOT%admin\node_modules" (
    echo  [2/3] Installing frontend dependencies...
    cd /d "%ROOT%admin"
    call npm install
    if %ERRORLEVEL% neq 0 (
        echo  [ERROR] npm install failed.
        pause
        exit /b 1
    )
    echo  [2/3] Frontend dependencies installed.
) else (
    echo  [2/3] Frontend dependencies already installed.
)
echo.

:: ── Start services ─────────────────────────────────────
echo  [3/3] Starting services...
echo.
echo  Backend API:   http://127.0.0.1:8080
echo  Admin Panel:   http://localhost:3000
echo.
echo  Press Ctrl+C in each window to stop.
echo.

:: Write temp launchers to avoid quoting issues
(
    echo @echo off
    echo cd /d "%ROOT%"
    echo echo SkyMemory Backend running on http://127.0.0.1:8080
    echo echo.
    echo cargo run --release -- serve --addr 127.0.0.1:8080
    echo pause
) > "%TEMP%\skymem_backend.bat"

(
    echo @echo off
    echo cd /d "%ROOT%admin"
    echo echo SkyMemory Admin Panel running on http://localhost:3000
    echo echo.
    echo npm run dev
    echo pause
) > "%TEMP%\skymem_frontend.bat"

:: Start backend in a new window
start "SkyMemory Backend" "%TEMP%\skymem_backend.bat"

:: Wait for backend to initialize
timeout /t 2 /nobreak >nul

:: Start frontend in a new window
start "SkyMemory Frontend" "%TEMP%\skymem_frontend.bat"

echo  Both services started in separate windows.
echo  Close those windows or press Ctrl+C in each to stop.
echo.
pause
