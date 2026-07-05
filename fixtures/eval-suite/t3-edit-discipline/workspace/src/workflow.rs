#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TicketState {
    New,
    Active,
    Done,
    Archived,
}

pub fn label(state: TicketState) -> &'static str {
    match state {
        TicketState::New => "new",
        TicketState::Active => "active",
        TicketState::Done => "done",
        TicketState::Archived => "done",
    }
}

pub fn can_close(state: TicketState) -> bool {
    matches!(state, TicketState::Done)
}
