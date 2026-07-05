use greco_t3_edit_discipline_fixture::workflow::{TicketState, can_close, label};

#[test]
fn archived_state_has_own_label() {
    assert_eq!(label(TicketState::Archived), "archived");
}

#[test]
fn archived_state_can_close() {
    assert!(can_close(TicketState::Archived));
}

#[test]
fn active_state_stays_open() {
    assert!(!can_close(TicketState::Active));
}
