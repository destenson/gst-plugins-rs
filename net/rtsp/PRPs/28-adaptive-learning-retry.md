# PRP-RTSP-28: Adaptive Learning Retry Mode

## Overview
Implement an advanced "adaptive" retry mode that learns from network behavior over time, building a statistical model to select optimal retry strategies for each server.

## Current State
- No learning from past connections
- Same mistakes repeated
- No per-server optimization
- Cannot adapt to changing conditions

## Success Criteria
- [ ] Learn optimal strategy per server
- [ ] Build statistical confidence model
- [ ] Persist learning across sessions
- [ ] Adapt to network changes
- [ ] Tests verify learning behavior

## Technical Details

### Learning System Architecture

```
┌─────────────────┐
│ Connection      │
│ Attempts        │
└────────┬────────┘
         ↓
┌─────────────────┐
│ Metrics         │
│ Collector       │
└────────┬────────┘
         ↓
┌─────────────────┐
│ Statistical     │
│ Model           │
└────────┬────────┘
         ↓
┌─────────────────┐
│ Strategy        │
│ Selector        │
└────────┬────────┘
         ↓
┌─────────────────┐
│ Persistent      │
│ Cache           │
└─────────────────┘
```

### Metrics Tracked

```rust
struct ServerMetrics {
    // Per-strategy statistics
    strategies: HashMap<Strategy, StrategyStats>,
    
    // Network characteristics
    avg_latency: Duration,
    packet_loss_rate: f32,
    connection_drop_pattern: DropPattern,
    
    // Historical data
    total_attempts: u64,
    last_updated: Instant,
    confidence_score: f32,
}

struct StrategyStats {
    attempts: u64,
    successes: u64,
    avg_recovery_time: Duration,
    recent_performance: RingBuffer<bool>, // Last 20 attempts
    score: f32,
}
```

### Learning Phases

#### Phase 1: Discovery (First 30 seconds)
```rust
// Try each strategy at least once
for strategy in ALL_STRATEGIES {
    attempt_with(strategy);
    collect_metrics();
}
// Build initial model
```

#### Phase 2: Exploitation (90% of time)
```rust
// Use best known strategy
let best = model.best_strategy();
attempt_with(best);
update_model(outcome);
```

#### Phase 3: Exploration (10% of time)
```rust
// Occasionally try alternatives
if random() < 0.1 {
    let alternative = model.select_exploration();
    attempt_with(alternative);
    update_model(outcome);
}
```

### Scoring Algorithm

```rust
fn calculate_score(stats: &StrategyStats) -> f32 {
    let success_rate = stats.successes as f32 / stats.attempts as f32;
    let speed_score = 1.0 / stats.avg_recovery_time.as_secs_f32();
    let recency_weight = calculate_recency_weight(&stats.recent_performance);
    
    // Weighted combination
    success_rate * 0.5 +          // 50% weight on success
    speed_score * 0.3 +           // 30% weight on speed
    recency_weight * 0.2          // 20% weight on recent performance
}
```

### Adaptation to Change

```rust
fn detect_network_change(&self) -> bool {
    // Monitor recent performance
    let recent_success_rate = self.recent_20_attempts.success_rate();
    let historical_rate = self.overall_success_rate;
    
    // Significant change detected
    (recent_success_rate - historical_rate).abs() > 0.3
}

fn adapt_to_change(&mut self) {
    // Reduce confidence in current model
    self.confidence_score *= 0.5;
    
    // Increase exploration rate temporarily
    self.exploration_rate = 0.3;
    
    // Re-enter discovery phase briefly
    self.phase = Phase::MiniDiscovery(Duration::from_secs(10));
}
```

### Persistence

