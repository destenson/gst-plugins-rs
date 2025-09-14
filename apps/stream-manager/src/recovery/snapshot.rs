#![allow(unused)]
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{debug, info};

#[derive(Debug, Clone)]
pub struct ComponentSnapshot {
    pub component_id: String,
    pub timestamp: Instant,
    pub system_time: SystemTime,
    pub data: Vec<u8>,
    pub metadata: HashMap<String, String>,
}


impl ComponentSnapshot {
    pub fn new(component_id: String, data: Vec<u8>) -> Self {
        Self {
            component_id,
            timestamp: Instant::now(),
            system_time: SystemTime::now(),
            data,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn age(&self) -> Duration {
        self.timestamp.elapsed()
    }

    pub fn is_stale(&self, max_age: Duration) -> bool {
        self.age() > max_age
    }

    pub fn size_bytes(&self) -> usize {
        self.data.len() + 
        self.component_id.len() + 
        self.metadata.iter()
            .map(|(k, v)| k.len() + v.len())
            .sum::<usize>()
    }
}

#[derive(Debug)]
struct SnapshotStore {
    snapshots: HashMap<String, Vec<ComponentSnapshot>>,
    max_snapshots_per_component: usize,
    max_total_size: usize,
    current_size: usize,
}

impl SnapshotStore {
    fn new(max_snapshots_per_component: usize, max_total_size: usize) -> Self {
        Self {
            snapshots: HashMap::new(),
            max_snapshots_per_component,
            max_total_size,
            current_size: 0,
        }
    }

    fn store(&mut self, snapshot: ComponentSnapshot) {
        let size = snapshot.size_bytes();
        
        // Check if we need to make room
        while self.current_size + size > self.max_total_size && !self.snapshots.is_empty() {
            self.evict_oldest();
        }

        let component_snapshots = self.snapshots
            .entry(snapshot.component_id.clone())
            .or_insert_with(Vec::new);

        // Maintain max snapshots per component
        while component_snapshots.len() >= self.max_snapshots_per_component {
            let removed = component_snapshots.remove(0);
            self.current_size = self.current_size.saturating_sub(removed.size_bytes());
        }

        self.current_size += size;
        component_snapshots.push(snapshot);
    }

    fn get_latest(&self, component_id: &str) -> Option<&ComponentSnapshot> {
        self.snapshots
            .get(component_id)
            .and_then(|snapshots| snapshots.last())
    }

    fn get_all(&self, component_id: &str) -> Vec<&ComponentSnapshot> {
        self.snapshots
            .get(component_id)
            .map(|snapshots| snapshots.iter().collect())
            .unwrap_or_default()
    }

    fn evict_oldest(&mut self) {
        let oldest_component = self.snapshots
            .iter()
            .min_by_key(|(_, snapshots)| {
                snapshots.first()
                    .map(|s| s.timestamp)
                    .unwrap_or_else(Instant::now)
            })
            .map(|(id, _)| id.clone());

        if let Some(component_id) = oldest_component {
            if let Some(snapshots) = self.snapshots.get_mut(&component_id) {
                if !snapshots.is_empty() {
                    let removed = snapshots.remove(0);
                    self.current_size = self.current_size.saturating_sub(removed.size_bytes());
                    debug!(
                        component = %component_id,
                        age_secs = removed.age().as_secs(),
                        "Evicted oldest snapshot"
                    );
                }
                
                if snapshots.is_empty() {
                    self.snapshots.remove(&component_id);
                }
            }
        }
    }

    fn cleanup_stale(&mut self, max_age: Duration) {
        let mut components_to_clean = Vec::new();
        
        for (component_id, snapshots) in self.snapshots.iter_mut() {
            snapshots.retain(|snapshot| {
                let stale = snapshot.is_stale(max_age);
                if stale {
                    self.current_size = self.current_size.saturating_sub(snapshot.size_bytes());
                    debug!(
                        component = %snapshot.component_id,
                        age_secs = snapshot.age().as_secs(),
                        "Removed stale snapshot"
                    );
                }
                !stale
            });
            
            if snapshots.is_empty() {
                components_to_clean.push(component_id.clone());
            }
        }
        
        for component_id in components_to_clean {
            self.snapshots.remove(&component_id);
        }
    }

    fn get_stats(&self) -> SnapshotStats {
        let total_snapshots: usize = self.snapshots.values()
            .map(|v| v.len())
            .sum();
        
        SnapshotStats {
            total_components: self.snapshots.len(),
            total_snapshots,
            total_size_bytes: self.current_size,
            max_size_bytes: self.max_total_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SnapshotStats {
    pub total_components: usize,
    pub total_snapshots: usize,
    pub total_size_bytes: usize,
    pub max_size_bytes: usize,
}

pub struct SnapshotManager {
    store: Arc<RwLock<SnapshotStore>>,
    cleanup_interval: Duration,
    max_snapshot_age: Duration,
}

impl SnapshotManager {
    pub fn new(cleanup_interval: Duration) -> Self {
        let manager = Self {
            store: Arc::new(RwLock::new(SnapshotStore::new(
                10,  // max snapshots per component
                100 * 1024 * 1024,  // 100MB max total size
            ))),
            cleanup_interval,
            max_snapshot_age: Duration::from_secs(3600), // 1 hour
        };
        
        manager.start_cleanup_task();
        manager
    }

    pub fn with_limits(
        cleanup_interval: Duration,
        max_snapshots_per_component: usize,
        max_total_size: usize,
        max_snapshot_age: Duration,
    ) -> Self {
        let manager = Self {
            store: Arc::new(RwLock::new(SnapshotStore::new(
                max_snapshots_per_component,
                max_total_size,
            ))),
            cleanup_interval,
            max_snapshot_age,
        };
        
        manager.start_cleanup_task();
        manager
    }

    pub async fn store_snapshot(&self, snapshot: ComponentSnapshot) {
        let mut store = self.store.write().await;
        
        info!(
            component = %snapshot.component_id,
            size_bytes = snapshot.size_bytes(),
            "Storing component snapshot"
        );
        
        store.store(snapshot);
    }

    pub async fn get_latest_snapshot(&self, component_id: &str) -> Option<ComponentSnapshot> {
        let store = self.store.read().await;
        store.get_latest(component_id).cloned()
    }

    pub async fn get_all_snapshots(&self, component_id: &str) -> Vec<ComponentSnapshot> {
        let store = self.store.read().await;
        store.get_all(component_id)
            .into_iter()
            .cloned()
            .collect()
    }

    pub async fn rollback_to_snapshot(
        &self,
        component_id: &str,
        timestamp: Option<Instant>,
    ) -> Option<ComponentSnapshot> {
        let store = self.store.read().await;
        
        if let Some(timestamp) = timestamp {
            // Find snapshot closest to the specified timestamp
            store.get_all(component_id)
                .into_iter()
                .min_by_key(|s| {
                    let diff = if s.timestamp > timestamp {
                        s.timestamp - timestamp
                    } else {
                        timestamp - s.timestamp
                    };
                    diff.as_millis()
                })
                .cloned()
        } else {
            // Return the latest snapshot
            store.get_latest(component_id).cloned()
        }
    }

    pub async fn cleanup_stale_snapshots(&self) {
        let mut store = self.store.write().await;
        let before_stats = store.get_stats();
        
        store.cleanup_stale(self.max_snapshot_age);
        
        let after_stats = store.get_stats();
        
        if before_stats.total_snapshots != after_stats.total_snapshots {
            info!(
                removed = before_stats.total_snapshots - after_stats.total_snapshots,
                freed_bytes = before_stats.total_size_bytes - after_stats.total_size_bytes,
                "Cleaned up stale snapshots"
            );
        }
    }

    pub async fn get_stats(&self) -> SnapshotStats {
        let store = self.store.read().await;
        store.get_stats()
    }

    fn start_cleanup_task(&self) {
        let store = self.store.clone();
        let cleanup_interval = self.cleanup_interval;
        let max_age = self.max_snapshot_age;
        
        tokio::spawn(async move {
            let mut interval = interval(cleanup_interval);
            interval.tick().await; // Skip first immediate tick
            
            loop {
                interval.tick().await;
                
                let mut store = store.write().await;
                store.cleanup_stale(max_age);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_creation() {
        let snapshot = ComponentSnapshot::new(
            "test-component".to_string(),
            vec![1, 2, 3, 4, 5],
        );
        
        assert_eq!(snapshot.component_id, "test-component");
        assert_eq!(snapshot.data, vec![1, 2, 3, 4, 5]);
        assert!(snapshot.metadata.is_empty());
    }

    #[test]
    fn test_snapshot_with_metadata() {
        let snapshot = ComponentSnapshot::new(
            "test-component".to_string(),
            vec![1, 2, 3],
        )
        .with_metadata("version".to_string(), "1.0.0".to_string())
        .with_metadata("type".to_string(), "pipeline".to_string());
        
        assert_eq!(snapshot.metadata.len(), 2);
        assert_eq!(snapshot.metadata.get("version"), Some(&"1.0.0".to_string()));
        assert_eq!(snapshot.metadata.get("type"), Some(&"pipeline".to_string()));
    }

    #[test]
    fn test_snapshot_age() {
        let snapshot = ComponentSnapshot::new(
            "test-component".to_string(),
            vec![],
        );
        
        std::thread::sleep(Duration::from_millis(100));
        
        assert!(snapshot.age() >= Duration::from_millis(100));
    }

    #[test]
    fn test_snapshot_staleness() {
        let snapshot = ComponentSnapshot::new(
            "test-component".to_string(),
            vec![],
        );
        
        assert!(!snapshot.is_stale(Duration::from_secs(1)));
        
        std::thread::sleep(Duration::from_millis(100));
        
        assert!(snapshot.is_stale(Duration::from_millis(50)));
        assert!(!snapshot.is_stale(Duration::from_secs(1)));
    }

    #[tokio::test]
    async fn test_snapshot_store() {
        let mut store = SnapshotStore::new(3, 1024);
        
        for i in 0..5 {
            let snapshot = ComponentSnapshot::new(
                "test-component".to_string(),
                vec![i; 10],
            );
            store.store(snapshot);
        }
        
        // Should only keep last 3 snapshots
        let snapshots = store.get_all("test-component");
        assert_eq!(snapshots.len(), 3);
        
        // Latest should be the last one stored
        let latest = store.get_latest("test-component").unwrap();
        assert_eq!(latest.data[0], 4);
    }

    #[tokio::test]
    async fn test_snapshot_manager() {
        let manager = SnapshotManager::new(Duration::from_secs(60));
        
        let snapshot = ComponentSnapshot::new(
            "test-component".to_string(),
            vec![1, 2, 3, 4, 5],
        );
        
        manager.store_snapshot(snapshot.clone()).await;
        
        let retrieved = manager.get_latest_snapshot("test-component").await;
        assert!(retrieved.is_some());
        
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.component_id, "test-component");
        assert_eq!(retrieved.data, vec![1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    async fn test_snapshot_rollback() {
        let manager = SnapshotManager::new(Duration::from_secs(60));
        
        // Store multiple snapshots
        for i in 0..3 {
            let snapshot = ComponentSnapshot::new(
                "test-component".to_string(),
                vec![i],
            );
            manager.store_snapshot(snapshot).await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        // Rollback to latest
        let rollback = manager.rollback_to_snapshot("test-component", None).await;
        assert!(rollback.is_some());
        assert_eq!(rollback.unwrap().data, vec![2]);
    }

    #[tokio::test]
    async fn test_snapshot_stats() {
        let manager = SnapshotManager::new(Duration::from_secs(60));
        
        for i in 0..3 {
            let snapshot = ComponentSnapshot::new(
                format!("component-{}", i),
                vec![i; 100],
            );
            manager.store_snapshot(snapshot).await;
        }
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_components, 3);
        assert_eq!(stats.total_snapshots, 3);
        assert!(stats.total_size_bytes > 300);
    }
}
