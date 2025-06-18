#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# --- Configuration ---
INSTALL_DIR="/opt/node-nexus"
SERVICE_NAME="node-nexus-agent"
SERVICE_USER="root" # Default user, can be changed with --secure-user
CONFIG_FILE_PATH="$INSTALL_DIR/config.toml"
SERVICE_FILE_PATH="/etc/systemd/system/$SERVICE_NAME.service"
GITHUB_REPO="moonheart/NodeNexus"
AGENT_BINARY_NAME="" # This will be set dynamically

# --- Helper Functions ---
print_info() {
    echo -e "\e[34m[INFO]\e[0m $1" >&2
}

print_success() {
    echo -e "\e[32m[SUCCESS]\e[0m $1" >&2
}

print_error() {
    echo -e "\e[31m[ERROR]\e[0m $1" >&2
    exit 1
}

check_root() {
    if [ "$(id -u)" -ne 0 ]; then
        print_error "This script must be run as root. Please use sudo."
    fi
}

check_dependencies() {
    print_info "Checking for dependencies..."
    local missing_deps=()
    for cmd in curl jq; do
        if ! command -v "$cmd" &> /dev/null; then
            missing_deps+=("$cmd")
        fi
    done

    if [ ${#missing_deps[@]} -gt 0 ]; then
        print_info "Missing dependencies: ${missing_deps[*]}. Attempting to install..."
        if command -v apt-get &> /dev/null; then
            apt-get update
            apt-get install -y "${missing_deps[@]}"
        elif command -v yum &> /dev/null; then
            yum install -y "${missing_deps[@]}"
        elif command -v dnf &> /dev/null; then
            dnf install -y "${missing_deps[@]}"
        else
            print_error "Could not find a supported package manager (apt, yum, dnf). Please install dependencies manually: ${missing_deps[*]}"
        fi
    else
        print_info "All dependencies are satisfied."
    fi
}

detect_arch() {
    local arch
    arch=$(uname -m)
    case $arch in
        x86_64)
            AGENT_BINARY_NAME="agent-linux-amd64"
            ;;
        aarch64)
            AGENT_BINARY_NAME="agent-linux-arm64"
            ;;
        *)
            print_error "Unsupported architecture: $arch. Only x86_64 and aarch64 are supported."
            ;;
    esac
    print_info "Detected architecture: $arch. Using binary: $AGENT_BINARY_NAME"
}

get_latest_release_url() {
    print_info "Fetching latest release from GitHub repository: $GITHUB_REPO"
    local api_url="https://api.github.com/repos/$GITHUB_REPO/releases/latest"
    
    local response
    response=$(curl -s "$api_url")
    
    if echo "$response" | jq -e '.assets' &> /dev/null; then
        local download_url
        download_url=$(echo "$response" | jq -r ".assets[] | select(.name == \"$AGENT_BINARY_NAME\") | .browser_download_url")
        
        if [ -z "$download_url" ]; then
            print_error "Could not find an asset named '$AGENT_BINARY_NAME' in the latest release."
        else
            echo "$download_url"
        fi
    else
        print_error "Failed to fetch release information. Check repository name and network connection."
    fi
}

show_usage() {
    echo "Usage: $0 <command> [options]"
    echo
    echo "Commands:"
    echo "  install                   Install or update the agent. This is the default command."
    echo "  uninstall                 Uninstall the agent."
    echo
    echo "Options for 'install' command:"
    echo "  -s, --server-address <url>  The address of the server (e.g., http://your-server.com:8080)."
    echo "  -i, --vps-id <id>           The ID of the VPS."
    echo "  -k, --agent-secret <secret> The secret key for the agent."
    echo "  -d, --download-url <url>    Optional. Direct URL to the agent binary. Overrides GitHub release check."
    echo "      --secure-user           Create a dedicated user 'node-nexus' to run the service for enhanced security."
    echo "  -h, --help                  Show this help message."
}

# --- Main Installation Steps ---
setup_secure_user() {
    if id "node-nexus" &>/dev/null; then
        print_info "User 'node-nexus' already exists."
    else
        print_info "Creating dedicated user 'node-nexus'..."
        useradd --system --no-create-home --shell /bin/false node-nexus
    fi
    print_info "Setting ownership of $INSTALL_DIR to 'node-nexus' user..."
    chown -R node-nexus:node-nexus "$INSTALL_DIR"
    SERVICE_USER="node-nexus"
}

setup_environment() {
    print_info "Setting up installation directory: $INSTALL_DIR"
    mkdir -p "$INSTALL_DIR"
}