```rust
// Save learned patterns
fn persist(&self) -> Result<()> {
    let cache_dir = dirs::cache_dir()
        .unwrap()
        .join("gstreamer")
        .join("rtspsrc2");
    
    let server_hash = hash(&self.server_url);
    let cache_file = cache_dir.join(format!("{}.json", server_hash));
    
    let data = serde_json::to_string(&self.metrics)?;
    fs::write(cache_file, data)?;
    Ok(())
}

// Load on connection
fn load_metrics(server_url: &str) -> Option<ServerMetrics> {
    // Load from cache if exists and recent (< 7 days old)
}
```

### Configuration

```rust
retry-strategy: adaptive
adaptive-learning: true (default)          # Enable learning
adaptive-persistence: true (default)       # Save learning
adaptive-cache-ttl: 7 days                # Cache lifetime
adaptive-discovery-time: 30s              # Initial learning
adaptive-exploration-rate: 0.1            # Exploration frequency
adaptive-confidence-threshold: 0.8        # Min confidence
adaptive-change-detection: true           # Detect network changes
```

## Implementation Blueprint

1. Create `adaptive_retry` module
2. Implement `MetricsCollector` struct
3. Create `StatisticalModel` for scoring
4. Implement exploration/exploitation balance
5. Add change detection algorithm
6. Create persistence layer
7. Add cache management
8. Implement smooth transitions
9. Add detailed telemetry
10. Test learning convergence

## Multi-Armed Bandit Algorithm

Using Thompson Sampling for exploration:

```rust
fn select_strategy(&self) -> Strategy {
    if self.phase == Phase::Discovery {
        return self.next_untested_strategy();
    }
    
    // Thompson Sampling
    let mut best_sample = f32::MIN;
    let mut best_strategy = Strategy::ExponentialJitter;
    
    for (strategy, stats) in &self.strategies {
        // Sample from Beta distribution
        let sample = sample_beta(
            stats.successes + 1,
            stats.attempts - stats.successes + 1
        );
        
        if sample > best_sample {
            best_sample = sample;
            best_strategy = *strategy;
        }
    }
    
    best_strategy
}
```

## Resources
- Multi-armed bandits: https://en.wikipedia.org/wiki/Multi-armed_bandit
- Thompson Sampling: https://en.wikipedia.org/wiki/Thompson_sampling
- Online learning algorithms: https://arxiv.org/abs/1912.06116
- TCP congestion control (similar adaptation): RFC 5681

## Validation Gates
```bash
# Test learning convergence
cargo test -p gst-plugin-rtsp adaptive_convergence -- --nocapture

# Test exploration/exploitation
cargo test -p gst-plugin-rtsp adaptive_balance -- --nocapture

# Test change detection
cargo test -p gst-plugin-rtsp adaptive_change -- --nocapture

# Test persistence
cargo test -p gst-plugin-rtsp adaptive_persistence -- --nocapture

# Benchmark vs auto mode
cargo bench -p gst-plugin-rtsp adaptive_vs_auto
```

## Dependencies
- PRP-RTSP-06 (All strategies)
- PRP-RTSP-17 (Connection racing)
- PRP-RTSP-15 (Telemetry)
- PRP-RTSP-27 (Fallback to auto mode)

## Estimated Effort
4 hours

## Risk Assessment
- Medium-high complexity
- Challenge: Tuning exploration/exploitation balance
- Challenge: Avoiding local optima
- Benefit: Optimal performance after learning

## Success Confidence Score
7/10 - Well-understood algorithms but needs careful tuning

## Expected Performance

### Learning Curve
```
Time        | Optimality
------------|------------
0-30s       | 60% (discovery)
30s-1min    | 80% (initial model)
1-5min      | 90% (refined model)
5min+       | 95% (converged)
```

### Comparison with Auto Mode
| Metric | Auto | Adaptive (after learning) |
|--------|------|---------------------------|
| Decision Time | Instant | 30s warmup |
| Accuracy | 80% | 95% |
| Memory | 0 | ~1KB/server |
| Network Changes | Re-detect | Smooth adaptation |
| Per-server Optimization | No | Yes |