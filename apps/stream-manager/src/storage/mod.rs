// Storage management and disk monitoring
// TODO: Implement in PRP-16

use std::path::PathBuf;

pub struct StorageManager {
    base_path: PathBuf,
    max_usage_percent: f32,
}

impl StorageManager {
    pub fn new(base_path: PathBuf, max_usage_percent: f32) -> Self {
        Self {
            base_path,
            max_usage_percent,
        }
    }
}