pub mod alert_service;
pub mod alert_evaluation_service;
pub mod performance_service;
pub mod user_service;
pub mod tasks;
pub mod writer;
pub mod vps_renewal_service;
pub mod vps_service;
pub mod vps_traffic_service;
pub mod vps_detail_service;
pub mod settings_service;
pub mod service_monitor_service;
pub mod batch_command_service;
pub mod command_script_service;
pub mod oauth_service;
pub mod theme_service;

pub mod notification_service;
use self::writer::metrics_writer_task;
use crate::db::entities::performance_metric;
pub mod tag_service;
use duckdb::{ffi, types::ValueRef, Connection, Result, Row};
use serde_json;
use std::{path::Path, sync::mpsc, thread};
use tracing::{error, info};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    DuckDB(#[from] duckdb::Error),
    #[error("Connection pool error: {0}")]
    Pool(#[from] r2d2::Error),
    #[error("Not Found: {0}")]
    NotFound(String),
    #[error("Internal Server Error")]
    InternalServerError,
    #[error(transparent)]
    App(#[from] crate::web::error::AppError),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let body = self.to_string();
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

pub type DuckDbPool = r2d2::Pool<duckdb::DuckdbConnectionManager>;

// The service now only holds the sender part of the channel.
// The connection is created and managed exclusively in the writer thread.
// This struct is now cheap to clone and is Send + Sync.
#[derive(Clone, Debug)]
pub struct DuckDBService {
    metric_sender: mpsc::Sender<performance_metric::Model>,
}

impl DuckDBService {
    pub fn new(pool: DuckDbPool) -> std::result::Result<Self, Error> {
        info!("Initializing DuckDB service with connection pool.");

        // The connection is created here only to run initial migrations.
        // It will be closed immediately after. The writer task will manage its own connection.
        let conn = pool.get().map_err(Error::Pool)?;
        Self::initialize_db(&conn)?;

        let (tx, rx) = mpsc::channel();
        let writer_pool = pool.clone();

        // Spawn a dedicated OS thread for the blocking DuckDB writer task.
        // This prevents blocking the Tokio runtime.
        thread::spawn(move || {
            metrics_writer_task(writer_pool, rx);
        });

        Ok(Self { metric_sender: tx })
    }

    pub fn get_sender(&self) -> mpsc::Sender<performance_metric::Model> {
        self.metric_sender.clone()
    }

    // This is now a static method that takes a connection.
    fn initialize_db(conn: &Connection) -> Result<()> {
        info!("Running DuckDB migrations...");
        let migrations = include_str!(
            "../../../../../duckdb_migrations/20250726000000_create_initial_tables.sql"
        );
        conn.execute_batch(migrations).map_err(|e| {
            error!("Failed to execute DuckDB migrations: {}", e);
            e
        })?;
        info!("DuckDB migrations completed successfully.");
        Ok(())
    }
}

pub fn json_from_row(row: &Row<'_>, col_name: &str) -> Result<Option<serde_json::Value>, duckdb::Error> {
    let value: Option<String> = row.get(col_name)?;
    match value {
        Some(s) => {
            if s.is_empty() {
                Ok(None)
            } else {
                serde_json::from_str(&s).map(Some).map_err(|e| {
                    // We don't have the index here, so we can't construct the error perfectly.
                    // However, this is a reasonable approximation.
                    duckdb::Error::FromSqlConversionFailure(
                        0, // Placeholder for index
                        duckdb::types::Type::Text,
                        Box::new(e),
                    )
                })
            }
        }
        None => Ok(None),
    }
}
