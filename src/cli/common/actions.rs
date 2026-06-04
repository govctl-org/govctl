use clap::{Args, ValueEnum};

#[derive(Args, Clone, Debug)]
pub(crate) struct EditActionArgs {
    /// Set a scalar value (omit VALUE only when using --stdin)
    #[arg(long, group = "edit_action", num_args = 0..=1)]
    pub(crate) set: Option<Option<String>>,
    /// Append a value to a list (omit VALUE only when using --stdin)
    #[arg(long, group = "edit_action", num_args = 0..=1)]
    pub(crate) add: Option<Option<String>>,
    /// Remove a matching value, or omit PATTERN when removing an indexed path
    #[arg(long, group = "edit_action", num_args = 0..=1)]
    pub(crate) remove: Option<Option<String>>,
    /// Update checklist-style item status
    #[arg(long, group = "edit_action")]
    pub(crate) tick: Option<TickStatus>,
    /// Read set/add value from stdin
    #[arg(long)]
    pub(crate) stdin: bool,
    /// Match by index for remove/tick
    #[arg(long, allow_hyphen_values = true)]
    pub(crate) at: Option<i32>,
    /// Exact match for remove/tick
    #[arg(long)]
    pub(crate) exact: bool,
    /// Regex match for remove/tick
    #[arg(long)]
    pub(crate) regex: bool,
    /// Remove all matches
    #[arg(long)]
    pub(crate) all: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum TickStatus {
    /// Mark work items as done
    Done,
    /// Mark work items as pending
    Pending,
    /// Mark work items as cancelled
    Cancelled,
    /// Mark ADR alternatives as accepted
    Accepted,
    /// Mark ADR alternatives as considered
    Considered,
    /// Mark ADR alternatives as rejected
    Rejected,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum WorkTickStatus {
    /// Mark work items as done
    Done,
    /// Mark work items as pending
    Pending,
    /// Mark work items as cancelled
    Cancelled,
}

impl From<WorkTickStatus> for TickStatus {
    fn from(value: WorkTickStatus) -> Self {
        match value {
            WorkTickStatus::Done => TickStatus::Done,
            WorkTickStatus::Pending => TickStatus::Pending,
            WorkTickStatus::Cancelled => TickStatus::Cancelled,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum AdrTickStatus {
    /// Mark ADR alternatives as accepted
    Accepted,
    /// Mark ADR alternatives as considered
    Considered,
    /// Mark ADR alternatives as rejected
    Rejected,
}

impl From<AdrTickStatus> for TickStatus {
    fn from(value: AdrTickStatus) -> Self {
        match value {
            AdrTickStatus::Accepted => TickStatus::Accepted,
            AdrTickStatus::Considered => TickStatus::Considered,
            AdrTickStatus::Rejected => TickStatus::Rejected,
        }
    }
}
