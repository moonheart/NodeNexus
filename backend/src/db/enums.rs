use sea_orm::entity::prelude::*;
use serde::{Serialize, Deserialize};
 // Import ActiveEnum to use as_str()
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text", enum_name = "batch_command_status_enum")]
pub enum BatchCommandStatus {
    #[sea_orm(string_value = "PENDING")]
    Pending,
    #[sea_orm(string_value = "DISPATCHING")]
    Dispatching,
    #[sea_orm(string_value = "EXECUTING")]
    Executing,
    #[sea_orm(string_value = "COMPLETED_SUCCESSFULLY")]
    CompletedSuccessfully,
    #[sea_orm(string_value = "COMPLETED_WITH_ERRORS")]
    CompletedWithErrors,
    #[sea_orm(string_value = "TERMINATING")]
    Terminating,
    #[sea_orm(string_value = "TERMINATED")]
    Terminated,
    #[sea_orm(string_value = "FAILED_TO_DISPATCH")]
    FailedToDispatch,
}

impl fmt::Display for BatchCommandStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Text", enum_name = "child_command_status_enum")]
pub enum ChildCommandStatus {
    #[sea_orm(string_value = "PENDING")]
    Pending,
    #[sea_orm(string_value = "SENT_TO_AGENT")]
    SentToAgent,
    #[sea_orm(string_value = "AGENT_ACCEPTED")]
    AgentAccepted,
    #[sea_orm(string_value = "EXECUTING")]
    Executing,
    #[sea_orm(string_value = "COMPLETED_SUCCESSFULLY")]
    CompletedSuccessfully,
    #[sea_orm(string_value = "COMPLETED_WITH_FAILURE")]
    CompletedWithFailure,
    #[sea_orm(string_value = "TERMINATING")]
    Terminating,
    #[sea_orm(string_value = "TERMINATED")]
    Terminated,
    #[sea_orm(string_value = "AGENT_UNREACHABLE")]
    AgentUnreachable,
    #[sea_orm(string_value = "TIMED_OUT")]
    TimedOut,
    #[sea_orm(string_value = "AGENT_ERROR")]
    AgentError,
}

impl fmt::Display for ChildCommandStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}