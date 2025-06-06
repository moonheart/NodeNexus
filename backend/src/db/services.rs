use chrono::Utc;
use sqlx::{PgPool, Result};
use uuid::Uuid; // Added for generating agent_secret
use super::models::{User, Vps, AggregatedPerformanceMetric};
use crate::websocket_models::{ServerBasicInfo, ServerMetricsSnapshot, ServerWithDetails}; // Added for cache population
use sqlx::FromRow; // Added for deriving FromRow for helper struct
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
    let initial_tags: Option<String> = None;
    let initial_group: Option<String> = None;

    let vps = sqlx::query_as!(
        Vps,
        r#"
        INSERT INTO vps (user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, tags, "group")
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        RETURNING id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, tags, "group"
        "#,
        user_id,
        name,
        initial_ip_address, // Use generated/default value
        initial_os_type,    // Use generated/default value
        generated_agent_secret, // Use generated value
        initial_status,     // Use generated/default value
        initial_metadata,   // Use generated/default value
        now,
        now,
        initial_tags,
        initial_group
    )
    .fetch_one(pool)
    .await?;
    Ok(vps)
}

/// Retrieves a VPS by its ID.
pub async fn get_vps_by_id(pool: &PgPool, vps_id: i32) -> Result<Option<Vps>> {
    sqlx::query_as!(Vps, r#"SELECT id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, tags, "group" FROM vps WHERE id = $1"#, vps_id)
        .fetch_optional(pool)
        .await
}

/// Retrieves all VPS entries for a given user.
pub async fn get_vps_by_user_id(pool: &PgPool, user_id: i32) -> Result<Vec<Vps>> {
    sqlx::query_as!(Vps, r#"SELECT id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, tags, "group" FROM vps WHERE user_id = $1 ORDER BY created_at DESC"#, user_id)
        .fetch_all(pool)
        .await
}

/// Retrieves all VPS entries for a given user.
/// This is an alias for get_vps_by_user_id, but could be different in the future if needed.
pub async fn get_all_vps_for_user(pool: &PgPool, user_id: i32) -> Result<Vec<Vps>> {
    get_vps_by_user_id(pool, user_id).await
}

/// Updates a VPS's editable fields.
pub async fn update_vps(
    pool: &PgPool,
    vps_id: i32,
    user_id: i32, // To ensure ownership
    name: Option<String>,
    tags: Option<String>,
    group: Option<String>,
) -> Result<u64> {
    let now = Utc::now();
    let rows_affected = sqlx::query!(
        r#"
        UPDATE vps
        SET
            name = COALESCE($1, name),
            tags = COALESCE($2, tags),
            "group" = COALESCE($3, "group"),
            updated_at = $4
        WHERE id = $5 AND user_id = $6
        "#,
        name,
        tags,
        group,
        now,
        vps_id,
        user_id
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected)
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
        // Perform aggregation and calculate network BPS using window functions
        sqlx::query_as!(
            AggregatedPerformanceMetric,
            r#"
            WITH TimeBucketed AS (
                SELECT
                    time_bucket($4::interval, time) AS bucket_time,
                    vps_id,
                    AVG(cpu_usage_percent) AS avg_cpu_usage_percent,
                    AVG(memory_usage_bytes) AS avg_memory_usage_bytes,
                    MAX(memory_total_bytes) AS max_memory_total_bytes,
                    -- Calculate average of the instantaneous BPS values stored in the new columns
                    AVG(network_rx_instant_bps) AS avg_network_rx_instant_bps,
                    AVG(network_tx_instant_bps) AS avg_network_tx_instant_bps
                    -- Removed old logic based on cumulative values and duration
                FROM performance_metrics
                WHERE vps_id = $1 AND time >= $2 AND time <= $3
                GROUP BY bucket_time, vps_id
            )
            SELECT
                bucket_time AS time, -- Alias bucket_time to time for the final struct
                vps_id,
                avg_cpu_usage_percent,
                avg_memory_usage_bytes::FLOAT8, -- Cast AVG result to FLOAT8
                max_memory_total_bytes,
                -- Select the calculated averages directly
                avg_network_rx_instant_bps::FLOAT8, -- Cast AVG result to FLOAT8
                avg_network_tx_instant_bps::FLOAT8  -- Cast AVG result to FLOAT8
            FROM TimeBucketed
            ORDER BY time ASC
            "#,
            vps_id,
            start_time,
            end_time,
            interval_value
        )
        .fetch_all(pool)
        .await
    } else {
        // Fetch raw data and calculate instantaneous BPS using LAG window function
        sqlx::query_as!(
             AggregatedPerformanceMetric, // Map directly to AggregatedPerformanceMetric
            r#"
            WITH RankedMetrics AS (
                SELECT
                    id, time, vps_id, cpu_usage_percent, memory_usage_bytes, memory_total_bytes,
                    swap_usage_bytes, swap_total_bytes,
                    disk_io_read_bps, disk_io_write_bps,
                    network_rx_bps, network_tx_bps, -- Cumulative
                    network_rx_instant_bps, network_tx_instant_bps, -- Instantaneous
                    uptime_seconds, total_processes_count, running_processes_count,
                    tcp_established_connection_count
                    -- Removed LAG calculation as we now select instantaneous values directly
                FROM performance_metrics
                WHERE vps_id = $1 AND time >= $2 AND time <= $3
            )
            SELECT
                time, -- Keep original timestamp
                vps_id,
                cpu_usage_percent AS avg_cpu_usage_percent, -- Use raw value as 'avg'
                memory_usage_bytes::FLOAT8 AS avg_memory_usage_bytes, -- Use raw value as 'avg'
                memory_total_bytes AS max_memory_total_bytes, -- Use raw value as 'max'
                -- Select the stored instantaneous BPS directly and alias to match AggregatedPerformanceMetric
                network_rx_instant_bps::FLOAT8 AS avg_network_rx_instant_bps,
                network_tx_instant_bps::FLOAT8 AS avg_network_tx_instant_bps
            FROM RankedMetrics
            ORDER BY time ASC
            "#,
            vps_id,
            start_time,
            end_time
        )
        .fetch_all(pool)
        .await
        // The result is already Vec<AggregatedPerformanceMetric>, no need for further mapping
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
            disk_io_read_bps, disk_io_write_bps,
            network_rx_bps, network_tx_bps, -- Cumulative
            network_rx_instant_bps, network_tx_instant_bps, -- Instantaneous
            uptime_seconds, total_processes_count, running_processes_count,
            tcp_established_connection_count
            -- Removed load_average fields from selection
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

        // 输出 snapshot 的内容到日志
        // println!("Saving snapshot for vps_id {} at time {}: {:?}", vps_id, timestamp, snapshot);

       // Insert into performance_metrics and get the ID
        let metric_id = sqlx::query!(
            r#"
            INSERT INTO performance_metrics (
                time, vps_id, cpu_usage_percent, memory_usage_bytes, memory_total_bytes,
                swap_usage_bytes, swap_total_bytes,
                disk_io_read_bps, disk_io_write_bps,
                network_rx_bps, network_tx_bps, -- Cumulative
                network_rx_instant_bps, network_tx_instant_bps, -- Instantaneous
                uptime_seconds, total_processes_count, running_processes_count,
                tcp_established_connection_count
                -- Removed load_average fields from INSERT list
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17) -- Adjusted placeholders (removed 3)
            RETURNING id
            "#,
            timestamp,
            vps_id,
            snapshot.cpu_overall_usage_percent as f64, // proto is float, db is double precision
            snapshot.memory_usage_bytes as i64,
            snapshot.memory_total_bytes as i64,
            snapshot.swap_usage_bytes as i64,
            snapshot.swap_total_bytes as i64,
            snapshot.disk_total_io_read_bytes_per_sec as i64, // Note: Field name implies BPS, but value is cumulative
            snapshot.disk_total_io_write_bytes_per_sec as i64, // Note: Field name implies BPS, but value is cumulative
            snapshot.network_rx_bytes_cumulative as i64, // Map proto cumulative RX to DB cumulative RX (network_rx_bps)
            snapshot.network_tx_bytes_cumulative as i64, // Map proto cumulative TX to DB cumulative TX (network_tx_bps)
            snapshot.network_rx_bytes_per_sec as i64, // Map proto instant RX to DB instant RX (network_rx_instant_bps)
            snapshot.network_tx_bytes_per_sec as i64, // Map proto instant TX to DB instant TX (network_tx_instant_bps)
            // Removed load_average mappings
            snapshot.uptime_seconds as i64,
            snapshot.total_processes_count as i32,
            snapshot.running_processes_count as i32,
            snapshot.tcp_established_connection_count as i32
            // Adjusted parameter indices implicitly by removing 3 lines above

        )
        .fetch_one(&mut *tx) // Use &mut *tx for the executor
        .await?
        .id;

        // Insert disk usages (remains the same)
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
                disk_usage.fstype,
                disk_usage.usage_percent as f64
            )
            .execute(&mut *tx)
            .await?;
        }

        // Insertion into performance_network_interface_stats remains removed
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

/// Retrieves the summary of total and used disk space from the latest performance metric.
pub async fn get_latest_disk_usage_summary(
    pool: &PgPool,
    vps_id: i32,
) -> Result<Option<(i64, i64)>> { // Returns (total_bytes, used_bytes)
    let result = sqlx::query_as!(
        DiskUsageSummary,
        r#"
        WITH LatestMetric AS (
            SELECT id
            FROM performance_metrics
            WHERE vps_id = $1
            ORDER BY time DESC
            LIMIT 1
        )
        SELECT
            SUM(pdu.total_bytes)::BIGINT as total_sum_bytes,
            SUM(pdu.used_bytes)::BIGINT as used_sum_bytes
        FROM performance_disk_usages pdu
        JOIN LatestMetric lm ON pdu.performance_metric_id = lm.id
        WHERE EXISTS (SELECT 1 FROM LatestMetric) -- Ensure we only proceed if a latest metric exists
        GROUP BY lm.id -- Though lm.id will be unique here due to LIMIT 1
        "#,
        vps_id
    )
    .fetch_optional(pool)
    .await?;

    match result {
        Some(summary) => {
            // Handle cases where SUM might return NULL if no rows are found by the JOIN,
            // though EXISTS should prevent this. SUM on no rows is NULL.
            let total = summary.total_sum_bytes.unwrap_or(0);
            let used = summary.used_sum_bytes.unwrap_or(0);
            if total == 0 && used == 0 && summary.total_sum_bytes.is_none() { // Check if SUMs were actually NULL
                Ok(None) // No disk usage data found for the latest metric
            } else {
                Ok(Some((total, used)))
            }
        }
        None => Ok(None), // No latest metric found, or no disk usage for it
    }
}

// Helper struct for the above query
struct DiskUsageSummary {
    total_sum_bytes: Option<i64>,
    used_sum_bytes: Option<i64>,
}

// Helper struct for the get_all_vps_with_details_for_cache query result
#[derive(FromRow, Debug)]
struct VpsDetailQueryResult {
    vps_id: i32,
    vps_name: String,
    vps_ip_address: Option<String>,
    vps_status: String,
    vps_os_type: Option<String>,
    vps_group: Option<String>,
    vps_tags: Option<String>,
    vps_created_at: chrono::DateTime<Utc>,
    // Metrics fields (all optional because of LEFT JOIN)
    cpu_usage_percent: Option<f64>,
    memory_usage_bytes: Option<i64>,
    memory_total_bytes: Option<i64>,
    network_rx_instant_bps: Option<i64>,
    network_tx_instant_bps: Option<i64>,
    uptime_seconds: Option<i64>,
    total_disk_used_bytes: Option<i64>,
    total_disk_total_bytes: Option<i64>,
    metric_time: Option<chrono::DateTime<Utc>>, // Added to store the time of the metric
}

/// Retrieves all VPS along with their latest metrics and disk usage for cache initialization.
pub async fn get_all_vps_with_details_for_cache(pool: &PgPool) -> Result<Vec<ServerWithDetails>> {
    let query_results = sqlx::query_as::<_, VpsDetailQueryResult>(
        r#"
        WITH RankedMetrics AS (
            SELECT
                *,
                ROW_NUMBER() OVER (PARTITION BY vps_id ORDER BY time DESC) as rn
            FROM performance_metrics
        ),
        LatestVpsMetrics AS (
            SELECT * FROM RankedMetrics WHERE rn = 1
        ),
        LatestVpsDiskUsage AS (
            SELECT
                lvm.vps_id,
                SUM(pdu.used_bytes) as total_disk_used_bytes,
                SUM(pdu.total_bytes) as total_disk_total_bytes
            FROM performance_disk_usages pdu
            JOIN LatestVpsMetrics lvm ON pdu.performance_metric_id = lvm.id
            GROUP BY lvm.vps_id
        )
        SELECT
            v.id as vps_id,
            v.name as vps_name,
            v.ip_address as vps_ip_address,
            v.status as vps_status,
            v.os_type as vps_os_type,
            v."group" as vps_group,
            v.tags as vps_tags,
            v.created_at as vps_created_at,
            lvm.cpu_usage_percent,
            lvm.memory_usage_bytes,
            lvm.memory_total_bytes,
            lvm.network_rx_instant_bps,
            lvm.network_tx_instant_bps,
            lvm.uptime_seconds,
            lvdu.total_disk_used_bytes,
            lvdu.total_disk_total_bytes,
            lvm.time as metric_time -- Select the metric time
        FROM vps v
        LEFT JOIN LatestVpsMetrics lvm ON v.id = lvm.vps_id
        LEFT JOIN LatestVpsDiskUsage lvdu ON v.id = lvdu.vps_id
        ORDER BY v.id;
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut servers_with_details = Vec::new();
    for row in query_results {
        let basic_info = ServerBasicInfo {
            id: row.vps_id,
            name: row.vps_name,
            ip_address: row.vps_ip_address,
            status: row.vps_status,
            group: row.vps_group,
            tags: row.vps_tags,
        };

        let latest_metrics = if row.cpu_usage_percent.is_some() && row.metric_time.is_some() { // Ensure metric_time is also present
            Some(ServerMetricsSnapshot {
                time: row.metric_time.unwrap(), // Use the metric_time
                cpu_usage_percent: row.cpu_usage_percent.unwrap_or(0.0) as f32,
                memory_usage_bytes: row.memory_usage_bytes.unwrap_or(0) as u64,
                memory_total_bytes: row.memory_total_bytes.unwrap_or(0) as u64,
                network_rx_instant_bps: row.network_rx_instant_bps.map(|val| val as u64),
                network_tx_instant_bps: row.network_tx_instant_bps.map(|val| val as u64),
                uptime_seconds: row.uptime_seconds.map(|val| val as u64),
                disk_used_bytes: row.total_disk_used_bytes.map(|val| val as u64),
                disk_total_bytes: row.total_disk_total_bytes.map(|val| val as u64),
            })
        } else {
            None
        };

        servers_with_details.push(ServerWithDetails {
            basic_info,
            latest_metrics,
            os_type: row.vps_os_type,
            created_at: row.vps_created_at,
        });
    }

    Ok(servers_with_details)
}