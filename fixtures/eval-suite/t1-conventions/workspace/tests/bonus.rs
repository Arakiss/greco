use greco_t1_conventions_fixture::{CalcError, compute_bonus};

#[test]
fn bonus_when_points_are_positive() {
    assert_eq!(compute_bonus(12), Ok(24));
}

#[test]
fn bonus_when_points_are_too_large() {
    assert_eq!(compute_bonus(101), Err(CalcError::ScoreTooLarge));
}
