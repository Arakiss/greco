#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalcError {
    EmptyInput,
    ScoreTooLarge,
}

pub fn grade_score(points: u32) -> Result<&'static str, CalcError> {
    match points {
        0 => Err(CalcError::EmptyInput),
        1..=49 => Ok("bronze"),
        50..=89 => Ok("silver"),
        90..=100 => Ok("gold"),
        _ => Err(CalcError::ScoreTooLarge),
    }
}
