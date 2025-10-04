#!/bin/sh


# degrade_src_ip.sh - Script to degrade packets from a specific source IP using 'tc'
# Usage: sudo ./degrade_src_ip.sh <source-ip> <duration-sec> <loss-pct>
# Example: sudo ./degrade_src_ip.sh 192.168.1.100 100 10

if ! type iptables 2>&1 > /dev/null; then
    echo "Error: 'iptables' command not found. Please install iptables package."
    exit 1
fi

SOURCE_IP="$1"
DURATION_SEC="${2:-10}"
LOSS_PERCENTAGE="${3:-10}"
LOSS=$(awk "BEGIN {print $LOSS_PERCENTAGE/100}")  # Convert percentage to a decimal for probability

if [ -z "$SOURCE_IP" ] || [ -z "$DURATION_SEC" ] || [ -z "$LOSS_PERCENTAGE" ]; then
    echo "Usage: sudo ./degrade_src_ip.sh <source-ip> <duration-sec> <loss-pct>"
    exit 1
fi

if [ "$(id -u)" -ne 0 ]; then
    echo "This script must be run as root. Please use sudo."
    exit 1
fi

# Trap ctrl-c to ensure cleanup
trap 'iptables -s $SOURCE_IP -D INPUT -m statistic --mode random --probability $LOSS -j DROP; echo "Restored iptables rules"; exit' INT TERM

echo "Dropping ${LOSS_PERCENTAGE}% ($LOSS) of packets from $SOURCE_IP for $DURATION_SEC seconds"
iptables -s $SOURCE_IP -A INPUT -m statistic --mode random --probability $LOSS -j DROP && sleep $DURATION_SEC &&\
iptables -s $SOURCE_IP -D INPUT -m statistic --mode random --probability $LOSS -j DROP

echo "Traffic from $SOURCE_IP was dropped for ${DURATION_SEC}s with ${LOSS_PERCENTAGE}% packet loss."
