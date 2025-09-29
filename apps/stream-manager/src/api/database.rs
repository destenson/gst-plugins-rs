use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
struct TableInfo {
    name: String,
    #[serde(rename = "rowCount")]
    row_count: i64,
    columns: Vec<String>,
}

/// List all tables in the database
pub async fn list_tables(
    state: web::Data<crate::api::AppState>,
) -> Result<HttpResponse> {
    let database = state.database.as_ref()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Database not available"))?;

    let pool = database.pool();

    let tables = sqlx::query(
        r#"
        SELECT name
        FROM sqlite_master
        WHERE type='table'
        AND name NOT LIKE 'sqlite_%'
        AND name NOT LIKE '_sqlx_%'
        ORDER BY name
        "#
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list tables: {}", e);
        actix_web::error::ErrorInternalServerError(e)
    })?;

    let mut table_infos = Vec::new();

    for table in tables {
        let table_name: String = table.try_get("name").unwrap_or_default();

        // Get row count
        let count_query = format!("SELECT COUNT(*) as count FROM {}", table_name);
        let count_result = sqlx::query(&count_query)
            .fetch_one(pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to count rows in {}: {}", table_name, e);
                actix_web::error::ErrorInternalServerError(e)
            })?;

        let row_count: i64 = count_result.try_get("count").unwrap_or(0);

        // Get column info
        let pragma_query = format!("PRAGMA table_info({})", table_name);
        let columns_result = sqlx::query(&pragma_query)
            .fetch_all(pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get columns for {}: {}", table_name, e);
                actix_web::error::ErrorInternalServerError(e)
            })?;

        let columns: Vec<String> = columns_result
            .iter()
            .filter_map(|row| row.try_get::<String, _>("name").ok())
            .collect();

        table_infos.push(TableInfo {
            name: table_name,
            row_count,
            columns,
        });
    }

    Ok(HttpResponse::Ok().json(&table_infos))
}

/// Get all data from a specific table
pub async fn get_table_data(
    state: web::Data<crate::api::AppState>,
    table_name: web::Path<String>,
) -> Result<HttpResponse> {
    let database = state.database.as_ref()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Database not available"))?;

    let pool = database.pool();

    // Validate table name to prevent SQL injection
    if !table_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid table name"
        })));
    }

    // Get column names first
    let pragma_query = format!("PRAGMA table_info({})", table_name);
    let columns_result = sqlx::query(&pragma_query)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get columns for {}: {}", table_name, e);
            actix_web::error::ErrorInternalServerError(e)
        })?;

    if columns_result.is_empty() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Table not found"
        })));
    }

    let column_names: Vec<String> = columns_result
        .iter()
        .filter_map(|row| row.try_get::<String, _>("name").ok())
        .collect();

    // Fetch all data
    let query = format!("SELECT * FROM {} LIMIT 1000", table_name);
    let rows = sqlx::query(&query)
        .fetch_all(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch data from {}: {}", table_name, e);
            actix_web::error::ErrorInternalServerError(e)
        })?;

    let mut records = Vec::new();
    for row in rows {
        let mut record = HashMap::new();

        for col_name in &column_names {
            // Try to get value as different types
            let value = if let Ok(val) = row.try_get::<String, _>(col_name.as_str()) {
                serde_json::Value::String(val)
            } else if let Ok(val) = row.try_get::<i64, _>(col_name.as_str()) {
                serde_json::Value::Number(val.into())
            } else if let Ok(val) = row.try_get::<f64, _>(col_name.as_str()) {
                serde_json::Number::from_f64(val)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else if let Ok(val) = row.try_get::<bool, _>(col_name.as_str()) {
                serde_json::Value::Bool(val)
            } else {
                // Try to get as Option<String> for NULL values
                row.try_get::<Option<String>, _>(col_name.as_str())
                    .ok()
                    .and_then(|opt| opt.map(serde_json::Value::String))
                    .unwrap_or(serde_json::Value::Null)
            };

            record.insert(col_name.clone(), value);
        }

        records.push(record);
    }

    Ok(HttpResponse::Ok().json(&records))
}

#[derive(Debug, Deserialize)]
struct UpdateRequest {
    data: HashMap<String, serde_json::Value>,
    where_clause: HashMap<String, serde_json::Value>,
}

/// Update a record in a table
pub async fn update_record(
    state: web::Data<crate::api::AppState>,
    table_name: web::Path<String>,
    req: web::Json<UpdateRequest>,
) -> Result<HttpResponse> {
    let database = state.database.as_ref()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Database not available"))?;

    let pool = database.pool();

    // Validate table name
    if !table_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid table name"
        })));
    }

    if req.data.is_empty() || req.where_clause.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Update data and where clause required"
        })));
    }

    // Build UPDATE query
    let set_clause: Vec<String> = req.data.keys()
        .map(|k| format!("{} = ?", k))
        .collect();

    let where_parts: Vec<String> = req.where_clause.keys()
        .map(|k| format!("{} = ?", k))
        .collect();

    let query = format!(
        "UPDATE {} SET {} WHERE {}",
        table_name.as_ref(),
        set_clause.join(", "),
        where_parts.join(" AND ")
    );

    // Execute query with bound parameters
    let mut query_builder = sqlx::query(&query);

    // Bind SET values
    for value in req.data.values() {
        query_builder = match value {
            serde_json::Value::String(s) => query_builder.bind(s.clone()),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    query_builder.bind(i)
                } else if let Some(f) = n.as_f64() {
                    query_builder.bind(f)
                } else {
                    query_builder.bind(n.to_string())
                }
            },
            serde_json::Value::Bool(b) => query_builder.bind(*b),
            serde_json::Value::Null => query_builder.bind(None::<String>),
            _ => query_builder.bind(value.to_string()),
        };
    }

    // Bind WHERE values
    for value in req.where_clause.values() {
        query_builder = match value {
            serde_json::Value::String(s) => query_builder.bind(s.clone()),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    query_builder.bind(i)
                } else if let Some(f) = n.as_f64() {
                    query_builder.bind(f)
                } else {
                    query_builder.bind(n.to_string())
                }
            },
            serde_json::Value::Bool(b) => query_builder.bind(*b),
            serde_json::Value::Null => query_builder.bind(None::<String>),
            _ => query_builder.bind(value.to_string()),
        };
    }

    let result = query_builder
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update record: {}", e);
            actix_web::error::ErrorInternalServerError(e)
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "rows_affected": result.rows_affected()
    })))
}

