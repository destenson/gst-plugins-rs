#!/bin/bash
set -e

# Stream Manager systemd service installation script

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SERVICE_NAME="stream-manager"
SERVICE_FILE="systemd/${SERVICE_NAME}.service"
BINARY_NAME="stream-manager"
INSTALL_PREFIX="${INSTALL_PREFIX:-/usr/local}"
CONFIG_DIR="/etc/${SERVICE_NAME}"
DATA_DIR="/var/lib/${SERVICE_NAME}"
LOG_DIR="/var/log/${SERVICE_NAME}"
USER="${SERVICE_NAME}"
GROUP="${SERVICE_NAME}"

# Functions
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_root() {
    if [[ $EUID -ne 0 ]]; then
        print_error "This script must be run as root"
        exit 1
    fi
}

create_user() {
    if ! id -u ${USER} >/dev/null 2>&1; then
        print_info "Creating service user: ${USER}"
        useradd --system --home-dir ${DATA_DIR} --shell /bin/false ${USER}
    else
        print_info "User ${USER} already exists"
    fi
}

create_directories() {
    print_info "Creating required directories..."
    
    # Create configuration directory
    mkdir -p ${CONFIG_DIR}
    chown ${USER}:${GROUP} ${CONFIG_DIR}
    chmod 755 ${CONFIG_DIR}
    
    # Create data directory
    mkdir -p ${DATA_DIR}
    chown ${USER}:${GROUP} ${DATA_DIR}
    chmod 755 ${DATA_DIR}
    
    # Create log directory
    mkdir -p ${LOG_DIR}
    chown ${USER}:${GROUP} ${LOG_DIR}
    chmod 755 ${LOG_DIR}
    
    # Create recording directories
    mkdir -p ${DATA_DIR}/recordings
    chown ${USER}:${GROUP} ${DATA_DIR}/recordings
    chmod 755 ${DATA_DIR}/recordings
}

install_binary() {
    if [[ ! -f "target/release/${BINARY_NAME}" ]]; then
        print_error "Binary not found at target/release/${BINARY_NAME}"
        print_info "Please build the project first with: cargo build --release"
        exit 1
    fi
    
    print_info "Installing binary to ${INSTALL_PREFIX}/bin/${BINARY_NAME}"
    install -m 755 -o root -g root "target/release/${BINARY_NAME}" "${INSTALL_PREFIX}/bin/${BINARY_NAME}"
}

install_config() {
    if [[ ! -f "${CONFIG_DIR}/config.toml" ]]; then
        if [[ -f "config.example.toml" ]]; then
            print_info "Installing example configuration..."
            install -m 644 -o ${USER} -g ${GROUP} "config.example.toml" "${CONFIG_DIR}/config.toml"
            print_warn "Please edit ${CONFIG_DIR}/config.toml to configure the service"
        else
            print_warn "No example configuration found, creating minimal config..."
            cat > "${CONFIG_DIR}/config.toml" << EOF
# Stream Manager Configuration
[api]
host = "127.0.0.1"
port = 8080

[storage]
paths = ["/var/lib/stream-manager/recordings"]

[monitoring]
enabled = true
EOF
            chown ${USER}:${GROUP} "${CONFIG_DIR}/config.toml"
            chmod 644 "${CONFIG_DIR}/config.toml"
        fi
    else
        print_info "Configuration file already exists, skipping..."
    fi
    
    # Create environment file
    if [[ ! -f "${CONFIG_DIR}/environment" ]]; then
        print_info "Creating environment file..."
        cat > "${CONFIG_DIR}/environment" << EOF
# Stream Manager Environment Variables
RUST_LOG=info,stream_manager=debug
RUST_BACKTRACE=1
GST_DEBUG=2
EOF
        chown ${USER}:${GROUP} "${CONFIG_DIR}/environment"
        chmod 644 "${CONFIG_DIR}/environment"
    fi
}

install_service() {
    if [[ ! -f "${SERVICE_FILE}" ]]; then
        print_error "Service file not found at ${SERVICE_FILE}"
        exit 1
    fi
    
    print_info "Installing systemd service..."
    
    # Update paths in service file
    sed -e "s|/usr/local/bin/${BINARY_NAME}|${INSTALL_PREFIX}/bin/${BINARY_NAME}|g" \
        -e "s|User=${USER}|User=${USER}|g" \
        -e "s|Group=${GROUP}|Group=${GROUP}|g" \
        "${SERVICE_FILE}" > "/etc/systemd/system/${SERVICE_NAME}.service"
    
    # Reload systemd
    print_info "Reloading systemd daemon..."
    systemctl daemon-reload
}

enable_service() {
    print_info "Enabling ${SERVICE_NAME} service..."
    systemctl enable ${SERVICE_NAME}.service
}

start_service() {
    read -p "Do you want to start the service now? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_info "Starting ${SERVICE_NAME} service..."
        systemctl start ${SERVICE_NAME}.service
        
        # Check status
        sleep 2
        if systemctl is-active --quiet ${SERVICE_NAME}.service; then
            print_info "Service started successfully!"
            systemctl status ${SERVICE_NAME}.service --no-pager
        else
            print_error "Service failed to start. Check logs with: journalctl -u ${SERVICE_NAME}"
            exit 1
        fi
    fi
}

# Main installation flow
main() {
    print_info "Stream Manager Service Installation"
    print_info "===================================="
    
    check_root
    create_user
    create_directories
    install_binary
    install_config
    install_service
    enable_service
    start_service
    
    print_info ""
    print_info "Installation complete!"
    print_info ""
    print_info "Service management commands:"
    print_info "  Start:   systemctl start ${SERVICE_NAME}"
    print_info "  Stop:    systemctl stop ${SERVICE_NAME}"
    print_info "  Restart: systemctl restart ${SERVICE_NAME}"
    print_info "  Status:  systemctl status ${SERVICE_NAME}"
    print_info "  Logs:    journalctl -u ${SERVICE_NAME} -f"
    print_info ""
    print_info "Configuration: ${CONFIG_DIR}/config.toml"
    print_info "Data directory: ${DATA_DIR}"
    print_info "Log directory: ${LOG_DIR}"
}

# Run main function
main "$@"