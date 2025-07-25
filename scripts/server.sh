#!/bin/bash

# Exit immediately if a command exits with a non-zero status.
set -e

# --- Configuration ---
INSTALL_DIR="/opt/node-nexus-server"
SERVICE_NAME="node-nexus-server"
SERVICE_USER="root" # Default user, can be changed with --secure-user
CONFIG_FILE_PATH="$INSTALL_DIR/config.toml"
SERVICE_FILE_PATH="/etc/systemd/system/$SERVICE_NAME.service"
GITHUB_REPO="moonheart/NodeNexus"
SERVER_BINARY_NAME="" # This will be set dynamically

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
            SERVER_BINARY_NAME="server-linux-amd64"
            ;;
        aarch64)
            SERVER_BINARY_NAME="server-linux-arm64"
            ;;
        *)
            print_error "Unsupported architecture: $arch. Only x86_64 and aarch64 are supported."
            ;;
    esac
    print_info "Detected architecture: $arch. Using binary: $SERVER_BINARY_NAME"
}

get_latest_release_url() {
    print_info "Fetching latest release from GitHub repository: $GITHUB_REPO"
    local api_url="https://api.github.com/repos/$GITHUB_REPO/releases/latest"
    
    local response
    response=$(curl -s "$api_url")
    
    if echo "$response" | jq -e '.assets' &> /dev/null; then
        local download_url
        download_url=$(echo "$response" | jq -r ".assets[] | select(.name == \"$SERVER_BINARY_NAME\") | .browser_download_url")
        
        if [ -z "$download_url" ]; then
            print_error "Could not find an asset named '$SERVER_BINARY_NAME' in the latest release."
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
    echo "  install                   Install or update the server. This is the default command."
    echo "  uninstall                 Uninstall the server."
    echo
    echo "Options for 'install' command:"
    echo "  -d, --download-url <url>    Optional. Direct URL to the server binary. Overrides GitHub release check."
    echo "      --secure-user           Create a dedicated user 'node-nexus-server' to run the service for enhanced security."
    echo "  -h, --help                  Show this help message."
}

# --- Main Installation Steps ---
setup_secure_user() {
    if id "node-nexus-server" &>/dev/null; then
        print_info "User 'node-nexus-server' already exists."
    else
        print_info "Creating dedicated user 'node-nexus-server'..."
        useradd --system --no-create-home --shell /bin/false node-nexus-server
    fi
    print_info "Setting ownership of $INSTALL_DIR to 'node-nexus-server' user..."
    chown -R node-nexus-server:node-nexus-server "$INSTALL_DIR"
    SERVICE_USER="node-nexus-server"
}

setup_environment() {
    print_info "Setting up installation directory: $INSTALL_DIR"
    mkdir -p "$INSTALL_DIR"
}

create_config_file() {
    if [ -f "$CONFIG_FILE_PATH" ]; then
        print_info "Configuration file already exists. Skipping creation."
        return
    fi
    
    print_info "Creating a default configuration file..."

    cat > "$CONFIG_FILE_PATH" <<EOF
# Node-Nexus Server Configuration
# Please edit this file with your actual database and other configurations.

# Example for PostgreSQL
# database_url = "postgres://user:password@localhost/node_nexus"

# Example for SQLite
database_url = "sqlite:$INSTALL_DIR/node_nexus.db"

# JWT secret for signing tokens
# PLEASE CHANGE THIS to a long, random string for security
jwt_secret = "your-super-secret-and-long-jwt-secret"

# Server listen address
listen_address = "0.0.0.0:8080"

# Log level (e.g., "info", "debug", "warn", "error")
log_level = "info"
EOF
    print_success "Default configuration file created at $CONFIG_FILE_PATH"
    print_info "IMPORTANT: Please review and edit the configuration file before starting the server for the first time."
}

download_and_install_server() {
    local download_url=$1
    local server_path="$INSTALL_DIR/server"

    print_info "Downloading server from: $download_url"
    curl -L --progress-bar "$download_url" -o "$server_path"
    
    if [ $? -ne 0 ]; then
        print_error "Failed to download the server."
    fi

    print_info "Setting permissions for the server binary..."
    chmod +x "$server_path"
    
    print_success "Server binary installed at $server_path"
}

setup_systemd_service() {
    print_info "Configuring systemd service to run as user '$SERVICE_USER'..."
    
    cat > "$SERVICE_FILE_PATH" <<EOF
[Unit]
Description=Node-Nexus Server
After=network.target

[Service]
Type=simple
User=$SERVICE_USER
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/server --config $CONFIG_FILE_PATH
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
Environment="NEXUS_SERVER_SERVICE_NAME=$SERVICE_NAME"

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

stop_service_for_update() {
    if systemctl is-active --quiet "$SERVICE_NAME"; then
        print_info "Stopping $SERVICE_NAME service for update..."
        systemctl stop "$SERVICE_NAME"
    fi
}

uninstall_server() {
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
        read -p "Do you want to remove the installation directory ($INSTALL_DIR)? This will delete the server binary, config, and SQLite database. [y/N] " -r
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            print_info "Removing installation directory: $INSTALL_DIR"
            rm -rf "$INSTALL_DIR"
        fi
    fi

    if id "node-nexus-server" &>/dev/null; then
        read -p "Do you want to remove the 'node-nexus-server' user? [y/N] " -r
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            print_info "Removing 'node-nexus-server' user..."
            userdel node-nexus-server
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
        uninstall_server
        exit 0
    fi

    # --- Install/Update Logic ---
    local download_url=""
    local use_secure_user=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        key="$1"
        case $key in
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

    if [ -f "$CONFIG_FILE_PATH" ]; then
        print_info "Existing installation detected. Proceeding with update..."
        
        stop_service_for_update

        if [ -z "$download_url" ]; then
            download_url=$(get_latest_release_url)
        else
            print_info "Using provided download URL: $download_url"
        fi

        setup_environment
        
        if [ "$use_secure_user" = true ]; then
            setup_secure_user
        fi
        
        download_and_install_server "$download_url"
        setup_systemd_service

        print_success "Update complete!"
    else
        # New Installation
        print_info "No existing installation found. Proceeding with new installation."
        
        if [ -z "$download_url" ]; then
            download_url=$(get_latest_release_url)
        else
            print_info "Using provided download URL: $download_url"
        fi

        setup_environment
        create_config_file
        
        if [ "$use_secure_user" = true ]; then
            setup_secure_user
        fi
        
        download_and_install_server "$download_url"
        setup_systemd_service

        print_success "Installation complete!"
    fi
}

main "$@"