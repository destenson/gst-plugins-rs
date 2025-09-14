#![allow(unused)]
// GStreamer RTSP plugin parallel connection racing implementation
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use crate::rtspsrc::proxy::{ProxyConfig, ProxyConnection};
use futures::future::select_all;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::{sleep, timeout};
use gst::prelude::*;
use super::debug::{DecisionHistory, DecisionType, CAT_RACING};
use crate::debug_decision;

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "rtspsrc2-racer",
        gst::DebugColorFlags::empty(),
        Some("RTSP connection racing"),
    )
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionRacingStrategy {
    None,
    FirstWins,
    LastWins,
    Hybrid,
}

impl Default for ConnectionRacingStrategy {
    fn default() -> Self {
        ConnectionRacingStrategy::None
    }
}

impl ConnectionRacingStrategy {
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "none" => ConnectionRacingStrategy::None,
            "first-wins" | "first_wins" | "happy-eyeballs" => ConnectionRacingStrategy::FirstWins,
            "last-wins" | "last_wins" => ConnectionRacingStrategy::LastWins,
            "hybrid" => ConnectionRacingStrategy::Hybrid,
            _ => ConnectionRacingStrategy::None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionRacingStrategy::None => "none",
            ConnectionRacingStrategy::FirstWins => "first-wins",
            ConnectionRacingStrategy::LastWins => "last-wins",
            ConnectionRacingStrategy::Hybrid => "hybrid",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionRacingConfig {
    pub strategy: ConnectionRacingStrategy,
    pub max_parallel_connections: u32,
    pub racing_delay_ms: u32,
    pub racing_timeout: Duration,
    pub proxy_config: Option<ProxyConfig>,
}

impl Default for ConnectionRacingConfig {
    fn default() -> Self {
        Self {
            strategy: ConnectionRacingStrategy::None,
            max_parallel_connections: 3,
            racing_delay_ms: 250,
            racing_timeout: Duration::from_secs(5),
            proxy_config: None,
        }
    }
}

pub struct ConnectionRacer {
    config: ConnectionRacingConfig,
    decision_history: Option<DecisionHistory>,
}

impl ConnectionRacer {
    pub fn new(config: ConnectionRacingConfig) -> Self {
        Self { 
            config,
            decision_history: Some(DecisionHistory::default()),
        }
    }

    /// Update the racing strategy dynamically
    pub fn update_strategy(&mut self, strategy: ConnectionRacingStrategy) {
        if self.config.strategy != strategy {
            let reason = match strategy {
                ConnectionRacingStrategy::FirstWins => "High packet loss detected",
                ConnectionRacingStrategy::LastWins => "Connection-limited device detected",
                ConnectionRacingStrategy::Hybrid => "Mixed network conditions",
                ConnectionRacingStrategy::None => "Stable network detected",
            };
            
            debug_decision!(
                CAT_RACING,
                self.decision_history.as_ref(),
                DecisionType::RacingModeUpdate {
                    mode: strategy.as_str().to_string(),
                    reason: reason.to_string(),
                },
                "Racing strategy updated: {} -> {} ({})",
                self.config.strategy.as_str(),
                strategy.as_str(),
                reason
            );
            
            gst::info!(
                CAT,
                "Racing strategy changed from {:?} to {:?}",
                self.config.strategy,
                strategy
            );
            self.config.strategy = strategy;
        }
    }

    /// Get the current racing strategy
    pub fn current_strategy(&self) -> ConnectionRacingStrategy {
        self.config.strategy
    }

