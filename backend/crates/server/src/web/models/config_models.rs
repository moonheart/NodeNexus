use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WebAgentConfig {
    pub metrics_collect_interval_seconds: u32,
    pub metrics_upload_batch_max_size: u32,
    pub metrics_upload_interval_seconds: u32,
    pub docker_info_collect_interval_seconds: u32,
    pub docker_info_upload_interval_seconds: u32,
    pub generic_metrics_upload_batch_max_size: u32,
    pub generic_metrics_upload_interval_seconds: u32,
    pub feature_flags: HashMap<String, String>,
    pub log_level: String,
    #[serde(default)]
    pub service_monitor_tasks: Vec<WebServiceMonitorTask>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WebServiceMonitorTask {
    pub monitor_id: i32,
    pub name: String,
    pub monitor_type: String,
    pub target: String,
    pub frequency_seconds: i32,
    pub monitor_config_json: String,
    pub timeout_seconds: i32,
}

impl From<nodenexus_common::agent_service::AgentConfig> for WebAgentConfig {
    fn from(proto: nodenexus_common::agent_service::AgentConfig) -> Self {
        Self {
            metrics_collect_interval_seconds: proto.metrics_collect_interval_seconds,
            metrics_upload_batch_max_size: proto.metrics_upload_batch_max_size,
            metrics_upload_interval_seconds: proto.metrics_upload_interval_seconds,
            docker_info_collect_interval_seconds: proto.docker_info_collect_interval_seconds,
            docker_info_upload_interval_seconds: proto.docker_info_upload_interval_seconds,
            generic_metrics_upload_batch_max_size: proto.generic_metrics_upload_batch_max_size,
            generic_metrics_upload_interval_seconds: proto.generic_metrics_upload_interval_seconds,
            feature_flags: proto.feature_flags,
            log_level: proto.log_level,
            service_monitor_tasks: proto.service_monitor_tasks.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<WebAgentConfig> for nodenexus_common::agent_service::AgentConfig {
    fn from(web: WebAgentConfig) -> Self {
        Self {
            metrics_collect_interval_seconds: web.metrics_collect_interval_seconds,
            metrics_upload_batch_max_size: web.metrics_upload_batch_max_size,
            metrics_upload_interval_seconds: web.metrics_upload_interval_seconds,
            docker_info_collect_interval_seconds: web.docker_info_collect_interval_seconds,
            docker_info_upload_interval_seconds: web.docker_info_upload_interval_seconds,
            generic_metrics_upload_batch_max_size: web.generic_metrics_upload_batch_max_size,
            generic_metrics_upload_interval_seconds: web.generic_metrics_upload_interval_seconds,
            feature_flags: web.feature_flags,
            log_level: web.log_level,
            service_monitor_tasks: web.service_monitor_tasks.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<nodenexus_common::agent_service::ServiceMonitorTask> for WebServiceMonitorTask {
    fn from(proto: nodenexus_common::agent_service::ServiceMonitorTask) -> Self {
        Self {
            monitor_id: proto.monitor_id,
            name: proto.name,
            monitor_type: proto.monitor_type,
            target: proto.target,
            frequency_seconds: proto.frequency_seconds,
            monitor_config_json: proto.monitor_config_json,
            timeout_seconds: proto.timeout_seconds,
        }
    }
}

impl From<WebServiceMonitorTask> for nodenexus_common::agent_service::ServiceMonitorTask {
    fn from(web: WebServiceMonitorTask) -> Self {
        Self {
            monitor_id: web.monitor_id,
            name: web.name,
            monitor_type: web.monitor_type,
            target: web.target,
            frequency_seconds: web.frequency_seconds,
            monitor_config_json: web.monitor_config_json,
            timeout_seconds: web.timeout_seconds,
        }
    }
}