create_config_file() {
    local server_address=$1
    local vps_id=$2
    local agent_secret=$3

    if [ -f "$CONFIG_FILE_PATH" ]; then
        print_info "Configuration file already exists. Skipping creation."
        return
    fi
    
    print_info "Creating configuration file..."

    if [ -z "$server_address" ]; then
        read -p "Enter the server address (e.g., http://your-server.com:8080): " server_address
    fi
    if [ -z "$vps_id" ]; then
        read -p "Enter the VPS ID: " vps_id
    fi
    if [ -z "$agent_secret" ]; then
        read -p "Enter the Agent Secret: " agent_secret
    fi

    if [ -z "$server_address" ] || [ -z "$vps_id" ] || [ -z "$agent_secret" ]; then
        print_error "Server Address, VPS ID, and Agent Secret are required for the first installation."
        show_usage
        exit 1
    fi

    cat > "$CONFIG_FILE_PATH" <<EOF
# Node-Nexus Agent Configuration
server_address = "$server_address"
vps_id = $vps_id
agent_secret = "$agent_secret"

# Default values, can be adjusted later
log_level = "info"
heartbeat_interval_seconds = 30
metrics_collect_interval_seconds = 5
metrics_upload_interval_seconds = 7
metrics_upload_batch_max_size = 10
data_collection_interval_seconds = 15
generic_metrics_upload_interval_seconds = 300
generic_metrics_upload_batch_max_size = 100

[docker_monitoring]
enabled = true
docker_info_collect_interval_seconds = 600
docker_info_upload_interval_seconds = 900
EOF
    print_success "Configuration file created at $CONFIG_FILE_PATH"
}

download_and_install_agent() {
    local download_url=$1
    local agent_path="$INSTALL_DIR/agent"

    print_info "Downloading agent from: $download_url"
    curl -L --progress-bar "$download_url" -o "$agent_path"
    
    if [ $? -ne 0 ]; then
        print_error "Failed to download the agent."
    fi

    print_info "Setting permissions for the agent binary..."
    chmod +x "$agent_path"
    
    print_success "Agent binary installed at $agent_path"
}

setup_systemd_service() {
    print_info "Configuring systemd service to run as user '$SERVICE_USER'..."
    
    cat > "$SERVICE_FILE_PATH" <<EOF
[Unit]
Description=Node-Nexus Agent
After=network.target

[Service]
Type=simple
User=$SERVICE_USER
ExecStart=$INSTALL_DIR/agent --config $CONFIG_FILE_PATH
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF

    print_info "Reloading systemd daemon..."
    systemctl daemon-reload

    if systemctl is-active --quiet "$SERVICE_NAME"; then
        print_info "Restarting service to apply changes..."
        systemctl restart "$SERVICE_NAME"
    else
        print_info "Enabling and starting the service..."
        systemctl enable "$SERVICE_NAME"
        systemctl start "$SERVICE_NAME"
    fi

    print_success "$SERVICE_NAME service is running."
    print_info "You can check the status with: systemctl status $SERVICE_NAME"
    print_info "You can view logs with: journalctl -u $SERVICE_NAME -f"
}

uninstall_agent() {
    print_info "Starting uninstallation process..."
    check_root

    if systemctl is-active --quiet "$SERVICE_NAME"; then
        print_info "Stopping $SERVICE_NAME service..."
        systemctl stop "$SERVICE_NAME"
    fi

    if systemctl is-enabled --quiet "$SERVICE_NAME"; then
        print_info "Disabling $SERVICE_NAME service..."
        systemctl disable "$SERVICE_NAME"
    fi

    if [ -f "$SERVICE_FILE_PATH" ]; then
        print_info "Removing systemd service file..."
        rm -f "$SERVICE_FILE_PATH"
        print_info "Reloading systemd daemon..."
        systemctl daemon-reload
    fi

    if [ -d "$INSTALL_DIR" ]; then
        print_info "Removing installation directory: $INSTALL_DIR"
        rm -rf "$INSTALL_DIR"
    fi

    if id "node-nexus" &>/dev/null; then
        read -p "Do you want to remove the 'node-nexus' user? [y/N] " -r
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            print_info "Removing 'node-nexus' user..."
            userdel node-nexus
        fi
    fi

    print_success "Uninstallation complete."
}

# --- Script Entry Point ---
main() {
    local command="install"
    if [[ "$1" == "install" || "$1" == "uninstall" ]]; then
        command=$1
        shift
    elif [ $# -gt 0 ] && [[ ! "$1" =~ ^- ]]; then
        show_usage
        exit 1
    fi

    if [ "$command" == "uninstall" ]; then
        uninstall_agent
        exit 0
    fi

    # --- Install/Update Logic ---
    local download_url=""
    local server_address=""
    local vps_id=""
    local agent_secret=""
    local use_secure_user=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        key="$1"
        case $key in
            -s|--server-address) server_address="$2"; shift 2 ;;
            -i|--vps-id) vps_id="$2"; shift 2 ;;
            -k|--agent-secret) agent_secret="$2"; shift 2 ;;
            -d|--download-url) download_url="$2"; shift 2 ;;
            --secure-user) use_secure_user=true; shift ;;
            -h|--help) show_usage; exit 0 ;;
            *)
                print_error "Unknown argument: $1. Use -h or --help for usage."
                shift
            ;;
        esac
    done

    check_root
    check_dependencies
    detect_arch

    if [ -z "$download_url" ]; then
        download_url=$(get_latest_release_url)
    else
        print_info "Using provided download URL: $download_url"
    fi

    setup_environment
    create_config_file "$server_address" "$vps_id" "$agent_secret"
    
    if [ "$use_secure_user" = true ]; then
        setup_secure_user
    fi
    
    download_and_install_agent "$download_url"
    setup_systemd_service

    print_success "Installation/Update complete!"
}

main "$@"