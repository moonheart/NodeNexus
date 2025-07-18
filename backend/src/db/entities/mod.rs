//! SeaORM Entity Crate
//!
//! Defines the SeaORM entities that map to database tables.
//! Each entity is typically defined in its own module (e.g., `user.rs`, `vps.rs`).

// Declare entity modules here
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
pub mod user_identity_provider;
pub mod vps;
pub mod vps_monthly_traffic;
pub mod vps_renewal_info;
pub mod vps_tag;
// ... add other entity modules as they are created

// Prelude module for easy importing of all entities and their related types
pub mod prelude {
    pub use super::user::ActiveModel as UserActiveModel;
    pub use super::user::Column as UserColumn;
    pub use super::user::Entity as User;
    pub use super::user::Model as UserModel;

    pub use super::vps::ActiveModel as VpsActiveModel;
    pub use super::vps::Column as VpsColumn;
    pub use super::vps::Entity as Vps;
    pub use super::vps::Model as VpsModel;

    pub use super::performance_metric::ActiveModel as PerformanceMetricActiveModel;
    pub use super::performance_metric::Column as PerformanceMetricColumn;
    pub use super::performance_metric::Entity as PerformanceMetric;
    pub use super::performance_metric::Model as PerformanceMetricModel;

    pub use super::docker_container::ActiveModel as DockerContainerActiveModel;
    pub use super::docker_container::Column as DockerContainerColumn;
    pub use super::docker_container::Entity as DockerContainer;
    pub use super::docker_container::Model as DockerContainerModel;

    pub use super::docker_metric::ActiveModel as DockerMetricActiveModel;
    pub use super::docker_metric::Column as DockerMetricColumn;
    pub use super::docker_metric::Entity as DockerMetric;
    pub use super::docker_metric::Model as DockerMetricModel;

    pub use super::task::ActiveModel as TaskActiveModel;
    pub use super::task::Column as TaskColumn;
    pub use super::task::Entity as Task;
    pub use super::task::Model as TaskModel;

    pub use super::task_run::ActiveModel as TaskRunActiveModel;
    pub use super::task_run::Column as TaskRunColumn;
    pub use super::task_run::Entity as TaskRun;
    pub use super::task_run::Model as TaskRunModel;

    pub use super::alert_rule::ActiveModel as AlertRuleActiveModel;
    pub use super::alert_rule::Column as AlertRuleColumn;
    pub use super::alert_rule::Entity as AlertRule;
    pub use super::alert_rule::Model as AlertRuleModel;

    pub use super::alert_event::ActiveModel as AlertEventActiveModel;
    pub use super::alert_event::Column as AlertEventColumn;
    pub use super::alert_event::Entity as AlertEvent;
    pub use super::alert_event::Model as AlertEventModel;

    pub use super::vps_monthly_traffic::ActiveModel as VpsMonthlyTrafficActiveModel;
    pub use super::vps_monthly_traffic::Column as VpsMonthlyTrafficColumn;
    pub use super::vps_monthly_traffic::Entity as VpsMonthlyTraffic;
    pub use super::vps_monthly_traffic::Model as VpsMonthlyTrafficModel;

    pub use super::setting::ActiveModel as SettingActiveModel;
    pub use super::setting::Column as SettingColumn;
    pub use super::setting::Entity as Setting;
    pub use super::setting::Model as SettingModel;

    pub use super::tag::ActiveModel as TagActiveModel;
    pub use super::tag::Column as TagColumn;
    pub use super::tag::Entity as Tag;
    pub use super::tag::Model as TagModel;

    pub use super::vps_tag::ActiveModel as VpsTagActiveModel;
    pub use super::vps_tag::Column as VpsTagColumn;
    pub use super::vps_tag::Entity as VpsTag;
    pub use super::vps_tag::Model as VpsTagModel;

