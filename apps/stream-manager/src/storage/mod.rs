pub mod manager;

pub use manager::{
    StorageManager, StoragePath, StorageStats, CleanupPolicy,
    PathSelectionStrategy, StorageEvent, StorageError, StorageConfig,
};