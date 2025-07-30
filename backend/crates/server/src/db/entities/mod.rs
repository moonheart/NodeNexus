pub mod alert_event;
pub mod alert_rule;
pub mod alert_rule_channel;
pub mod batch_command_task;
pub mod child_command_task;
pub mod command_script;
pub mod docker_container;
pub mod docker_metric;
pub mod notification_channel;
pub mod oauth2_provider;
pub mod performance_metric;
pub mod service_monitor;
pub mod service_monitor_agent;
pub mod service_monitor_result;
pub mod service_monitor_tag;
pub mod setting;
pub mod tag;
pub mod task;
pub mod task_run;
pub mod theme;
pub mod user;
pub mod vps;
pub mod vps_monthly_traffic;
pub mod vps_renewal_info;
pub mod vps_tag;
pub mod user_identity_provider;

// Prelude module for easy importing of all entities and their related types
pub mod prelude {
    pub use super::user::Model as UserModel;

    pub use super::vps::Model as VpsModel;

    pub use super::performance_metric::Model as PerformanceMetricModel;

    pub use super::docker_container::Model as DockerContainerModel;

    pub use super::docker_metric::Model as DockerMetricModel;

    pub use super::task::Model as TaskModel;

    pub use super::task_run::Model as TaskRunModel;

    pub use super::alert_rule::Model as AlertRuleModel;

    pub use super::alert_event::Model as AlertEventModel;

    pub use super::vps_monthly_traffic::Model as VpsMonthlyTrafficModel;

    pub use super::setting::Model as SettingModel;

    pub use super::tag::Model as TagModel;

    pub use super::vps_tag::Model as VpsTagModel;

    pub use super::notification_channel::Model as NotificationChannelModel;

    pub use super::alert_rule_channel::Model as AlertRuleChannelModel;

    pub use super::vps_renewal_info::Model as VpsRenewalInfoModel;

    pub use super::batch_command_task::Model as BatchCommandTaskModel;

    pub use super::child_command_task::Model as ChildCommandTaskModel;

    pub use super::command_script::Model as CommandScriptModel;

    pub use super::service_monitor::Model as ServiceMonitorModel;

    pub use super::service_monitor_agent::Model as ServiceMonitorAgentModel;

    pub use super::service_monitor_tag::Model as ServiceMonitorTagModel;

    pub use super::service_monitor_result::Model as ServiceMonitorResultModel;

    pub use super::oauth2_provider::Model as Oauth2ProviderModel;

    pub use super::user_identity_provider::Model as UserIdentityProviderModel;

}

// Optional: Keep direct re-exports if some parts of the code already use them,
// but prefer using the prelude for new code or during refactoring.
// ...
