#!/bin/sh

# degrade_dst_ip.sh - Script to degrade packets to a specific destination IP using 'tc'
# Usage: sudo ./degrade_dst_ip.sh <destination-ip> <duration-sec> <loss-pct>
# Example: sudo ./degrade_dst_ip.sh 192.168.1.100 10 10

if ! type iptables 2>&1 > /dev/null; then
    echo "Error: 'iptables' command not found. Please install iptables package."
    exit 1
fi

DESTINATION_IP="$1"
DURATION_SEC="${2:-10}"
LOSS_PERCENTAGE="${3:-10}"
LOSS=$(awk "BEGIN {print $LOSS_PERCENTAGE/100}")  # Convert percentage to a decimal for probability

if [ -z "$DESTINATION_IP" ] || [ -z "$DURATION_SEC" ] || [ -z "$LOSS_PERCENTAGE" ]; then
    echo "Usage: sudo ./degrade_dst_ip.sh <destination-ip> <duration-sec> <loss-pct>"
    exit 1
fi

if [ "$(id -u)" -ne 0 ]; then
    echo "This script must be run as root. Please use sudo."
    exit 1
fi

# Trap ctrl-c to ensure cleanup
trap 'iptables -d $DESTINATION_IP -D OUTPUT -m statistic --mode random --probability $LOSS -j DROP; echo "Restored iptables rules"; exit' INT TERM

echo "Dropping ${LOSS_PERCENTAGE}% ($LOSS) of packets from $DESTINATION_IP for $DURATION_SEC seconds"
iptables -d $DESTINATION_IP -A OUTPUT -m statistic --mode random --probability $LOSS -j DROP && sleep $DURATION_SEC &&\
iptables -d $DESTINATION_IP -D OUTPUT -m statistic --mode random --probability $LOSS -j DROP

echo "Traffic to $DESTINATION_IP was dropped for ${DURATION_SEC}s with ${LOSS_PERCENTAGE}% packet loss."
