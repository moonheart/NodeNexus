//! SeaORM Entity Crate
//!
//! Defines the SeaORM entities that map to database tables.
//! Each entity is typically defined in its own module (e.g., `user.rs`, `vps.rs`).

// Declare entity modules here
pub mod user;
pub mod vps;
pub mod performance_metric;
pub mod docker_container;
pub mod docker_metric;
pub mod task;
pub mod task_run;
pub mod alert_rule;
pub mod alert_event;
pub mod vps_monthly_traffic;
pub mod performance_disk_usage;
pub mod performance_network_interface_stat;
pub mod setting;
pub mod tag;
pub mod vps_tag;
pub mod notification_channel;
pub mod alert_rule_channel;
pub mod vps_renewal_info;
// ... add other entity modules as they are created

// Prelude module for easy importing of all entities and their related types
pub mod prelude {
    pub use super::user::Entity as User;
    pub use super::user::Model as UserModel;
    pub use super::user::ActiveModel as UserActiveModel;
    pub use super::user::Column as UserColumn;

    pub use super::vps::Entity as Vps;
    pub use super::vps::Model as VpsModel;
    pub use super::vps::ActiveModel as VpsActiveModel;
    pub use super::vps::Column as VpsColumn;

    pub use super::performance_metric::Entity as PerformanceMetric;
    pub use super::performance_metric::Model as PerformanceMetricModel;
    pub use super::performance_metric::ActiveModel as PerformanceMetricActiveModel;
    pub use super::performance_metric::Column as PerformanceMetricColumn;

    pub use super::docker_container::Entity as DockerContainer;
    pub use super::docker_container::Model as DockerContainerModel;
    pub use super::docker_container::ActiveModel as DockerContainerActiveModel;
    pub use super::docker_container::Column as DockerContainerColumn;

    pub use super::docker_metric::Entity as DockerMetric;
    pub use super::docker_metric::Model as DockerMetricModel;
    pub use super::docker_metric::ActiveModel as DockerMetricActiveModel;
    pub use super::docker_metric::Column as DockerMetricColumn;

    pub use super::task::Entity as Task;
    pub use super::task::Model as TaskModel;
    pub use super::task::ActiveModel as TaskActiveModel;
    pub use super::task::Column as TaskColumn;

    pub use super::task_run::Entity as TaskRun;
    pub use super::task_run::Model as TaskRunModel;
    pub use super::task_run::ActiveModel as TaskRunActiveModel;
    pub use super::task_run::Column as TaskRunColumn;

    pub use super::alert_rule::Entity as AlertRule;
    pub use super::alert_rule::Model as AlertRuleModel;
    pub use super::alert_rule::ActiveModel as AlertRuleActiveModel;
    pub use super::alert_rule::Column as AlertRuleColumn;

    pub use super::alert_event::Entity as AlertEvent;
    pub use super::alert_event::Model as AlertEventModel;
    pub use super::alert_event::ActiveModel as AlertEventActiveModel;
    pub use super::alert_event::Column as AlertEventColumn;

    pub use super::vps_monthly_traffic::Entity as VpsMonthlyTraffic;
    pub use super::vps_monthly_traffic::Model as VpsMonthlyTrafficModel;
    pub use super::vps_monthly_traffic::ActiveModel as VpsMonthlyTrafficActiveModel;
    pub use super::vps_monthly_traffic::Column as VpsMonthlyTrafficColumn;

    pub use super::performance_disk_usage::Entity as PerformanceDiskUsage;
    pub use super::performance_disk_usage::Model as PerformanceDiskUsageModel;
    pub use super::performance_disk_usage::ActiveModel as PerformanceDiskUsageActiveModel;
    pub use super::performance_disk_usage::Column as PerformanceDiskUsageColumn;

    pub use super::performance_network_interface_stat::Entity as PerformanceNetworkInterfaceStat;
    pub use super::performance_network_interface_stat::Model as PerformanceNetworkInterfaceStatModel;
    pub use super::performance_network_interface_stat::ActiveModel as PerformanceNetworkInterfaceStatActiveModel;
    pub use super::performance_network_interface_stat::Column as PerformanceNetworkInterfaceStatColumn;

    pub use super::setting::Entity as Setting;
    pub use super::setting::Model as SettingModel;
    pub use super::setting::ActiveModel as SettingActiveModel;
    pub use super::setting::Column as SettingColumn;

    pub use super::tag::Entity as Tag;
    pub use super::tag::Model as TagModel;
    pub use super::tag::ActiveModel as TagActiveModel;
    pub use super::tag::Column as TagColumn;

    pub use super::vps_tag::Entity as VpsTag;
    pub use super::vps_tag::Model as VpsTagModel;
    pub use super::vps_tag::ActiveModel as VpsTagActiveModel;
    pub use super::vps_tag::Column as VpsTagColumn;

    pub use super::notification_channel::Entity as NotificationChannel;
    pub use super::notification_channel::Model as NotificationChannelModel;
    pub use super::notification_channel::ActiveModel as NotificationChannelActiveModel;
    pub use super::notification_channel::Column as NotificationChannelColumn;

    pub use super::alert_rule_channel::Entity as AlertRuleChannel;
    pub use super::alert_rule_channel::Model as AlertRuleChannelModel;
    pub use super::alert_rule_channel::ActiveModel as AlertRuleChannelActiveModel;
    pub use super::alert_rule_channel::Column as AlertRuleChannelColumn;
    
    pub use super::vps_renewal_info::Entity as VpsRenewalInfo;
    pub use super::vps_renewal_info::Model as VpsRenewalInfoModel;
    pub use super::vps_renewal_info::ActiveModel as VpsRenewalInfoActiveModel;
    pub use super::vps_renewal_info::Column as VpsRenewalInfoColumn;
}

// Optional: Keep direct re-exports if some parts of the code already use them,
// but prefer using the prelude for new code or during refactoring.
// pub use user::Entity as UserEnt; // Example of renaming to avoid conflict if needed
// ...