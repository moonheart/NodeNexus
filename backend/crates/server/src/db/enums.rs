use serde::{Deserialize, Serialize};
// Import ActiveEnum to use as_str()
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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


impl FromStr for BatchCommandStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PENDING" => Ok(BatchCommandStatus::Pending),
            "DISPATCHING" => Ok(BatchCommandStatus::Dispatching),
            "EXECUTING" => Ok(BatchCommandStatus::Executing),
            "COMPLETED_SUCCESSFULLY" => Ok(BatchCommandStatus::CompletedSuccessfully),
            "COMPLETED_WITH_ERRORS" => Ok(BatchCommandStatus::CompletedWithErrors),
            "TERMINATING" => Ok(BatchCommandStatus::Terminating),
            "TERMINATED" => Ok(BatchCommandStatus::Terminated),
            "FAILED_TO_DISPATCH" => Ok(BatchCommandStatus::FailedToDispatch),
            _ => Err(()),
        }
    }
}

impl fmt::Display for BatchCommandStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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


impl FromStr for ChildCommandStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "PENDING" => Ok(ChildCommandStatus::Pending),
            "SENT_TO_AGENT" => Ok(ChildCommandStatus::SentToAgent),
            "AGENT_ACCEPTED" => Ok(ChildCommandStatus::AgentAccepted),
            "EXECUTING" => Ok(ChildCommandStatus::Executing),
            "COMPLETED_SUCCESSFULLY" => Ok(ChildCommandStatus::CompletedSuccessfully),
            "COMPLETED_WITH_FAILURE" => Ok(ChildCommandStatus::CompletedWithFailure),
            "TERMINATING" => Ok(ChildCommandStatus::Terminating),
            "TERMINATED" => Ok(ChildCommandStatus::Terminated),
            "AGENT_UNREACHABLE" => Ok(ChildCommandStatus::AgentUnreachable),
            "TIMED_OUT" => Ok(ChildCommandStatus::TimedOut),
            "AGENT_ERROR" => Ok(ChildCommandStatus::AgentError),
            _ => Err(()),
        }
    }
}

impl fmt::Display for ChildCommandStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
