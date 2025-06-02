use chrono::Utc;
use sqlx::{PgPool, Result};
use uuid::Uuid; // Added for generating agent_secret
use super::models::{User, Vps, PerformanceMetric, AggregatedPerformanceMetric};
use sqlx::postgres::types::PgInterval; // For mapping in handlers, keep imports tidy
use serde_json::json; // Added for JSON manipulation

// --- User Service Functions ---

/// Creates a new user.
pub async fn create_user(
    pool: &PgPool,
    username: &str,
    password_hash: &str,
    email: &str,
) -> Result<User> {
    let now = Utc::now();
    let user = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (username, password_hash, email, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, username, password_hash, email, created_at, updated_at
        "#,
        username,
        password_hash,
        email,
        now,
        now
    )
    .fetch_one(pool)
    .await?;
    Ok(user)
}

/// Retrieves a user by their ID.
pub async fn get_user_by_id(pool: &PgPool, user_id: i32) -> Result<Option<User>> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_optional(pool)
        .await
}

/// Retrieves a user by their username.
pub async fn get_user_by_username(pool: &PgPool, username: &str) -> Result<Option<User>> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE username = $1", username)
        .fetch_optional(pool)
        .await
}

// --- Vps Service Functions ---

/// Creates a new VPS entry.
pub async fn create_vps(
    pool: &PgPool,
    user_id: i32,
    name: &str,
) -> Result<Vps> {
    let now = Utc::now();
    let generated_agent_secret = Uuid::new_v4().to_string();
    let initial_status = "pending";
    let initial_ip_address: Option<String> = None;
    let initial_os_type: Option<String> = None;
    let initial_metadata: Option<serde_json::Value> = None;

    let vps = sqlx::query_as!(
        Vps,
        r#"
        INSERT INTO vps (user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at
        "#,
        user_id,
        name,
        initial_ip_address, // Use generated/default value
        initial_os_type,    // Use generated/default value
        generated_agent_secret, // Use generated value
        initial_status,     // Use generated/default value
        initial_metadata,   // Use generated/default value
        now,
        now
    )
    .fetch_one(pool)
    .await?;
    Ok(vps)
}

/// Retrieves a VPS by its ID.
pub async fn get_vps_by_id(pool: &PgPool, vps_id: i32) -> Result<Option<Vps>> {
    sqlx::query_as!(Vps, "SELECT * FROM vps WHERE id = $1", vps_id)
        .fetch_optional(pool)
        .await
}

/// Retrieves all VPS entries for a given user.
pub async fn get_vps_by_user_id(pool: &PgPool, user_id: i32) -> Result<Vec<Vps>> {
    sqlx::query_as!(Vps, "SELECT * FROM vps WHERE user_id = $1 ORDER BY created_at DESC", user_id)
        .fetch_all(pool)
        .await
}

/// Retrieves all VPS entries for a given user.
/// This is an alias for get_vps_by_user_id, but could be different in the future if needed.
pub async fn get_all_vps_for_user(pool: &PgPool, user_id: i32) -> Result<Vec<Vps>> {
    get_vps_by_user_id(pool, user_id).await
}

/// Updates the status of a VPS.
pub async fn update_vps_status(pool: &PgPool, vps_id: i32, status: &str) -> Result<u64> {
    let now = Utc::now();
    let rows_affected = sqlx::query!(
        "UPDATE vps SET status = $1, updated_at = $2 WHERE id = $3",
        status,
        now,
        vps_id
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected)
}

/// Updates VPS information based on AgentHandshake data.
/// This includes OS details, hostname, and public IP addresses.
/// Also sets status to "online".
pub async fn update_vps_info_on_handshake(
    pool: &PgPool,
    vps_id: i32,
    os_type_str: &str,
    os_name: &str,
    arch: &str,
    hostname: &str,
    public_ip_list: &[String], // Changed from CSV to slice of Strings
) -> Result<u64> {
    let now = Utc::now();

    // Find the first IPv4 address from the list
    let first_ipv4 = public_ip_list.iter()
        .find_map(|ip_str| {
            ip_str.parse::<std::net::IpAddr>().ok().and_then(|ip_addr| {
                if ip_addr.is_ipv4() {
                    Some(ip_str.clone())
                } else {
                    None
                }
            })
        });

    // Construct the metadata JSON to be updated
    let agent_info_metadata = json!({
        "os_name": os_name,
        "arch": arch,
        "hostname": hostname,
        "public_ip_addresses": public_ip_list // Store the full list here
    });

    let rows_affected = sqlx::query!(
        r#"
        UPDATE vps
        SET os_type = $1,                               -- Direct column
            ip_address = $2,                            -- Direct column, stores first IPv4 (VARCHAR(45) for now)
            metadata = COALESCE(metadata, '{}'::jsonb) || $3::jsonb, -- Merge new info into metadata
            status = $4,                                -- Direct column
            updated_at = $5                             -- Direct column
        WHERE id = $6
        "#,
        os_type_str,
        first_ipv4, // This will be Option<String>, SQL NULL if None. Max length 45 for ip_address column.
        agent_info_metadata, // Pass the serde_json::Value directly
        "online",   // Set status to online on successful handshake
        now,
        vps_id
    )
    .execute(pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        eprintln!("VPS info update on handshake for vps_id {} affected 0 rows. This might indicate the VPS ID was not found for update.", vps_id);
    }
    Ok(rows_affected)
}
 
// --- PerformanceMetric Service Functions ---

// /// Inserts a single performance metric snapshot.
// /// This function is outdated due to changes in PerformanceMetric struct and table.
// /// The new save_performance_snapshot_batch should be used for inserting metrics.
// pub async fn insert_performance_metric(
//     pool: &PgPool,
//     metric: &super::models::PerformanceMetric,
// ) -> Result<()> {
//     sqlx::query!(
//         r#"
//         INSERT INTO performance_metrics (
//             time, vps_id, cpu_usage_percent, memory_usage_bytes, memory_total_bytes,
//             disk_io_read_bps, disk_io_write_bps, network_rx_bps, network_tx_bps
//             // Note: This INSERT statement is missing new fields like id, swap, load_avg etc.
//             // and would need to be updated if this function were to be used.
//         )
//         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
//         "#,
//         metric.time,
//         metric.vps_id,
//         metric.cpu_usage_percent,
//         metric.memory_usage_bytes,
//         metric.memory_total_bytes,
//         metric.disk_io_read_bps,
//         metric.disk_io_write_bps,
//         metric.network_rx_bps,
//         metric.network_tx_bps
//     )
//     .execute(pool)
//     .await?;
//     Ok(())
// }

/// Retrieves performance metrics for a given VPS within a time range.
pub async fn get_performance_metrics_for_vps(
    pool: &PgPool,
    vps_id: i32,
    start_time: chrono::DateTime<chrono::Utc>,
    end_time: chrono::DateTime<chrono::Utc>,
    interval_minutes: Option<u32>,
) -> Result<Vec<AggregatedPerformanceMetric>> {
    if let Some(minutes) = interval_minutes {
        let interval_value = PgInterval {
            months: 0,
            days: 0,
            microseconds: (minutes.max(1) as i64) * 60 * 1_000_000,
        };

        // Perform aggregation
        // Note: id is not applicable for aggregated data, so we select vps_id directly.
        // Other non-aggregated fields from PerformanceMetric are omitted here.
        // If needed, they could be added with appropriate aggregate functions (MAX, MIN, etc.)
        // or by selecting them if the GROUP BY clause allows (e.g. if they are constant within the bucket).
        // For simplicity, focusing on CPU and Memory as per plan.
        sqlx::query_as!(
            AggregatedPerformanceMetric,
            r#"
            SELECT
                time_bucket($4::interval, time) AS time,
                vps_id,
                AVG(cpu_usage_percent) AS avg_cpu_usage_percent,
                AVG(memory_usage_bytes)::FLOAT8 AS avg_memory_usage_bytes,
                MAX(memory_total_bytes) AS max_memory_total_bytes
            FROM performance_metrics
            WHERE vps_id = $1 AND time >= $2 AND time <= $3
            GROUP BY time_bucket($4::interval, time), vps_id
            ORDER BY time ASC
            "#,
            vps_id,
            start_time,
            end_time,
            interval_value // Corrected: Use interval_value instead of interval_str
        )
        .fetch_all(pool)
        .await
    } else {
        // Fetch raw data and map to AggregatedPerformanceMetric
        // This branch might need adjustment if raw PerformanceMetric and AggregatedPerformanceMetric
        // are not directly compatible or if some fields in AggregatedPerformanceMetric should be None.
        // For now, assuming a direct mapping for required fields and None for others.
        sqlx::query_as!(
            PerformanceMetric, // Fetch as original PerformanceMetric first
            r#"
            SELECT
                id, time, vps_id, cpu_usage_percent, memory_usage_bytes, memory_total_bytes,
                swap_usage_bytes, swap_total_bytes,
                disk_io_read_bps, disk_io_write_bps, network_rx_bps, network_tx_bps,
                load_average_one_min, load_average_five_min, load_average_fifteen_min,
                uptime_seconds, total_processes_count, running_processes_count,
                tcp_established_connection_count
            FROM performance_metrics
            WHERE vps_id = $1 AND time >= $2 AND time <= $3
            ORDER BY time ASC
            "#,
            vps_id,
            start_time,
            end_time
        )
        .fetch_all(pool)
        .await
        .map(|metrics| {
            metrics
                .into_iter()
                .map(|m| AggregatedPerformanceMetric {
                    time: Some(m.time), // Wrapped in Some()
                    vps_id: m.vps_id,
                    avg_cpu_usage_percent: Some(m.cpu_usage_percent),
                    avg_memory_usage_bytes: Some(m.memory_usage_bytes as f64), // Cast to f64 for AVG type
                    max_memory_total_bytes: Some(m.memory_total_bytes),
                    // other fields from PerformanceMetric would be None or mapped if AggregatedPerformanceMetric had them
                })
                .collect()
        })
    }
}

/// Retrieves the latest performance metric for a given VPS.
pub async fn get_latest_performance_metric_for_vps(
    pool: &PgPool,
    vps_id: i32,
) -> Result<Option<super::models::PerformanceMetric>> {
    sqlx::query_as!(
        super::models::PerformanceMetric,
        r#"
        SELECT
            id, time, vps_id, cpu_usage_percent, memory_usage_bytes, memory_total_bytes,
            swap_usage_bytes, swap_total_bytes,
            disk_io_read_bps, disk_io_write_bps, network_rx_bps, network_tx_bps,
            load_average_one_min, load_average_five_min, load_average_fifteen_min,
            uptime_seconds, total_processes_count, running_processes_count,
            tcp_established_connection_count
        FROM performance_metrics
        WHERE vps_id = $1
        ORDER BY time DESC
        LIMIT 1
        "#,
        vps_id
    )
    .fetch_optional(pool)
    .await
}
use crate::agent_service::PerformanceSnapshotBatch; // Corrected path for protobuf generated types
use chrono::{TimeZone, Utc as ChronoUtc}; // Alias Utc from chrono to avoid conflict if any

/// Saves a batch of performance snapshots for a given VPS.
/// This includes the main metrics, detailed disk usage, and detailed network interface stats.
pub async fn save_performance_snapshot_batch(
    pool: &PgPool,
    vps_id: i32,
    batch: &PerformanceSnapshotBatch,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    for snapshot in &batch.snapshots {
        // Convert timestamp
        let timestamp = ChronoUtc.timestamp_millis_opt(snapshot.timestamp_unix_ms).single()
            .unwrap_or_else(|| ChronoUtc::now()); // Fallback to now if conversion fails, or handle error

        // Calculate aggregate network stats
        let mut total_network_rx_bps: i64 = 0;
        let mut total_network_tx_bps: i64 = 0;
        for net_stat in &snapshot.network_interface_stats {
            total_network_rx_bps += net_stat.rx_bytes_per_sec as i64;
            total_network_tx_bps += net_stat.tx_bytes_per_sec as i64;
        }

        // Insert into performance_metrics and get the ID
        let metric_id = sqlx::query!(
            r#"
            INSERT INTO performance_metrics (
                time, vps_id, cpu_usage_percent, memory_usage_bytes, memory_total_bytes,
                swap_usage_bytes, swap_total_bytes,
                disk_io_read_bps, disk_io_write_bps,
                network_rx_bps, network_tx_bps,
                load_average_one_min, load_average_five_min, load_average_fifteen_min,
                uptime_seconds, total_processes_count, running_processes_count,
                tcp_established_connection_count
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
            RETURNING id
            "#,
            timestamp,
            vps_id,
            snapshot.cpu_overall_usage_percent as f64, // proto is float, db is double precision
            snapshot.memory_usage_bytes as i64,
            snapshot.memory_total_bytes as i64,
            snapshot.swap_usage_bytes as i64,
            snapshot.swap_total_bytes as i64,
            snapshot.disk_total_io_read_bytes_per_sec as i64,
            snapshot.disk_total_io_write_bytes_per_sec as i64,
            total_network_rx_bps,
            total_network_tx_bps,
            snapshot.load_average_one_min as f64,
            snapshot.load_average_five_min as f64,
            snapshot.load_average_fifteen_min as f64,
            snapshot.uptime_seconds as i64,
            snapshot.total_processes_count as i32,
            snapshot.running_processes_count as i32,
            snapshot.tcp_established_connection_count as i32
        )
        .fetch_one(&mut *tx) // Use &mut *tx for the executor
        .await?
        .id;

        // Insert disk usages
        for disk_usage in &snapshot.disk_usages {
            sqlx::query!(
                r#"
                INSERT INTO performance_disk_usages (
                    performance_metric_id, mount_point, used_bytes, total_bytes, fstype, usage_percent
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
                metric_id,
                disk_usage.mount_point,
                disk_usage.used_bytes as i64,
                disk_usage.total_bytes as i64,
                disk_usage.fstype, // fstype is string in proto, Option<String> in model, TEXT in DB
                disk_usage.usage_percent as f64
            )
            .execute(&mut *tx)
            .await?;
        }

        // Insert network interface stats
        for net_stat in &snapshot.network_interface_stats {
            sqlx::query!(
                r#"
                INSERT INTO performance_network_interface_stats (
                    performance_metric_id, interface_name,
                    rx_bytes_per_sec, tx_bytes_per_sec,
                    rx_packets_per_sec, tx_packets_per_sec,
                    rx_errors_total_cumulative, tx_errors_total_cumulative
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                "#,
                metric_id,
                net_stat.interface_name,
                net_stat.rx_bytes_per_sec as i64,
                net_stat.tx_bytes_per_sec as i64,
                net_stat.rx_packets_per_sec as i64,
                net_stat.tx_packets_per_sec as i64,
                net_stat.rx_errors_total_cumulative as i64,
                net_stat.tx_errors_total_cumulative as i64
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}
// TODO: Implement service functions for other models:
// - PerformanceMetric (batch insert, query by time range)
// - DockerContainer (create, update, get by vps_id)
// - DockerMetric (batch insert, query by time range)
// - Task (create, update, get, list)
// - TaskRun (create, update, get by task_id)
// - AlertRule (create, update, get, list)
// - AlertEvent (create, list by rule_id/vps_id)
// - VpsMonthlyTraffic (upsert, get)