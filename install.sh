#!/bin/bash
# GreedyClaw Installer for macOS/Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/GreedyClaw/GreedyClaw/main/install.sh | bash

set -e

VERSION="0.1.0"
REPO="GreedyClaw/GreedyClaw"
INSTALL_DIR="$HOME/.greedyclaw/src"
BIN_DIR="$HOME/.greedyclaw/bin"

GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
GRAY='\033[0;90m'
NC='\033[0m'

echo ""
echo -e "  ${GREEN}GreedyClaw Installer v${VERSION}${NC}"
echo -e "  ${GRAY}AI-Native Trading Execution Gateway${NC}"
echo ""

# ── Step 1: Check prerequisites ────────────────────────────────────

echo -e "${CYAN}[1/5] Checking prerequisites...${NC}"

# Check for Rust
if command -v cargo &>/dev/null; then
    RUST_VER=$(cargo --version | sed 's/cargo //')
    echo -e "  ${GREEN}Rust: $RUST_VER${NC}"
else
    echo -e "  ${YELLOW}Rust not found. Installing via rustup...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "  ${GREEN}Rust installed!${NC}"
fi

# Check for Python
if command -v python3 &>/dev/null; then
    PY_VER=$(python3 --version 2>&1 | sed 's/Python //')
    echo -e "  ${GREEN}Python: $PY_VER (CCXT bridge available)${NC}"
    HAS_PYTHON=1
else
    echo -e "  ${YELLOW}Python: not found (optional — needed for CCXT bridge)${NC}"
    HAS_PYTHON=0
fi

# Check for protoc
if command -v protoc &>/dev/null; then
    echo -e "  ${GREEN}protoc: found${NC}"
else
    echo -e "  ${YELLOW}protoc not found. Installing...${NC}"
    if [[ "$(uname)" == "Darwin" ]]; then
        brew install protobuf 2>/dev/null || {
            echo -e "  ${YELLOW}Install protobuf: brew install protobuf${NC}"
        }
    else
        # Linux
        PROTOC_URL="https://github.com/protocolbuffers/protobuf/releases/download/v28.3/protoc-28.3-linux-x86_64.zip"
        PROTOC_DIR="$HOME/.local"
        curl -fsSL "$PROTOC_URL" -o /tmp/protoc.zip
        unzip -o /tmp/protoc.zip -d "$PROTOC_DIR" bin/protoc
        chmod +x "$PROTOC_DIR/bin/protoc"
        export PATH="$PROTOC_DIR/bin:$PATH"
        echo -e "  ${GREEN}protoc installed${NC}"
    fi
fi

# ── Step 2: Clone/update repo ──────────────────────────────────────

echo ""
echo -e "${CYAN}[2/5] Getting GreedyClaw source...${NC}"

if [ -d "$INSTALL_DIR/.git" ]; then
    echo "  Updating existing installation..."
    cd "$INSTALL_DIR" && git pull --ff-only
else
    echo "  Cloning from GitHub..."
    mkdir -p "$(dirname "$INSTALL_DIR")"
    git clone "https://github.com/$REPO.git" "$INSTALL_DIR"
fi

# ── Step 3: Build ──────────────────────────────────────────────────

echo ""
echo -e "${CYAN}[3/5] Building GreedyClaw (release mode)...${NC}"

cd "$INSTALL_DIR"
cargo build --release 2>&1

BINARY="$INSTALL_DIR/target/release/greedyclaw"
if [ ! -f "$BINARY" ]; then
    echo -e "  ${RED}Build failed!${NC}"
    exit 1
fi
echo -e "  ${GREEN}Built: $BINARY${NC}"

# ── Step 4: Install to PATH ────────────────────────────────────────

echo ""
echo -e "${CYAN}[4/5] Installing to PATH...${NC}"

mkdir -p "$BIN_DIR"
cp "$BINARY" "$BIN_DIR/greedyclaw"
chmod +x "$BIN_DIR/greedyclaw"

# Add to PATH in shell rc
SHELL_RC="$HOME/.bashrc"
if [ -n "$ZSH_VERSION" ] || [ -f "$HOME/.zshrc" ]; then
    SHELL_RC="$HOME/.zshrc"
fi

if ! grep -q "greedyclaw/bin" "$SHELL_RC" 2>/dev/null; then
    echo 'export PATH="$HOME/.greedyclaw/bin:$PATH"' >> "$SHELL_RC"
    export PATH="$BIN_DIR:$PATH"
    echo -e "  ${GREEN}Added to PATH in $SHELL_RC${NC}"
fi

# ── Step 5: Initialize config ──────────────────────────────────────

echo ""
echo -e "${CYAN}[5/5] Initializing configuration...${NC}"

"$BIN_DIR/greedyclaw" init

# ── Install Python bridges (optional) ──────────────────────────────

if [ "$HAS_PYTHON" = "1" ]; then
    echo ""
    echo -e "${CYAN}Installing Python bridge dependencies...${NC}"
    python3 -m pip install -r "$INSTALL_DIR/mt5-bridge/requirements.txt" --quiet 2>/dev/null || true
    python3 -m pip install ccxt --quiet 2>/dev/null || true
    echo -e "  ${GREEN}Python bridges ready!${NC}"
fi

# ── Done ────────────────────────────────────────────────────────────

echo ""
echo -e "  ${GREEN}GreedyClaw installed successfully!${NC}"
echo ""
echo "  Quick start:"
echo -e "    ${GRAY}1. Edit ~/.greedyclaw/.env — set your API keys${NC}"
echo -e "    ${GRAY}2. Edit ~/.greedyclaw/config.toml — choose exchange${NC}"
echo -e "    ${GRAY}3. greedyclaw serve${NC}"
echo ""
echo "  Supported exchanges:"
echo -e "    ${GRAY}binance, pumpfun, pumpswap, mt5${NC}"
echo -e "    ${GRAY}+ 100 more via CCXT: bybit, okx, kraken, coinbase...${NC}"
echo ""
echo -e "  ${YELLOW}Dashboard: http://127.0.0.1:7878/dashboard${NC}"
echo ""
