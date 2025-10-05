#!/bin/bash
# Setup script for dual-stream RTSP testing
# This helps configure the system for testing stream independence

set -e

ACTION="${1:-help}"

case "$ACTION" in
    setup-ip)
        echo "Adding 127.0.0.2 as a loopback alias..."
        sudo ip addr add 127.0.0.2/8 dev lo 2>/dev/null || echo "127.0.0.2 already exists"
        echo "Done. You can now use rtsp://127.0.0.2:8554/stream2"
        ;;
    
    remove-ip)
        echo "Removing 127.0.0.2 loopback alias..."
        sudo ip addr del 127.0.0.2/8 dev lo 2>/dev/null || echo "127.0.0.2 not found"
        echo "Done"
        ;;
    
    block-stream2)
        echo "Blocking UDP packets from 127.0.0.2 (simulating network failure)..."
        sudo iptables -A INPUT -s 127.0.0.2 -p udp -j DROP
        sudo iptables -A OUTPUT -d 127.0.0.2 -p udp -j DROP
        echo "Stream 2 UDP traffic blocked. Stream 1 should continue working."
        echo "Run: $0 unblock-stream2 to restore"
        ;;
    
    unblock-stream2)
        echo "Unblocking UDP packets from 127.0.0.2..."
        sudo iptables -D INPUT -s 127.0.0.2 -p udp -j DROP 2>/dev/null || true
        sudo iptables -D OUTPUT -d 127.0.0.2 -p udp -j DROP 2>/dev/null || true
        echo "Stream 2 UDP traffic restored"
        ;;
    
    start-mediamtx)
        echo "Starting mediamtx with dual-stream config..."
        mediamtx mediamtx-dual.yml
        ;;
    
    publish-test-streams)
        echo "Publishing two test video streams to mediamtx..."
        echo "Stream 1 to rtsp://127.0.0.1:8554/stream1"
        echo "Stream 2 to rtsp://127.0.0.2:8554/stream2"
        echo ""
        echo "Starting ffmpeg publishers (press Ctrl+C to stop)..."
        
        # Stream 1 - blue background with timestamp
        ffmpeg -re -f lavfi -i "testsrc=size=640x480:rate=30,format=yuv420p" \
            -f lavfi -i "sine=frequency=1000:sample_rate=44100" \
            -c:v libx264 -preset ultrafast -tune zerolatency -profile:v baseline \
            -c:a aac -f rtsp rtsp://127.0.0.1:8554/stream1 &
        PID1=$!
        
        sleep 2
        
        # Stream 2 - different pattern
        ffmpeg -re -f lavfi -i "smptebars=size=640x480:rate=30" \
            -f lavfi -i "sine=frequency=440:sample_rate=44100" \
            -c:v libx264 -preset ultrafast -tune zerolatency -profile:v baseline \
            -c:a aac -f rtsp rtsp://127.0.0.2:8554/stream2 &
        PID2=$!
        
        echo ""
        echo "Test streams running (PIDs: $PID1, $PID2)"
        echo "Press Ctrl+C to stop"
        
        trap "kill $PID1 $PID2 2>/dev/null; exit" INT TERM
        wait
        ;;
    
    test-dual)
        echo "Running dual stream example (independent streams)..."
        RTSP_URL1="rtsp://127.0.0.1:8554/stream1" \
        RTSP_URL2="rtsp://127.0.0.2:8554/stream2" \
        cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup_dual_stream
        ;;
    
    test-synced)
        echo "Running synchronized dual stream example (compositor mux)..."
        RTSP_URL1="rtsp://127.0.0.1:8554/stream1" \
        RTSP_URL2="rtsp://127.0.0.2:8554/stream2" \
        cargo run -p gst-plugin-rtsp --example rtspsrc_synced_dual_stream
        ;;
    
    help|*)
        cat <<EOF
Dual-Stream RTSP Testing Helper

Usage: $0 <command>

Commands:
  setup-ip              Add 127.0.0.2 as loopback alias
  remove-ip             Remove 127.0.0.2 loopback alias
  
  block-stream2         Block UDP packets to/from 127.0.0.2 (test stream isolation)
  unblock-stream2       Restore UDP packets to/from 127.0.0.2
  
  start-mediamtx        Start mediamtx with dual-stream configuration
  publish-test-streams  Publish two test video streams using ffmpeg
  test-dual             Run the dual stream example (independent streams)
  test-synced           Run the synchronized dual stream example (compositor mux)
  
  help                  Show this help message

Example workflow:
  1. $0 setup-ip
  2. $0 start-mediamtx          # In one terminal
  3. $0 publish-test-streams    # In another terminal  
  4. $0 test-dual               # Test independent streams
     OR
     $0 test-synced             # Test synchronized/muxed streams
  5. $0 block-stream2           # Test stream independence
  6. $0 unblock-stream2         # Restore stream 2

Key Differences:
  test-dual:   Independent streams, each with own sink (like multi-camera monitoring)
  test-synced: Compositor-muxed streams, frames kept in sync (like nvstreammux)

EOF
        ;;
esac
