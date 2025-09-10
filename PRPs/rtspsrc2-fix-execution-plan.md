# PRP: rtspsrc2 Fix Execution Plan and Coordination

## Overview

This PRP provides the coordination plan for fixing the broken rtspsrc2 element. The element currently connects to RTSP servers but fails to stream any media data due to architectural and implementation issues.

## Problem Summary

**Critical Issue**: rtspsrc2 treats unlinked pad errors as fatal, causing data flow tasks to terminate immediately. This results in endless "pad has no peer" messages and no video/audio output, despite successful RTSP negotiation.

**Root Cause**: TODO comment at `net/rtsp/src/rtspsrc/imp.rs:1813` indicates known issue with unlinked source pad handling.

## PRP Execution Sequence

### Phase 1: Core Data Flow Fixes (Immediate - Critical)
**Duration**: 2-3 days

1. **[PRIORITY 1] Fix Unlinked Pad Error Handling** 
   - **File**: `PRPs/fix-rtspsrc2-unlinked-pad-handling.md`
   - **Impact**: Enables basic data flow
   - **Risk**: Low (addresses known TODO)
   - **Confidence**: 9/10

2. **[PRIORITY 2] Fix Ghost Pad Target Timing Race Condition**
   - **File**: `PRPs/fix-rtspsrc2-ghost-pad-timing.md`  
   - **Impact**: Prevents data loss during pad setup
   - **Risk**: Medium (timing changes)
   - **Confidence**: 7/10

### Phase 2: Robustness Improvements (Follow-up - Important)
**Duration**: 3-4 days

3. **[PRIORITY 3] Add Buffer Management**
   - **File**: `PRPs/add-rtspsrc2-buffer-management.md`
   - **Impact**: Prevents memory issues, improves reliability
   - **Risk**: Medium (memory management complexity)
   - **Confidence**: 6/10

4. **[PRIORITY 4] Improve State Transitions**
   - **File**: `PRPs/improve-rtspsrc2-state-transitions.md`
   - **Impact**: Better startup/shutdown behavior
   - **Risk**: Low-Medium (established patterns)
   - **Confidence**: 7/10

### Phase 3: Testing and Validation (Parallel - Essential)
**Duration**: 2-3 days (can overlap with Phase 2)

5. **[PRIORITY 5] Add Comprehensive Testing**
   - **File**: `PRPs/add-rtspsrc2-comprehensive-testing.md`
   - **Impact**: Prevents regressions, validates fixes
   - **Risk**: Low (testing only)
   - **Confidence**: 8/10

### Phase 4: Long-term Planning (Future - Optional)
**Duration**: 1-2 days

6. **[PRIORITY 6] Research Architecture Redesign**
   - **File**: `PRPs/research-rtspsrc2-architecture-redesign.md`
   - **Impact**: Guides future improvements
   - **Risk**: Very Low (research only)
   - **Confidence**: 9/10

## Critical Dependencies

### Hard Dependencies (Must Follow Order)
1. **Unlinked Pad Handling** → **Ghost Pad Timing** (timing fix builds on error handling)
2. **Core Fixes** → **Buffer Management** (buffer mgmt requires working data flow)
3. **Basic Functionality** → **Comprehensive Testing** (need something to test)

### Parallel Opportunities
- **State Transitions** can be worked on in parallel with **Buffer Management**
- **Testing Infrastructure** can be started once basic data flow works
- **Architecture Research** can happen anytime

## Success Metrics

### Phase 1 Success (Minimum Viable)
- ✅ rtspsrc2 streams video from RTSP source
- ✅ No "pad has no peer" error spam
- ✅ Basic pipeline functionality working

### Phase 2 Success (Production Ready)
- ✅ Reliable under various network conditions
- ✅ Clean startup and shutdown
- ✅ No memory leaks during extended operation

### Phase 3 Success (Maintainable)
- ✅ Comprehensive test coverage
- ✅ Automated regression detection
- ✅ Performance benchmarks established

## Risk Mitigation

### High-Risk Items
1. **Ghost Pad Timing Changes**: Test thoroughly with rapid connect/disconnect
2. **Buffer Management**: Monitor memory usage carefully during testing

### Rollback Strategy
- Each PRP is independent enough to be reverted if issues arise
- Focus on Phase 1 completion before moving to higher-risk Phase 2 items

## Resource Requirements

### Development Environment
- **Test Setup**: MediaMTX RTSP server (already configured)
- **Validation**: Multiple RTSP sources for testing
- **Tools**: Memory profiling for buffer management validation

### Expertise Needed
- **GStreamer Knowledge**: Understanding of pad management and data flow
- **Rust Async**: For proper async/GStreamer integration
- **RTSP Protocol**: For debugging connectivity issues

## Communication Plan

### Progress Tracking
- **Daily Updates**: Status of current PRP implementation
- **Milestone Reports**: After each phase completion
- **Issue Escalation**: If any PRP confidence drops below 5/10

### Validation Gates
- **Phase 1**: Must achieve basic video streaming before Phase 2
- **Phase 2**: Must pass memory and stability tests before Phase 3
- **Each PRP**: Must pass individual validation gates before merge

## Timeline Estimate

**Total Duration**: 1-2 weeks
- **Phase 1 (Critical)**: 2-3 days
- **Phase 2 (Important)**: 3-4 days  
- **Phase 3 (Testing)**: 2-3 days (parallel)
- **Phase 4 (Research)**: 1-2 days (future)

**Minimum Viable Fix**: Phase 1 only (2-3 days for basic functionality)

## Confidence Assessment

**Overall Confidence**: 7.5/10
- **Phase 1**: High confidence (9/10, 7/10) - addresses known issues
- **Phase 2**: Medium confidence (6/10, 7/10) - more complex implementations  
- **Phase 3**: High confidence (8/10) - testing is well-understood
- **Phase 4**: Very high confidence (9/10) - research only

The execution plan balances immediate fixes with long-term robustness while maintaining clear dependencies and risk management.