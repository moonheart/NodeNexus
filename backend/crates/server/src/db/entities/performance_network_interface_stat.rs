use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub performance_metric_id: i32,
    pub interface_name: String,
    pub rx_bytes_per_sec: i64,
    pub tx_bytes_per_sec: i64,
    pub rx_packets_per_sec: i64,
    pub tx_packets_per_sec: i64,
    pub rx_errors_total_cumulative: i64,
    pub tx_errors_total_cumulative: i64,
}