    /// Attempt to connect using the configured racing strategy
    pub async fn connect(&self, hostname_port: &str) -> Result<TcpStream, std::io::Error> {
        match self.config.strategy {
            ConnectionRacingStrategy::None => {
                // Simple single connection attempt
                gst::debug!(CAT, "Using no racing strategy, single connection attempt");
                self.connect_with_proxy(hostname_port).await
            }
            ConnectionRacingStrategy::FirstWins => self.connect_first_wins(hostname_port).await,
            ConnectionRacingStrategy::LastWins => self.connect_last_wins(hostname_port).await,
            ConnectionRacingStrategy::Hybrid => {
                // Try first-wins first, if that fails try last-wins
                gst::debug!(CAT, "Using hybrid strategy");
                match self.connect_first_wins(hostname_port).await {
                    Ok(stream) => Ok(stream),
                    Err(_) => {
                        gst::debug!(CAT, "First-wins failed, trying last-wins strategy");
                        self.connect_last_wins(hostname_port).await
                    }
                }
            }
        }
    }

    /// First-wins strategy (Happy Eyeballs)
    /// Launch multiple connections with staggered delays, use first successful
    async fn connect_first_wins(&self, hostname_port: &str) -> Result<TcpStream, std::io::Error> {
        gst::debug!(
            CAT,
            "Using first-wins racing strategy with {} parallel connections",
            self.config.max_parallel_connections
        );

        let mut futures = Vec::new();
        let mut handles = Vec::new();

        for i in 0..self.config.max_parallel_connections {
            let hostname_port = hostname_port.to_string();
            let delay = Duration::from_millis((i * self.config.racing_delay_ms) as u64);
            let racing_timeout = self.config.racing_timeout;

            let proxy_config = self.config.proxy_config.clone();
            let handle = tokio::spawn(async move {
                if i > 0 {
                    sleep(delay).await;
                }
                gst::trace!(
                    CAT,
                    "Starting connection attempt {} after {}ms delay",
                    i + 1,
                    delay.as_millis()
                );
                timeout(
                    racing_timeout,
                    Self::connect_with_proxy_static(&hostname_port, &proxy_config),
                )
                .await
            });

            handles.push(handle);
        }

        // Convert handles to futures
        for handle in handles {
            futures.push(handle);
        }

        // Race all connections
        while !futures.is_empty() {
            let (result, _index, remaining) = select_all(futures).await;
            futures = remaining;

            match result {
                Ok(Ok(Ok(stream))) => {
                    gst::debug!(
                        CAT,
                        "First-wins: connection successful, cancelling other attempts"
                    );
                    // Cancel remaining futures
                    for future in futures {
                        future.abort();
                    }
                    return Ok(stream);
                }
                Ok(Ok(Err(e))) => {
                    gst::trace!(CAT, "First-wins: connection attempt failed: {}", e);
                    // Continue with remaining futures
                }
                Ok(Err(e)) => {
                    gst::trace!(CAT, "First-wins: connection attempt timed out: {}", e);
                    // Continue with remaining futures
                }
                Err(e) => {
                    gst::trace!(CAT, "First-wins: task join error: {}", e);
                    // Continue with remaining futures
                }
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "All connection attempts failed in first-wins racing",
        ))
    }

