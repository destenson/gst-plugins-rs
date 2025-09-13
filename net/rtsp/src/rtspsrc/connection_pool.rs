#![allow(unused)]
// TCP Connection pooling for reducing connection overhead
//
// This module provides connection pooling to reuse TCP connections
// when connecting to multiple streams from the same server

use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time;

// Configuration constants
const DEFAULT_MAX_CONNECTIONS_PER_SERVER: usize = 4;
const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const CONNECTION_HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    pub enabled: bool,
    pub max_connections_per_server: usize,
    pub idle_timeout: Duration,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_connections_per_server: DEFAULT_MAX_CONNECTIONS_PER_SERVER,
            idle_timeout: DEFAULT_IDLE_TIMEOUT,
        }
    }
}

// Connection wrapper with metadata
struct PooledConnection {
    stream: Option<TcpStream>,
    server_addr: SocketAddr,
    created_at: Instant,
    last_used: Instant,
    use_count: usize,
    is_healthy: bool,
}

impl PooledConnection {
    fn new(stream: TcpStream, server_addr: SocketAddr) -> Self {
        let now = Instant::now();
        Self {
            stream: Some(stream),
            server_addr,
            created_at: now,
            last_used: now,
            use_count: 0,
            is_healthy: true,
        }
    }

    fn checkout(&mut self) -> Option<TcpStream> {
        self.last_used = Instant::now();
        self.use_count += 1;
        self.stream.take()
    }

    fn checkin(&mut self, stream: TcpStream) {
        self.last_used = Instant::now();
        self.stream = Some(stream);
    }

    fn is_idle_expired(&self, timeout: Duration) -> bool {
        self.stream.is_some() && self.last_used.elapsed() > timeout
    }
}

// Connection pool for a specific server
struct ServerPool {
    connections: Vec<PooledConnection>,
    max_connections: usize,
}

impl ServerPool {
    fn new(max_connections: usize) -> Self {
        Self {
            connections: Vec::with_capacity(max_connections),
            max_connections,
        }
    }

    fn checkout(&mut self) -> Option<TcpStream> {
        // Find an available healthy connection
        for conn in &mut self.connections {
            if conn.stream.is_some() && conn.is_healthy {
                return conn.checkout();
            }
        }
        None
    }

    fn checkin(&mut self, stream: TcpStream, server_addr: SocketAddr) -> Result<()> {
        // Try to find the connection slot this stream came from
        for conn in &mut self.connections {
            if conn.stream.is_none() && conn.server_addr == server_addr {
                conn.checkin(stream);
                return Ok(());
            }
        }

        // If not found and we have room, create a new slot
        if self.connections.len() < self.max_connections {
            self.connections
                .push(PooledConnection::new(stream, server_addr));
            Ok(())
        } else {
            // Pool is full, don't accept the connection
            Err(anyhow!("Connection pool full for server"))
        }
    }

    fn add_new(&mut self, stream: TcpStream, server_addr: SocketAddr) -> Result<()> {
        if self.connections.len() >= self.max_connections {
            return Err(anyhow!("Connection pool full for server"));
        }
        self.connections
            .push(PooledConnection::new(stream, server_addr));
        Ok(())
    }

    fn cleanup_idle(&mut self, timeout: Duration) {
        self.connections
            .retain(|conn| !conn.is_idle_expired(timeout));
    }

    fn health_check_all(&mut self) {
        for conn in &mut self.connections {
            // Simple synchronous health check
            if let Some(ref stream) = conn.stream {
                let mut buf = [0u8; 1];
                match stream.try_read(&mut buf) {
                    Ok(0) => {
                        // Connection closed by peer
                        conn.is_healthy = false;
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No data available, connection is healthy
                        conn.is_healthy = true;
                    }
                    Err(_) => {
                        // Some other error, mark as unhealthy
                        conn.is_healthy = false;
                    }
                    _ => {}
                }
            }
        }
    }

    fn remove_unhealthy(&mut self) {
        self.connections.retain(|conn| conn.is_healthy);
    }

