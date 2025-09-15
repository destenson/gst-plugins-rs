# Buffer Mode None Investigation Results

## Problem
Setting `buffer-mode=none` on rtspsrc2 results in no frames being displayed.

## Root Cause
The rtpjitterbuffer requires minimal buffering to handle RTP timestamps correctly. When buffer-mode is set to "none":
- Only RTP timestamps are used without any buffering
- The jitterbuffer outputs the first frame then drops subsequent frames as duplicates
- No synchronization occurs between packets
- Lost packet events are not generated properly

## Technical Details
- Mode 0 (None) tells rtpjitterbuffer to only use RTP timestamps
- Without buffering, packets with the same or similar timestamps are considered duplicates
- The jitterbuffer needs some buffering to reorder packets and handle timing

## Solution
**DO NOT use buffer-mode=none as default.** Instead use buffer-mode=slave which provides minimal buffering while maintaining low latency.

### Buffer Mode Recommendations
1. **For lowest latency with reliability**: Use `slave` mode (mode 1)
   - Slaves receiver to sender clock
   - Minimal buffering 
   - Still handles timestamps properly

2. **For lowest latency without reliability**: Reduce latency property to 0-200ms with slave mode
   - `buffer-mode=slave latency=200`

3. **Never use**: `buffer-mode=none` unless you have a specific use case with perfect RTP timestamps

## Implementation Changes
The fix is to avoid None mode and use Slave mode as the minimal-buffering default when optimizing for low latency.