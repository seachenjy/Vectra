#!/usr/bin/env bash
set -e

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

cleanup() {
    echo ""
    echo -e "${YELLOW}Shutting down...${NC}"
    if [ -n "$BACKEND_PID" ]; then
        kill "$BACKEND_PID" 2>/dev/null || true
        echo -e "  Backend stopped (PID: $BACKEND_PID)"
    fi
    if [ -n "$FRONTEND_PID" ]; then
        kill "$FRONTEND_PID" 2>/dev/null || true
        echo -e "  Frontend stopped (PID: $FRONTEND_PID)"
    fi
    echo -e "${GREEN}SkyMemory stopped.${NC}"
    exit 0
}

trap cleanup SIGINT SIGTERM EXIT

echo ""
echo -e "${CYAN} ============================================${NC}"
echo -e "${CYAN}  SkyMemory - Cognitive Memory Engine${NC}"
echo -e "${CYAN}  One-click Launcher${NC}"
echo -e "${CYAN} ============================================${NC}"
echo ""

if ! command -v cargo &>/dev/null; then
    echo -e "${RED} [ERROR] cargo not found. Install Rust: https://rustup.rs${NC}"
    exit 1
fi

if ! command -v node &>/dev/null; then
    echo -e "${RED} [ERROR] node not found. Install Node.js: https://nodejs.org${NC}"
    exit 1
fi

if ! command -v npm &>/dev/null; then
    echo -e "${RED} [ERROR] npm not found. Install Node.js: https://nodejs.org${NC}"
    exit 1
fi

echo -e " ${GREEN}[1/3]${NC} Building backend (release)..."
cargo build --release
echo -e " ${GREEN}[1/3]${NC} Backend built successfully."
echo ""

if [ ! -d "$ROOT/admin/node_modules" ]; then
    echo -e " ${GREEN}[2/3]${NC} Installing frontend dependencies..."
    cd "$ROOT/admin"
    npm install
    echo -e " ${GREEN}[2/3]${NC} Frontend dependencies installed."
else
    echo -e " ${GREEN}[2/3]${NC} Frontend dependencies already installed."
fi
cd "$ROOT"
echo ""

echo -e " ${GREEN}[3/3]${NC} Starting services..."
echo ""
echo -e "  ${CYAN}Backend API:${NC}   http://127.0.0.1:8080"
echo -e "  ${CYAN}Admin Panel:${NC}   http://localhost:3000"
echo ""
echo -e "  Press ${YELLOW}Ctrl+C${NC} to stop all services."
echo ""

cargo run --release -- serve --addr 127.0.0.1:8080 &
BACKEND_PID=$!

sleep 2

cd "$ROOT/admin"
npm run dev &
FRONTEND_PID=$!
cd "$ROOT"

wait