    pub use super::notification_channel::ActiveModel as NotificationChannelActiveModel;
    pub use super::notification_channel::Column as NotificationChannelColumn;
    pub use super::notification_channel::Entity as NotificationChannel;
    pub use super::notification_channel::Model as NotificationChannelModel;

    pub use super::alert_rule_channel::ActiveModel as AlertRuleChannelActiveModel;
    pub use super::alert_rule_channel::Column as AlertRuleChannelColumn;
    pub use super::alert_rule_channel::Entity as AlertRuleChannel;
    pub use super::alert_rule_channel::Model as AlertRuleChannelModel;

    pub use super::vps_renewal_info::ActiveModel as VpsRenewalInfoActiveModel;
    pub use super::vps_renewal_info::Column as VpsRenewalInfoColumn;
    pub use super::vps_renewal_info::Entity as VpsRenewalInfo;
    pub use super::vps_renewal_info::Model as VpsRenewalInfoModel;

    pub use super::batch_command_task::ActiveModel as BatchCommandTaskActiveModel;
    pub use super::batch_command_task::Column as BatchCommandTaskColumn;
    pub use super::batch_command_task::Entity as BatchCommandTask;
    pub use super::batch_command_task::Model as BatchCommandTaskModel;

    pub use super::child_command_task::ActiveModel as ChildCommandTaskActiveModel;
    pub use super::child_command_task::Column as ChildCommandTaskColumn;
    pub use super::child_command_task::Entity as ChildCommandTask;
    pub use super::child_command_task::Model as ChildCommandTaskModel;

    pub use super::command_script::ActiveModel as CommandScriptActiveModel;
    pub use super::command_script::Column as CommandScriptColumn;
    pub use super::command_script::Entity as CommandScript;
    pub use super::command_script::Model as CommandScriptModel;

    pub use super::service_monitor::ActiveModel as ServiceMonitorActiveModel;
    pub use super::service_monitor::Column as ServiceMonitorColumn;
    pub use super::service_monitor::Entity as ServiceMonitor;
    pub use super::service_monitor::Model as ServiceMonitorModel;

    pub use super::service_monitor_agent::ActiveModel as ServiceMonitorAgentActiveModel;
    pub use super::service_monitor_agent::Column as ServiceMonitorAgentColumn;
    pub use super::service_monitor_agent::Entity as ServiceMonitorAgent;
    pub use super::service_monitor_agent::Model as ServiceMonitorAgentModel;

    pub use super::service_monitor_tag::ActiveModel as ServiceMonitorTagActiveModel;
    pub use super::service_monitor_tag::Column as ServiceMonitorTagColumn;
    pub use super::service_monitor_tag::Entity as ServiceMonitorTag;
    pub use super::service_monitor_tag::Model as ServiceMonitorTagModel;

    pub use super::service_monitor_result::ActiveModel as ServiceMonitorResultActiveModel;
    pub use super::service_monitor_result::Column as ServiceMonitorResultColumn;
    pub use super::service_monitor_result::Entity as ServiceMonitorResult;
    pub use super::service_monitor_result::Model as ServiceMonitorResultModel;

    pub use super::oauth2_provider::ActiveModel as Oauth2ProviderActiveModel;
    pub use super::oauth2_provider::Column as Oauth2ProviderColumn;
    pub use super::oauth2_provider::Entity as Oauth2Provider;
    pub use super::oauth2_provider::Model as Oauth2ProviderModel;

    pub use super::user_identity_provider::ActiveModel as UserIdentityProviderActiveModel;
    pub use super::user_identity_provider::Column as UserIdentityProviderColumn;
    pub use super::user_identity_provider::Entity as UserIdentityProvider;
    pub use super::user_identity_provider::Model as UserIdentityProviderModel;
}

// Optional: Keep direct re-exports if some parts of the code already use them,
// but prefer using the prelude for new code or during refactoring.
// pub use user::Entity as UserEnt; // Example of renaming to avoid conflict if needed
// ...
