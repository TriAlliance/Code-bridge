#!/bin/bash
#
# Code Bridge MCP Server Installation Script for macOS
# Installs the MCP server and configures it for Claude Code
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}"
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║           Code Bridge MCP Server Installer                    ║"
echo "║                    for Claude Code                            ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# Check if running on macOS
if [[ "$(uname)" != "Darwin" ]]; then
    echo -e "${RED}Error: This script is intended for macOS only.${NC}"
    exit 1
fi

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}Rust is not installed. Installing via rustup...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

echo -e "${GREEN}✓ Rust is installed${NC}"

# Determine installation directory
INSTALL_DIR="${HOME}/.code-bridge"
BIN_DIR="${INSTALL_DIR}/bin"
CONFIG_DIR="${HOME}/.config/code-bridge"
CLAUDE_CONFIG_DIR="${HOME}/.claude"

echo -e "${BLUE}Installation directory: ${INSTALL_DIR}${NC}"

# Create directories
mkdir -p "${BIN_DIR}"
mkdir -p "${CONFIG_DIR}"
mkdir -p "${CLAUDE_CONFIG_DIR}"

# Find the Code-bridge repository
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "${SCRIPT_DIR}")"

if [[ ! -f "${REPO_DIR}/Cargo.toml" ]]; then
    echo -e "${RED}Error: Could not find Code Bridge repository.${NC}"
    echo "Please run this script from the Code Bridge repository."
    exit 1
fi

echo -e "${BLUE}Building MCP server...${NC}"
cd "${REPO_DIR}"

# Build the MCP server in release mode
cargo build --release --package bridge-mcp

# Copy the binary
cp "${REPO_DIR}/target/release/bridge-mcp-server" "${BIN_DIR}/"
chmod +x "${BIN_DIR}/bridge-mcp-server"

echo -e "${GREEN}✓ MCP server built and installed${NC}"

# Create default configuration
if [[ ! -f "${CONFIG_DIR}/config.toml" ]]; then
    echo -e "${BLUE}Creating default configuration...${NC}"

    # Generate device ID
    DEVICE_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
    DEVICE_NAME=$(scutil --get ComputerName 2>/dev/null || hostname)

    cat > "${CONFIG_DIR}/config.toml" << EOF
# Code Bridge Configuration

[device]
id = "${DEVICE_ID}"
name = "${DEVICE_NAME}"

[network]
enable_mdns = true
enable_dht = true
enable_quic = true
enable_tcp = true
listen_port = 0

[storage]
data_dir = "${INSTALL_DIR}/data"
max_size_gb = 10

[sync]
auto_sync = true
sync_interval_secs = 30

[clipboard]
enabled = true
history_size = 1000
sync_images = true

[notifications]
enabled = true
show_build_status = true
show_test_results = true
show_pr_updates = true

[terminal]
enabled = true
record_commands = true
sync_history = true
EOF

    echo -e "${GREEN}✓ Configuration created${NC}"
fi

# Create Claude Code MCP configuration
echo -e "${BLUE}Configuring Claude Code MCP server...${NC}"

# Check if mcp_servers.json exists
MCP_CONFIG="${CLAUDE_CONFIG_DIR}/mcp_servers.json"

if [[ -f "${MCP_CONFIG}" ]]; then
    # Backup existing config
    cp "${MCP_CONFIG}" "${MCP_CONFIG}.backup"

    # Add code-bridge to existing config using Python (available on macOS)
    python3 << EOF
import json

config_path = "${MCP_CONFIG}"

try:
    with open(config_path, 'r') as f:
        config = json.load(f)
except:
    config = {"mcpServers": {}}

if "mcpServers" not in config:
    config["mcpServers"] = {}

config["mcpServers"]["code-bridge"] = {
    "command": "${BIN_DIR}/bridge-mcp-server",
    "args": ["--stdio"],
    "env": {
        "CODE_BRIDGE_CONFIG": "${CONFIG_DIR}/config.toml",
        "RUST_LOG": "info"
    }
}

with open(config_path, 'w') as f:
    json.dump(config, f, indent=2)

print("Updated existing Claude Code configuration")
EOF
else
    # Create new config
    cat > "${MCP_CONFIG}" << EOF
{
  "mcpServers": {
    "code-bridge": {
      "command": "${BIN_DIR}/bridge-mcp-server",
      "args": ["--stdio"],
      "env": {
        "CODE_BRIDGE_CONFIG": "${CONFIG_DIR}/config.toml",
        "RUST_LOG": "info"
      }
    }
  }
}
EOF
fi

echo -e "${GREEN}✓ Claude Code MCP configuration created${NC}"

# Create data directories
mkdir -p "${INSTALL_DIR}/data/clipboard"
mkdir -p "${INSTALL_DIR}/data/terminal"
mkdir -p "${INSTALL_DIR}/data/screenshots"
mkdir -p "${INSTALL_DIR}/data/files"

echo -e "${GREEN}✓ Data directories created${NC}"

# Create uninstall script
cat > "${INSTALL_DIR}/uninstall.sh" << 'EOF'
#!/bin/bash
echo "Uninstalling Code Bridge MCP Server..."

# Remove binary
rm -f "${HOME}/.code-bridge/bin/bridge-mcp-server"

# Remove from Claude Code config
if [[ -f "${HOME}/.claude/mcp_servers.json" ]]; then
    python3 << 'PYTHON'
import json
config_path = "${HOME}/.claude/mcp_servers.json"
try:
    with open(config_path, 'r') as f:
        config = json.load(f)
    if "mcpServers" in config and "code-bridge" in config["mcpServers"]:
        del config["mcpServers"]["code-bridge"]
        with open(config_path, 'w') as f:
            json.dump(config, f, indent=2)
        print("Removed code-bridge from Claude Code configuration")
except Exception as e:
    print(f"Note: Could not update Claude Code config: {e}")
PYTHON
fi

echo "Code Bridge MCP Server uninstalled."
echo "Configuration and data preserved in ~/.code-bridge"
echo "To fully remove, run: rm -rf ~/.code-bridge ~/.config/code-bridge"
EOF
chmod +x "${INSTALL_DIR}/uninstall.sh"

echo ""
echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║         Installation Complete!                               ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${BLUE}MCP Server:${NC} ${BIN_DIR}/bridge-mcp-server"
echo -e "${BLUE}Configuration:${NC} ${CONFIG_DIR}/config.toml"
echo -e "${BLUE}Claude Config:${NC} ${MCP_CONFIG}"
echo ""
echo -e "${YELLOW}Available MCP Tools:${NC}"
echo "  Core:      bridge_status, bridge_sync, bridge_share, bridge_peers"
echo "  Files:     bridge_files, bridge_screenshots"
echo "  Clipboard: clipboard_get, clipboard_set, clipboard_history, clipboard_favorites"
echo "  Notify:    notifications_list, notifications_send, notifications_mark_read"
echo "  Terminal:  terminal_history, terminal_recordings, terminal_environment"
echo ""
echo -e "${YELLOW}Next Steps:${NC}"
echo "  1. Restart Claude Code to load the new MCP server"
echo "  2. Try: 'Use the bridge_status tool to check Code Bridge'"
echo ""
echo -e "${BLUE}To uninstall:${NC} ${INSTALL_DIR}/uninstall.sh"