    /// Last-wins strategy
    /// For devices that drop older connections, use the newest successful connection
    async fn connect_last_wins(&self, hostname_port: &str) -> Result<TcpStream, std::io::Error> {
        gst::debug!(
            CAT,
            "Using last-wins racing strategy with {} parallel connections",
            self.config.max_parallel_connections
        );

        let mut futures = Vec::new();
        let mut handles = Vec::new();

        for i in 0..self.config.max_parallel_connections {
            let hostname_port = hostname_port.to_string();
            let delay = Duration::from_millis((i * self.config.racing_delay_ms) as u64);
            let racing_timeout = self.config.racing_timeout;

            let proxy_config = self.config.proxy_config.clone();
            let handle = tokio::spawn(async move {
                if i > 0 {
                    sleep(delay).await;
                }
                gst::trace!(
                    CAT,
                    "Starting connection attempt {} after {}ms delay",
                    i + 1,
                    delay.as_millis()
                );
                timeout(
                    racing_timeout,
                    Self::connect_with_proxy_static(&hostname_port, &proxy_config),
                )
                .await
            });

            handles.push(handle);
        }

        // Convert handles to futures
        for handle in handles {
            futures.push(handle);
        }

        let mut last_successful: Option<TcpStream> = None;

        // Collect all results, keeping the last successful one
        while !futures.is_empty() {
            let (result, _index, remaining) = select_all(futures).await;
            futures = remaining;

            match result {
                Ok(Ok(Ok(stream))) => {
                    gst::debug!(
                        CAT,
                        "Last-wins: new successful connection, replacing previous"
                    );
                    // Drop the old connection if we have one
                    if let Some(old_stream) = last_successful.take() {
                        drop(old_stream);
                    }
                    last_successful = Some(stream);
                }
                Ok(Ok(Err(e))) => {
                    gst::trace!(CAT, "Last-wins: connection attempt failed: {}", e);
                }
                Ok(Err(e)) => {
                    gst::trace!(CAT, "Last-wins: connection attempt timed out: {}", e);
                }
                Err(e) => {
                    gst::trace!(CAT, "Last-wins: task join error: {}", e);
                }
            }
        }

        if let Some(stream) = last_successful {
            gst::debug!(CAT, "Last-wins: using final successful connection");
            Ok(stream)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                "No successful connections in last-wins racing",
            ))
        }
    }

    /// Connect with proxy support
    async fn connect_with_proxy(&self, hostname_port: &str) -> Result<TcpStream, std::io::Error> {
        Self::connect_with_proxy_static(hostname_port, &self.config.proxy_config).await
    }

    /// Static version for use in spawned tasks
    async fn connect_with_proxy_static(
        hostname_port: &str,
        proxy_config: &Option<ProxyConfig>,
    ) -> Result<TcpStream, std::io::Error> {
        // Parse hostname and port from "host:port" string
        let parts: Vec<&str> = hostname_port.split(':').collect();
        if parts.len() != 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid hostname:port format: {}", hostname_port),
            ));
        }
        let host = parts[0];
        let port = parts[1]
            .parse::<u16>()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        // Connect through proxy if configured, otherwise direct connection
        if let Some(proxy) = proxy_config {
            ProxyConnection::connect(proxy, host, port)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        } else {
            ProxyConnection::connect_direct(host, port)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_from_string() {
        assert_eq!(
            ConnectionRacingStrategy::from_string("none"),
            ConnectionRacingStrategy::None
        );
        assert_eq!(
            ConnectionRacingStrategy::from_string("first-wins"),
            ConnectionRacingStrategy::FirstWins
        );
        assert_eq!(
            ConnectionRacingStrategy::from_string("first_wins"),
            ConnectionRacingStrategy::FirstWins
        );
        assert_eq!(
            ConnectionRacingStrategy::from_string("happy-eyeballs"),
            ConnectionRacingStrategy::FirstWins
        );
        assert_eq!(
            ConnectionRacingStrategy::from_string("last-wins"),
            ConnectionRacingStrategy::LastWins
        );
        assert_eq!(
            ConnectionRacingStrategy::from_string("last_wins"),
            ConnectionRacingStrategy::LastWins
        );
        assert_eq!(
            ConnectionRacingStrategy::from_string("hybrid"),
            ConnectionRacingStrategy::Hybrid
        );
        assert_eq!(
            ConnectionRacingStrategy::from_string("unknown"),
            ConnectionRacingStrategy::None
        );
    }

    #[test]
    fn test_strategy_as_str() {
        assert_eq!(ConnectionRacingStrategy::None.as_str(), "none");
        assert_eq!(ConnectionRacingStrategy::FirstWins.as_str(), "first-wins");
        assert_eq!(ConnectionRacingStrategy::LastWins.as_str(), "last-wins");
        assert_eq!(ConnectionRacingStrategy::Hybrid.as_str(), "hybrid");
    }

    #[test]
    fn test_config_defaults() {
        let config = ConnectionRacingConfig::default();
        assert_eq!(config.strategy, ConnectionRacingStrategy::None);
        assert_eq!(config.max_parallel_connections, 3);
        assert_eq!(config.racing_delay_ms, 250);
        assert_eq!(config.racing_timeout, Duration::from_secs(5));
        assert!(config.proxy_config.is_none());
    }
}