    fn stats(&self) -> PoolStats {
        PoolStats {
            total_connections: self.connections.len(),
            available_connections: self
                .connections
                .iter()
                .filter(|c| c.stream.is_some())
                .count(),
            checked_out_connections: self
                .connections
                .iter()
                .filter(|c| c.stream.is_none())
                .count(),
            total_uses: self.connections.iter().map(|c| c.use_count).sum(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_connections: usize,
    pub available_connections: usize,
    pub checked_out_connections: usize,
    pub total_uses: usize,
}

pub struct ConnectionPool {
    pools: Arc<Mutex<HashMap<SocketAddr, ServerPool>>>,
    config: ConnectionPoolConfig,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl ConnectionPool {
    pub fn new(config: ConnectionPoolConfig) -> Self {
        let pools: Arc<Mutex<HashMap<SocketAddr, ServerPool>>> =
            Arc::new(Mutex::new(HashMap::new()));

        if config.enabled {
            // Start background cleanup task
            let pools_clone = Arc::downgrade(&pools);
            let idle_timeout = config.idle_timeout;
            let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

            tokio::spawn(async move {
                let mut interval = time::interval(CONNECTION_HEALTH_CHECK_INTERVAL);

                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if let Some(pools) = pools_clone.upgrade() {
                                // Collect work to do outside the lock
                                let work_items: Vec<SocketAddr> = {
                                    let pools = pools.lock().unwrap();
                                    pools.keys().cloned().collect()
                                };

                                // Process each pool without holding the main lock
                                for addr in work_items {
                                    let mut pools = pools.lock().unwrap();
                                    if let Some(pool) = pools.get_mut(&addr) {
                                        pool.cleanup_idle(idle_timeout);
                                        pool.health_check_all();
                                        pool.remove_unhealthy();
                                    }
                                }
                            } else {
                                // Pool has been dropped, exit
                                break;
                            }
                        }
                        _ = shutdown_rx.recv() => {
                            break;
                        }
                    }
                }
            });

            Self {
                pools,
                config,
                shutdown_tx: Some(shutdown_tx),
            }
        } else {
            Self {
                pools,
                config,
                shutdown_tx: None,
            }
        }
    }

    // Checkout a connection from the pool
    pub async fn checkout(&self, server_addr: SocketAddr) -> Option<TcpStream> {
        if !self.config.enabled {
            return None;
        }

        let mut pools = self.pools.lock().unwrap();

        if let Some(pool) = pools.get_mut(&server_addr) {
            pool.checkout()
        } else {
            None
        }
    }

    // Return a connection to the pool
    pub async fn checkin(&self, stream: TcpStream, server_addr: SocketAddr) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut pools = self.pools.lock().unwrap();

        let pool = pools
            .entry(server_addr)
            .or_insert_with(|| ServerPool::new(self.config.max_connections_per_server));

        pool.checkin(stream, server_addr)
    }

    // Add a new connection to the pool
    pub async fn add_new(&self, stream: TcpStream, server_addr: SocketAddr) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let mut pools = self.pools.lock().unwrap();

        let pool = pools
            .entry(server_addr)
            .or_insert_with(|| ServerPool::new(self.config.max_connections_per_server));

        pool.add_new(stream, server_addr)
    }

    // Get statistics for all pools
    pub fn stats(&self) -> HashMap<SocketAddr, PoolStats> {
        let pools = self.pools.lock().unwrap();
        pools
            .iter()
            .map(|(addr, pool)| (*addr, pool.stats()))
            .collect()
    }

    // Clear all connections
    pub fn clear(&self) {
        let mut pools = self.pools.lock().unwrap();
        pools.clear();
    }

    // Shutdown the pool and cleanup task
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        self.clear();
    }
}

// Global connection pool instance
use std::sync::LazyLock;

static GLOBAL_CONNECTION_POOL: LazyLock<Arc<ConnectionPool>> =
    LazyLock::new(|| Arc::new(ConnectionPool::new(ConnectionPoolConfig::default())));

// Convenience functions for global pool access
pub async fn checkout_connection(server_addr: SocketAddr) -> Option<TcpStream> {
    GLOBAL_CONNECTION_POOL.checkout(server_addr).await
}

pub async fn checkin_connection(stream: TcpStream, server_addr: SocketAddr) -> Result<()> {
    GLOBAL_CONNECTION_POOL.checkin(stream, server_addr).await
}

pub async fn add_new_connection(stream: TcpStream, server_addr: SocketAddr) -> Result<()> {
    GLOBAL_CONNECTION_POOL.add_new(stream, server_addr).await
}

pub fn connection_pool_stats() -> HashMap<SocketAddr, PoolStats> {
    GLOBAL_CONNECTION_POOL.stats()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn test_connection_pool_basic() {
        let config = ConnectionPoolConfig {
            enabled: true,
            max_connections_per_server: 2,
            idle_timeout: Duration::from_secs(5),
        };

        let pool = ConnectionPool::new(config);

        // Start a test server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Accept connections in background
        tokio::spawn(async move {
            loop {
                if listener.accept().await.is_err() {
                    break;
                }
            }
        });

        // Create and add a connection
        let stream = TcpStream::connect(addr).await.unwrap();
        pool.add_new(stream, addr).await.unwrap();

        // Checkout should return the connection
        let stream = pool.checkout(addr).await;
        assert!(stream.is_some());

        // Return it to the pool
        pool.checkin(stream.unwrap(), addr).await.unwrap();

        // Should be able to checkout again
        let stream = pool.checkout(addr).await;
        assert!(stream.is_some());
    }

    #[tokio::test]
    async fn test_connection_pool_limits() {
        let config = ConnectionPoolConfig {
            enabled: true,
            max_connections_per_server: 2,
            idle_timeout: Duration::from_secs(5),
        };

        let pool = ConnectionPool::new(config);

        // Start a test server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Accept connections in background
        tokio::spawn(async move {
            loop {
                if listener.accept().await.is_err() {
                    break;
                }
            }
        });

        // Add max connections
        for _ in 0..2 {
            let stream = TcpStream::connect(addr).await.unwrap();
            pool.add_new(stream, addr).await.unwrap();
        }

        // Adding one more should fail
        let stream = TcpStream::connect(addr).await.unwrap();
        let result = pool.add_new(stream, addr).await;
        assert!(result.is_err());
    }
}
