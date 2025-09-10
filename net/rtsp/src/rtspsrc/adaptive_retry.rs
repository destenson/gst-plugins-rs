#![allow(unused)]
#![cfg(feature = "adaptive")]
// GStreamer RTSP Adaptive Learning Retry System
//
// This module implements an advanced adaptive retry system that learns optimal
// retry strategies for each server using a multi-armed bandit approach.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::retry::RetryStrategy;

const CACHE_TTL_DAYS: u64 = 7;
const DISCOVERY_DURATION_SECS: u64 = 30;
const DEFAULT_EXPLORATION_RATE: f32 = 0.1;
const CONFIDENCE_THRESHOLD: f32 = 0.8;
const RECENT_HISTORY_SIZE: usize = 20;
const CHANGE_DETECTION_THRESHOLD: f32 = 0.3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Strategy {
    Immediate,
    Linear,
    Exponential,
    ExponentialJitter,
}

impl Strategy {
    fn all() -> &'static [Strategy] {
        &[
            Strategy::Immediate,
            Strategy::Linear,
            Strategy::Exponential,
            Strategy::ExponentialJitter,
        ]
    }
    
    fn to_retry_strategy(&self) -> RetryStrategy {
        match self {
            Strategy::Immediate => RetryStrategy::Immediate,
            Strategy::Linear => RetryStrategy::Linear,
            Strategy::Exponential => RetryStrategy::Exponential,
            Strategy::ExponentialJitter => RetryStrategy::ExponentialJitter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Phase {
    Discovery,
    Exploitation,
    Exploration,
    MiniDiscovery(Duration),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DropPattern {
    Random,
    Periodic,
    Burst,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyStats {
    pub attempts: u64,
    pub successes: u64,
    pub avg_recovery_time: Duration,
    pub recent_performance: VecDeque<bool>,
    pub score: f32,
    #[serde(skip)]
    pub last_attempt: Option<Instant>,
    #[serde(skip)]
    recovery_times: Vec<Duration>,
}

impl Default for StrategyStats {
    fn default() -> Self {
        Self {
            attempts: 0,
            successes: 0,
            avg_recovery_time: Duration::ZERO,
            recent_performance: VecDeque::with_capacity(RECENT_HISTORY_SIZE),
            score: 0.0,
            last_attempt: None,
            recovery_times: Vec::new(),
        }
    }
}

impl StrategyStats {
    fn record_attempt(&mut self, success: bool, recovery_time: Duration) {
        self.attempts += 1;
        if success {
            self.successes += 1;
        }
        
        self.recent_performance.push_back(success);
        if self.recent_performance.len() > RECENT_HISTORY_SIZE {
            self.recent_performance.pop_front();
        }
        
        self.recovery_times.push(recovery_time);
        self.avg_recovery_time = Duration::from_secs_f64(
            self.recovery_times
                .iter()
                .map(|d| d.as_secs_f64())
                .sum::<f64>()
                / self.recovery_times.len() as f64,
        );
        
        self.last_attempt = Some(Instant::now());
        self.score = self.calculate_score();
    }
    
    fn calculate_score(&self) -> f32 {
        if self.attempts == 0 {
            return 0.0;
        }
        
        let success_rate = self.successes as f32 / self.attempts as f32;
        
        let speed_score = if self.avg_recovery_time.as_secs_f64() > 0.0 {
            (1.0 / self.avg_recovery_time.as_secs_f64()).min(1.0) as f32
        } else {
            1.0
        };
        
        let recency_weight = if !self.recent_performance.is_empty() {
            let recent_successes = self.recent_performance.iter().filter(|&&s| s).count();
            recent_successes as f32 / self.recent_performance.len() as f32
        } else {
            success_rate
        };
        
        // Weighted combination
        success_rate * 0.5 + speed_score * 0.3 + recency_weight * 0.2
    }
    
    fn success_rate(&self) -> f32 {
        if self.attempts == 0 {
            0.0
        } else {
            self.successes as f32 / self.attempts as f32
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetrics {
    pub strategies: HashMap<Strategy, StrategyStats>,
    pub avg_latency: Duration,
    pub packet_loss_rate: f32,
    pub connection_drop_pattern: DropPattern,
    pub total_attempts: u64,
    pub last_updated: SystemTime,
    pub confidence_score: f32,
    pub server_hash: String,
}

impl ServerMetrics {
    fn new(server_url: &str) -> Self {
        let mut strategies = HashMap::new();
        for strategy in Strategy::all() {
            strategies.insert(*strategy, StrategyStats::default());
        }
        
        Self {
            strategies,
            avg_latency: Duration::ZERO,
            packet_loss_rate: 0.0,
            connection_drop_pattern: DropPattern::None,
            total_attempts: 0,
            last_updated: SystemTime::now(),
            confidence_score: 0.0,
            server_hash: Self::hash_server_url(server_url),
        }
    }
    
    fn hash_server_url(url: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
    
    fn update_confidence(&mut self) {
        // Confidence increases with more data points
        let data_points = self.total_attempts as f32;
        let base_confidence = (data_points / (data_points + 10.0)).min(0.9);
        
        // Adjust based on consistency of results
        let consistency_factor = self.calculate_consistency();
        
        self.confidence_score = base_confidence * consistency_factor;
    }
    
    fn calculate_consistency(&self) -> f32 {
        let scores: Vec<f32> = self.strategies.values().map(|s| s.score).collect();
        if scores.is_empty() {
            return 0.0;
        }
        
        let mean = scores.iter().sum::<f32>() / scores.len() as f32;
        let variance = scores
            .iter()
            .map(|s| (s - mean).powi(2))
            .sum::<f32>()
            / scores.len() as f32;
        
        // Lower variance means more consistent results
        1.0 / (1.0 + variance)
    }
    
    fn detect_network_change(&self) -> bool {
        for stats in self.strategies.values() {
            if stats.recent_performance.len() < 10 {
                continue;
            }
            
            let recent_rate = stats
                .recent_performance
                .iter()
                .rev()
                .take(10)
                .filter(|&&s| s)
                .count() as f32
                / 10.0;
            
            let historical_rate = stats.success_rate();
            
            if (recent_rate - historical_rate).abs() > CHANGE_DETECTION_THRESHOLD {
                return true;
            }
        }
        
        false
    }
}

pub struct AdaptiveRetryConfig {
    pub enabled: bool,
    pub persistence: bool,
    pub cache_ttl: Duration,
    pub discovery_time: Duration,
    pub exploration_rate: f32,
    pub confidence_threshold: f32,
    pub change_detection: bool,
}

impl Default for AdaptiveRetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            persistence: true,
            cache_ttl: Duration::from_secs(CACHE_TTL_DAYS * 24 * 3600),
            discovery_time: Duration::from_secs(DISCOVERY_DURATION_SECS),
            exploration_rate: DEFAULT_EXPLORATION_RATE,
            confidence_threshold: CONFIDENCE_THRESHOLD,
            change_detection: true,
        }
    }
}

pub struct AdaptiveRetryManager {
    config: AdaptiveRetryConfig,
    metrics: Arc<Mutex<ServerMetrics>>,
    phase: Phase,
    discovery_started: Option<Instant>,
    current_strategy: Option<Strategy>,
    untested_strategies: Vec<Strategy>,
    exploration_rate: f32,
}

impl AdaptiveRetryManager {
    pub fn new(server_url: &str, config: AdaptiveRetryConfig) -> Self {
        let metrics = if config.persistence {
            Self::load_metrics(server_url).unwrap_or_else(|| ServerMetrics::new(server_url))
        } else {
            ServerMetrics::new(server_url)
        };
        
        let phase = Phase::Discovery;
        let untested_strategies = Strategy::all().to_vec();
        
        Self {
            config,
            metrics: Arc::new(Mutex::new(metrics)),
            phase,
            discovery_started: None,
            current_strategy: None,
            untested_strategies,
            exploration_rate: DEFAULT_EXPLORATION_RATE,
        }
    }
    
    pub fn select_strategy(&mut self) -> Strategy {
        // Update phase if needed
        self.update_phase();
        
        let strategy = match self.phase {
            Phase::Discovery => self.select_discovery_strategy(),
            Phase::Exploitation => self.select_best_strategy(),
            Phase::Exploration => self.select_exploration_strategy(),
            Phase::MiniDiscovery(_) => self.select_discovery_strategy(),
        };
        
        self.current_strategy = Some(strategy);
        strategy
    }
    
    fn update_phase(&mut self) {
        match self.phase {
            Phase::Discovery => {
                if self.discovery_started.is_none() {
                    self.discovery_started = Some(Instant::now());
                }
                
                if let Some(start) = self.discovery_started {
                    if start.elapsed() > self.config.discovery_time {
                        self.phase = Phase::Exploitation;
                    }
                }
            }
            Phase::MiniDiscovery(duration) => {
                if let Some(start) = self.discovery_started {
                    if start.elapsed() > duration {
                        self.phase = Phase::Exploitation;
                        self.exploration_rate = self.config.exploration_rate;
                    }
                }
            }
            Phase::Exploitation | Phase::Exploration => {
                if self.config.change_detection {
                    let metrics = self.metrics.lock().unwrap();
                    if metrics.detect_network_change() {
                        drop(metrics);
                        self.adapt_to_change();
                    }
                }
            }
        }
    }
    
    fn select_discovery_strategy(&mut self) -> Strategy {
        if let Some(strategy) = self.untested_strategies.pop() {
            strategy
        } else {
            // All strategies tested, move to exploitation
            self.phase = Phase::Exploitation;
            self.select_best_strategy()
        }
    }
    
    fn select_best_strategy(&self) -> Strategy {
        let metrics = self.metrics.lock().unwrap();
        
        // Thompson Sampling
        let mut best_sample = f32::MIN;
        let mut best_strategy = Strategy::ExponentialJitter;
        
        for (strategy, stats) in &metrics.strategies {
            let sample = self.sample_beta(stats.successes + 1, stats.attempts - stats.successes + 1);
            
            if sample > best_sample {
                best_sample = sample;
                best_strategy = *strategy;
            }
        }
        
        best_strategy
    }
    
    fn select_exploration_strategy(&self) -> Strategy {
        let mut rng = rand::rng();
        
        if rng.random::<f32>() < self.exploration_rate {
            // Explore: randomly select a strategy
            let strategies = Strategy::all();
            strategies[rng.random_range(0..strategies.len())]
        } else {
            // Exploit: use best known strategy
            self.select_best_strategy()
        }
    }
    
    fn sample_beta(&self, alpha: u64, beta: u64) -> f32 {
        // Simplified Beta sampling using uniform random
        // For production, use a proper Beta distribution
        let mut rng = rand::rng();
        let alpha_f = alpha as f32;
        let beta_f = beta as f32;
        
        // Approximate Beta using uniform random (simplified)
        let u: f32 = rng.random();
        u * (alpha_f / (alpha_f + beta_f)) + (1.0 - u) * 0.5
    }
    
    fn adapt_to_change(&mut self) {
        let mut metrics = self.metrics.lock().unwrap();
        
        // Reduce confidence in current model
        metrics.confidence_score *= 0.5;
        
        // Increase exploration rate temporarily
        self.exploration_rate = 0.3;
        
        // Re-enter mini discovery phase
        self.phase = Phase::MiniDiscovery(Duration::from_secs(10));
        self.discovery_started = Some(Instant::now());
        
        // Reset untested strategies for mini-discovery
        self.untested_strategies = Strategy::all().to_vec();
    }
    
    pub fn record_attempt(&self, strategy: Strategy, success: bool, recovery_time: Duration) {
        let mut metrics = self.metrics.lock().unwrap();
        
        if let Some(stats) = metrics.strategies.get_mut(&strategy) {
            stats.record_attempt(success, recovery_time);
        }
        
        metrics.total_attempts += 1;
        metrics.last_updated = SystemTime::now();
        metrics.update_confidence();
        
        // Persist if enabled
        if self.config.persistence {
            let _ = self.persist_metrics(&metrics);
        }
    }
    
    pub fn get_current_confidence(&self) -> f32 {
        self.metrics.lock().unwrap().confidence_score
    }
    
    pub fn get_best_strategy(&self) -> Strategy {
        let metrics = self.metrics.lock().unwrap();
        
        metrics
            .strategies
            .iter()
            .max_by(|a, b| a.1.score.partial_cmp(&b.1.score).unwrap())
            .map(|(s, _)| *s)
            .unwrap_or(Strategy::ExponentialJitter)
    }
    
    fn cache_dir() -> Option<PathBuf> {
        dirs::cache_dir().map(|d| d.join("gstreamer").join("rtspsrc2"))
    }
    
    fn load_metrics(server_url: &str) -> Option<ServerMetrics> {
        let cache_dir = Self::cache_dir()?;
        let server_hash = ServerMetrics::hash_server_url(server_url);
        let cache_file = cache_dir.join(format!("{}.json", server_hash));
        
        if !cache_file.exists() {
            return None;
        }
        
        // Check if cache is too old
        let metadata = fs::metadata(&cache_file).ok()?;
        let modified = metadata.modified().ok()?;
        let age = SystemTime::now().duration_since(modified).ok()?;
        
        if age > Duration::from_secs(CACHE_TTL_DAYS * 24 * 3600) {
            return None;
        }
        
        let data = fs::read_to_string(&cache_file).ok()?;
        serde_json::from_str(&data).ok()
    }
    
    fn persist_metrics(&self, metrics: &ServerMetrics) -> Result<(), std::io::Error> {
        let cache_dir = Self::cache_dir().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Cache directory not found")
        })?;
        
        fs::create_dir_all(&cache_dir)?;
        
        let cache_file = cache_dir.join(format!("{}.json", metrics.server_hash));
        let data = serde_json::to_string_pretty(metrics)?;
        fs::write(cache_file, data)?;
        
        Ok(())
    }
    
    pub fn get_stats_summary(&self) -> String {
        let metrics = self.metrics.lock().unwrap();
        let mut summary = String::new();
        
        summary.push_str(&format!("Total attempts: {}\n", metrics.total_attempts));
        summary.push_str(&format!("Confidence: {:.2}\n", metrics.confidence_score));
        summary.push_str(&format!("Phase: {:?}\n", self.phase));
        
        for (strategy, stats) in &metrics.strategies {
            summary.push_str(&format!(
                "{:?}: {} attempts, {:.0}% success, score: {:.2}\n",
                strategy,
                stats.attempts,
                stats.success_rate() * 100.0,
                stats.score
            ));
        }
        
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_strategy_stats_scoring() {
        let mut stats = StrategyStats::default();
        
        // Record some successful attempts
        stats.record_attempt(true, Duration::from_millis(100));
        stats.record_attempt(true, Duration::from_millis(150));
        stats.record_attempt(false, Duration::from_millis(200));
        stats.record_attempt(true, Duration::from_millis(120));
        
        assert_eq!(stats.attempts, 4);
        assert_eq!(stats.successes, 3);
        assert!(stats.score > 0.5);
        assert_eq!(stats.success_rate(), 0.75);
    }
    
    #[test]
    fn test_server_metrics_consistency() {
        let mut metrics = ServerMetrics::new("rtsp://test.server");
        
        // Add some data to strategies
        for strategy in Strategy::all() {
            if let Some(stats) = metrics.strategies.get_mut(strategy) {
                stats.record_attempt(true, Duration::from_millis(100));
                stats.record_attempt(true, Duration::from_millis(100));
                metrics.total_attempts += 2;
            }
        }
        
        metrics.update_confidence();
        assert!(metrics.confidence_score > 0.0);
        
        let consistency = metrics.calculate_consistency();
        assert!(consistency > 0.8); // Should be high since all strategies have same performance
    }
    
    #[test]
    fn test_network_change_detection() {
        let mut metrics = ServerMetrics::new("rtsp://test.server");
        
        // Establish baseline with good performance
        if let Some(stats) = metrics.strategies.get_mut(&Strategy::Exponential) {
            for _ in 0..20 {
                stats.record_attempt(true, Duration::from_millis(100));
            }
        }
        
        assert!(!metrics.detect_network_change());
        
        // Now add recent failures
        if let Some(stats) = metrics.strategies.get_mut(&Strategy::Exponential) {
            for _ in 0..15 {
                stats.record_attempt(false, Duration::from_millis(500));
            }
        }
        
        assert!(metrics.detect_network_change());
    }
    
    #[test]
    fn test_thompson_sampling() {
        let config = AdaptiveRetryConfig::default();
        let mut manager = AdaptiveRetryManager::new("rtsp://test.server", config);
        
        // Record different performance for different strategies
        manager.record_attempt(Strategy::Immediate, false, Duration::from_millis(50));
        manager.record_attempt(Strategy::Linear, true, Duration::from_millis(200));
        manager.record_attempt(Strategy::Exponential, true, Duration::from_millis(150));
        manager.record_attempt(Strategy::ExponentialJitter, true, Duration::from_millis(100));
        
        // Force exploitation phase
        manager.phase = Phase::Exploitation;
        
        // Should tend to select better performing strategies
        let selected = manager.select_strategy();
        assert!(selected != Strategy::Immediate); // Should avoid the failed strategy
    }
    
    #[test]
    fn test_discovery_phase() {
        let mut config = AdaptiveRetryConfig::default();
        config.discovery_time = Duration::from_millis(100);
        
        let mut manager = AdaptiveRetryManager::new("rtsp://test.server", config);
        
        // Should cycle through all strategies during discovery
        let mut strategies_seen = Vec::new();
        for _ in 0..Strategy::all().len() {
            let strategy = manager.select_strategy();
            strategies_seen.push(strategy);
        }
        
        // Should have tried each strategy at least once
        for strategy in Strategy::all() {
            assert!(strategies_seen.contains(strategy));
        }
    }
    
    #[test]
    fn test_cache_hash() {
        let hash1 = ServerMetrics::hash_server_url("rtsp://server1.com");
        let hash2 = ServerMetrics::hash_server_url("rtsp://server2.com");
        let hash3 = ServerMetrics::hash_server_url("rtsp://server1.com");
        
        assert_ne!(hash1, hash2);
        assert_eq!(hash1, hash3);
    }
}
