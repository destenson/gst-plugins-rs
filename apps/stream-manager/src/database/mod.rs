// Database persistence
// TODO: Implement in PRP-20

use sqlx::SqlitePool;

pub struct Database {
    pool: Option<SqlitePool>,
}

impl Database {
    pub fn new() -> Self {
        Self { pool: None }
    }
}