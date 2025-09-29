#!/bin/bash
set -e

# Stream Manager systemd service uninstallation script

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SERVICE_NAME="stream-manager"
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

stop_service() {
    if systemctl is-active --quiet ${SERVICE_NAME}.service; then
        print_info "Stopping ${SERVICE_NAME} service..."
        systemctl stop ${SERVICE_NAME}.service
    else
        print_info "Service is not running"
    fi
}

disable_service() {
    if systemctl is-enabled --quiet ${SERVICE_NAME}.service 2>/dev/null; then
        print_info "Disabling ${SERVICE_NAME} service..."
        systemctl disable ${SERVICE_NAME}.service
    else
        print_info "Service is not enabled"
    fi
}

remove_service() {
    if [[ -f "/etc/systemd/system/${SERVICE_NAME}.service" ]]; then
        print_info "Removing systemd service file..."
        rm -f "/etc/systemd/system/${SERVICE_NAME}.service"
        systemctl daemon-reload
    else
        print_info "Service file not found"
    fi
}

remove_binary() {
    if [[ -f "${INSTALL_PREFIX}/bin/${BINARY_NAME}" ]]; then
        print_info "Removing binary..."
        rm -f "${INSTALL_PREFIX}/bin/${BINARY_NAME}"
    else
        print_info "Binary not found"
    fi
}

backup_config() {
    if [[ -d "${CONFIG_DIR}" ]]; then
        BACKUP_DIR="${HOME}/stream-manager-backup-$(date +%Y%m%d-%H%M%S)"
        print_info "Backing up configuration to ${BACKUP_DIR}"
        mkdir -p "${BACKUP_DIR}"
        cp -r "${CONFIG_DIR}" "${BACKUP_DIR}/"
        print_info "Configuration backed up successfully"
    fi
}

remove_config() {
    read -p "Remove configuration files? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        backup_config
        print_info "Removing configuration directory..."
        rm -rf "${CONFIG_DIR}"
    else
        print_info "Keeping configuration files"
    fi
}

remove_data() {
    read -p "Remove data and recordings? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if [[ -d "${DATA_DIR}/recordings" ]] && [[ "$(ls -A ${DATA_DIR}/recordings)" ]]; then
            BACKUP_DIR="${HOME}/stream-manager-recordings-$(date +%Y%m%d-%H%M%S)"
            print_warn "Found recordings, backing up to ${BACKUP_DIR}"
            mkdir -p "${BACKUP_DIR}"
            cp -r "${DATA_DIR}/recordings" "${BACKUP_DIR}/"
        fi
        print_info "Removing data directory..."
        rm -rf "${DATA_DIR}"
    else
        print_info "Keeping data directory"
    fi
}

remove_logs() {
    read -p "Remove log files? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_info "Removing log directory..."
        rm -rf "${LOG_DIR}"
    else
        print_info "Keeping log files"
    fi
}

remove_user() {
    read -p "Remove service user ${USER}? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if id -u ${USER} >/dev/null 2>&1; then
            print_info "Removing user ${USER}..."
            userdel ${USER}
        else
            print_info "User ${USER} not found"
        fi
    else
        print_info "Keeping user ${USER}"
    fi
}

purge_journald_logs() {
    read -p "Remove journald logs for ${SERVICE_NAME}? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        print_info "Removing journald logs..."
        journalctl --rotate
        journalctl --vacuum-time=1s --unit=${SERVICE_NAME}.service
    else
        print_info "Keeping journald logs"
    fi
}

# Main uninstallation flow
main() {
    print_info "Stream Manager Service Uninstallation"
    print_info "======================================"
    print_warn "This will remove the Stream Manager service from your system"
    
    read -p "Are you sure you want to continue? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_info "Uninstallation cancelled"
        exit 0
    fi
    
    check_root
    stop_service
    disable_service
    remove_service
    remove_binary
    remove_config
    remove_data
    remove_logs
    remove_user
    purge_journald_logs
    
    print_info ""
    print_info "Uninstallation complete!"
    print_info ""
    if [[ -d "${HOME}/stream-manager-backup"* ]]; then
        print_info "Backups have been saved to your home directory"
    fi
}

# Run main function
main "$@"