/// Insert a new record into a table
pub async fn insert_record(
    state: web::Data<crate::api::AppState>,
    table_name: web::Path<String>,
    req: web::Json<HashMap<String, serde_json::Value>>,
) -> Result<HttpResponse> {
    let database = state.database.as_ref()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Database not available"))?;

    let pool = database.pool();

    // Validate table name
    if !table_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid table name"
        })));
    }

    if req.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Insert data required"
        })));
    }

    let columns: Vec<String> = req.keys().cloned().collect();
    let placeholders: Vec<String> = (0..columns.len()).map(|_| "?".to_string()).collect();

    let query = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table_name.as_ref(),
        columns.join(", "),
        placeholders.join(", ")
    );

    let mut query_builder = sqlx::query(&query);

    for value in req.values() {
        query_builder = match value {
            serde_json::Value::String(s) => query_builder.bind(s.clone()),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    query_builder.bind(i)
                } else if let Some(f) = n.as_f64() {
                    query_builder.bind(f)
                } else {
                    query_builder.bind(n.to_string())
                }
            },
            serde_json::Value::Bool(b) => query_builder.bind(*b),
            serde_json::Value::Null => query_builder.bind(None::<String>),
            _ => query_builder.bind(value.to_string()),
        };
    }

    let result = query_builder
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to insert record: {}", e);
            actix_web::error::ErrorInternalServerError(e)
        })?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "rows_affected": result.rows_affected(),
        "last_insert_rowid": result.last_insert_rowid()
    })))
}

/// Delete a record from a table
pub async fn delete_record(
    state: web::Data<crate::api::AppState>,
    table_name: web::Path<String>,
    req: web::Json<HashMap<String, serde_json::Value>>,
) -> Result<HttpResponse> {
    let database = state.database.as_ref()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Database not available"))?;

    let pool = database.pool();

    // Validate table name
    if !table_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid table name"
        })));
    }

    if req.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Where clause required for delete"
        })));
    }

    let where_parts: Vec<String> = req.keys()
        .map(|k| format!("{} = ?", k))
        .collect();

    let query = format!(
        "DELETE FROM {} WHERE {}",
        table_name.as_ref(),
        where_parts.join(" AND ")
    );

    let mut query_builder = sqlx::query(&query);

    for value in req.values() {
        query_builder = match value {
            serde_json::Value::String(s) => query_builder.bind(s.clone()),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    query_builder.bind(i)
                } else if let Some(f) = n.as_f64() {
                    query_builder.bind(f)
                } else {
                    query_builder.bind(n.to_string())
                }
            },
            serde_json::Value::Bool(b) => query_builder.bind(*b),
            serde_json::Value::Null => query_builder.bind(None::<String>),
            _ => query_builder.bind(value.to_string()),
        };
    }

    let result = query_builder
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete record: {}", e);
            actix_web::error::ErrorInternalServerError(e)
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "rows_affected": result.rows_affected()
    })))
}

/// Clear all data from a table (with safety check)
pub async fn clear_table(
    state: web::Data<crate::api::AppState>,
    table_name: web::Path<String>,
) -> Result<HttpResponse> {
    let database = state.database.as_ref()
        .ok_or_else(|| actix_web::error::ErrorInternalServerError("Database not available"))?;

    let pool = database.pool();

    // Validate table name
    if !table_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid table name"
        })));
    }

    // Don't allow clearing critical system tables
    let protected_tables = ["users", "auth_tokens", "config"];
    if protected_tables.contains(&table_name.as_str()) {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Cannot clear protected table"
        })));
    }

    let query = format!("DELETE FROM {}", table_name.as_ref());
    let result = sqlx::query(&query)
        .execute(pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to clear table: {}", e);
            actix_web::error::ErrorInternalServerError(e)
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "rows_affected": result.rows_affected()
    })))
}

/// Register database routes
pub fn register_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/database")
            .route("/tables", web::get().to(list_tables))
            .route("/tables/{table_name}", web::get().to(get_table_data))
            .route("/tables/{table_name}/update", web::post().to(update_record))
            .route("/tables/{table_name}/insert", web::post().to(insert_record))
            .route("/tables/{table_name}/delete", web::post().to(delete_record))
            .route("/tables/{table_name}/clear", web::post().to(clear_table))
    );
}
