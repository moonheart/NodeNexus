use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "PascalCase")]
#[strum(serialize_all = "PascalCase")]
pub enum BatchCommandStatus {
    Pending,
    Dispatching,
    Executing,
    CompletedSuccessfully,
    CompletedWithErrors,
    Terminating,
    Terminated,
    FailedToDispatch,
}

impl BatchCommandStatus {
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            BatchCommandStatus::CompletedSuccessfully
                | BatchCommandStatus::CompletedWithErrors
                | BatchCommandStatus::Terminated
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
#[serde(rename_all = "PascalCase")]
#[strum(serialize_all = "PascalCase")]
pub enum ChildCommandStatus {
    Pending,
    SentToAgent,
    AgentAccepted,
    Executing,
    CompletedSuccessfully,
    CompletedWithFailure,
    Terminating,
    Terminated,
    AgentUnreachable,
    TimedOut,
    AgentError,
}

impl ChildCommandStatus {
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            ChildCommandStatus::CompletedSuccessfully
                | ChildCommandStatus::CompletedWithFailure
                | ChildCommandStatus::Terminated
                | ChildCommandStatus::AgentUnreachable
                | ChildCommandStatus::TimedOut
                | ChildCommandStatus::AgentError
        )
    }
}
